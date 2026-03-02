// ─── OnyxNode ──────────────────────────────────────────────────────
// Top-level coordinator: wraps Iroh Endpoint + Gossip + Router.
//
// On startup:
//   1. Load (or generate) the device's VoidIdentity
//   2. Create an Iroh Endpoint with that identity
//   3. Spawn the Gossip protocol on the endpoint
//   4. Start the Router to accept incoming connections
//
// The OnyxNode is the single networking primitive that the rest of
// the application interacts with.
// ────────────────────────────────────────────────────────────────────

use iroh::protocol::Router;
use iroh::Endpoint;
use iroh_gossip::Gossip;
use onyx_core::identity::VoidIdentity;
use onyx_core::protocol::ONYX_SYNC_ALPN;
use std::sync::Arc;
use tracing::info;

/// The top-level Onyx networking node.
///
/// Owns the Iroh endpoint and gossip protocol instance.
/// Clone is cheap (all inner state is Arc'd).
#[derive(Clone)]
pub struct OnyxNode {
    /// The iroh QUIC endpoint (our network identity).
    endpoint: Endpoint,
    /// The gossip protocol instance for pub/sub discovery.
    gossip: Gossip,
    /// Our persistent identity.
    identity: Arc<VoidIdentity>,
    /// The protocol router (handles incoming connections).
    _router: Arc<Router>,
}

impl OnyxNode {
    /// Create and bind a new OnyxNode.
    ///
    /// This is an async operation that:
    /// 1. Converts the VoidIdentity into an iroh SecretKey
    /// 2. Binds a QUIC endpoint
    /// 3. Spawns the gossip protocol
    /// 4. Sets up the protocol router
    pub async fn spawn(identity: VoidIdentity) -> anyhow::Result<Self> {
        // Convert our identity to iroh's SecretKey
        let secret_key = identity.secret_key().clone();

        // Build the iroh endpoint with our persistent identity
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![
                ONYX_SYNC_ALPN.to_vec(),
                iroh_gossip::ALPN.to_vec(),
            ])
            .bind()
            .await?;

        info!(
            endpoint_id = %endpoint.id(),
            "iroh endpoint bound"
        );

        // Spawn the gossip protocol
        let gossip = Gossip::builder().spawn(endpoint.clone());

        // Set up the protocol router
        let router = Router::builder(endpoint.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn();

        info!("OnyxNode ready — gossip and router active");

        Ok(Self {
            endpoint,
            gossip,
            identity: Arc::new(identity),
            _router: Arc::new(router),
        })
    }

    // ── Accessors ────────────────────────────────────────────────

    /// The underlying iroh Endpoint.
    #[inline]
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The gossip protocol instance.
    #[inline]
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    /// Our device identity.
    #[inline]
    pub fn identity(&self) -> &VoidIdentity {
        &self.identity
    }

    /// Our public EndpointId (= Void Address).
    #[inline]
    pub fn id(&self) -> iroh::EndpointId {
        self.endpoint.id()
    }

    /// Wait until the endpoint is online (connected to a relay).
    pub async fn wait_online(&self) {
        self.endpoint.online().await;
        info!(
            endpoint_id = %self.endpoint.id(),
            "endpoint online — reachable via relay"
        );
    }

    /// Gracefully shut down the node.
    pub async fn shutdown(&self) {
        info!("shutting down OnyxNode");
        self.endpoint.close().await;
    }
}

impl std::fmt::Debug for OnyxNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnyxNode")
            .field("endpoint_id", &self.endpoint.id().to_string())
            .finish_non_exhaustive()
    }
}
