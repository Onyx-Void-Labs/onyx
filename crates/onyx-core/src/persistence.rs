// ─── Onyx Core — Persistence (Atomic Save / Load / Autosave) ────────

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use zeroize::Zeroizing;

use crate::crypto;
use crate::document::OnyxWorkspace;

/// Dev-only encryption key. In production this would come from user input.
pub(crate) fn dev_encryption_key() -> Result<Zeroizing<[u8; 32]>> {
    crypto::derive_key("onyx_dev_key", b"fixed_salt_for_dev_1234")
}

/// Internal helper: ensure that a leftover `.tmp` file is recovered if the
/// real file is missing.  Called by both save and load paths.
fn ensure_tmp_recovered(path: &str) {
    let tmp_path = format!("{}.tmp", path);
    if Path::new(&tmp_path).exists() && !Path::new(path).exists() {
        if let Err(e) = std::fs::rename(&tmp_path, path) {
            tracing::warn!("failed to recover orphan tmp file {}: {e}", tmp_path);
        } else {
            tracing::warn!("recovered orphan tmp file {}", tmp_path);
        }
    }
}

/// Write encrypted snapshot bytes atomically with fsync and cleanup.
fn save_snapshot_bytes(snapshot: &[u8], path: &str) -> Result<()> {
    let key = dev_encryption_key().context("derive dev key")?;
    let encrypted = crypto::encrypt_data(snapshot, &key).context("encrypt snapshot")?;

    let dest = Path::new(path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).context("create parent directories")?;
    }

    let tmp_path = format!("{}.tmp", path);
    let mut file = File::create(&tmp_path).context("create tmp file")?;
    if let Err(e) = file.write_all(&encrypted) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(anyhow::anyhow!("write tmp file: {e}"));
    }
    if let Err(e) = file.sync_all() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    if let Err(e) = std::fs::rename(&tmp_path, dest) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }

    Ok(())
}

/// Save the workspace's LoroDoc snapshot into a directory by writing
/// `workspace.onyx` inside `dir`.  The file is encrypted and written
/// atomically.
///
/// This is the primary public entrypoint; it exports a snapshot from the
/// workspace and then hands the bytes off to the internal helper that
/// performs encryption and atomic write.  Previously this function forwarded
/// to `save_workspace_to_dir` which in turn called back here, creating an
/// infinite recursion that hung tests and real code.
pub fn save_workspace(ws: &OnyxWorkspace, dir: &Path) -> Result<()> {
    // export the LoroDoc snapshot while holding the caller's lock
    let snapshot = ws
        .doc
        .export(loro::ExportMode::Snapshot)
        .context("export snapshot")?;

    // build the destination path string
    let path = dir.join("workspace.onyx").to_string_lossy().to_string();
    save_snapshot_bytes(&snapshot, &path)
}

/// Load a workspace from a directory containing `workspace.onyx`.
///
/// This function reads the encrypted file, decrypts it with the dev key, and
/// then hands the raw bytes to [`OnyxWorkspace::from_snapshot`] which rebuilds
/// the in‑memory state.  It does **not** perform crash recovery; callers such
/// as `load_workspace_with_recovery` are responsible for attempting recovery
/// before invoking this helper.
pub fn load_workspace(dir: &Path) -> Result<OnyxWorkspace> {
    let path = dir.join("workspace.onyx");
    let encrypted = std::fs::read(&path).context("read workspace file")?;
    let key = dev_encryption_key().context("derive dev key")?;
    let snapshot = crypto::decrypt_data(&encrypted, &key).context("decrypt workspace")?;
    OnyxWorkspace::from_snapshot(&snapshot)
}

/// Start a background autosave thread that periodically saves the workspace.
/// `interval` is the autosave period in seconds.  The mutex is held only long
/// enough to export a snapshot; encryption and I/O happen afterward.
pub fn start_autosave(ws: Arc<Mutex<OnyxWorkspace>>, path: String, interval: u64) {
    if let Ok(handle) = thread::Builder::new()
        .name("onyx-autosave".into())
        .spawn(move || loop {
            thread::sleep(Duration::from_secs(interval));
            // export snapshot under lock
            let snapshot_opt = match ws.lock() {
                Ok(guard) => match guard.doc.export(loro::ExportMode::Snapshot) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        tracing::error!("Autosave export failed: {e:#}");
                        None
                    }
                },
                Err(err) => {
                    tracing::error!("Autosave thread failed to lock workspace: {err}");
                    None
                }
            };
            if let Some(snapshot) = snapshot_opt {
                if let Err(e) = save_snapshot_bytes(&snapshot, &path) {
                    tracing::error!("Autosave failed: {e:#}");
                } else {
                    tracing::debug!("Autosave complete: {}", path);
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

/// Write raw bytes atomically: write to `.tmp`, fsync, then rename.
/// No encryption is applied — the caller controls the content.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create parent directories")?;
    }
    let tmp_path = path.with_extension("tmp");
    let mut file = File::create(&tmp_path).context("create tmp file")?;
    if let Err(e) = file.write_all(data) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(anyhow::anyhow!("write tmp file: {e}"));
    }
    if let Err(e) = file.sync_all() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(())
}

/// Save workspace snapshot to a specific `.tmp` file (for crash-recovery tests).
pub fn save_workspace_to_tmp(ws: &OnyxWorkspace, tmp_path: &Path) -> Result<()> {
    let snapshot = ws
        .doc
        .export(loro::ExportMode::Snapshot)
        .context("export snapshot")?;
    let key = dev_encryption_key().context("derive dev key")?;
    let encrypted = crypto::encrypt_data(&snapshot, &key).context("encrypt snapshot")?;
    if let Some(parent) = tmp_path.parent() {
        std::fs::create_dir_all(parent).context("create parent directories")?;
    }
    let mut file = File::create(tmp_path).context("create tmp file")?;
    file.write_all(&encrypted).context("write tmp file")?;
    file.sync_all().context("fsync tmp file")?;
    Ok(())
}

/// Load workspace with crash-recovery: if the main file is missing but a
/// `.tmp` sibling exists, recover from the tmp file first.
pub fn load_workspace_with_recovery(dir: &Path) -> Result<OnyxWorkspace> {
    let path = dir.join("workspace.onyx");
    let path_str = path.to_string_lossy().to_string();
    ensure_tmp_recovered(&path_str);
    // now load normally
    load_workspace(dir)
}

/// Convenience wrapper: save workspace into a directory as `workspace.onyx`.
///
/// This helper exists for callers who already think in terms of a directory
/// rather than a full path; it delegates directly to [`save_workspace`].
pub fn save_workspace_to_dir(ws: &OnyxWorkspace, dir: &Path) -> Result<()> {
    save_workspace(ws, dir)
}

/// Convenience wrapper: load workspace from a directory's `workspace.onyx`.
pub fn load_workspace_from_dir(dir: &Path) -> Result<OnyxWorkspace> {
    load_workspace(dir)
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

        save_workspace(&ws, dir.as_path())?;
        let loaded = load_workspace(dir.as_path())?;

        let nodes = loaded.get_tree_nodes();
        assert!(!nodes.is_empty());
        // genesis void may appear first; look for our test void among titles.
        let titles: Vec<_> = nodes.iter().map(|(n, _)| n.title.clone()).collect();
        assert!(titles.iter().any(|t| t == "Test Void"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
