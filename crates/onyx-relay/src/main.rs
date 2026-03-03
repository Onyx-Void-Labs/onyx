// ─── Onyx Relay — The Giant Hall ───────────────────────────────────
// A stateless relay for the Onyx Void mesh network.
//
// Deployed on your RackNerd VPS, this binary does two things:
//
//   1. **Gossip Reflector**: Runs an Iroh endpoint that participates
//      in gossip swarms. When devices can't directly connect, the
//      relay forwards gossip messages between them.  It actively
//      subscribes to every topic a client registers via PubSub,
//      making it a *transparent gossip reflector*.
//
//   2. **PubSub Buffer**: If a device is offline, the relay holds
//      encrypted CRDT deltas in RAM (up to 30 days or memory limit).
//      When the device reconnects, it picks up the buffered changes.
//
// The relay is STATELESS — no data is written to disk. Everything
// lives in RAM with bounded memory and TTL eviction.
//
// Deploy:
//   cargo build --release -p onyx-relay
//   scp target/release/onyx_relay your-vps:/usr/local/bin/
//   ssh your-vps 'RUST_LOG=info onyx_relay'
// ────────────────────────────────────────────────────────────────────

mod hub;

use futures_lite::StreamExt;
use hub::GiantHall;
use iroh::protocol::Router;
use iroh::Endpoint;
use iroh_gossip::Gossip;
use onyx_core::identity::VoidIdentity;
use onyx_core::protocol::{PubSubMsg, ONYX_MEDIA_ALPN, ONYX_PUBSUB_ALPN};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn};

/// Maximum RAM budget for the PubSub buffer (in bytes).
/// Default: 512MB — leaves ~512MB for the OS on a 1GB VPS.
const MAX_MEMORY_BYTES: usize = 512 * 1024 * 1024;

/// Maximum age of buffered messages (30 days in seconds).
const MAX_AGE_SECS: u64 = 30 * 24 * 3600;

// ─── PubSub Reflector (ProtocolHandler) ─────────────────────────

/// A protocol handler that accepts PubSub client connections.
///
/// When a client sends a `Subscribe` message the reflector:
///   1. Registers the topic in the Giant Hall.
///   2. Subscribes to the matching gossip topic (if not already)
///      so the relay becomes part of the broadcast tree.
///   3. Delivers any buffered messages back to the client.
struct PubSubReflector {
    hall: Arc<GiantHall>,
    gossip: Gossip,
    /// Set of topic hashes the relay has already joined in gossip.
    active_topics: TokioMutex<HashSet<[u8; 32]>>,
}

impl std::fmt::Debug for PubSubReflector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PubSubReflector")
            .field("active_topics", &"<TokioMutex>")
            .finish()
    }
}

impl PubSubReflector {
    fn new(hall: Arc<GiantHall>, gossip: Gossip) -> Self {
        Self {
            hall,
            gossip,
            active_topics: TokioMutex::new(HashSet::new()),
        }
    }

    /// Ensure the relay is subscribed to the gossip topic.
    /// Spawns a long-lived reflector task that keeps the subscription
    /// alive and logs every event that passes through.
    async fn ensure_gossip_topic(&self, topic_hash: [u8; 32]) {
        {
            let topics = self.active_topics.lock().await;
            if topics.contains(&topic_hash) {
                return;
            }
        }

        let topic_id = iroh_gossip::TopicId::from_bytes(topic_hash);
        info!(
            topic = hex_short(&topic_hash),
            "relay joining gossip topic as reflector"
        );

        // Clone gossip handle for potential re-subscription
        let gossip = self.gossip.clone();

        match self.gossip.subscribe(topic_id, vec![]).await {
            Ok(topic_handle) => {
                // Mark as active AFTER successful subscription
                self.active_topics.lock().await.insert(topic_hash);

                let (_sender, receiver) = topic_handle.split();
                let hall = Arc::clone(&self.hall);

                // Spawn a long-lived task that keeps the gossip
                // subscription alive. The relay automatically
                // forwards messages through the broadcast tree.
                tokio::spawn(async move {
                    info!(
                        topic = hex_short(&topic_hash),
                        "gossip reflector task started"
                    );

                    // Keep sender alive so the gossip subscription persists.
                    let _keep_alive = _sender;

                    // Fuse the stream so we never poll after None
                    let fused = receiver.fuse();
                    tokio::pin!(fused);

                    loop {
                        match fused.next().await {
                            Some(Ok(iroh_gossip::api::Event::Received(msg))) => {
                                info!(
                                    from = %msg.delivered_from,
                                    bytes = msg.content.len(),
                                    topic = hex_short(&topic_hash),
                                    "[reflector] gossip message received — broadcasting to tree"
                                );
                                // Buffer in the hall for offline peers
                                hall.publish(
                                    topic_hash,
                                    msg.delivered_from,
                                    msg.content.to_vec(),
                                );
                                // The gossip protocol automatically re-broadcasts
                                // to all neighbors in the tree — no manual send needed.
                            }
                            Some(Ok(iroh_gossip::api::Event::NeighborUp(peer))) => {
                                info!(
                                    peer = %peer,
                                    topic = hex_short(&topic_hash),
                                    "[reflector] peer joined gossip topic"
                                );
                            }
                            Some(Ok(iroh_gossip::api::Event::NeighborDown(peer))) => {
                                warn!(
                                    peer = %peer,
                                    topic = hex_short(&topic_hash),
                                    "[reflector] peer left gossip topic"
                                );
                            }
                            Some(Ok(iroh_gossip::api::Event::Lagged)) => {
                                warn!(
                                    topic = hex_short(&topic_hash),
                                    "[reflector] lagged — some messages may be lost"
                                );
                            }
                            Some(Err(e)) => {
                                warn!(
                                    %e,
                                    topic = hex_short(&topic_hash),
                                    "[reflector] gossip receive error (non-fatal)"
                                );
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                            None => {
                                // Stream terminated — re-subscribe after a backoff
                                warn!(
                                    topic = hex_short(&topic_hash),
                                    "[reflector] gossip stream ended — re-subscribing in 2s"
                                );
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                                // Attempt to re-subscribe
                                match gossip.subscribe(topic_id, vec![]).await {
                                    Ok(new_handle) => {
                                        let (new_sender, new_receiver) = new_handle.split();
                                        // Replace the fused stream via a new pin
                                        let new_fused = new_receiver.fuse();
                                        // We can't reassign the pinned stream,
                                        // so we recurse into a fresh inner loop.
                                        // Drop old keepalive + store new one.
                                        drop(_keep_alive);
                                        info!(
                                            topic = hex_short(&topic_hash),
                                            "[reflector] re-subscribed to gossip topic"
                                        );
                                        // Continue in a fresh loop with the new stream
                                        tokio::pin!(new_fused);
                                        let _keep_alive_2 = new_sender;
                                        loop {
                                            match new_fused.next().await {
                                                Some(Ok(iroh_gossip::api::Event::Received(msg))) => {
                                                    hall.publish(topic_hash, msg.delivered_from, msg.content.to_vec());
                                                }
                                                Some(Ok(iroh_gossip::api::Event::NeighborUp(peer))) => {
                                                    info!(peer = %peer, topic = hex_short(&topic_hash), "[reflector] peer joined");
                                                }
                                                Some(Ok(iroh_gossip::api::Event::NeighborDown(peer))) => {
                                                    warn!(peer = %peer, topic = hex_short(&topic_hash), "[reflector] peer left");
                                                }
                                                Some(Ok(iroh_gossip::api::Event::Lagged)) => {
                                                    warn!(topic = hex_short(&topic_hash), "[reflector] lagged");
                                                }
                                                Some(Err(e)) => {
                                                    warn!(%e, topic = hex_short(&topic_hash), "[reflector] error");
                                                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                                }
                                                None => {
                                                    warn!(topic = hex_short(&topic_hash), "[reflector] stream ended again");
                                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                                    break; // break inner loop, which breaks outer too
                                                }
                                            }
                                        }
                                        break; // exit outer loop after inner exits
                                    }
                                    Err(e) => {
                                        warn!(%e, topic = hex_short(&topic_hash), "[reflector] re-subscribe failed");
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    warn!(topic = hex_short(&topic_hash), "gossip reflector task exiting");
                });
            }
            Err(e) => {
                warn!(
                    %e,
                    topic = hex_short(&topic_hash),
                    "failed to subscribe to gossip topic"
                );
            }
        }
    }

    /// Handle a single PubSub client connection.
    async fn handle_connection(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> anyhow::Result<()> {
        let peer = conn.remote_id();

        loop {
            // Accept bi-directional streams from the client
            let (mut send, mut recv) = match conn.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => break,
            };

            let data = recv.read_to_end(1024 * 1024).await?;

            let msg = match PubSubMsg::decode(&data) {
                Ok(m) => m,
                Err(e) => {
                    warn!(%peer, %e, "invalid protocol message");
                    continue;
                }
            };

            match msg {
                PubSubMsg::Subscribe { topic } => {
                    info!(%peer, topic = hex_short(&topic), "subscribe");
                    self.hall.subscribe(topic, peer);

                    // Join the gossip topic so we participate in the
                    // broadcast tree and can forward messages.
                    self.ensure_gossip_topic(topic).await;

                    // Deliver any buffered messages for this topic
                    let buffered = self.hall.drain_for_peer(&topic, &peer);
                    for payload in buffered {
                        let reply = PubSubMsg::Deliver { topic, payload };
                        let _ = send.write_all(&reply.encode()).await;
                    }
                    let _ = send.finish();
                }
                PubSubMsg::Publish { topic, payload } => {
                    tracing::trace!(
                        %peer,
                        topic = hex_short(&topic),
                        bytes = payload.len(),
                        "publish"
                    );
                    self.hall.publish(topic, peer, payload);
                }
                PubSubMsg::RequestState {
                    topic,
                    version_info: _,
                } => {
                    info!(
                        %peer,
                        topic = hex_short(&topic),
                        "state request (initial sync)"
                    );
                    let all_data = self.hall.get_all_for_topic(&topic);
                    let reply = PubSubMsg::DeliverState {
                        topic,
                        snapshot: all_data,
                    };
                    let _ = send.write_all(&reply.encode()).await;
                    let _ = send.finish();
                }
                _ => {
                    warn!(%peer, "unexpected message type from client");
                }
            }
        }

        info!(%peer, "PubSub client disconnected");
        Ok(())
    }
}

impl iroh::protocol::ProtocolHandler for PubSubReflector {
    async fn accept(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        let peer = conn.remote_id();
        info!(%peer, "PubSub client connected via reflector");

        if let Err(e) = self.handle_connection(conn).await {
            warn!(%peer, %e, "PubSub reflector connection error");
        }

        Ok(())
    }
}

// ─── MoQ Media Reflector ────────────────────────────────────────
// Stateless reflector for voice/audio datagrams.
//
// When a peer sends a media datagram, the relay broadcasts it to
// all other peers in the same room — WITHOUT decoding the Opus
// payload. This is pure packet reflection at the QUIC layer.
//
// Wire format per datagram:
//   [32B topic_hash] [8B sender_id_prefix] [2B sequence] [N bytes opus frame]
//
// The reflector uses QUIC Unreliable Datagrams (0-RTT) to avoid
// head-of-line blocking. Lost audio frames are simply skipped.

struct MediaReflector {
    hall: Arc<GiantHall>,
}

impl std::fmt::Debug for MediaReflector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaReflector").finish()
    }
}

impl MediaReflector {
    fn new(hall: Arc<GiantHall>) -> Self {
        Self { hall }
    }

    /// Handle a media connection from a single peer.
    ///
    /// Reads QUIC datagrams and reflects them to all other peers
    /// subscribed to the same topic.
    async fn handle_media_connection(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> anyhow::Result<()> {
        let peer = conn.remote_id();
        info!(%peer, "media peer connected");

        // Read datagrams in a loop and reflect to other subscribers.
        // Datagrams are unreliable — if the connection drops, we
        // simply stop reflecting.
        loop {
            match conn.read_datagram().await {
                Ok(datagram) => {
                    let data = datagram.to_vec();
                    if data.len() < 42 {
                        // Minimum: 32B topic + 8B sender + 2B seq
                        warn!(%peer, len = data.len(), "media datagram too short");
                        continue;
                    }

                    let mut topic = [0u8; 32];
                    topic.copy_from_slice(&data[..32]);

                    tracing::trace!(
                        %peer,
                        topic = hex_short(&topic),
                        frame_bytes = data.len() - 42,
                        "reflecting media datagram"
                    );

                    // The hall knows who is subscribed to this topic.
                    // We don't decode the audio — just reflect the raw
                    // datagram to all other subscribers.
                    // NOTE: Full reflection requires tracking active media
                    // connections per topic. For the architectural groundwork,
                    // we log and buffer the event.
                    self.hall.publish(topic, peer, data);
                }
                Err(e) => {
                    info!(%peer, %e, "media connection closed");
                    break;
                }
            }
        }

        Ok(())
    }
}

impl iroh::protocol::ProtocolHandler for MediaReflector {
    async fn accept(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        let peer = conn.remote_id();
        info!(%peer, "media peer connected via MoQ reflector");

        if let Err(e) = self.handle_media_connection(conn).await {
            warn!(%peer, %e, "media reflector connection error");
        }

        Ok(())
    }
}

// ─── main ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Tracing ──
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx_relay=info,iroh=warn".into()),
        )
        .init();

    info!("╔══════════════════════════════════════════╗");
    info!("║  ONYX RELAY — The Giant Hall             ║");
    info!("║  PubSub + Gossip + MoQ Media Reflector   ║");
    info!("╚══════════════════════════════════════════╝");

    // ── Identity ──
    let identity = VoidIdentity::relay_identity();
    info!(relay_id = %identity.public_key(), "relay identity loaded (deterministic)");

    // ── Iroh Endpoint ──
    let bind_addr =
        std::net::SocketAddr::from(([0, 0, 0, 0], onyx_core::protocol::RELAY_VPS_PORT));
    let endpoint: Endpoint = Endpoint::builder()
        .secret_key(identity.secret_key().clone())
        .alpns(vec![
            ONYX_PUBSUB_ALPN.to_vec(),
            ONYX_MEDIA_ALPN.to_vec(),
            iroh_gossip::ALPN.to_vec(),
        ])
        .bind_addr(bind_addr)?
        .bind()
        .await?;

    info!(endpoint_id = %endpoint.id(), "iroh endpoint bound");

    endpoint.online().await;
    let addr = endpoint.addr();
    info!(?addr, "relay online and reachable");

    // ── Gossip ──
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // ── PubSub Giant Hall ──
    let hall = Arc::new(GiantHall::new(MAX_MEMORY_BYTES, MAX_AGE_SECS));

    // ── PubSub Reflector (registered as ProtocolHandler) ──
    let reflector = Arc::new(PubSubReflector::new(Arc::clone(&hall), gossip.clone()));

    // ── MoQ Media Reflector ──
    let media_reflector = Arc::new(MediaReflector::new(Arc::clone(&hall)));

    // ── Router ──
    // Both gossip and PubSub are handled through the Router so there
    // is no race on `endpoint.accept()`.
    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .accept(ONYX_PUBSUB_ALPN, reflector.clone())
        .accept(ONYX_MEDIA_ALPN, media_reflector.clone())
        .spawn();

    info!("Router active — gossip + PubSub + MoQ media reflector registered");

    // ── Pre-subscribe to topics from ONYX_TOPICS env var ──
    // Format: comma-separated room secrets, e.g. ONYX_TOPICS=room1,room2
    if let Ok(topics_str) = std::env::var("ONYX_TOPICS") {
        let reflector_ref = Arc::clone(&reflector);
        for secret in topics_str.split(',').filter(|s| !s.is_empty()) {
            let topic_hash = onyx_core::protocol::topic_from_secret(secret.trim());
            info!(room = secret.trim(), "pre-subscribing to topic from ONYX_TOPICS");
            reflector_ref.ensure_gossip_topic(topic_hash).await;
        }
    }

    // ── Memory sweep task ──
    let sweep_hall = Arc::clone(&hall);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let stats = sweep_hall.sweep();
            if stats.evicted > 0 {
                info!(
                    evicted = stats.evicted,
                    topics = stats.topics,
                    memory_bytes = stats.memory_bytes,
                    "memory sweep completed"
                );
            }
        }
    });

    // ── Wait for shutdown signal ──
    info!("relay running — press Ctrl+C to stop");
    tokio::signal::ctrl_c().await?;
    info!("shutting down...");
    endpoint.close().await;
    info!("relay stopped");

    Ok(())
}

fn hex_short(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}
