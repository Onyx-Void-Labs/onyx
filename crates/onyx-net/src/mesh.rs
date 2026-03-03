// ─── Shadow Mesh ───────────────────────────────────────────────────
// Device discovery via Iroh Gossip.
//
// How it works:
//   1. User enters a "secret room key" (e.g. a passphrase)
//   2. We derive TopicId = SHA256("onyx-void-topic-v1:" || key)
//   3. All devices with the same key join the same gossip swarm
//   4. Gossip broadcasts ZSTD-compressed CRDT deltas to all peers
//
// No server-side state needed. No IP addresses exchanged between
// users. Just a shared secret → shared topic → shared state.
// ────────────────────────────────────────────────────────────────────

use futures_lite::StreamExt;
use iroh::EndpointId;
use iroh_gossip::api::{GossipReceiver, GossipSender};
use iroh_gossip::{Gossip, TopicId};
use onyx_core::protocol::{topic_from_secret, TopicHash};

use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// A joined gossip mesh for a single room/topic.
///
/// Provides send/receive channels for CRDT deltas.
pub struct ShadowMesh {
    /// The topic hash for this room.
    topic_hash: TopicHash,
    /// The iroh-gossip TopicId.
    topic_id: TopicId,
    /// Sender half — broadcasts to all peers in the swarm.
    sender: GossipSender,
    /// Receiver half — incoming messages from peers.
    receiver: GossipReceiver,
}

impl ShadowMesh {
    /// Join a gossip swarm using a shared secret room key.
    ///
    /// `bootstrap_peers`: known peers to connect to initially.
    /// Can be empty if using relay/DNS discovery.
    pub async fn join(
        gossip: &Gossip,
        room_secret: &str,
        bootstrap_peers: Vec<EndpointId>,
    ) -> anyhow::Result<Self> {
        let topic_hash = topic_from_secret(room_secret);
        let topic_id = TopicId::from_bytes(topic_hash);

        info!(
            topic = hex::encode_short(&topic_hash),
            peers = bootstrap_peers.len(),
            "joining Shadow Mesh"
        );

        // Subscribe to the gossip topic
        let (sender, receiver) = gossip
            .subscribe(topic_id, bootstrap_peers)
            .await?
            .split();

        Ok(Self {
            topic_hash,
            topic_id,
            sender,
            receiver,
        })
    }

    /// Broadcast a compressed CRDT delta to all peers.
    ///
    /// This is fire-and-forget — it does NOT block the UI thread.
    /// The delta should already be ZSTD-compressed.
    pub async fn broadcast(&self, compressed_delta: Vec<u8>) -> anyhow::Result<()> {
        trace!(
            bytes = compressed_delta.len(),
            "broadcasting delta to mesh"
        );
        self.sender
            .broadcast(compressed_delta.into())
            .await?;
        Ok(())
    }

    /// Wait until we've joined at least one peer.
    pub async fn wait_for_peers(&mut self) -> anyhow::Result<()> {
        info!("waiting for peers to join the mesh...");
        self.receiver.joined().await?;
        info!("joined the mesh — peers connected");
        Ok(())
    }

    /// Spawn a background task that reads incoming gossip messages
    /// and forwards them to a channel for processing.
    ///
    /// Returns a receiver that yields mesh events (deltas + peer join/leave).
    pub fn spawn_receiver(
        mut self,
    ) -> (GossipSender, mpsc::Receiver<MeshEvent>) {
        let (tx, rx) = mpsc::channel(256);
        let sender = self.sender;

        tokio::spawn(async move {
            info!("mesh receiver task started — listening for gossip events");

            // Keep the gossip loop alive until the MeshEvent channel
            // is dropped (i.e. the app shuts down). Never exit on
            // transient stream errors.
            loop {
                match self.receiver.next().await {
                    Some(Ok(iroh_gossip::api::Event::Received(msg))) => {
                        info!(
                            from = %msg.delivered_from,
                            bytes = msg.content.len(),
                            "[gossip] raw event: Received delta"
                        );
                        let evt = MeshEvent::Delta(IncomingDelta {
                            from: msg.delivered_from,
                            data: msg.content.to_vec(),
                        });
                        if tx.send(evt).await.is_err() {
                            debug!("mesh event channel closed, stopping gossip loop");
                            break;
                        }
                    }
                    Some(Ok(iroh_gossip::api::Event::NeighborUp(peer))) => {
                        info!(peer = %peer, "[gossip] raw event: NeighborUp — peer joined");
                        if tx.send(MeshEvent::PeerJoined(peer.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(iroh_gossip::api::Event::NeighborDown(peer))) => {
                        warn!(peer = %peer, "[gossip] raw event: NeighborDown — peer left");
                        if tx.send(MeshEvent::PeerLeft(peer.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(iroh_gossip::api::Event::Lagged)) => {
                        warn!("[gossip] raw event: Lagged — some messages may be lost");
                    }
                    Some(Err(e)) => {
                        // Transient error — log but do NOT exit the loop.
                        // The stream may recover on the next poll.
                        warn!(%e, "[gossip] raw event: receive error (non-fatal, continuing)");
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    }
                    None => {
                        // Stream ended — likely the gossip subscription was
                        // closed. Wait a bit and keep looping so the task
                        // doesn't die; the caller may re-subscribe.
                        warn!("[gossip] stream yielded None — waiting before retry");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
            info!("mesh receiver task exiting (channel closed)");
        });

        (sender, rx)
    }

    /// The topic hash for this mesh.
    pub fn topic_hash(&self) -> &TopicHash {
        &self.topic_hash
    }

    /// The gossip TopicId.
    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }
}

/// Events coming from the gossip mesh.
#[derive(Debug, Clone)]
pub enum MeshEvent {
    /// Received a CRDT delta from a peer.
    Delta(IncomingDelta),
    /// A peer joined the mesh (NeighborUp). Contains their NodeId string.
    PeerJoined(String),
    /// A peer left the mesh (NeighborDown). Contains their NodeId string.
    PeerLeft(String),
}

/// An incoming CRDT delta from a peer.
#[derive(Debug, Clone)]
pub struct IncomingDelta {
    /// The peer that sent or relayed this delta.
    pub from: EndpointId,
    /// The ZSTD-compressed Loro delta bytes.
    pub data: Vec<u8>,
}

// ── Helpers ──────────────────────────────────────────────────────

mod hex {
    /// Encode the first 8 bytes of a hash as hex for logging.
    pub fn encode_short(bytes: &[u8]) -> String {
        bytes
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    }
}
