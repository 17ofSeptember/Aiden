#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use aiden_core::engine::{self, Handle};
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
#[tauri::command]
async fn request(
    state: tauri::State<'_, Handle>,
    operation: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let handle = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || handle.request(operation, data))
        .await
        .map_err(|e| e.to_string())?
}
fn main() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        app.state::<Handle>()
                            .emergency
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let logs = dir.join("logs");
            std::fs::create_dir_all(&logs)?;
            let writer = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("aiden")
                .max_log_files(7)
                .build(logs)?;
            tracing_subscriber::fmt()
                .json()
                .with_writer(writer)
                .try_init()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let handle = engine::start(&dir.join("aiden.db"))?;
            app.manage(handle);
            app.global_shortcut()
                .register("CommandOrControl+Shift+F12")?;
            tracing::info!("Aiden started; monitoring off");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![request])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<Handle>();
                state
                    .emergency
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                if let Err(e) = state.request("shutdown".into(), serde_json::Value::Null) {
                    tracing::error!(error=%e,"shutdown failed");
                }
            }
        }),
        Err(e) => {
            eprintln!("Aiden could not start: {e}");
            std::process::exit(1);
        }
    }
}
