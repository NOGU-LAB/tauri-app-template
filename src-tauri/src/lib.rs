use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager, RunEvent};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

// 接続情報をアプリ状態として保持（Reactがイベントを見逃した場合のフォールバック用）。
// あわせてバックエンドの CommandChild も持つ: フロントウィンドウが閉じた時に
// 明示的に kill しないと Go プロセスが残り続ける (Tauri の CommandChild の Drop
// では子プロセスを kill しない仕様)。
#[derive(Clone, Serialize)]
struct BackendInfo {
    port: u16,
    token: String,
}

struct AppState {
    backend_info: Mutex<Option<BackendInfo>>,
    backend_child: Mutex<Option<CommandChild>>,
}

// Reactから接続情報を取得する。起動イベントを見逃した場合のフォールバック用。
#[tauri::command]
fn get_backend_info(state: tauri::State<AppState>) -> Option<BackendInfo> {
    state.backend_info.lock().ok()?.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppState {
            backend_info: Mutex::new(None),
            backend_child: Mutex::new(None),
        })
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Goをairで別起動するホットリロード用。releaseビルドでは必ず
            // 同梱サイドカーを使い、環境変数による接続先変更を許可しない。
            #[cfg(debug_assertions)]
            if let (Ok(port), Ok(token)) = (
                std::env::var("TAURI_EXTERNAL_BACKEND_PORT"),
                std::env::var("TAURI_EXTERNAL_BACKEND_TOKEN"),
            ) {
                let port = port.parse::<u16>()?;
                let info = BackendInfo { port, token };
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Ok(mut backend_info) = state.backend_info.lock() {
                        *backend_info = Some(info.clone());
                    }
                }
                let _ = app_handle.emit("backend-ready", info);
                return Ok(());
            }

            let data_dir = app_handle.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("app.db");

            let sidecar_command = app_handle
                .shell()
                .sidecar("backend")?
                .args(["--db", &db_path.to_string_lossy()]);

            let (mut rx, child) = sidecar_command.spawn()?;

            // CommandChild を State に保管。Tauri の CommandChild は Drop で
            // 子プロセスを kill しない仕様なので、ウィンドウクローズ時に
            // 明示的に kill する必要がある (RunEvent::ExitRequested で実施)。
            if let Some(state) = app_handle.try_state::<AppState>() {
                if let Ok(mut backend_child) = state.backend_child.lock() {
                    *backend_child = Some(child);
                }
            }

            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            let text = String::from_utf8_lossy(&line);
                            if let Some(payload) = text.strip_prefix("BACKEND_READY:") {
                                if let Some((port_str, token)) = payload.trim().split_once(':') {
                                    if let Ok(port) = port_str.parse::<u16>() {
                                        let info = BackendInfo {
                                            port,
                                            token: token.to_string(),
                                        };
                                        // 状態に保存（Reactがイベントを見逃してもコマンドで取得できる）
                                        if let Some(state) = app_handle.try_state::<AppState>() {
                                            if let Ok(mut backend_info) = state.backend_info.lock()
                                            {
                                                *backend_info = Some(info.clone());
                                            }
                                        }
                                        let _ = app_handle.emit("backend-ready", info);
                                    }
                                }
                            }
                        }
                        CommandEvent::Stderr(line) => {
                            let text = String::from_utf8_lossy(&line);
                            eprintln!("[backend stderr] {}", text);
                        }
                        CommandEvent::Error(message) => {
                            eprintln!("[backend error] {}", message);
                            let _ = app_handle.emit("backend-error", message);
                        }
                        CommandEvent::Terminated(payload) => {
                            let ready = app_handle
                                .try_state::<AppState>()
                                .and_then(|state| {
                                    state.backend_info.lock().ok().map(|info| info.is_some())
                                })
                                .unwrap_or(false);
                            if !ready {
                                let message = format!(
                                    "バックエンドが起動前に終了しました (code={:?}, signal={:?})",
                                    payload.code, payload.signal
                                );
                                eprintln!("[backend error] {}", message);
                                let _ = app_handle.emit("backend-error", message);
                            } else {
                                let _ = app_handle
                                    .emit("backend-error", "バックエンドとの接続が終了しました");
                            }
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_backend_info])
        .build(tauri::generate_context!())
        .expect("Tauriアプリの起動中にエラーが発生しました");

    // ウィンドウが全部閉じる / アプリ終了が要求された時に Go サイドカーを kill する。
    // 二段構えで保険を入れている:
    //   1. RunEvent::ExitRequested: × ボタンや Cmd+Q で発火する通常経路。ここで
    //      kill すれば、その後の Exit までに Go が止まっているのが理想。
    //   2. RunEvent::Exit: イベントループが本当に終わる直前の最終フック。
    //      何らかの理由 (プラグイン側で ExitRequested を prevent しているケース等)
    //      で 1 が走らなかった場合の最後の砦。
    // kill_sidecar は内部で take() するので、両方発火しても 2 重 kill にならない。
    //
    // 注意: taskkill /F のような強制終了 (SIGKILL 相当) では Rust 側のフックは
    // どれも呼ばれないため、Go 側の `monitorParentStdin()` で stdin EOF を見て
    // 自己終了する経路も併用する (backend/main.go 参照)。
    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { .. } | RunEvent::Exit => kill_sidecar(app_handle),
        _ => {}
    });
}

fn kill_sidecar(app_handle: &tauri::AppHandle) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        let child_opt = state
            .backend_child
            .lock()
            .ok()
            .and_then(|mut child| child.take());
        if let Some(child) = child_opt {
            if let Err(e) = child.kill() {
                eprintln!("[shutdown] backend kill failed: {}", e);
            } else {
                eprintln!("[shutdown] backend killed");
            }
        }
    }
}
