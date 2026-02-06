use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use tauri::State;

#[derive(Serialize, FromRow)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub updated_at: String,
    pub pb_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct NoteDetail {
    pub id: i64,
    pub title: String,
    pub content: Option<String>,
    pub updated_at: String,
    pub pb_id: Option<String>,
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
    println!("Backend: get_notes called");
    let notes = sqlx::query_as::<_, Note>(
        "SELECT id, title, updated_at, pb_id FROM notes ORDER BY updated_at DESC, id DESC",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    println!("Backend: Found {} notes", notes.len());
    Ok(notes)
}

#[tauri::command]
pub async fn get_note_content(
    id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Option<NoteDetail>, String> {
    let note = sqlx::query_as::<_, NoteDetail>(
        "SELECT id, title, content, updated_at, pb_id FROM notes WHERE id = $1",
    )
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
    content: String,
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
pub async fn update_note_pb_id(
    pool: State<'_, SqlitePool>,
    id: i64,
    pb_id: String,
) -> Result<(), String> {
    println!("Backend: update_note_pb_id: id={} pb_id={}", id, pb_id);
    sqlx::query("UPDATE notes SET pb_id = $1 WHERE id = $2")
        .bind(pb_id)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn import_note_from_pb(
    pool: State<'_, SqlitePool>,
    pb_id: String,
    title: String,
    content: String,
    updated_at: String,
) -> Result<i64, String> {
    println!("Backend: import_note_from_pb: {}", title);

    let result = sqlx::query(
        "INSERT INTO notes (title, content, updated_at, pb_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(title)
    .bind(content)
    .bind(updated_at)
    .bind(pb_id)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
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

#[tauri::command]
pub async fn delete_note_by_pb_id(
    pool: State<'_, SqlitePool>,
    pb_id: String,
) -> Result<(), String> {
    println!("Backend: delete_note_by_pb_id: {}", pb_id);
    sqlx::query("DELETE FROM notes WHERE pb_id = $1")
        .bind(pb_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
