// ─── Onyx Relay — The Giant Hall ───────────────────────────────────
// A stateless relay for the Onyx Void mesh network.
//
// Deployed on your RackNerd VPS, this binary does two things:
//
//   1. **Gossip Reflector**: Runs an Iroh endpoint that participates
//      in gossip swarms. When devices can't directly connect, the
//      relay forwards gossip messages between them.
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

use hub::GiantHall;
use iroh::protocol::Router;
use iroh::Endpoint;
use iroh_gossip::Gossip;
use onyx_core::identity::VoidIdentity;
use onyx_core::protocol::{PubSubMsg, ONYX_PUBSUB_ALPN};
use std::sync::Arc;
use tracing::{info, warn};

/// Maximum RAM budget for the PubSub buffer (in bytes).
/// Default: 512MB — leaves ~512MB for the OS on a 1GB VPS.
const MAX_MEMORY_BYTES: usize = 512 * 1024 * 1024;

/// Maximum age of buffered messages (30 days in seconds).
const MAX_AGE_SECS: u64 = 30 * 24 * 3600;

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
    info!("║  Stateless PubSub + Gossip Reflector     ║");
    info!("╚══════════════════════════════════════════╝");

    // ── Identity ──
    // The relay gets its own persistent identity
    let identity = VoidIdentity::load_or_create(None)?;
    info!(relay_id = %identity.public_key(), "relay identity loaded");

    // ── Iroh Endpoint ──
    let endpoint = Endpoint::builder()
        .secret_key(identity.secret_key().clone())
        .alpns(vec![
            ONYX_PUBSUB_ALPN.to_vec(),
            iroh_gossip::ALPN.to_vec(),
        ])
        .bind()
        .await?;

    info!(
        endpoint_id = %endpoint.id(),
        "iroh endpoint bound"
    );

    // Wait for relay connectivity
    endpoint.online().await;
    let addr = endpoint.addr();
    info!(?addr, "relay online and reachable");

    // ── Gossip ──
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // ── Router ──
    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    // ── PubSub Giant Hall ──
    let hall = Arc::new(GiantHall::new(MAX_MEMORY_BYTES, MAX_AGE_SECS));

    // ── PubSub accept loop ──
    let pubsub_endpoint = endpoint.clone();
    let pubsub_hall = Arc::clone(&hall);
    tokio::spawn(async move {
        info!("PubSub accept loop started");
        loop {
            let incoming = pubsub_endpoint.accept().await;
            let connecting = match incoming {
                Some(c) => c,
                None => {
                    info!("endpoint closed, stopping accept loop");
                    break;
                }
            };

            let hall = Arc::clone(&pubsub_hall);
            tokio::spawn(async move {
                match connecting.await {
                    Ok(conn) => {
                        let peer = conn.remote_id();
                        info!(%peer, "PubSub client connected");
                        if let Err(e) = handle_pubsub_connection(conn, hall).await {
                            warn!(%peer, %e, "PubSub connection error");
                        }
                    }
                    Err(e) => {
                        warn!(%e, "failed to accept connection");
                    }
                }
            });
        }
    });

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

/// Handle a single PubSub client connection.
async fn handle_pubsub_connection(
    conn: iroh::endpoint::Connection,
    hall: Arc<GiantHall>,
) -> anyhow::Result<()> {
    let peer = conn.remote_id();

    loop {
        // Accept bi-directional streams from the client
        let (mut send, mut recv) = match conn.accept_bi().await {
            Ok(streams) => streams,
            Err(_) => {
                // Connection closed
                break;
            }
        };

        // Read the full message (protocol messages are small)
        let data = recv.read_to_end(1024 * 1024).await?; // 1MB max

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
                hall.subscribe(topic, peer);

                // Deliver any buffered messages for this topic
                let buffered = hall.drain_for_peer(&topic, &peer);
                for payload in buffered {
                    let reply = PubSubMsg::Deliver {
                        topic,
                        payload,
                    };
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
                // Buffer the message and deliver to online subscribers
                hall.publish(topic, peer, payload);
            }
            PubSubMsg::RequestState { topic, version_info: _ } => {
                info!(
                    %peer,
                    topic = hex_short(&topic),
                    "state request (initial sync)"
                );
                // Return all buffered messages as a combined snapshot
                let all_data = hall.get_all_for_topic(&topic);
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

fn hex_short(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}
