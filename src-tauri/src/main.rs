#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod import;
mod steam;
mod terminal;

use std::sync::Mutex;
use tauri::Manager;
use terminal::*;

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(Terminal::default()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let _ = cancel_download(window.state());
            }
        })
        .invoke_handler(tauri::generate_handler![
            import::import_download,
            engine_version,
            start_download,
            cancel_download,
            async_write_to_pty,
            async_resize_pty,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
