use serde::Serialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tauri::{DragDropEvent, Emitter, Manager, RunEvent, WebviewEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

const MAX_IMPORT_SIZE: u64 = 5 << 20;

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
    dropped_files: Mutex<HashSet<PathBuf>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportedFile {
    name: String,
    path: String,
    content: String,
    size: u64,
}

// Reactから接続情報を取得する。起動イベントを見逃した場合のフォールバック用。
#[tauri::command]
fn get_backend_info(state: tauri::State<AppState>) -> Option<BackendInfo> {
    state.backend_info.lock().ok()?.clone()
}

#[tauri::command]
async fn pick_import_file(app: tauri::AppHandle) -> Result<Option<ImportedFile>, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("CSV / JSON", &["csv", "json"])
        .pick_file(move |selected| {
            let _ = sender.send(selected);
        });
    let selected = receiver
        .await
        .map_err(|_| "ファイル選択ダイアログが終了しました".to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    read_import_file(&path).map(Some)
}

#[tauri::command]
fn read_dropped_file(path: String, state: tauri::State<AppState>) -> Result<ImportedFile, String> {
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("ファイルを開けません: {error}"))?;
    let allowed = state
        .dropped_files
        .lock()
        .map_err(|_| "ドラッグ＆ドロップ状態を取得できません".to_string())?
        .remove(&canonical);
    if !allowed {
        return Err("ドラッグ＆ドロップで選択されたファイルではありません".into());
    }
    read_import_file(&canonical)
}

#[tauri::command]
async fn save_export_file(
    app: tauri::AppHandle,
    suggested_name: String,
    content: String,
) -> Result<Option<String>, String> {
    if content.len() as u64 > MAX_IMPORT_SIZE * 2 {
        return Err("保存データが大きすぎます".into());
    }
    let safe_name = Path::new(&suggested_name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("processed.json");
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name(safe_name)
        .add_filter("JSON", &["json"])
        .save_file(move |selected| {
            let _ = sender.send(selected);
        });
    let selected = receiver
        .await
        .map_err(|_| "保存ダイアログが終了しました".to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    std::fs::write(&path, content).map_err(|error| format!("保存に失敗しました: {error}"))?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
fn notify_if_hidden(app: tauri::AppHandle, title: String, body: String) -> Result<bool, String> {
    let hidden = app.get_webview_window("main").is_some_and(|window| {
        let visible = window.is_visible().unwrap_or(true);
        let minimized = window.is_minimized().unwrap_or(false);
        !visible || minimized
    });
    if hidden {
        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| error.to_string())?;
    }
    Ok(hidden)
}

fn read_import_file(path: &Path) -> Result<ImportedFile, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != "csv" && extension != "json" {
        return Err("CSVまたはJSONファイルを選択してください".into());
    }
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("ファイル情報を取得できません: {error}"))?;
    if !metadata.is_file() {
        return Err("通常のファイルを選択してください".into());
    }
    if metadata.len() > MAX_IMPORT_SIZE {
        return Err("ファイルサイズは5MB以下にしてください".into());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("UTF-8テキストとして読み込めません: {error}"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("data")
        .to_string();
    Ok(ImportedFile {
        name,
        path: path.to_string_lossy().into_owned(),
        content,
        size: metadata.len(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppState {
            backend_info: Mutex::new(None),
            backend_child: Mutex::new(None),
            dropped_files: Mutex::new(HashSet::new()),
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .on_webview_event(|webview, event| {
            if webview.label() != "main" {
                return;
            }
            if let WebviewEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                if let Ok(mut dropped_files) = webview.state::<AppState>().dropped_files.lock() {
                    dropped_files.clear();
                    dropped_files.extend(paths.iter().filter_map(|path| path.canonicalize().ok()));
                }
            }
        })
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
                show_main_window(app_handle.clone());
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
                                        show_main_window(app_handle.clone());
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
                            show_main_window(app_handle.clone());
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
                                show_main_window(app_handle.clone());
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
        .invoke_handler(tauri::generate_handler![
            get_backend_info,
            pick_import_file,
            read_dropped_file,
            save_export_file,
            notify_if_hidden
        ])
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

fn show_main_window(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        // 高速な環境でも起動状態が視認でき、ちらつかない程度だけ表示する。
        std::thread::sleep(Duration::from_millis(650));
        if let Some(splashscreen) = app_handle.get_webview_window("splashscreen") {
            let _ = splashscreen.close();
        }
        if let Some(main) = app_handle.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
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
