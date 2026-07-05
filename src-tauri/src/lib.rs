/// Reports the headless core's version — a first, trivial bridge proving the UI
/// shell can call into the curve engine. Real drawing/fitting commands arrive
/// in Phase 2 (see docs/PLAN.md).
#[tauri::command]
fn engine_version() -> String {
    curve_engine::engine_version().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
