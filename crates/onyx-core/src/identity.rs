// ─── Void Identity ─────────────────────────────────────────────────
// Every Onyx device gets a persistent Ed25519 cryptographic identity.
// The SecretKey is generated on first launch and stored to disk.
// The PublicKey (= NodeId / EndpointId) is your "Void Address."
//
// Privacy: No IP addresses are shared. The public key is just a
// 32-byte identifier — completely disconnected from network topology.
// ────────────────────────────────────────────────────────────────────

use crate::error::{OnyxError, OnyxResult};
use iroh_base::{PublicKey, SecretKey};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// A persistent cryptographic identity for an Onyx device.
///
/// Wraps an Ed25519 `SecretKey` from iroh-base. The corresponding
/// `PublicKey` is the device's "Void Address" — the only identifier
/// that peers ever see.
#[derive(Clone)]
pub struct VoidIdentity {
    secret: SecretKey,
}

impl VoidIdentity {
    // ── Construction ─────────────────────────────────────────────

    /// Generate a brand-new identity using OS entropy.
    pub fn generate() -> Self {
        let secret = SecretKey::generate(&mut rand::rng());
        info!(
            public_key = %secret.public(),
            "generated new Void identity"
        );
        Self { secret }
    }

    /// Reconstruct from raw 32-byte secret key material.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            secret: SecretKey::from_bytes(bytes),
        }
    }

    /// Create the well-known relay identity.
    ///
    /// Both the relay binary and clients derive the same identity
    /// from a fixed seed so that clients can bootstrap gossip
    /// through the relay without prior configuration.
    pub fn relay_identity() -> Self {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"onyx-void-relay-bootstrap-v1");
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&hash);
        Self::from_bytes(&key_bytes)
    }

    // ── Accessors ────────────────────────────────────────────────

    /// The Ed25519 secret key (keep this safe!).
    #[inline]
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret
    }

    /// The public key / Void Address.
    #[inline]
    pub fn public_key(&self) -> PublicKey {
        self.secret.public()
    }

    /// Export the secret key as raw bytes (for persistence).
    #[inline]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes()
    }

    // ── Persistence ──────────────────────────────────────────────

    /// Default identity file path: `~/.onyx/identity.key`
    pub fn default_path() -> OnyxResult<PathBuf> {
        let home = dirs_path()?;
        Ok(home.join("identity.key"))
    }

    /// Identity file path for a named profile: `~/.onyx/<profile>/identity.key`
    ///
    /// Each profile gets its own sub-folder so multiple instances can
    /// run on the same machine with different NodeIDs.
    pub fn profile_path(profile: &str) -> OnyxResult<PathBuf> {
        let home = dirs_path()?;
        Ok(home.join(profile).join("identity.key"))
    }

    /// Load an identity from a file, or generate and save a new one.
    ///
    /// This is the primary entry point on app startup:
    /// ```ignore
    /// let id = VoidIdentity::load_or_create(None)?;
    /// ```
    pub fn load_or_create(path: Option<&Path>) -> OnyxResult<Self> {
        let path = match path {
            Some(p) => p.to_path_buf(),
            None => Self::default_path()?,
        };

        if path.exists() {
            debug!(path = %path.display(), "loading existing identity");
            Self::load(&path)
        } else {
            info!(path = %path.display(), "no identity found, generating new one");
            let id = Self::generate();
            id.save(&path)?;
            Ok(id)
        }
    }

    /// Load from a key file (32 raw bytes).
    pub fn load(path: &Path) -> OnyxResult<Self> {
        let bytes = std::fs::read(path)
            .map_err(|e| OnyxError::Identity(format!("failed to read key: {e}")))?;

        if bytes.len() != 32 {
            return Err(OnyxError::Identity(format!(
                "invalid key file: expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        let id = Self::from_bytes(&key_bytes);

        debug!(
            public_key = %id.public_key(),
            "loaded identity from disk"
        );
        Ok(id)
    }

    /// Persist to a key file (32 raw bytes, chmod 600 on unix).
    pub fn save(&self, path: &Path) -> OnyxResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OnyxError::Identity(format!("mkdir failed: {e}")))?;
        }

        let bytes = self.to_bytes();
        std::fs::write(path, bytes)
            .map_err(|e| OnyxError::Identity(format!("write key failed: {e}")))?;

        // On Unix, restrict permissions to owner-only
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)
                .map_err(|e| OnyxError::Identity(format!("chmod failed: {e}")))?;
        }

        info!(
            path = %path.display(),
            public_key = %self.public_key(),
            "saved identity to disk"
        );
        Ok(())
    }
}

impl std::fmt::Debug for VoidIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoidIdentity")
            .field("public_key", &self.public_key().to_string())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for VoidIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "void:{}", self.public_key())
    }
}

// ── Helpers ──────────────────────────────────────────────────────

/// Resolve the `~/.onyx/` directory.
fn dirs_path() -> OnyxResult<PathBuf> {
    // Use platform-appropriate home directory
    #[cfg(target_os = "windows")]
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .map_err(|_| OnyxError::Identity("cannot determine home directory".into()))?;

    #[cfg(not(target_os = "windows"))]
    let base = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| OnyxError::Identity("cannot determine home directory".into()))?;

    Ok(base.join(".onyx"))
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_roundtrip() {
        let id = VoidIdentity::generate();
        let bytes = id.to_bytes();
        let id2 = VoidIdentity::from_bytes(&bytes);
        assert_eq!(
            id.public_key().as_bytes(),
            id2.public_key().as_bytes(),
            "roundtrip must preserve identity"
        );
    }

    #[test]
    fn two_identities_differ() {
        let a = VoidIdentity::generate();
        let b = VoidIdentity::generate();
        assert_ne!(
            a.public_key().as_bytes(),
            b.public_key().as_bytes(),
            "two generated identities must differ"
        );
    }

    #[test]
    fn display_format() {
        let id = VoidIdentity::generate();
        let s = format!("{id}");
        assert!(s.starts_with("void:"), "display should start with void:");
    }
}
