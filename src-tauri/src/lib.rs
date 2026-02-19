mod commands;
mod database;
mod email; // Tell Rust to look for commands.rs

use database::Database;
use tauri::Manager;

// We "use" everything from the commands module so the generate_handler can see them
use commands::*;
use email::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_oauth::init())
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                let db_pool = Database::setup(app.handle()).await;
                app.manage(db_pool);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            send_magic_link_email,
            send_otp_email,
            check_relay_health,
            create_note,
            get_notes,
            get_note_content,
            update_note,
            update_note_pb_id,
            import_note_from_pb,
            delete_note,
            delete_note_by_pb_id,
            ensure_local_uuid,
            move_to_trash
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
