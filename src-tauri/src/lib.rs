use std::sync::Mutex;
use tauri::{Emitter, Manager, RunEvent};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

// ポートをアプリ状態として保持（Reactがイベントを見逃した場合のフォールバック用）。
// あわせてバックエンドの CommandChild も持つ: フロントウィンドウが閉じた時に
// 明示的に kill しないと Go プロセスが残り続ける (Tauri の CommandChild の Drop
// では子プロセスを kill しない仕様)。
struct AppState {
    backend_port: Mutex<Option<u16>>,
    backend_child: Mutex<Option<CommandChild>>,
}

// Reactから直接ポートを取得するコマンド
#[tauri::command]
fn get_backend_port(state: tauri::State<AppState>) -> Option<u16> {
    *state.backend_port.lock().unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppState {
            backend_port: Mutex::new(None),
            backend_child: Mutex::new(None),
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("app_data_dir の取得に失敗しました");
            std::fs::create_dir_all(&data_dir)
                .expect("データディレクトリの作成に失敗しました");
            let db_path = data_dir.join("app.db");

            let sidecar_command = app_handle
                .shell()
                .sidecar("backend")
                .expect("バックエンドバイナリが見つかりません")
                .args(["--db", db_path.to_str().unwrap()]);

            let (mut rx, child) = sidecar_command
                .spawn()
                .expect("バックエンドの起動に失敗しました");

            // CommandChild を State に保管。Tauri の CommandChild は Drop で
            // 子プロセスを kill しない仕様なので、ウィンドウクローズ時に
            // 明示的に kill する必要がある (RunEvent::ExitRequested で実施)。
            if let Some(state) = app_handle.try_state::<AppState>() {
                *state.backend_child.lock().unwrap() = Some(child);
            }

            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            let text = String::from_utf8_lossy(&line);
                            if text.starts_with("PORT:") {
                                let port_str = text.trim_start_matches("PORT:").trim();
                                if let Ok(port) = port_str.parse::<u16>() {
                                    // 状態に保存（Reactがイベントを見逃してもコマンドで取得できる）
                                    if let Some(state) = app_handle.try_state::<AppState>() {
                                        *state.backend_port.lock().unwrap() = Some(port);
                                    }
                                    let _ = app_handle.emit("backend-ready", port);
                                }
                            }
                        }
                        CommandEvent::Stderr(line) => {
                            let text = String::from_utf8_lossy(&line);
                            eprintln!("[backend stderr] {}", text);
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_backend_port])
        .build(tauri::generate_context!())
        .expect("Tauriアプリの起動中にエラーが発生しました");

    // ウィンドウが全部閉じる / アプリ終了が要求された時に Go サイドカーを kill する。
    // RunEvent::ExitRequested は最後のウィンドウが閉じられた直後に発火し、
    // この時点なら state にアクセスして子プロセスをきれいに終了できる。
    // 取り出しは take() で行うので 2 重 kill は起きない (idempotent)。
    //
    // 注意: taskkill /F のような強制終了 (SIGKILL 相当) ではこのコールバックは
    // 呼ばれないため、Go 側の `monitorParentStdin()` で stdin EOF を見て
    // 自己終了する経路も併用する (backend/main.go 参照)。
    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { .. } = event {
            kill_sidecar(app_handle);
        }
    });
}

fn kill_sidecar(app_handle: &tauri::AppHandle) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        let child_opt = state.backend_child.lock().unwrap().take();
        if let Some(child) = child_opt {
            if let Err(e) = child.kill() {
                eprintln!("[shutdown] backend kill failed: {}", e);
            } else {
                eprintln!("[shutdown] backend killed");
            }
        }
    }
}
