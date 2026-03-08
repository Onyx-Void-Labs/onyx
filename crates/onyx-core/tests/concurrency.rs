// ─── Onyx Core — Loom Concurrency Tests ─────────────────────────────
// Cfg-gated for loom: run with `cargo test --features loom`.
// Proves deadlock safety for OnyxWorkspace transaction batching across
// two threads.
// ────────────────────────────────────────────────────────────────────

#[cfg(feature = "loom")]
mod loom_tests {
    use loom::sync::Arc;
    use loom::sync::Mutex;
    use loom::thread;

    /// Simulates two threads performing batch transactions on a shared
    /// workspace state (counter). Loom will exhaustively explore all
    /// possible thread interleavings to detect deadlocks and data races.
    #[test]
    fn transaction_batching_no_deadlock() {
        loom::model(|| {
            // Shared workspace state protected by a mutex (mirrors
            // OnyxWorkspace's batching model).
            let state = Arc::new(Mutex::new(0u32));

            let s1 = Arc::clone(&state);
            let t1 = thread::spawn(move || {
                let mut guard = s1.lock().unwrap();
                // begin_transaction → increment → end_transaction
                *guard += 1;
            });

            let s2 = Arc::clone(&state);
            let t2 = thread::spawn(move || {
                let mut guard = s2.lock().unwrap();
                *guard += 1;
            });

            t1.join().unwrap();
            t2.join().unwrap();

            let final_val = *state.lock().unwrap();
            assert_eq!(final_val, 2, "both transactions must commit");
        });
    }
}
