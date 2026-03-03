// ─── Network Bridge ────────────────────────────────────────────────
// Bridges the synchronous Makepad UI thread to the async Iroh
// networking stack running on a background tokio runtime.
//
// Communication is via two std::sync::mpsc channels:
//   UI → Network: NetCommand (connect, send delta, disconnect)
//   Network → UI: NetEvent  (connected, peer joined/left, delta rx)
//
// The UI polls `evt_rx` every ~50ms via a Makepad Timer.
// ────────────────────────────────────────────────────────────────────

use std::sync::mpsc;
use std::thread;
use tracing::{debug, error, info, warn};

// ── Commands (UI → Network) ──────────────────────────────────────

#[derive(Debug)]
pub enum NetCommand {
    /// Join a gossip mesh with the given room secret.
    Connect(String),
    /// Broadcast a CRDT delta to all peers.
    SendDelta(Vec<u8>),
    /// Leave the mesh and shut down.
    Disconnect,
}

// ── Events (Network → UI) ────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum NetEvent {
    /// Successfully connected to the mesh.
    Connected { our_id: String },
    /// A peer joined the mesh.
    PeerJoined(String),
    /// A peer left the mesh.
    PeerLeft(String),
    /// Received a CRDT delta from a peer (already decompressed).
    DeltaReceived(Vec<u8>),
    /// An error occurred on the network side.
    Error(String),
}

// ── Bridge ───────────────────────────────────────────────────────

/// The UI-side handle to the network bridge.
pub struct NetBridge {
    cmd_tx: mpsc::Sender<NetCommand>,
    pub evt_rx: mpsc::Receiver<NetEvent>,
}

impl NetBridge {
    /// Spawn the bridge: creates a background thread with a tokio runtime.
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<NetCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<NetEvent>();

        thread::Builder::new()
            .name("onyx-net".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .thread_name("iroh-worker")
                    .build()
                    .expect("failed to create tokio runtime");

                rt.block_on(async move {
                    network_loop(cmd_rx, evt_tx).await;
                });
            })
            .expect("failed to spawn network thread");

        Self { cmd_tx, evt_rx }
    }

    /// Send a connect command with the given room secret.
    pub fn connect(&self, room_secret: String) {
        let _ = self.cmd_tx.send(NetCommand::Connect(room_secret));
    }

    /// Broadcast a CRDT delta to peers.
    pub fn send_delta(&self, delta: Vec<u8>) {
        let _ = self.cmd_tx.send(NetCommand::SendDelta(delta));
    }

    /// Disconnect from the mesh.
    pub fn disconnect(&self) {
        let _ = self.cmd_tx.send(NetCommand::Disconnect);
    }

    /// Drain all pending events from the network thread.
    /// Call this from the UI timer handler.
    pub fn drain_events(&self) -> Vec<NetEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.evt_rx.try_recv() {
            events.push(evt);
        }
        events
    }
}

// ── Background network loop ─────────────────────────────────────

async fn network_loop(
    cmd_rx: mpsc::Receiver<NetCommand>,
    evt_tx: mpsc::Sender<NetEvent>,
) {
    use onyx_core::identity::VoidIdentity;
    use onyx_net::{OnyxNode, ShadowMesh};

    info!("network bridge loop started");

    // Wait for a Connect command first.
    let mut node: Option<OnyxNode> = None;
    let mut gossip_sender: Option<iroh_gossip::api::GossipSender> = None;
    let mut mesh_rx: Option<tokio::sync::mpsc::Receiver<onyx_net::mesh::MeshEvent>> = None;

    loop {
        // Check for incoming commands (non-blocking when we have an active mesh)
        let cmd = if gossip_sender.is_some() {
            // Non-blocking check — we'll also poll deltas below
            match cmd_rx.try_recv() {
                Ok(cmd) => Some(cmd),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    debug!("command channel closed, shutting down");
                    break;
                }
            }
        } else {
            // Blocking wait — nothing else to do until we get a command
            match cmd_rx.recv() {
                Ok(cmd) => Some(cmd),
                Err(_) => {
                    debug!("command channel closed, shutting down");
                    break;
                }
            }
        };

        if let Some(cmd) = cmd {
            match cmd {
                NetCommand::Connect(room_secret) => {
                    info!(room = %room_secret, "connecting to mesh...");

                    // Load (or generate) identity based on profile
                    let identity = {
                        let profile = crate::PROFILE.get().and_then(|p| p.as_ref());
                        match profile {
                            Some(name) => {
                                let path = VoidIdentity::profile_path(name)
                                    .expect("failed to resolve profile path");
                                VoidIdentity::load_or_create(Some(&path))
                                    .expect("failed to load/create profile identity")
                            }
                            None => {
                                VoidIdentity::load_or_create(None)
                                    .expect("failed to load/create default identity")
                            }
                        }
                    };
                    let our_id = identity.public_key().to_string();

                    match OnyxNode::spawn(identity).await {
                        Ok(n) => {
                            // Wait for the endpoint to connect to a relay
                            // so we can be reached by other peers.
                            info!("OnyxNode spawned, waiting for relay connectivity...");
                            n.wait_online().await;

                            // Bootstrap gossip through the well-known relay.
                            let relay_id = onyx_core::protocol::relay_endpoint_id();
                            info!(relay_id = %relay_id, "bootstrapping gossip via relay");

                            match ShadowMesh::join(n.gossip(), &room_secret, vec![relay_id]).await {
                                Ok(mesh) => {
                                    let (sender, rx) = mesh.spawn_receiver();
                                    gossip_sender = Some(sender);
                                    mesh_rx = Some(rx);
                                    node = Some(n);

                                    let _ = evt_tx.send(NetEvent::Connected {
                                        our_id: our_id.clone(),
                                    });
                                    info!(id = %our_id, "connected to mesh");
                                }
                                Err(e) => {
                                    let msg = format!("failed to join mesh: {e}");
                                    error!("{msg}");
                                    let _ = evt_tx.send(NetEvent::Error(msg));
                                }
                            }
                        }
                        Err(e) => {
                            let msg = format!("failed to spawn node: {e}");
                            error!("{msg}");
                            let _ = evt_tx.send(NetEvent::Error(msg));
                        }
                    }
                }
                NetCommand::SendDelta(delta) => {
                    if let Some(ref sender) = gossip_sender {
                        // Compress and broadcast
                        match zstd::encode_all(delta.as_slice(), 1) {
                            Ok(compressed) => {
                                if let Err(e) = sender.broadcast(compressed.into()).await {
                                    warn!(%e, "broadcast failed");
                                }
                            }
                            Err(e) => {
                                warn!(%e, "ZSTD compression failed");
                            }
                        }
                    }
                }
                NetCommand::Disconnect => {
                    info!("disconnecting from mesh");
                    gossip_sender = None;
                    mesh_rx = None;
                    if let Some(ref n) = node {
                        n.shutdown().await;
                    }
                    node = None;
                }
            }
        }

        // Poll for incoming mesh events (non-blocking)
        if let Some(ref mut rx) = mesh_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    onyx_net::mesh::MeshEvent::Delta(delta) => {
                        // Decompress CRDT delta
                        match zstd::decode_all(delta.data.as_slice()) {
                            Ok(raw) => {
                                let _ = evt_tx.send(NetEvent::DeltaReceived(raw));
                            }
                            Err(e) => {
                                warn!(%e, "failed to decompress incoming delta");
                            }
                        }
                    }
                    onyx_net::mesh::MeshEvent::PeerJoined(peer_id) => {
                        info!(peer = %peer_id, "NeighborUp → forwarding to UI");
                        let _ = evt_tx.send(NetEvent::PeerJoined(peer_id));
                    }
                    onyx_net::mesh::MeshEvent::PeerLeft(peer_id) => {
                        info!(peer = %peer_id, "NeighborDown → forwarding to UI");
                        let _ = evt_tx.send(NetEvent::PeerLeft(peer_id));
                    }
                }
            }
        }

        // Yield briefly so we don't spin at 100% CPU when idle
        if gossip_sender.is_some() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    info!("network bridge loop exited");
}
