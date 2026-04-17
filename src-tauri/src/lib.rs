use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

// ポートをアプリ状態として保持（Reactがイベントを見逃した場合のフォールバック用）
struct AppState {
    backend_port: Mutex<Option<u16>>,
}

// Reactから直接ポートを取得するコマンド
#[tauri::command]
fn get_backend_port(state: tauri::State<AppState>) -> Option<u16> {
    *state.backend_port.lock().unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { backend_port: Mutex::new(None) })
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

            let (mut rx, _child) = sidecar_command
                .spawn()
                .expect("バックエンドの起動に失敗しました");

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
        .run(tauri::generate_context!())
        .expect("Tauriアプリの起動中にエラーが発生しました");
}
