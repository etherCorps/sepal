// The Tracker on Tauri 2: compare/PLAN.md. The app is examples/tracker, built
// for the DOM by ../vite.config.js.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

/// A bench line from the app (compare/shared/PROTOCOL.md), to stderr.
#[tauri::command]
fn mark(line: String) {
    eprintln!("{line}");
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // Sepal's fs creates missing parents on write; plugin-fs does not.
            std::fs::create_dir_all(app.path().app_data_dir()?)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![mark])
        .run(tauri::generate_context!())
        .expect("error while running the Tracker");
}
