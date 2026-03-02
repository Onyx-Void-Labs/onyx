// ─── Onyx ID ───────────────────────────────────────────────────────
// 128-bit identifiers for every entity in the system.
// rkyv-serializable, zero-copy safe, aligned for both ARM64 and x86.
// ────────────────────────────────────────────────────────────────────

use rkyv::{Archive, Deserialize, Serialize};
use std::fmt;

/// A 128-bit unique identifier.
///
/// Stored as two u64s to guarantee 8-byte alignment on both ARM64 and
/// x86_64 without padding issues—critical for true zero-copy with rkyv.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash,
    Archive, Serialize, Deserialize,
    serde::Serialize, serde::Deserialize,
)]
#[repr(C)]              // Deterministic layout across architectures
pub struct OnyxId {
    hi: u64,
    lo: u64,
}

impl OnyxId {
    /// Generate a new random ID (v4-style, not RFC-compliant—just fast).
    pub fn new() -> Self {
        // Use a simple combination of address-space entropy + counter.
        // In production we will use a proper CSPRNG; for Phase 1 this
        // is sufficient and avoids pulling in `uuid`/`rand`.
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let ctr = COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            hi: ts,
            lo: ctr ^ 0xDEAD_BEEF_CAFE_BABE,
        }
    }

    /// Construct from raw parts (for deserialization).
    #[inline]
    pub const fn from_parts(hi: u64, lo: u64) -> Self {
        Self { hi, lo }
    }
}

impl Default for OnyxId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for OnyxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}{:016x}", self.hi, self.lo)
    }
}
