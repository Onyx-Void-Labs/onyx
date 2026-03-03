// ─── The Vault ─────────────────────────────────────────────────────
// Encryption at rest for Onyx's local storage.
//
// Uses XChaCha20-Poly1305 (AEAD) with Argon2id key derivation.
// When unlocked, every byte written to disk is encrypted.
// Without the key, the database file is indistinguishable from
// random noise.
//
// Key Derivation:
//   password (or biometric hash) → Argon2id → 32-byte symmetric key
//                                  (with random 16-byte salt)
//
// Encryption:
//   plaintext → XChaCha20-Poly1305(key, random 24-byte nonce)
//   Wire format: [16B salt] [24B nonce] [ciphertext + 16B auth tag]
//
// Gated behind the `vault` feature flag.
// ────────────────────────────────────────────────────────────────────

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use argon2::Argon2;
use rand::Rng;
use tracing::info;

/// Salt length for Argon2 key derivation (16 bytes).
const SALT_LEN: usize = 16;

/// Nonce length for XChaCha20-Poly1305 (24 bytes).
const NONCE_LEN: usize = 24;

/// Derived key length (32 bytes for XChaCha20).
const KEY_LEN: usize = 32;

/// Encrypted storage vault.
///
/// Derive from a user password (or biometric hash), then use
/// `encrypt()` / `decrypt()` for all disk-bound data.
pub struct Vault {
    cipher: XChaCha20Poly1305,
    /// The salt used for key derivation (stored alongside encrypted data).
    salt: [u8; SALT_LEN],
}

impl Vault {
    /// Create a new Vault from a password.
    ///
    /// Derives a 256-bit key using Argon2id with a random salt.
    /// Store the salt alongside encrypted data (it's not secret).
    pub fn from_password(password: &str) -> Result<Self, VaultError> {
        let mut salt = [0u8; SALT_LEN];
        rand::rng().fill(&mut salt[..]);

        Self::from_password_and_salt(password, salt)
    }

    /// Recreate a Vault from a password and a previously-used salt.
    ///
    /// Use this when decrypting existing data — the salt is stored
    /// in the encrypted file header.
    pub fn from_password_and_salt(
        password: &str,
        salt: [u8; SALT_LEN],
    ) -> Result<Self, VaultError> {
        let key = derive_key(password, &salt)?;
        let cipher = XChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| VaultError::KeyDerivation("invalid key length".into()))?;

        info!("vault unlocked (XChaCha20-Poly1305 + Argon2id)");

        Ok(Self { cipher, salt })
    }

    /// Encrypt plaintext into a self-contained blob.
    ///
    /// Output format:
    ///   [16B salt] [24B nonce] [ciphertext + 16B AEAD tag]
    ///
    /// The salt is included so decryption only needs the password.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rng().fill(&mut nonce_bytes[..]);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| VaultError::Encryption)?;

        // Assemble: salt + nonce + ciphertext
        let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&self.salt);
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        Ok(blob)
    }

    /// Decrypt a blob produced by `encrypt()`.
    ///
    /// Extracts the nonce from the blob header, then decrypts.
    /// The salt is also in the header (used by `open()` for key derivation).
    pub fn decrypt(&self, blob: &[u8]) -> Result<Vec<u8>, VaultError> {
        let min_len = SALT_LEN + NONCE_LEN + 16; // 16 = auth tag
        if blob.len() < min_len {
            return Err(VaultError::Decryption(
                "ciphertext too short".into(),
            ));
        }

        let nonce = XNonce::from_slice(&blob[SALT_LEN..SALT_LEN + NONCE_LEN]);
        let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| VaultError::Decryption(
                "decryption failed — wrong password or corrupted data".into(),
            ))
    }

    /// Open an encrypted blob with a password.
    ///
    /// Reads the salt from the blob header, derives the key, and decrypts.
    /// This is the "one-shot" API: `Vault::open(password, blob)`.
    pub fn open(password: &str, blob: &[u8]) -> Result<Vec<u8>, VaultError> {
        if blob.len() < SALT_LEN {
            return Err(VaultError::Decryption("blob too short for salt".into()));
        }

        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&blob[..SALT_LEN]);

        let vault = Self::from_password_and_salt(password, salt)?;
        vault.decrypt(blob)
    }

    /// Get the salt (for persisting alongside encrypted data).
    pub fn salt(&self) -> &[u8; SALT_LEN] {
        &self.salt
    }
}

/// Derive a 256-bit key from a password using Argon2id.
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], VaultError> {
    let argon2 = Argon2::default(); // Argon2id with safe defaults
    let mut key = [0u8; KEY_LEN];

    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| VaultError::KeyDerivation(e.to_string()))?;

    Ok(key)
}

/// Vault-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("encryption failed")]
    Encryption,

    #[error("decryption failed: {0}")]
    Decryption(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let vault = Vault::from_password("test-password-123").unwrap();
        let plaintext = b"Hello, Onyx Void!";

        let encrypted = vault.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);
        assert!(encrypted.len() > plaintext.len());

        let decrypted = vault.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn open_one_shot() {
        let vault = Vault::from_password("my-secret").unwrap();
        let encrypted = vault.encrypt(b"sensitive data").unwrap();

        let decrypted = Vault::open("my-secret", &encrypted).unwrap();
        assert_eq!(decrypted, b"sensitive data");
    }

    #[test]
    fn wrong_password_fails() {
        let vault = Vault::from_password("correct").unwrap();
        let encrypted = vault.encrypt(b"secret").unwrap();

        let result = Vault::open("wrong", &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn different_encryptions_differ() {
        let vault = Vault::from_password("password").unwrap();
        let plaintext = b"same input";

        let a = vault.encrypt(plaintext).unwrap();
        let b = vault.encrypt(plaintext).unwrap();

        // Different nonces → different ciphertexts (semantic security).
        assert_ne!(a, b);

        // Both decrypt to the same plaintext.
        assert_eq!(vault.decrypt(&a).unwrap(), plaintext);
        assert_eq!(vault.decrypt(&b).unwrap(), plaintext);
    }
}
