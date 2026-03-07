// ─── Onyx Core — Snapshot Encryption (XChaCha20-Poly1305 + Argon2id) ──

use anyhow::Result;
use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::XChaCha20Poly1305;
use rand::RngCore;
use zeroize::Zeroizing;

/// Derive a 32-byte encryption key from a password and salt using Argon2id.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|e| anyhow::anyhow!("argon2 key derivation failed: {e}"))?;
    Ok(key)
}

/// Encrypt `data` with XChaCha20-Poly1305. Returns nonce (24 bytes) || ciphertext.
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());

    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = chacha20poly1305::XNonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    let mut out = Vec::with_capacity(24 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt data produced by `encrypt_data`. Expects nonce (24 bytes) || ciphertext.
pub fn decrypt_data(encrypted_data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if encrypted_data.len() < 24 {
        anyhow::bail!("encrypted data too short");
    }
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(24);
    let nonce = chacha20poly1305::XNonce::from_slice(nonce_bytes);
    let cipher = XChaCha20Poly1305::new(key.into());

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() -> Result<()> {
        let key = derive_key("test_password", b"some_salt_12345!")?;
        let plaintext = b"Hello, Onyx Void!";
        let encrypted = encrypt_data(plaintext, &*key)?;
        let decrypted = decrypt_data(&encrypted, &*key)?;
        assert_eq!(decrypted, plaintext);
        Ok(())
    }

    #[test]
    fn wrong_key_fails() -> Result<()> {
        let key1 = derive_key("password_a", b"some_salt_12345!")?;
        let key2 = derive_key("password_b", b"some_salt_12345!")?;
        let encrypted = encrypt_data(b"secret", &*key1)?;
        assert!(decrypt_data(&encrypted, &*key2).is_err());
        Ok(())
    }
}
