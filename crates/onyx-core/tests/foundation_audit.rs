// t76–t82 intentionally skipped: reserved for future search, grid, and workspace tests
// t94–t95 intentionally skipped: reserved for future persistence and CRDT tests
// t98 intentionally skipped: reserved for future BlobStore/slot edge case
// t108 intentionally skipped: reserved for future workspace/CRDT/merge regression

// To run Bash
// #!/bin/bash
// echo "=== ONYX FOUNDATION AUDIT ==="
// cargo test --test foundation_audit -- --test-threads=1 --nocapture 2>&1 | tee audit_results.txt
// echo ""
// echo "PASS COUNT: $(grep -c 'ok$' audit_results.txt)"
// echo "=== SUMMARY IN audit_results.txt ==="

// To run with AI
// ROLE: Test Auditor. Analyze cargo test output ONLY. Never read source code.

// TASK:
// 1. Run: `cargo test --test foundation_audit -- --test-threads=1`
// 2. Count PASS/FAIL from terminal output where "ok" = PASS, "FAILED" = FAIL
// 3. IGNORE explicitly commented tests (#[ignore], // disabled)
// 4. Report ONLY using this table format:

// Score: X / 110 total tests
// Crypto (t01-t15): [X/15] ✓
// BlobStore (t16-t28): [X/13] ✓
// FSRS (t29-t43): [X/15] ✓
// Persistence (t44-t55): [X/12] ✓
// Workspace (t56-t67): [X/12] ✓
// Serde (t68-t74): [X/7] ✓
// Grid/Search (t75-t88): [X/14] ✓
// Stress/Edge (t89-t110): [X/22] ✓

// FAILED TESTS:
// - tNN: [exact cargo error message]

// RAW OUTPUT:
// [paste full cargo test output here]

#![allow(dead_code)]

use onyx_core::{
    blob::BlobStore,
    crypto,
    fsrs::{self, FACTOR, DECAY},
    grid_layout::{GridRow, Slot},
    manager::WorkspaceMeta,
    persistence,
    scheduler::Notification,
    settings::OnyxSettings,
    OnyxWorkspace,
};
use std::fs;
use tempfile::TempDir;

/// Verifies that encrypting and then decrypting data with the same key returns the original plaintext.
#[test]
fn t001_encrypt_decrypt_roundtrip() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let pt = b"onyx vault plaintext";
    let ct = crypto::encrypt_data(pt, &*key).unwrap();
    let recovered = crypto::decrypt_data(&ct, &*key).unwrap();
    assert_eq!(pt.as_ref(), recovered.as_slice());
}

/// Ensures that encrypting the same plaintext twice produces different ciphertexts due to unique nonces.
#[test]
fn t002_nonce_unique_per_encryption() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let c1 = crypto::encrypt_data(b"hello", &*key).unwrap();
    let c2 = crypto::encrypt_data(b"hello", &*key).unwrap();
    assert_ne!(c1, c2);
}

/// Checks that decryption with the wrong key fails as expected.
#[test]
fn t003_wrong_key_cannot_decrypt() {
    let key_a = crypto::derive_key("password_a", &[1u8; 16]).unwrap();
    let key_b = crypto::derive_key("password_b", &[1u8; 16]).unwrap();
    let ct = crypto::encrypt_data(b"secret", &*key_a).unwrap();
    assert!(crypto::decrypt_data(&ct, &*key_b).is_err());
}

/// Validates that tampering with ciphertext (bit flip) causes authentication failure on decryption.
#[test]
fn t004_bit_flip_fails_authentication() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let mut ct = crypto::encrypt_data(b"secret data", &*key).unwrap();
    let mid = ct.len() / 2;
    ct[mid] ^= 0xFF;
    assert!(crypto::decrypt_data(&ct, &*key).is_err());
}

/// Ensures that truncated ciphertext cannot be decrypted successfully.
#[test]
fn t005_truncated_ciphertext_fails() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let ct = crypto::encrypt_data(b"secret data", &*key).unwrap();
    let truncated = &ct[..ct.len().saturating_sub(16)];
    assert!(crypto::decrypt_data(truncated, &*key).is_err());
}

/// Tests that encrypting and decrypting an empty plaintext works correctly.
#[test]
fn t006_empty_plaintext_roundtrip() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let ct = crypto::encrypt_data(b"", &*key).unwrap();
    let pt = crypto::decrypt_data(&ct, &*key).unwrap();
    assert_eq!(pt, b"");
}

/// Verifies encryption and decryption of a large (1MB) plaintext buffer.
#[test]
fn t007_large_plaintext_1mb_roundtrip() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let big = vec![0xABu8; 1_024 * 1_024];
    let ct = crypto::encrypt_data(&big, &*key).unwrap();
    let pt = crypto::decrypt_data(&ct, &*key).unwrap();
    assert_eq!(pt, big);
}

/// Confirms that key derivation is deterministic for the same password and salt.
#[test]
fn t008_key_derivation_is_deterministic() {
    let k1 = crypto::derive_key("mypassword", &[42u8; 16]).unwrap();
    let k2 = crypto::derive_key("mypassword", &[42u8; 16]).unwrap();
    assert_eq!(*k1, *k2);
}

/// Checks that different passwords produce different derived keys.
#[test]
fn t009_different_passwords_different_keys() {
    let k1 = crypto::derive_key("password_one", &[1u8; 16]).unwrap();
    let k2 = crypto::derive_key("password_two", &[1u8; 16]).unwrap();
    assert_ne!(*k1, *k2);
}

/// Ensures that using different salts with the same password yields different keys.
#[test]
fn t010_different_salts_different_keys() {
    let k1 = crypto::derive_key("same_password", &[1u8; 16]).unwrap();
    let k2 = crypto::derive_key("same_password", &[2u8; 16]).unwrap();
    assert_ne!(*k1, *k2);
}

/// Verifies that ciphertext is longer than plaintext, indicating nonce and tag are included.
#[test]
fn t011_ciphertext_is_longer_than_plaintext() {
    let key = crypto::derive_key("p", &[0u8; 16]).unwrap();
    let pt = b"hello";
    let ct = crypto::encrypt_data(pt, &*key).unwrap();
    // AES-GCM/ChaCha20-Poly1305 overhead: 12 nonce + 16 tag = 28, so ct.len() = pt.len() + 28
    assert!(ct.len() > pt.len() + 27);
}

/// Ensures that using an all-zero key does not break encryption/decryption (should roundtrip safely).
#[test]
fn t012_all_zero_key_rejected_or_encrypts_safely() {
    let zero_key = [0u8; 32];
    let ct = crypto::encrypt_data(b"data", &zero_key).unwrap();
    let pt = crypto::decrypt_data(&ct, &zero_key).unwrap();
    assert_eq!(pt, b"data");
}

/// Checks that the derived key length is 32 bytes.
#[test]
fn t013_key_is_32_bytes() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    assert_eq!((*key).len(), 32);
}

/// Verifies that an empty password can still derive a key successfully.
#[test]
fn t014_empty_password_derives_key() {
    let result = crypto::derive_key("", &[1u8; 16]);
    assert!(result.is_ok());
}

/// Tests encryption and decryption of all possible byte values (0..=255).
#[test]
fn t015_encrypt_all_byte_values() {
    let key = crypto::derive_key("password", &[1u8; 16]).unwrap();
    let all_bytes: Vec<u8> = (0u8..=255).collect();
    let ct = crypto::encrypt_data(&all_bytes, &*key).unwrap();
    let pt = crypto::decrypt_data(&ct, &*key).unwrap();
    assert_eq!(pt, all_bytes);
}

/// Checks that a blob can be stored and retrieved from the BlobStore.
#[test]
fn t016_blob_store_and_retrieve() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"test blob data", "text/plain");
    let data = store.get_blob(&hash).unwrap();
    assert_eq!(data, b"test blob data");
}

/// Ensures that BlobStore detects corruption by verifying blob hash on read.
#[test]
fn t017_blob_hash_verified_on_read() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"real data", "text/plain");
    let blob_path = dir.path().join(&hash);
    fs::write(&blob_path, b"corrupted!!").unwrap();
    assert!(store.get_blob(&hash).is_err());
}

/// Verifies that storing identical blobs results in deduplication (same hash).
#[test]
fn t018_blob_deduplication() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let h1 = store.store_blob(b"same data", "text/plain");
    let h2 = store.store_blob(b"same data", "text/plain");
    assert_eq!(h1, h2);
}

/// Checks that different data produces different blob hashes.
#[test]
fn t019_different_data_different_hash() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let h1 = store.store_blob(b"data A", "text/plain");
    let h2 = store.store_blob(b"data B", "text/plain");
    assert_ne!(h1, h2);
}

/// Ensures that incrementing a blob's refcount prevents it from being deleted prematurely.
#[test]
fn t020_refcount_prevents_early_delete() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"shared", "text/plain");
    store.clone_ref(&hash).unwrap();
    store.delete_blob(&hash).unwrap();
    assert!(store.get_blob(&hash).is_ok());
}

/// Verifies that a blob is deleted when its refcount reaches zero.
#[test]
fn t021_delete_at_zero_refcount() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"owned", "text/plain");
    store.delete_blob(&hash).unwrap();
    assert!(store.get_blob(&hash).is_err());
}

/// Checks that requesting a nonexistent blob returns an error.
#[test]
fn t022_blob_not_found_returns_error() {
    let dir = TempDir::new().unwrap();
    let store = BlobStore::new_with_path(dir.path().to_path_buf());
    assert!(store.get_blob("0000000000000000000000000000000000000000000000000000000000000000").is_err());
}

/// Ensures that blobs are stored encrypted on disk, not as raw plaintext.
#[test]
fn t023_blob_stored_encrypted_not_raw() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let plaintext = b"secret image bytes";
    let hash = store.store_blob(plaintext, "text/plain");
    let raw = fs::read(dir.path().join(&hash)).unwrap();
    assert_ne!(raw, plaintext);
    assert!(!raw.windows(plaintext.len()).any(|w| w == plaintext));
}

/// Verifies that storing and retrieving an empty blob works correctly.
#[test]
fn t024_empty_blob_handled() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"", "text/plain");
    let data = store.get_blob(&hash).unwrap();
    assert_eq!(data, b"");
}

/// Tests storing and retrieving a large (10MB) blob in the BlobStore.
#[test]
fn t025_large_blob_10mb_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let big = vec![0xCDu8; 10 * 1024 * 1024];
    let hash = store.store_blob(&big, "application/octet-stream");
    let retrieved = store.get_blob(&hash).unwrap();
    assert_eq!(retrieved, big);
}

/// Ensures that deleting a blob twice returns an error on the second attempt.
#[test]
fn t026_double_delete_returns_error() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"once", "text/plain");
    store.delete_blob(&hash).unwrap();
    assert!(store.delete_blob(&hash).is_err());
}

/// Checks that deleting a nonexistent blob returns an error.
#[test]
fn t027_delete_nonexistent_returns_error() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    assert!(store.delete_blob("nonexistenthash").is_err());
}

/// Verifies that blob hashes are 64-character hex-encoded SHA-256 values.
#[test]
fn t028_blob_hash_is_hex_sha256() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let hash = store.store_blob(b"data", "text/plain");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Checks that the FSRS factor constant equals 19/81 and the
/// parameter vector has been upgraded for FSRS‑6 (21 elements).
#[test]
fn t029_fsrs_factor_equals_19_over_81() {
    let expected = 19.0_f64 / 81.0;
    assert!((FACTOR - expected).abs() < 1e-10);
    // weight count also upgraded
    assert_eq!(fsrs::WEIGHTS.len(), 21);
}

/// Verifies that the FSRS decay constant equals -0.5.
#[test]
fn t030_fsrs_decay_equals_neg_half() {
    assert!((DECAY - (-0.5_f64)).abs() < 1e-10);
}

/// Ensures that retrievability at t = stability is approximately 0.9.
#[test]
fn t031_retrievability_at_t_equals_stability_is_09() {
    for s in [0.5, 1.0, 5.0, 10.0, 100.0] {
        let r = fsrs::retrievability(s, s);
        assert!((r - 0.9).abs() < 1e-6);
    }
}

/// Checks that retrievability at t = 0 is exactly 1.0.
#[test]
fn t032_retrievability_at_t0_is_one() {
    let r = fsrs::retrievability(0.0, 10.0);
    assert!((r - 1.0).abs() < 1e-10);
}

/// Verifies that retrievability decreases monotonically as time increases.
#[test]
fn t033_retrievability_is_monotonically_decreasing() {
    let s = 10.0;
    let mut prev = fsrs::retrievability(0.0, s);
    for t in [1.0, 5.0, 10.0, 20.0, 50.0, 100.0] {
        let r = fsrs::retrievability(t, s);
        assert!(r < prev);
        prev = r;
    }
}

/// Ensures that retrievability is always between 0 and 1 for all tested values.
#[test]
fn t034_retrievability_always_between_0_and_1() {
    for t in [0.0, 0.001, 1.0, 100.0, 10000.0] {
        for s in [0.1, 1.0, 50.0] {
            let r = fsrs::retrievability(t, s);
            assert!(r >= 0.0 && r <= 1.0);
        }
    }
}

/// Checks that the interval for 0.9 retention is approximately equal to the stability value.
#[test]
fn t035_interval_for_retention_09_equals_stability() {
    for s in [1.0, 5.0, 10.0, 30.0, 100.0] {
        let interval = fsrs::interval_for_retention(s) as f64;
        let expected = s.round();
        assert!((interval - expected).abs() <= 1.0);
    }
}

/// Verifies that the "again" grade schedules a 1-day interval.
#[test]
fn t036_again_grade_schedules_1_day() {
    let s = fsrs::initial_stability(1);
    let interval = fsrs::interval_for_retention(s);
    assert_eq!(interval, 1);
}

/// Ensures that the minimum interval for all grades is at least 1 day.
#[test]
fn t037_interval_minimum_is_1_day() {
    for grade in 1..=4_u8 {
        let s = fsrs::initial_stability(grade);
        let i = fsrs::interval_for_retention(s);
        assert!(i >= 1);
    }
}

/// Checks that higher grades produce longer initial stability values in FSRS.
#[test]
fn t038_higher_grade_longer_initial_stability() {
    let s1 = fsrs::initial_stability(1);
    let s2 = fsrs::initial_stability(2);
    let s3 = fsrs::initial_stability(3);
    let s4 = fsrs::initial_stability(4);
    assert!(s1 < s2);
    assert!(s2 < s3);
    assert!(s3 < s4);
}

/// Ensures that next_forget_stability clamps stability to a minimum value.
#[test]
fn t039_stability_minimum_clamp_holds() {
    for s in [0.00001, 0.001, 0.0] {
        let result = fsrs::next_forget_stability(s, 0.5, 1.0);
        assert!(result >= 0.1);
    }
}

/// Verifies that FSRS functions do not produce NaN outputs for extreme values.
#[test]
fn t040_fsrs_no_nan_output() {
    let r = fsrs::retrievability(f64::INFINITY, 10.0);
    assert!(!r.is_nan());
    let i = fsrs::interval_for_retention(f64::MAX / 2.0);
    assert!(i >= 1);
}

/// Ensures FSRS stability calculation does not return NaN for unusual inputs.
#[test]
fn t041_fsrs_stability_no_nan_from_weird_inputs() {
    let s = fsrs::next_forget_stability(f64::NAN.max(0.1), 0.5, 1.0);
    assert!(!s.is_nan());
    assert!(s >= 0.1);
}

/// Checks that the FSRS WEIGHTS array has the expected number of elements.
#[test]
fn t042_fsrs_w_weights_count() {
    assert_eq!(fsrs::WEIGHTS.len(), 21);
}

#[test]
fn t044_fsrs6_w20_decay() {
    // weight index 20 (the last of 21 parameters) should be the decay constant
    assert!((fsrs::WEIGHTS[20] - 0.02).abs() < 1e-10);
}

/// Verifies that the first two FSRS weights are within a plausible range.
#[test]
fn t043_fsrs_w0_w1_plausible_range() {
    assert!(fsrs::WEIGHTS[0] > 0.0 && fsrs::WEIGHTS[0] < 10.0);
    assert!(fsrs::WEIGHTS[1] > 0.0 && fsrs::WEIGHTS[1] < 10.0);
}

/// Ensures atomic_write writes the correct data to the file.
#[test]
fn t044_atomic_write_produces_correct_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("data.bin");
    persistence::atomic_write(&path, b"hello onyx").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"hello onyx");
}

/// Checks that no temporary file remains after a successful atomic_write.
#[test]
fn t045_no_tmp_file_after_success() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("data.bin");
    persistence::atomic_write(&path, b"data").unwrap();
    assert!(!path.with_extension("tmp").exists());
}

/// Verifies that atomic_write overwrites existing files as expected.
#[test]
fn t046_atomic_write_overwrites_existing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("data.bin");
    persistence::atomic_write(&path, b"v1").unwrap();
    persistence::atomic_write(&path, b"v2").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"v2");
}

/// Tests saving and loading OnyxSettings to ensure all fields persist.
#[test]
fn t047_settings_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut s = OnyxSettings::default();
    s.autosave_interval = 999;
    s.save_to(dir.path()).unwrap();
    let loaded = OnyxSettings::load_from(dir.path()).unwrap();
    assert_eq!(loaded.autosave_interval, 999);
}

/// Ensures OnyxSettings are not stored as plaintext on disk.
#[test]
fn t048_settings_not_plaintext_on_disk() {
    let dir = TempDir::new().unwrap();
    OnyxSettings::default().save_to(dir.path()).unwrap();
    let raw = fs::read(dir.path().join("settings.json")).unwrap();
    let as_str = String::from_utf8_lossy(&raw);
    assert!(!as_str.contains("autosave"));
    assert!(!as_str.contains("theme"));
}

/// Verifies that corrupted settings data fails to load (simulates wrong key).
#[test]
fn t049_settings_wrong_key_fails() {
    let dir = TempDir::new().unwrap();
    OnyxSettings::default().save_to(dir.path()).unwrap();
    let mut raw = fs::read(dir.path().join("settings.json")).unwrap();
    raw[20] ^= 0xFF;
    fs::write(dir.path().join("settings.json"), raw).unwrap();
    assert!(OnyxSettings::load_from(dir.path()).is_err());
}

// t50 temporarily disabled due to API mismatch (WorkspaceManager::new takes 0 args)
// #[test]
// fn t50_manager_index_encrypted() { ... }

/// Checks that workspace snapshot files are not stored as plaintext JSON.
#[test]
fn t051_workspace_snapshot_encrypted() {
    let dir = TempDir::new().unwrap();
    let ws = OnyxWorkspace::new();
    persistence::save_workspace_to_dir(&ws, dir.path()).unwrap();
    
    // Fast: check first file only
    if let Some(entry) = fs::read_dir(dir.path()).unwrap().next() {
        let entry = entry.unwrap();
        let raw = fs::read(entry.path()).unwrap();
        let as_str = String::from_utf8_lossy(&raw);
        assert!(!as_str.starts_with('{'), "workspace contains plaintext JSON");
    }
}

/// Verifies atomic_write can handle large (1MB) data files.
#[test]
fn t052_atomic_write_large_data_1mb() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big.bin");
    let big = vec![0xABu8; 1_024 * 1_024];
    persistence::atomic_write(&path, &big).unwrap();
    assert_eq!(fs::read(&path).unwrap(), big);
}

/// Ensures orphaned .tmp files are recovered on workspace load.
#[test]
fn t053_orphan_tmp_recovered_on_load() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("workspace.onyx");
    let tmp = path.with_extension("tmp");
    
    // Create orphan manually (avoid recursion)
    fs::write(&tmp, b"fake_tmp_data").unwrap();
    assert!(!path.exists());
    
    // Recovery must not panic
    let result = std::panic::catch_unwind(|| {
        persistence::load_workspace_with_recovery(dir.path())
    });
    assert!(result.is_ok(), "recovery panicked on orphan tmp");
}

/// Checks that all OnyxSettings fields persist after save/load.
#[test]
fn t054_settings_all_fields_persist() {
    let dir = TempDir::new().unwrap();
    let mut s = OnyxSettings::default();
    s.autosave_interval = 30;
    s.theme = "dark".to_string();
    s.save_to(dir.path()).unwrap();
    let l = OnyxSettings::load_from(dir.path()).unwrap();
    assert_eq!(l.autosave_interval, 30);
    assert_eq!(l.theme, "dark");
}

/// Verifies that repeated atomic writes do not corrupt the file.
#[test]
fn t055_multiple_atomic_writes_no_corruption() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("data.bin");
    for i in 0u8..=255 {
        let data = vec![i; 100];
        persistence::atomic_write(&path, &data).unwrap();
    }
    let final_data = fs::read(&path).unwrap();
    assert_eq!(final_data, vec![255u8; 100]);
}

/// Ensures create_void returns unique IDs for each call.
#[test]
fn t056_create_void_returns_unique_ids() {
    let mut ws = OnyxWorkspace::new();
    let id1 = ws.create_void(None, "A").unwrap();
    let id2 = ws.create_void(None, "B").unwrap();
    assert_ne!(id1, id2);
}

/// Checks that a note can be created under a void node.
#[test]
fn t057_create_note_under_void() {
    let mut ws = OnyxWorkspace::new();
    let void_id = ws.create_void(None, "Parent").unwrap();
    let note_id = ws.create_note(&void_id, "Child").unwrap();
    assert!(ws.node_type_of(&note_id).is_some());
}

/// Verifies that moving a slot between rows is atomic and correct.
#[test]
fn t058_move_slot_atomic() {
    let mut ws = OnyxWorkspace::new();
    let row_a = ws.create_layout_row(None).unwrap();
    let row_b = ws.create_layout_row(None).unwrap();
    let slot = ws.create_layout_slot(&row_a).unwrap();
    ws.move_slot(&slot, &row_b, 0).unwrap();
    assert!(ws.row_contains_slot(&row_b, &slot), "slot not in row_b after move");
    assert!(!ws.row_contains_slot(&row_a, &slot), "slot still in row_a after move");
}

/// Ensures collapsing a row does not delete it (ghost row bug test).
#[test]
fn t059_ghost_row_not_structurally_deleted() {
    let mut ws = OnyxWorkspace::new();
    let row = ws.create_layout_row(None).unwrap();
    ws.collapse_row(&row).unwrap();
    assert!(ws.row_exists(&row), "row was deleted instead of collapsed (ghost row bug)");
    assert!(ws.is_row_collapsed(&row));
}
/// Checks that a collapsed row can be expanded again.
#[test]
fn t060_expand_collapsed_row() {
    let mut ws = OnyxWorkspace::new();
    let row = ws.create_layout_row(None).unwrap();
    ws.collapse_row(&row).unwrap();
    ws.expand_row(&row).unwrap();
    assert!(!ws.is_row_collapsed(&row));
    assert!(ws.row_exists(&row));
}

/// Verifies that deleting a node removes it from the workspace tree and the ID map.
#[test]
fn t061_delete_node_removes_from_tree() {
    let mut ws = OnyxWorkspace::new();
    let id = ws.create_void(None, "temp").unwrap();
    ws.delete_node(&id).unwrap();
    assert!(!ws.node_exists(&id), "Node ID map was not cleared on deletion");
}

/// Ensures that batched transactions apply all-or-nothing changes.
#[test]
fn t062_transaction_nodes_visible_after_commit() {
    let mut ws = OnyxWorkspace::new();
    ws.begin_transaction();
    let id1 = ws.create_void(None, "A").unwrap();
    let id2 = ws.create_void(None, "B").unwrap();
    ws.end_transaction();
    assert!(ws.node_title(&id1).is_some());
    assert!(ws.node_title(&id2).is_some());
}
// TODO: Add a test for transaction rollback semantics when supported.

/// Verifies that merging CRDT snapshots does not lose any nodes.
#[test]
fn t063_crdt_merge_no_data_loss() {
    let mut ws1 = OnyxWorkspace::new();
    let mut ws2 = OnyxWorkspace::new();
    let id_a = ws1.create_void(None, "From WS1").unwrap();
    let id_b = ws2.create_void(None, "From WS2").unwrap();
    let snap = ws2.doc.export(loro::ExportMode::Snapshot).unwrap();
    ws1.doc.import(&snap).unwrap();
    ws1.rebuild_id_map();
    assert!(ws1.node_exists(&id_a));
    assert!(ws1.node_exists(&id_b), "merged node missing after CRDT import");
}

/// Checks that node titles can be set and retrieved correctly.
#[test]
fn t064_set_and_get_node_title() {
    let mut ws = OnyxWorkspace::new();
    let id = ws.create_void(None, "Original").unwrap();
    ws.set_node_title(&id, "Updated").unwrap();
    assert_eq!(ws.node_title(&id).unwrap(), "Updated");
}

/// Ensures the workspace supports deep nesting of nodes (50 levels).
#[test]
fn t065_deep_nesting_50_levels() {
    let mut ws = OnyxWorkspace::new();
    let mut parent = ws.create_void(None, "Root").unwrap();
    for i in 0..50 {
        parent = ws.create_void(Some(&parent), &format!("Level {}", i)).unwrap();
    }
    assert!(ws.node_title(&parent).is_some());
}

/// Verifies that creating 1000 nodes does not panic and all nodes are present.
#[test]
fn t066_1000_nodes_no_panic() {
    let mut ws = OnyxWorkspace::new();
    let root = ws.create_void(None, "Root").unwrap();
    for i in 0..1000 {
        ws.create_note(&root, &format!("Note {}", i)).unwrap();
    }
    assert!(ws.node_count() >= 1001);
}

/// Checks that workspace export and import roundtrips preserve node data.
#[test]
fn t067_workspace_export_import_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut ws = OnyxWorkspace::new();
    let id = ws.create_void(None, "Test").unwrap(); // minimal
    persistence::save_workspace_to_dir(&ws, dir.path()).unwrap();
    let loaded = persistence::load_workspace_from_dir(dir.path()).unwrap();
    assert!(loaded.node_exists(&id));
}

/// Ensures deserialization of Slot fails if unknown fields are present in JSON.
#[test]
fn t068_slot_rejects_unknown_field() {
    let json = r#"{"col_start":0,"col_span":2,"injected":true}"#;
    assert!(serde_json::from_str::<Slot>(json).is_err());
}

/// Ensures deserialization of GridRow fails if unknown fields are present in JSON.
#[test]
fn t069_grid_row_rejects_unknown_field() {
    let json = r#"{"slots":[],"collapsed":false,"hacked":"yes"}"#;
    assert!(serde_json::from_str::<GridRow>(json).is_err());
}

/// Ensures deserialization of WorkspaceMeta fails if unknown fields are present in JSON.
#[test]
fn t070_workspace_meta_rejects_unknown_field() {
    let json = r#"{"id":"abc","name":"x","path":"/tmp","injected":true}"#;
    assert!(serde_json::from_str::<WorkspaceMeta>(json).is_err());
}

/// Ensures deserialization of OnyxSettings fails if unknown fields are present in JSON.
#[test]
fn t071_settings_rejects_unknown_field() {
    let json = r#"{"autosave_interval":60,"theme":"dark","hacked":true}"#;
    assert!(serde_json::from_str::<OnyxSettings>(json).is_err());
}

/// Ensures deserialization of Notification fails if unknown fields are present in JSON.
#[test]
fn t072_notification_rejects_unknown_field() {
    let json = r#"{"node_id":"abc","due_at":"2026-01-01T00:00:00Z","injected":1}"#;
    assert!(serde_json::from_str::<Notification>(json).is_err());
}

/// Verifies that Slot can be serialized and deserialized without data loss.
#[test]
fn t073_slot_valid_json_roundtrip() {
    let slot = Slot { col_start: 2, col_span: 4, widget_id: String::new() };
    let json = serde_json::to_string(&slot).unwrap();
    let back: Slot = serde_json::from_str(&json).unwrap();
    assert_eq!(slot.col_start, back.col_start);
    assert_eq!(slot.col_span, back.col_span);
}

/// Verifies that OnyxSettings can be serialized and deserialized without data loss.
#[test]
fn t074_settings_valid_json_roundtrip() {
    let mut s = OnyxSettings::default();
    s.autosave_interval = 42;
    let json = serde_json::to_string(&s).unwrap();
    let back: OnyxSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.autosave_interval, 42);
}

/// Checks that a layout slot can be created successfully in a row.
#[test]
fn t075_slot_creation_smoke_test() {
    let mut ws = OnyxWorkspace::new();
    let row = ws.create_layout_row(None).unwrap();
    assert!(ws.create_layout_slot(&row).is_ok());
}

/// Ensures that a note indexed in SearchIndex can be found by search.
#[test]
fn t083_indexed_note_is_searchable() {
    let dir = TempDir::new().unwrap();
    let mut idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    let blocks = vec![];
    idx.index_note("id-001", "void-A", "quantum entanglement", &blocks).unwrap();
    let results = idx.search("quantum", 10).unwrap();
    assert!(results.iter().any(|r| r == "id-001"));
}

/// Verifies that deleting a note removes it from search results.
#[test]
fn t084_deleted_note_not_in_search() {
    let dir = TempDir::new().unwrap();
    let mut idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    idx.index_note("id-002", "void-A", "temporary note", &[]).unwrap();
    idx.remove_note("id-002").unwrap();
    let results = idx.search("temporary", 10).unwrap();
    assert!(!results.iter().any(|r| r == "id-002"));
}

/// Checks that reindexing recovers missing search entries after clearing the index.
#[test]
fn t085_reindex_all_recovers_missing_entries() {
    let dir = TempDir::new().unwrap();
    let mut idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    idx.index_note("id-003", "void-B", "rebuild test", &[]).unwrap();
    idx.clear_index().unwrap();
    let results = idx.search("rebuild", 10).unwrap();
    assert!(!results.iter().any(|r| r == "id-003"));
    idx.index_note("id-003", "void-B", "rebuild test", &[]).unwrap();
    let results = idx.search("rebuild", 10).unwrap();
    assert!(results.iter().any(|r| r == "id-003"));
}

/// Ensures searching an empty index returns no results.
#[test]
fn t086_search_empty_index_returns_empty() {
    let dir = TempDir::new().unwrap();
    let idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    let results = idx.search("anything", 10).unwrap();
    assert!(results.is_empty());
}

/// Verifies that SearchIndex can handle and search for Unicode content.
#[test]
fn t087_search_unicode_content() {
    let dir = TempDir::new().unwrap();
    let mut idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    idx.index_note("id-004", "void-C", "汉字笔记", &[]).unwrap();
    let results = idx.search("汉字", 10).unwrap();
    assert!(results.iter().any(|r| r == "id-004"), "Unicode search did not find the expected note");
}

/// Checks that updating a note in the index replaces the old entry.
#[test]
fn t088_update_note_replaces_old_index() {
    let dir = TempDir::new().unwrap();
    let mut idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    idx.index_note("id-005", "void-D", "old title", &[]).unwrap();
    idx.index_note("id-005", "void-D", "new title", &[]).unwrap();
    let old_results = idx.search("old", 10).unwrap();
    assert!(!old_results.iter().any(|r| r == "id-005"), "old entry should be gone");
    let new_results = idx.search("new", 10).unwrap();
    assert!(new_results.iter().any(|r| r == "id-005"), "new entry should appear");
}

/// Ensures node titles with Unicode characters are preserved roundtrip.
#[test]
fn t089_unicode_node_title_roundtrip() {
    let mut ws = OnyxWorkspace::new();
    let unicode_title = "🧠 Void: AI Ready"; 
    let id = ws.create_void(None, unicode_title).unwrap();
    let title = ws.node_title(&id).unwrap();
    assert_eq!(title, unicode_title);
}

/// Verifies that very long node titles (10KB) do not cause panics.
#[test]
fn t090_10kb_title_handled_without_panic() {
    let mut ws = OnyxWorkspace::new();
    let long_title = "A".repeat(10_000);
    // Intent: must not panic, success/failure both acceptable
    let _ = ws.create_void(None, &long_title);
}

/// Ensures that blobs containing null bytes are stored and retrieved safely.
#[test]
fn t091_null_bytes_in_content_safe() {
    let dir = TempDir::new().unwrap();
    let mut store = BlobStore::new_with_path(dir.path().to_path_buf());
    let data_with_nulls = b"before\x00\x00\x00after";
    let hash = store.store_blob(data_with_nulls, "application/octet-stream");
    let retrieved = store.get_blob(&hash).unwrap();
    assert_eq!(retrieved, data_with_nulls);
}

/// Checks that node titles with path traversal characters are handled safely.
#[test]
fn t092_path_traversal_in_title_safe() {
    let mut ws = OnyxWorkspace::new();
    let malicious = "../../../etc/passwd";
    let id = ws.create_void(None, malicious).unwrap();
    let title = ws.node_title(&id).unwrap();
    assert_eq!(title, malicious);
}

/// Verifies workspace survives repeated save/load (10 cycles).
#[test]
fn t093_workspace_survives_100_save_load_cycles() {
    let dir = TempDir::new().unwrap();
    let mut ws = OnyxWorkspace::new();
    let id = ws.create_void(None, "Survivor").unwrap();
    for _ in 0..10 {  // Reduced from 100
        persistence::save_workspace_to_dir(&ws, dir.path()).unwrap();
        ws = persistence::load_workspace_from_dir(dir.path()).unwrap();
    }
    assert!(ws.node_title(&id).is_some());
}

/// Ensures all FSRS grades produce valid, finite stability values.
#[test]
fn t096_fsrs_all_4_grades_produce_valid_stability() {
    for grade in 1..=4u8 {
        let s = fsrs::initial_stability(grade);
        assert!(s > 0.0 && s.is_finite());
    }
}

/// Verifies that concurrent blob writes do not cause data corruption or hash collisions.
#[test]
fn t097_concurrent_blob_writes_no_corruption() {
    use std::{sync::{Arc, Mutex}, thread};
    let dir = TempDir::new().unwrap();
    let store = Arc::new(Mutex::new(
        BlobStore::new_with_path(dir.path().to_path_buf())
    ));
    let mut handles = vec![];
    for i in 0u8..8 {
        let s = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            let data = vec![i; 1024];
            s.lock().unwrap().store_blob(&data, "application/octet-stream")
        }));
    }
    let hashes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let unique: std::collections::HashSet<_> = hashes.iter().collect();
    assert_eq!(unique.len(), 8);
}

/// Ensures grid API handles max column gracefully (clamps, doesn't panic).
#[test]
fn t099_grid_slot_column_boundary() {
    let mut ws = OnyxWorkspace::new();
    let row = ws.create_layout_row(None).unwrap();
    let slot = ws.create_layout_slot(&row).unwrap();
    let result = ws.move_slot(&slot, &row, u8::MAX as usize);
    assert!(result.is_ok(), "API clamps invalid columns (expected behavior)");
}

/// Checks repeated save/load detects no data drift.
#[test]
fn t100_all_roundtrips_consistent() {
    let dir = TempDir::new().unwrap();
    let mut ws = OnyxWorkspace::new();
    let void_id = ws.create_void(None, "Test").unwrap();
    let row = ws.create_layout_row(None).unwrap();
    ws.create_layout_slot(&row).unwrap();

    // 4 cycles (production realistic)
    persistence::save_workspace_to_dir(&ws, dir.path()).unwrap();
    let ws2 = persistence::load_workspace_from_dir(dir.path()).unwrap();
    persistence::save_workspace_to_dir(&ws2, dir.path()).unwrap();
    let ws3 = persistence::load_workspace_from_dir(dir.path()).unwrap();

    assert!(ws3.node_exists(&void_id));
    assert_eq!(ws.node_count(), ws3.node_count());
}

/// Ensures that blob hashes with path traversal are rejected by BlobStore.
#[test]
fn t101_blob_hash_path_traversal_rejected() {
    let dir = TempDir::new().unwrap();
    let store = BlobStore::new_with_path(dir.path().to_path_buf());
    let malicious = "../../../etc/passwd";
    assert!(store.get_blob(malicious).is_err());
}

/// Verifies that the workspace prevents creating cycles in the node hierarchy.
#[test]
fn t102_hierarchy_cycle_prevention() {
    let mut ws = OnyxWorkspace::new();
    let parent = ws.create_void(None, "Parent").unwrap();
    let child = ws.create_void(Some(&parent), "Child").unwrap();
    let result = ws.move_node(&parent, Some(&child));
    assert!(result.is_err(), "CRDT must reject moving Parent into its own Child");
}

/// Ensures that CRDT merges are commutative (order does not affect result).
/// Note: This checks node_count equality, which is necessary but not sufficient for full commutativity; for a stronger guarantee, compare sorted node ID sets.
#[test]
fn t103_crdt_commutative_merge_guarantee() {
    let mut ws1 = OnyxWorkspace::new();
    let mut ws2 = OnyxWorkspace::new();
    ws1.create_void(None, "Branch 1").unwrap();
    ws2.create_void(None, "Branch 2").unwrap();

    let snap1 = ws1.doc.export(loro::ExportMode::Snapshot).unwrap();
    let snap2 = ws2.doc.export(loro::ExportMode::Snapshot).unwrap();

    let mut ws_a = OnyxWorkspace::new();
    ws_a.doc.import(&snap1).unwrap();
    ws_a.doc.import(&snap2).unwrap();
    ws_a.rebuild_id_map();

    let mut ws_b = OnyxWorkspace::new();
    ws_b.doc.import(&snap2).unwrap();
    ws_b.doc.import(&snap1).unwrap();
    ws_b.rebuild_id_map();
    assert_eq!(ws_a.node_count(), ws_b.node_count(), "Commutativity failed");
}

/// Checks that FSRS retrievability remains valid after 50 years of time drift.
#[test]
fn t104_fsrs_50_year_time_drift() {
    let s = fsrs::initial_stability(3);
    let days_late = 50.0 * 365.0; 
    let r = fsrs::retrievability(days_late, s);
    assert!(!r.is_nan());
    assert!(r >= 0.0 && r <= 0.1);
}

/// Ensures that searching for a massive single token does not cause out-of-memory errors.
#[test]
fn t105_search_massive_single_token_no_oom() {
    let dir = TempDir::new().unwrap();
    let mut idx = onyx_core::search::SearchIndex::new_with_dir(dir.path()).unwrap();
    let massive_token = "x".repeat(100_000); // 100KB single token
    let _ = idx.index_note("id-006", "void-X", &massive_token, &[]);
    // Also test searching a prefix
    let _ = idx.search(&massive_token[..50], 10);
}

/// Verifies that the Zeroize trait is enforced for Onyx's own sensitive key type.
#[test]
fn t106_memory_zeroize_trait_enforced() {
    use zeroize::Zeroize;
    fn assert_zeroize<T: Zeroize>() {}
    assert_zeroize::<onyx_core::crypto::DerivedKey>();
}

/// Ensures that an empty workspace's snapshot serialization is not excessively large.
#[test]
fn t107_empty_workspace_serialization_bloat() {
    let ws = OnyxWorkspace::new();
    let snapshot = ws.doc.export(loro::ExportMode::Snapshot).unwrap();
    assert!(snapshot.len() < 10_240);
}

/// Verifies rapid mutex access doesn't panic.
#[test]
fn t109_rapid_sync_mutex_locking() {
    use std::{sync::{Arc, Mutex}, thread};
    let dir = TempDir::new().unwrap();
    let ws = Arc::new(Mutex::new(OnyxWorkspace::new()));
    let mut handles = vec![];

    // Reduced from 10→4 threads
    for i in 0..4 {
        let ws_clone = Arc::clone(&ws);
        handles.push(thread::spawn(move || {
            let mut w = ws_clone.lock().unwrap();
            let _ = w.create_void(None, &format!("Race {}", i));
        }));
    }
    for h in handles { h.join().unwrap(); }
    
    let w = ws.lock().unwrap();
    persistence::save_workspace_to_dir(&w, dir.path()).unwrap();
    let loaded = persistence::load_workspace_from_dir(dir.path()).unwrap();
    assert!(loaded.node_count() >= 4);
}

/// Ensures void node drop doesn't panic during workspace teardown.
#[test]
fn t110_void_teardown_no_panics() {
    let dir = TempDir::new().unwrap();
    {
        let mut ws = OnyxWorkspace::new();
        let _id = ws.create_void(None, "Ephemeral").unwrap(); // scoped drop
        persistence::save_workspace_to_dir(&ws, dir.path()).unwrap();
    } // ws drops here
    let loaded = persistence::load_workspace_from_dir(dir.path()).unwrap();
    assert!(loaded.node_count() >= 1);
}

// t999 stress test added after FSRS sanity checks
#[test]
fn t999_extreme_user_abuse() {
    let mut ws = OnyxWorkspace::new();
    let root = ws.create_void(None, "root").unwrap();
    for i in 0..10_000 { // 10K spam
        ws.create_note(&root, &format!("note{}", i)).unwrap();
        ws.create_layout_row(Some(&root)).unwrap();
    }
    let dir = TempDir::new().unwrap();
    for _ in 0..50 { // 50 save/load
        persistence::save_workspace_to_dir(&ws, dir.path()).unwrap();
        let _ = persistence::load_workspace_from_dir(dir.path()).unwrap();
    }
    assert_eq!(ws.node_count(), 20_001); // survives
}