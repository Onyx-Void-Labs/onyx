// ─── Sync Engine ───────────────────────────────────────────────────
// Bridges Loro CRDTs ↔ Iroh network.
//
// The sync loop:
//   1. User types a character → EditorBuffer + CrdtDoc updated
//   2. CrdtDoc exports the delta (updates since last sync)
//   3. ZSTD compresses the delta (~50 bytes for a keystroke)
//   4. ShadowMesh broadcasts to all peers via gossip
//   5. Incoming deltas from peers are decompressed and merged
//
// Optimistic UI (Dead Reckoning):
//   Local changes render at 144Hz IMMEDIATELY.
//   We never wait for network confirmation.
//   Loro's CRDT semantics guarantee eventual consistency.
// ────────────────────────────────────────────────────────────────────

use onyx_core::error::OnyxResult;
use onyx_store::CrdtDoc;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, trace, warn};

use crate::mesh::MeshEvent;
use iroh_gossip::api::GossipSender;

/// Compression level for CRDT deltas.
/// Level 1 = fastest, good enough for tiny keystroke deltas.
const ZSTD_LEVEL: i32 = 1;

/// The sync engine coordinates CRDT delta exchange over the mesh.
///
/// It holds a reference to the shared `CrdtDoc` and manages the
/// bidirectional flow of deltas.
pub struct SyncEngine {
    /// Shared CRDT document (thread-safe via Mutex).
    doc: Arc<Mutex<SyncState>>,
    /// Channel to send outbound deltas.
    outbound_tx: mpsc::Sender<Vec<u8>>,
}

/// Internal sync state for a single document.
struct SyncState {
    /// The Loro CRDT document.
    crdt: CrdtDoc,
}

impl SyncEngine {
    /// Create a new SyncEngine and spawn the background sync tasks.
    ///
    /// Returns the engine and a handle to the outbound task.
    pub fn spawn(
        crdt: CrdtDoc,
        gossip_sender: GossipSender,
        mut incoming_rx: mpsc::Receiver<MeshEvent>,
    ) -> Self {
        let state = Arc::new(Mutex::new(SyncState { crdt }));

        // Channel for outbound deltas (editor → network)
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(128);

        // ── Outbound task: compress and broadcast deltas ──
        let sender = gossip_sender;
        tokio::spawn(async move {
            while let Some(raw_delta) = outbound_rx.recv().await {
                match zstd::encode_all(raw_delta.as_slice(), ZSTD_LEVEL) {
                    Ok(compressed) => {
                        trace!(
                            raw = raw_delta.len(),
                            compressed = compressed.len(),
                            "outbound delta compressed"
                        );
                        if let Err(e) = sender.broadcast(compressed.into()).await {
                            warn!(%e, "failed to broadcast delta");
                        }
                    }
                    Err(e) => {
                        error!(%e, "ZSTD compression failed");
                    }
                }
            }
            debug!("outbound sync task finished");
        });

        // ── Inbound task: decompress and merge incoming deltas ──
        let inbound_state = Arc::clone(&state);
        tokio::spawn(async move {
            while let Some(event) = incoming_rx.recv().await {
                let delta = match event {
                    MeshEvent::Delta(d) => d,
                    MeshEvent::PeerJoined(peer) => {
                        debug!(peer = %peer, "sync engine: peer joined");
                        continue;
                    }
                    MeshEvent::PeerLeft(peer) => {
                        debug!(peer = %peer, "sync engine: peer left");
                        continue;
                    }
                    MeshEvent::StreamEnded => {
                        warn!("sync engine: gossip stream ended");
                        break;
                    }
                };
                trace!(
                    from = %delta.from,
                    bytes = delta.data.len(),
                    "received delta from peer"
                );

                // Decompress
                let raw = match zstd::decode_all(delta.data.as_slice()) {
                    Ok(raw) => raw,
                    Err(e) => {
                        warn!(%e, "failed to decompress incoming delta");
                        continue;
                    }
                };

                // Merge into local CRDT
                let state = inbound_state.lock().unwrap();
                if let Err(e) = state.crdt.import_snapshot(&raw) {
                    warn!(%e, "failed to import incoming delta");
                }
            }
            debug!("inbound sync task finished");
        });

        Self {
            doc: state,
            outbound_tx,
        }
    }

    /// Notify the engine that a local edit was made.
    ///
    /// Call this immediately after modifying the CrdtDoc.
    /// The engine will export the delta and queue it for broadcast.
    ///
    /// This is NON-BLOCKING — the actual compression and send
    /// happen on a background task. The editor keeps rendering at 144Hz.
    pub fn notify_edit(&self) {
        let state = self.doc.lock().unwrap();
        // Export the full snapshot (Loro deduplicates on import)
        // TODO: use incremental updates once Loro ExportMode::Updates
        //       API is verified
        let snapshot = state.crdt.export_snapshot();

        if snapshot.is_empty() {
            return;
        }

        let tx = self.outbound_tx.clone();
        // Fire and forget — don't block the caller
        tokio::spawn(async move {
            if let Err(e) = tx.send(snapshot).await {
                warn!(%e, "outbound channel full, delta dropped");
            }
        });
    }

    /// Get the current text content of the CRDT document.
    pub fn get_text(&self) -> OnyxResult<String> {
        let state = self.doc.lock().unwrap();
        state.crdt.get_text()
    }

    /// Insert text at a position in the CRDT document.
    ///
    /// This applies the edit locally (Optimistic UI) and queues
    /// the delta for network broadcast.
    pub fn insert(&self, pos: usize, text: &str) -> OnyxResult<()> {
        {
            let state = self.doc.lock().unwrap();
            state.crdt.insert(pos, text)?;
        }
        self.notify_edit();
        Ok(())
    }

    /// Delete text from the CRDT document.
    ///
    /// Same as insert: applies locally, syncs asynchronously.
    pub fn delete(&self, pos: usize, len: usize) -> OnyxResult<()> {
        {
            let state = self.doc.lock().unwrap();
            state.crdt.delete(pos, len)?;
        }
        self.notify_edit();
        Ok(())
    }

    /// Export a full compressed snapshot for initial sync.
    ///
    /// When a device comes online after being offline, it can
    /// request this snapshot and import it to catch up.
    pub fn export_full_snapshot(&self) -> OnyxResult<Vec<u8>> {
        let state = self.doc.lock().unwrap();
        state.crdt.export_compressed()
    }

    /// Import a full compressed snapshot (for initial sync).
    pub fn import_full_snapshot(&self, data: &[u8]) -> OnyxResult<()> {
        let state = self.doc.lock().unwrap();
        state.crdt.import_compressed(data)
    }
}

impl std::fmt::Debug for SyncEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncEngine")
            .field("outbound_channel", &"active")
            .finish_non_exhaustive()
    }
}
