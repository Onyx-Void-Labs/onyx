// ─── Onyx Core — Persistence (Atomic Save / Load / Autosave) ────────

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::crypto;
use crate::document::OnyxWorkspace;

/// Dev-only encryption key. In production this would come from user input.
fn dev_encryption_key() -> Result<[u8; 32]> {
    crypto::derive_key("onyx_dev_key", b"fixed_salt_for_dev_1234")
}

/// Save the workspace's LoroDoc snapshot to `path` using atomic write.
/// The snapshot is encrypted with XChaCha20-Poly1305 before writing.
pub fn save_workspace(ws: &OnyxWorkspace, path: &str) -> Result<()> {
    let snapshot = ws
        .doc
        .export(loro::ExportMode::Snapshot)
        .context("export snapshot")?;
    let key = dev_encryption_key().context("derive dev key")?;
    let encrypted = crypto::encrypt_data(&snapshot, &key).context("encrypt snapshot")?;

    let dest = Path::new(path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).context("create parent directories")?;
    }

    let tmp_path = format!("{}.tmp", path);
    std::fs::write(&tmp_path, &encrypted).context("write tmp file")?;
    std::fs::rename(&tmp_path, dest).context("atomic rename")?;

    Ok(())
}

/// Load a workspace from an encrypted LoroDoc snapshot file.
pub fn load_workspace(path: &str) -> Result<OnyxWorkspace> {
    let encrypted = std::fs::read(path).context("read workspace file")?;
    let key = dev_encryption_key().context("derive dev key")?;
    let data = crypto::decrypt_data(&encrypted, &key).context("decrypt snapshot")?;
    let ws = OnyxWorkspace::from_snapshot(&data).context("load snapshot")?;
    Ok(ws)
}

/// Start a background autosave thread that periodically saves the workspace.
/// `interval` is the autosave period in seconds.
pub fn start_autosave(ws: Arc<Mutex<OnyxWorkspace>>, path: String, interval: u64) {
    if let Ok(handle) = thread::Builder::new()
        .name("onyx-autosave".into())
        .spawn(move || loop {
            thread::sleep(Duration::from_secs(interval));
            match ws.lock() {
                Ok(guard) => {
                    if let Err(e) = save_workspace(&guard, &path) {
                        tracing::error!("Autosave failed: {e:#}");
                    } else {
                        tracing::debug!("Autosave complete: {}", path);
                    }
                }
                Err(err) => {
                    tracing::error!("Autosave thread failed to lock workspace: {err}");
                }
            }
        })
    {
        // we successfully started the thread; nothing else to do
        let _ = handle;
    } else {
        tracing::error!("Failed to spawn autosave thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() -> anyhow::Result<()> {
        let mut ws = OnyxWorkspace::new();
        let _ = ws.create_void(None, "Test Void")?;

        let dir = std::env::temp_dir().join("onyx_persistence_test");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("test_workspace.onyx");
        let path_str = path.to_string_lossy().to_string();

        save_workspace(&ws, &path_str)?;
        let loaded = load_workspace(&path_str)?;

        let nodes = loaded.get_tree_nodes();
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].0.title, "Test Void");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
