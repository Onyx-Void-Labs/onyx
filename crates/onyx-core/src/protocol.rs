// ─── Protocol ──────────────────────────────────────────────────────
// Wire protocol types shared between onyx-net (client) and
// onyx-relay (server). Minimal, zero-copy-friendly binary format.
//
// All messages are sent over QUIC bi-directional streams:
//   [1 byte tag] [32 byte topic] [4 byte payload_len BE] [payload]
// ────────────────────────────────────────────────────────────────────

use sha2::{Digest, Sha256};

// ── ALPN identifiers ─────────────────────────────────────────────

/// ALPN for the Onyx PubSub relay protocol.
pub const ONYX_PUBSUB_ALPN: &[u8] = b"onyx/pubsub/1";

/// ALPN for direct Onyx CRDT sync between peers.
pub const ONYX_SYNC_ALPN: &[u8] = b"onyx/sync/1";

// ── Relay Bootstrap ──────────────────────────────────────────────

/// The RackNerd VPS IP address where the Onyx Relay runs.
pub const RELAY_VPS_IP: [u8; 4] = [104, 168, 82, 148];

/// The fixed QUIC port the relay listens on.
pub const RELAY_VPS_PORT: u16 = 11204;

/// Get the relay's well-known EndpointId (PublicKey).
///
/// Derived deterministically so both clients and relay agree
/// without any out-of-band exchange.
pub fn relay_endpoint_id() -> iroh_base::PublicKey {
    crate::identity::VoidIdentity::relay_identity().public_key()
}

/// Get the relay's EndpointId as a string for UI filtering.
pub fn relay_node_id_string() -> String {
    relay_endpoint_id().to_string()
}

// ── Topic hashing ────────────────────────────────────────────────

/// A 32-byte topic hash — SHA256 of the secret room key.
pub type TopicHash = [u8; 32];

/// Derive a topic hash from a human-readable secret room key.
///
/// ```
/// let topic = onyx_core::protocol::topic_from_secret("my-secret-room");
/// assert_eq!(topic.len(), 32);
/// ```
pub fn topic_from_secret(secret: &str) -> TopicHash {
    let mut hasher = Sha256::new();
    hasher.update(b"onyx-void-topic-v1:");
    hasher.update(secret.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// ── Message tags ─────────────────────────────────────────────────

pub const TAG_SUBSCRIBE: u8 = 0x01;
pub const TAG_UNSUBSCRIBE: u8 = 0x02;
pub const TAG_PUBLISH: u8 = 0x03;
pub const TAG_DELIVER: u8 = 0x04;
pub const TAG_REQUEST_STATE: u8 = 0x05;
pub const TAG_DELIVER_STATE: u8 = 0x06;

// ── Control message tags (gossip channel) ────────────────────────
// These are sent as raw gossip broadcasts (no ZSTD compression).
// The 0xCC prefix distinguishes them from CRDT deltas (0x28 ZSTD magic)
// and batch frames (0xBB prefix).

/// Magic prefix for gossip control messages.
pub const CTRL_MAGIC: u8 = 0xCC;

/// Goodbye — peer is gracefully leaving the mesh.
pub const CTRL_GOODBYE: u8 = 0x01;

/// Heartbeat — peer is still alive (resets 15s TTL, 3 missed = disconnect).
pub const CTRL_HEARTBEAT: u8 = 0x02;

/// Cursor position — peer broadcasts their cursor offset.
pub const CTRL_CURSOR_POS: u8 = 0x03;

/// Media datagram — voice/audio frame (MoQ Phase 3).
pub const CTRL_MEDIA: u8 = 0x10;

/// ALPN for Onyx Media-over-QUIC datagrams.
pub const ONYX_MEDIA_ALPN: &[u8] = b"onyx/media/1";

/// Encode a gossip control message (no ZSTD wrapping).
pub fn encode_control(ctrl_type: u8) -> Vec<u8> {
    vec![CTRL_MAGIC, ctrl_type]
}

/// Encode a cursor position control message.
/// Wire format: [0xCC] [0x03] [4 bytes pos BE]
pub fn encode_cursor_control(pos: u32) -> Vec<u8> {
    let mut msg = vec![CTRL_MAGIC, CTRL_CURSOR_POS];
    msg.extend_from_slice(&pos.to_be_bytes());
    msg
}

/// Decode a cursor position from a control message.
/// Returns `Some(pos)` if the message is a valid cursor control.
pub fn decode_cursor_pos(data: &[u8]) -> Option<u32> {
    if data.len() >= 6 && data[0] == CTRL_MAGIC && data[1] == CTRL_CURSOR_POS {
        Some(u32::from_be_bytes([data[2], data[3], data[4], data[5]]))
    } else {
        None
    }
}

/// Check if raw gossip bytes are a control message.
/// Returns `Some(ctrl_type)` if so.
pub fn decode_control(data: &[u8]) -> Option<u8> {
    if data.len() >= 2 && data[0] == CTRL_MAGIC {
        Some(data[1])
    } else {
        None
    }
}

// ── Protocol message ─────────────────────────────────────────────

/// A message in the Onyx PubSub protocol.
#[derive(Debug, Clone)]
pub enum PubSubMsg {
    /// Client → Relay: subscribe to a topic.
    Subscribe { topic: TopicHash },
    /// Client → Relay: unsubscribe from a topic.
    Unsubscribe { topic: TopicHash },
    /// Client → Relay: publish a compressed CRDT delta to a topic.
    Publish { topic: TopicHash, payload: Vec<u8> },
    /// Relay → Client: deliver a buffered delta.
    Deliver { topic: TopicHash, payload: Vec<u8> },
    /// Client → Relay/Peer: request full state (initial sync).
    /// `version_info` is the local Loro VersionVector (serialized).
    RequestState {
        topic: TopicHash,
        version_info: Vec<u8>,
    },
    /// Relay/Peer → Client: full or partial state snapshot.
    DeliverState { topic: TopicHash, snapshot: Vec<u8> },
}

impl PubSubMsg {
    /// Encode to wire format:
    ///   [1B tag] [32B topic] [4B len BE] [payload]
    pub fn encode(&self) -> Vec<u8> {
        let (tag, topic, payload) = match self {
            Self::Subscribe { topic } => (TAG_SUBSCRIBE, topic, &[][..]),
            Self::Unsubscribe { topic } => (TAG_UNSUBSCRIBE, topic, &[][..]),
            Self::Publish { topic, payload } => (TAG_PUBLISH, topic, payload.as_slice()),
            Self::Deliver { topic, payload } => (TAG_DELIVER, topic, payload.as_slice()),
            Self::RequestState {
                topic,
                version_info,
            } => (TAG_REQUEST_STATE, topic, version_info.as_slice()),
            Self::DeliverState { topic, snapshot } => {
                (TAG_DELIVER_STATE, topic, snapshot.as_slice())
            }
        };

        let len = payload.len() as u32;
        let mut buf = Vec::with_capacity(1 + 32 + 4 + payload.len());
        buf.push(tag);
        buf.extend_from_slice(topic);
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    /// Decode from wire format.
    pub fn decode(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 37 {
            return Err("message too short (need at least 37 bytes)");
        }

        let tag = data[0];
        let mut topic = [0u8; 32];
        topic.copy_from_slice(&data[1..33]);
        let len = u32::from_be_bytes([data[33], data[34], data[35], data[36]]) as usize;

        if data.len() < 37 + len {
            return Err("message truncated (payload shorter than declared)");
        }

        let payload = data[37..37 + len].to_vec();

        match tag {
            TAG_SUBSCRIBE => Ok(Self::Subscribe { topic }),
            TAG_UNSUBSCRIBE => Ok(Self::Unsubscribe { topic }),
            TAG_PUBLISH => Ok(Self::Publish { topic, payload }),
            TAG_DELIVER => Ok(Self::Deliver { topic, payload }),
            TAG_REQUEST_STATE => Ok(Self::RequestState {
                topic,
                version_info: payload,
            }),
            TAG_DELIVER_STATE => Ok(Self::DeliverState {
                topic,
                snapshot: payload,
            }),
            _ => Err("unknown message tag"),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_hash_deterministic() {
        let a = topic_from_secret("test-room");
        let b = topic_from_secret("test-room");
        assert_eq!(a, b);
    }

    #[test]
    fn topic_hash_differs_for_different_secrets() {
        let a = topic_from_secret("room-alpha");
        let b = topic_from_secret("room-beta");
        assert_ne!(a, b);
    }

    #[test]
    fn pubsub_msg_roundtrip() {
        let topic = topic_from_secret("test");
        let original = PubSubMsg::Publish {
            topic,
            payload: b"hello void".to_vec(),
        };
        let encoded = original.encode();
        let decoded = PubSubMsg::decode(&encoded).unwrap();

        match decoded {
            PubSubMsg::Publish { topic: t, payload } => {
                assert_eq!(t, topic);
                assert_eq!(payload, b"hello void");
            }
            _ => panic!("wrong variant decoded"),
        }
    }

    #[test]
    fn subscribe_msg_roundtrip() {
        let topic = topic_from_secret("sub-test");
        let msg = PubSubMsg::Subscribe { topic };
        let encoded = msg.encode();
        let decoded = PubSubMsg::decode(&encoded).unwrap();
        match decoded {
            PubSubMsg::Subscribe { topic: t } => assert_eq!(t, topic),
            _ => panic!("wrong variant"),
        }
    }
}
