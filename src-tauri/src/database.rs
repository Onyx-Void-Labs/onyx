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
        println!("Checking 'notes' table...");
        if let Err(e) = sqlx::query(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                pb_id TEXT,
                local_uuid TEXT UNIQUE
            )",
        )
        .execute(&pool)
        .await
        {
            eprintln!("CRITICAL ERROR: Failed to create tables: {}", e);
            panic!("Database setup failed: {}", e);
        }

        // Migration: Add pb_id if missing
        let table_info: Vec<(i64, String, String, i64, Option<String>, i64)> =
            sqlx::query_as("PRAGMA table_info('notes')")
                .fetch_all(&pool)
                .await
                .unwrap_or_default();

        let has_pb_id = table_info.iter().any(|c| c.1 == "pb_id");
        let has_uuid = table_info.iter().any(|c| c.1 == "local_uuid");

        if !has_pb_id {
            println!("Applying migration: Adding pb_id column...");
            let _ = sqlx::query("ALTER TABLE notes ADD COLUMN pb_id TEXT")
                .execute(&pool)
                .await;
        }

        if !has_uuid {
            println!("Applying migration: Adding local_uuid column...");
            let _ = sqlx::query("ALTER TABLE notes ADD COLUMN local_uuid TEXT")
                .execute(&pool)
                .await;
            // Generate UUIDs for existing notes
            let _ = sqlx::query("UPDATE notes SET local_uuid = (lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2,3) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2,3) || '-' || lower(hex(randomblob(6)))) WHERE local_uuid IS NULL")
                .execute(&pool)
                .await;
        }

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
