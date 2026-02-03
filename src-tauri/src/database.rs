use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use std::fs;
use tauri::Manager;

// CONSTANTS:
// The name of our database file.
const DB_NAME: &str = "onyx.db";

// 1. THE BLUEPRINT
pub struct Database;

// 2. THE BEHAVIOR
impl Database {
    pub async fn get_db_path(app_handle: &tauri::AppHandle) -> String {
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).expect("failed to create app data dir");
        }

        let path = app_dir.join(DB_NAME);
        path.to_str().unwrap().to_string()
    }

    pub async fn setup(app_handle: &tauri::AppHandle) -> SqlitePool {
        let path = Self::get_db_path(app_handle).await;
        let db_url = format!("sqlite:{}", path);

        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url).await.unwrap();
        }

        let pool = SqlitePool::connect(&db_url).await.unwrap();

        // WAL Mode
        sqlx::query("PRAGMA journal_mode=WAL;")
            .execute(&pool)
            .await
            .unwrap();

        // Table Creation
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Trigger Creation
        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS update_note_timestamp 
             AFTER UPDATE ON notes
             BEGIN
                UPDATE notes SET updated_at = CURRENT_TIMESTAMP WHERE id = old.id;
             END;",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }
}
