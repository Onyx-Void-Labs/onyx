use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use tauri::State;

#[derive(Serialize, FromRow)]
pub struct Note {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct NoteDetail {
    pub id: i64,
    pub title: String,
    pub content: Option<String>, // BACK TO STRING: Stop Rust from parsing it
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Welcome to ONYX, Operator {}!", name)
}

#[tauri::command]
pub async fn create_note(
    pool: State<'_, SqlitePool>,
    title: String,
    content: String,
) -> Result<i64, String> {
    let result = sqlx::query("INSERT INTO notes (title, content) VALUES ($1, $2)")
        .bind(title)
        .bind(content)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn get_notes(pool: State<'_, SqlitePool>) -> Result<Vec<Note>, String> {
    let notes =
        sqlx::query_as::<_, Note>("SELECT id, title FROM notes ORDER BY updated_at DESC, id DESC")
            .fetch_all(&*pool)
            .await
            .map_err(|e| e.to_string())?;
    Ok(notes)
}

#[tauri::command]
pub async fn get_note_content(
    id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Option<NoteDetail>, String> {
    let note =
        sqlx::query_as::<_, NoteDetail>("SELECT id, title, content FROM notes WHERE id = $1")
            .bind(id)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| e.to_string())?;
    Ok(note)
}

#[tauri::command]
pub async fn update_note(
    pool: State<'_, SqlitePool>,
    id: i64,
    title: String,
    content: String, // FRONTEND WILL SEND STRINGIFIED JSON
) -> Result<(), String> {
    sqlx::query("UPDATE notes SET title = $1, content = $2 WHERE id = $3")
        .bind(title)
        .bind(content)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_note(pool: State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
