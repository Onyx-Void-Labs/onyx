// ─── Onyx Core — Workspace Manager ──────────────────────────────────

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::document::OnyxWorkspace;
use crate::persistence;

/// Metadata for a workspace entry in the index.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceMeta {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_opened: DateTime<Utc>,
}

/// Manages multiple workspaces: creates, lists, and tracks the active one.
pub struct WorkspaceManager {
    workspaces: HashMap<Uuid, WorkspaceMeta>,
    pub active_id: Option<Uuid>,
}

fn index_path() -> PathBuf {
    dirs_home().join("workspaces.json")
}

/// Derive the key used to encrypt the workspace index file.  
/// In production this should be user‑supplied.
fn index_encryption_key() -> anyhow::Result<[u8; 32]> {
    crate::crypto::derive_key("onyx_index_key", b"fixed_salt_for_dev_1234")
}

fn dirs_home() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".onyx")
}

impl WorkspaceManager {
    /// Load the workspace index from `~/.onyx/workspaces.json`, or start empty.
    pub fn new() -> Self {
        let mut mgr = Self {
            workspaces: HashMap::new(),
            active_id: None,
        };
        mgr.load_index();
        mgr
    }

    /// Create a new workspace file and register it in the index.
    pub fn create_workspace(&mut self, name: &str, path: &PathBuf) -> Result<Uuid> {
        let ws = OnyxWorkspace::new();
        let path_str = path.to_string_lossy().to_string();
        persistence::save_workspace(&ws, &path_str).context("save new workspace")?;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let meta = WorkspaceMeta {
            id,
            name: name.to_string(),
            path: path.clone(),
            created_at: now,
            last_opened: now,
        };
        self.workspaces.insert(id, meta);
        self.active_id = Some(id);
        self.save_index();
        Ok(id)
    }

    /// List all registered workspaces.
    pub fn list_workspaces(&self) -> Vec<WorkspaceMeta> {
        self.workspaces.values().cloned().collect()
    }

    /// Open (activate) a workspace by ID and return the loaded workspace.
    pub fn open_workspace(&mut self, id: Uuid) -> Result<OnyxWorkspace> {
        let meta = self
            .workspaces
            .get_mut(&id)
            .context("workspace not found")?;
        meta.last_opened = Utc::now();
        self.active_id = Some(id);
        let path_str = meta.path.to_string_lossy().to_string();
        self.save_index();
        persistence::load_workspace(&path_str)
    }

    /// Remove a workspace from the index (does not delete the file).
    pub fn remove_workspace(&mut self, id: Uuid) {
        self.workspaces.remove(&id);
        if self.active_id == Some(id) {
            self.active_id = None;
        }
        self.save_index();
    }

    fn load_index(&mut self) {
        let path = index_path();
        if let Ok(encrypted) = std::fs::read(&path) {
            if let Ok(key) = index_encryption_key() {
                if let Ok(decrypted) = crate::crypto::decrypt_data(&encrypted, &key) {
                    if let Ok(map) =
                        serde_json::from_slice::<HashMap<Uuid, WorkspaceMeta>>(&decrypted)
                    {
                        self.workspaces = map;
                    }
                }
            }
        }
    }

    fn save_index(&self) {
        let path = index_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.workspaces) {
            if let Ok(key) = index_encryption_key() {
                if let Ok(encrypted) = crate::crypto::encrypt_data(json.as_bytes(), &key) {
                    let _ = std::fs::write(&path, encrypted);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list_workspaces() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join("onyx_manager_test");
        std::fs::create_dir_all(&dir)?;
        let ws_path = dir.join("test.onyx");

        let mut mgr = WorkspaceManager {
            workspaces: HashMap::new(),
            active_id: None,
        };

        let id = mgr.create_workspace("Test", &ws_path)?;
        let list = mgr.list_workspaces();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Test");
        assert_eq!(mgr.active_id, Some(id));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
