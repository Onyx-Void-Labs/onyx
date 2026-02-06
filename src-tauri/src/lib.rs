mod commands;
mod database; // Tell Rust to look for commands.rs

use database::Database;
use tauri::Manager;

// We "use" everything from the commands module so the generate_handler can see them
use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                let db_pool = Database::setup(app.handle()).await;
                app.manage(db_pool);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            create_note,
            get_notes,
            get_note_content,
            update_note,
            update_note_pb_id,
            import_note_from_pb,
            delete_note,
            delete_note_by_pb_id
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
