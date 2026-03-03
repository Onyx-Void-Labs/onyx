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
use tracing::{debug, error, info, trace, warn};

// ── Commands (UI → Network) ──────────────────────────────────────

#[derive(Debug)]
pub enum NetCommand {
    /// Join a gossip mesh with the given room secret.
    Connect(String),
    /// Broadcast a CRDT delta to all peers.
    SendDelta(Vec<u8>),
    /// Broadcast a raw control message (no ZSTD compression).
    SendControl(Vec<u8>),
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
    /// A peer sent a Goodbye — remove them immediately.
    GoodbyeReceived(String),
    /// A peer sent a Heartbeat — reset their TTL.
    HeartbeatReceived(String),
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

    /// Broadcast a raw control message (Goodbye, Heartbeat, etc.).
    pub fn send_control(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(NetCommand::SendControl(data));
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

/// Read buffered Deliver messages from the relay's PubSub response stream.
/// This keeps the RecvStream alive and drains any data the relay pushes.
async fn drain_relay_delivers(
    mut recv: iroh::endpoint::RecvStream,
    evt_tx: mpsc::Sender<NetEvent>,
) -> anyhow::Result<()> {
    use onyx_core::protocol::PubSubMsg;

    info!("relay deliver reader started");

    // Read in a loop — the relay may push multiple Deliver messages
    // on this stream before it closes.
    let mut buf = Vec::new();
    loop {
        let mut chunk = [0u8; 8192];
        match recv.read(&mut chunk).await {
            Ok(Some(n)) => {
                buf.extend_from_slice(&chunk[..n]);

                // Try to decode complete messages from the buffer
                while buf.len() >= 37 {
                    let payload_len =
                        u32::from_be_bytes([buf[33], buf[34], buf[35], buf[36]]) as usize;
                    let total = 37 + payload_len;
                    if buf.len() < total {
                        break; // incomplete message, wait for more data
                    }
                    match PubSubMsg::decode(&buf[..total]) {
                        Ok(PubSubMsg::Deliver { payload, .. }) => {
                            // Decompress and forward to UI
                            match zstd::decode_all(payload.as_slice()) {
                                Ok(raw) => {
                                    let _ = evt_tx.send(NetEvent::DeltaReceived(raw));
                                }
                                Err(e) => {
                                    warn!(%e, "failed to decompress relay-delivered delta");
                                }
                            }
                        }
                        Ok(_) => { /* ignore non-Deliver messages */ }
                        Err(e) => {
                            warn!(%e, "invalid message from relay");
                        }
                    }
                    buf.drain(..total);
                }
            }
            Ok(None) => {
                info!("relay PubSub recv stream closed gracefully");
                break;
            }
            Err(e) => {
                warn!(%e, "relay PubSub recv stream error");
                break;
            }
        }
    }
    Ok(())
}

async fn network_loop(
    cmd_rx: mpsc::Receiver<NetCommand>,
    evt_tx: mpsc::Sender<NetEvent>,
) {
    use onyx_core::identity::VoidIdentity;
    use onyx_core::protocol::{self, PubSubMsg, ONYX_PUBSUB_ALPN};
    use onyx_net::{OnyxNode, ShadowMesh};

    info!("network bridge loop started");

    // Wait for a Connect command first.
    let mut node: Option<OnyxNode> = None;
    let mut gossip_sender: Option<iroh_gossip::api::GossipSender> = None;
    let mut mesh_rx: Option<tokio::sync::mpsc::Receiver<onyx_net::mesh::MeshEvent>> = None;
    let mut active_room_secret: Option<String> = None;
    // Keep the PubSub relay connection alive for the entire session.
    // If this is dropped, QUIC closes the connection and the relay
    // sees a disconnect 12ms later.
    let mut _relay_conn: Option<iroh::endpoint::Connection> = None;

    // ── In-Flight Batching ──
    // Stores a non-delta command that was pulled off the channel
    // while draining queued SendDeltas. Processed on the next
    // loop iteration so connect/disconnect are never lost.
    let mut carryover_cmd: Option<NetCommand> = None;

    loop {
        // Check for incoming commands (non-blocking when we have an active mesh).
        // A carryover from a previous batch drain takes priority.
        let cmd = if let Some(co) = carryover_cmd.take() {
            Some(co)
        } else if gossip_sender.is_some() {
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

                            // ── Register topic with relay ──
                            // Sends a PubSub Subscribe so the relay joins
                            // the gossip topic and can reflect messages.
                            let relay_id = protocol::relay_endpoint_id();
                            let topic_hash = protocol::topic_from_secret(&room_secret);
                            info!(relay_id = %relay_id, "registering topic with relay via PubSub");
                            match n.endpoint().connect(relay_id, ONYX_PUBSUB_ALPN).await {
                                Ok(conn) => {
                                    match conn.open_bi().await {
                                        Ok((mut send, recv)) => {
                                            let msg = PubSubMsg::Subscribe { topic: topic_hash };
                                            let _ = send.write_all(&msg.encode()).await;
                                            let _ = send.finish();
                                            info!("topic registered with relay");
                                            // Keep recv stream alive so the relay
                                            // can push buffered messages to us.
                                            // Spawn a reader task that drains any
                                            // Deliver messages from the relay.
                                            let evt_tx2 = evt_tx.clone();
                                            tokio::spawn(async move {
                                                let _ = drain_relay_delivers(recv, evt_tx2).await;
                                            });
                                        }
                                        Err(e) => {
                                            warn!(%e, "failed to open PubSub stream (continuing anyway)");
                                        }
                                    }
                                    // Store the connection so it stays alive
                                    // for the duration of the session.
                                    _relay_conn = Some(conn);
                                }
                                Err(e) => {
                                    warn!(%e, "failed to connect to relay via PubSub (continuing anyway)");
                                }
                            }

                            // Bootstrap gossip through the well-known relay.
                            info!(relay_id = %relay_id, "bootstrapping gossip via relay");

                            match ShadowMesh::join(n.gossip(), &room_secret, vec![relay_id]).await {
                                Ok(mesh) => {
                                    let (sender, rx) = mesh.spawn_receiver();
                                    gossip_sender = Some(sender);
                                    mesh_rx = Some(rx);
                                    active_room_secret = Some(room_secret.clone());
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
                NetCommand::SendDelta(first_delta) => {
                    if let Some(ref sender) = gossip_sender {
                        // ── In-Flight Batching (0ms artificial delay) ──
                        // Drain any additional SendDelta commands that
                        // queued up while the previous broadcast was
                        // in-transit over the network.
                        let mut deltas: Vec<Vec<u8>> = vec![first_delta];
                        loop {
                            match cmd_rx.try_recv() {
                                Ok(NetCommand::SendDelta(d)) => deltas.push(d),
                                Ok(other) => {
                                    // Non-delta command — stash for next
                                    // loop iteration so it isn't lost.
                                    carryover_cmd = Some(other);
                                    break;
                                }
                                Err(_) => break,
                            }
                        }

                        // Build the payload:
                        //   1 delta  → raw bytes (no framing overhead)
                        //   N deltas → batch frame: 0xBB | u16 count | [u32 len | bytes]…
                        let payload = if deltas.len() == 1 {
                            deltas.into_iter().next().unwrap()
                        } else {
                            let count = deltas.len();
                            let mut frame = Vec::with_capacity(
                                3 + deltas.iter().map(|d| 4 + d.len()).sum::<usize>(),
                            );
                            frame.push(0xBBu8); // batch magic
                            frame.extend_from_slice(&(count as u16).to_be_bytes());
                            for d in &deltas {
                                frame.extend_from_slice(&(d.len() as u32).to_be_bytes());
                                frame.extend_from_slice(d);
                            }
                            debug!(
                                count,
                                "in-flight batch: merged {count} deltas → 1 packet"
                            );
                            frame
                        };

                        // Compress and broadcast the (possibly batched) payload
                        match zstd::encode_all(payload.as_slice(), 1) {
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
                NetCommand::SendControl(raw) => {
                    // Broadcast raw control message (no compression)
                    if let Some(ref sender) = gossip_sender {
                        if let Err(e) = sender.broadcast(raw.into()).await {
                            warn!(%e, "control broadcast failed");
                        }
                    }
                }
                NetCommand::Disconnect => {
                    info!("disconnecting from mesh — broadcasting Goodbye");
                    // Broadcast Goodbye before leaving so peers remove us instantly
                    if let Some(ref sender) = gossip_sender {
                        let goodbye = onyx_core::protocol::encode_control(
                            onyx_core::protocol::CTRL_GOODBYE,
                        );
                        let _ = sender.broadcast(goodbye.into()).await;
                        // Brief delay to ensure the message propagates
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    }
                    gossip_sender = None;
                    mesh_rx = None;
                    _relay_conn = None;
                    if let Some(ref n) = node {
                        n.shutdown().await;
                    }
                    node = None;
                }
            }
        }

        // Poll for incoming mesh events (non-blocking).
        // We collect events and check for stream-end separately to
        // avoid borrow conflicts on `mesh_rx`.
        let mut stream_ended = false;
        if let Some(ref mut rx) = mesh_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    onyx_net::mesh::MeshEvent::Delta(delta) => {
                        // Check for control messages (0xCC prefix) BEFORE decompression
                        if let Some(ctrl_type) = onyx_core::protocol::decode_control(&delta.data) {
                            let from = delta.from.to_string();
                            match ctrl_type {
                                onyx_core::protocol::CTRL_GOODBYE => {
                                    info!(peer = %from, "received Goodbye control message");
                                    let _ = evt_tx.send(NetEvent::GoodbyeReceived(from));
                                }
                                onyx_core::protocol::CTRL_HEARTBEAT => {
                                    trace!(peer = %from, "received Heartbeat");
                                    let _ = evt_tx.send(NetEvent::HeartbeatReceived(from));
                                }
                                _ => {
                                    debug!(ctrl_type, "unknown control message type");
                                }
                            }
                            continue;
                        }

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
                    onyx_net::mesh::MeshEvent::StreamEnded => {
                        warn!("gossip stream ended — will attempt re-subscription");
                        stream_ended = true;
                        break;
                    }
                }
            }
        }

        // Handle stream-ended outside the borrow of mesh_rx
        if stream_ended {
            gossip_sender = None;
            mesh_rx = None;

            // Auto re-subscribe if we have a node + room secret
            if let (Some(ref n), Some(ref secret)) = (&node, &active_room_secret) {
                let relay_id = protocol::relay_endpoint_id();
                info!("re-subscribing to mesh after stream loss...");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                match ShadowMesh::join(n.gossip(), secret, vec![relay_id]).await {
                    Ok(mesh) => {
                        let (sender, rx) = mesh.spawn_receiver();
                        gossip_sender = Some(sender);
                        mesh_rx = Some(rx);
                        info!("re-subscribed to mesh successfully");
                    }
                    Err(e) => {
                        warn!(%e, "failed to re-subscribe, will retry on next poll");
                        let _ = evt_tx.send(NetEvent::Error(
                            format!("Gossip stream lost, reconnect failed: {e}"),
                        ));
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
