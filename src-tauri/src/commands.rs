use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use tauri::State;

#[derive(Serialize, FromRow)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub updated_at: String,
    pub pb_id: Option<String>,
    pub local_uuid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct NoteDetail {
    pub id: i64,
    pub title: String,
    pub content: Option<String>,
    pub updated_at: String,
    pub pb_id: Option<String>,
    pub local_uuid: Option<String>,
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
    let result = sqlx::query("INSERT INTO notes (title, content, local_uuid) VALUES ($1, $2, lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2,3) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2,3) || '-' || lower(hex(randomblob(6))))")
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
        "SELECT id, title, updated_at, pb_id, local_uuid FROM notes ORDER BY updated_at DESC, id DESC",
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
        "SELECT id, title, content, updated_at, pb_id, local_uuid FROM notes WHERE id = $1",
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
    local_uuid: Option<String>,
) -> Result<i64, String> {
    println!("Backend: import_note_from_pb: {}", title);

    // 1. Check for valid local_uuid if provided
    let uuid_query = if let Some(ref uuid) = local_uuid {
        Some(uuid.clone())
    } else {
        None
    };

    // 2. Check existence by PB_ID or UUID
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM notes WHERE pb_id = $1 OR (local_uuid IS NOT NULL AND local_uuid = $2)",
    )
    .bind(&pb_id)
    .bind(&uuid_query)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some((id,)) = existing {
        println!("Backend: Note already exists (id={}). Updating...", id);
        // Update existing note to match cloud state
        sqlx::query("UPDATE notes SET title = $1, content = $2, updated_at = $3, pb_id = $4, local_uuid = COALESCE(local_uuid, $5) WHERE id = $6")
            .bind(&title)
            .bind(&content)
            .bind(&updated_at)
            .bind(&pb_id)
            .bind(&local_uuid)
            .bind(id)
            .execute(&*pool)
            .await
            .map_err(|e| e.to_string())?;

        return Ok(id);
    }

    // 3. Insert New
    let result = sqlx::query(
        "INSERT INTO notes (title, content, updated_at, pb_id, local_uuid) VALUES ($1, $2, $3, $4, COALESCE($5, lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2,3) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2,3) || '-' || lower(hex(randomblob(6)))))",
    )
    .bind(title)
    .bind(content)
    .bind(updated_at)
    .bind(pb_id)
    .bind(local_uuid)
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

#[tauri::command]
pub async fn ensure_local_uuid(pool: State<'_, SqlitePool>, id: i64) -> Result<String, String> {
    // 1. Try to update if null (Generate UUID v4)
    sqlx::query("UPDATE notes SET local_uuid = (lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2,3) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2,3) || '-' || lower(hex(randomblob(6)))) WHERE id = $1 AND local_uuid IS NULL")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Fetch
    let row: (Option<String>,) = sqlx::query_as("SELECT local_uuid FROM notes WHERE id = $1")
        .bind(id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    match row.0 {
        Some(uuid) => Ok(uuid),
        None => Err("Failed to generate UUID".to_string()),
    }
}

/// Move a file to the system Recycle Bin (Windows) / Trash (macOS/Linux)
#[tauri::command]
pub fn move_to_trash(path: String) -> Result<(), String> {
    trash::delete(&path).map_err(|e| format!("Failed to move to trash: {}", e))
}
