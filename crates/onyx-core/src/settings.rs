// ─── Onyx Core — Settings (User Preferences) ───────────────────────

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Global application settings, persisted to `~/.onyx/settings.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnyxSettings {
    /// UI theme name (e.g. "dark", "light").
    pub theme: String,
    /// Autosave interval in seconds.
    pub autosave_interval: u64,
    /// Maximum flashcards to review per session.
    pub flashcard_limit: u32,
}

impl Default for OnyxSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            autosave_interval: 60,
            flashcard_limit: 50,
        }
    }
}

fn settings_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".onyx").join("settings.json")
}

/// Derive encryption key for settings file (dev-only).
use zeroize::Zeroizing;

fn settings_encryption_key() -> anyhow::Result<Zeroizing<[u8; 32]>> {
    crate::crypto::derive_key("onyx_settings_key", b"fixed_salt_for_dev_1234")
}

impl OnyxSettings {
    /// Load settings from `~/.onyx/settings.json`, or return defaults.
    pub fn load() -> Self {
        let path = settings_path();
        if let Ok(encrypted) = std::fs::read(&path) {
            if let Ok(key) = settings_encryption_key() {
                if let Ok(data) = crate::crypto::decrypt_data(&encrypted, &key) {
                    if let Ok(s) = serde_json::from_slice(&data) {
                        return s;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save settings to `~/.onyx/settings.json`.
    pub fn save(&self) -> Result<()> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create ~/.onyx directory")?;
        }
        let json = serde_json::to_string_pretty(self).context("serialize settings")?;
        // derive an encryption key and write encrypted data; fail loudly on any error
        let key = settings_encryption_key().context("derive settings encryption key")?;
        let encrypted =
            crate::crypto::encrypt_data(json.as_bytes(), &*key).context("encrypt settings")?;
        // Atomic write: tmp → fsync → rename
        let tmp_path = path.with_extension("tmp");
        let mut file =
            std::fs::File::create(&tmp_path).context("create settings tmp file")?;
        if let Err(e) = std::io::Write::write_all(&mut file, &encrypted) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e).context("write settings tmp file");
        }
        if let Err(e) = file.sync_all() {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e).context("fsync settings tmp file");
        }
        if let Err(e) = std::fs::rename(&tmp_path, &path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e).context("atomic rename settings file");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let s = OnyxSettings::default();
        assert_eq!(s.theme, "dark");
        assert_eq!(s.autosave_interval, 60);
        assert_eq!(s.flashcard_limit, 50);
    }

    #[test]
    fn round_trip_json() -> anyhow::Result<()> {
        let s = OnyxSettings {
            theme: "custom".to_string(),
            autosave_interval: 120,
            flashcard_limit: 25,
        };
        let json = serde_json::to_string(&s)?;
        let loaded: OnyxSettings = serde_json::from_str(&json)?;
        assert_eq!(loaded.theme, "custom");
        assert_eq!(loaded.autosave_interval, 120);
        assert_eq!(loaded.flashcard_limit, 25);
        Ok(())
    }
}
