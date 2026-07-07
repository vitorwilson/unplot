mod approximation;
mod calculus_commands;
mod curve_commands;
mod dto;
mod file_commands;
mod latex_commands;

use calculus_commands::apply_calculus;
use curve_commands::{extend_curve, fit_curve, refit_curve};
use file_commands::{open_curve, save_curve};
use latex_commands::curve_latex;

/// Reports the headless core's version — the first bridge from the UI shell into
/// the curve engine.
#[tauri::command]
fn engine_version() -> String {
    curve_engine::engine_version().to_string()
}

/// The application's version, for the About dialog. Read from the app crate at
/// compile time, so it always matches the released bundle.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
        .invoke_handler(tauri::generate_handler![
            engine_version,
            app_version,
            fit_curve,
            extend_curve,
            refit_curve,
            curve_latex,
            apply_calculus,
            save_curve,
            open_curve
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_version_reads_the_crate_version() {
        // The About dialog shows this; it must be a real version string.
        let version = super::app_version();
        assert!(
            version.split('.').count() >= 2,
            "expected a semver-like version, got {version:?}"
        );
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }
}
