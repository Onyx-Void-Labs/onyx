// ─── The Giant Hall ────────────────────────────────────────────────
// In-memory PubSub buffer for the Onyx Relay.
//
// Architecture:
//   • HashMap<TopicHash, TopicBuffer> — one buffer per room
//   • Each TopicBuffer holds messages with timestamps
//   • LRU eviction when total memory exceeds the budget
//   • TTL eviction: messages older than 30 days are swept
//
// Memory safety for 1GB VPS with 1000 topics:
//   • Max total memory: 512MB (configurable)
//   • Per-topic cap: 512KB default (can hold ~10,000 keystroke deltas)
//   • Background sweep every 60 seconds
//   • Oldest messages evicted first when limits are hit
//
// The relay NEVER writes to disk. It's a pure RAM buffer.
// If the relay restarts, all buffered messages are lost — that's OK
// because Loro CRDTs can reconstruct state from any peer.
// ────────────────────────────────────────────────────────────────────

use iroh::EndpointId;
use onyx_core::protocol::TopicHash;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Per-topic memory cap (512 KB).
const PER_TOPIC_MAX_BYTES: usize = 512 * 1024;

/// A timestamped message in the buffer.
struct BufferedMessage {
    /// When this message was received.
    timestamp: u64, // unix millis
    /// The compressed CRDT delta payload.
    payload: Vec<u8>,
    /// Who sent this message (so we don't echo it back).
    sender: EndpointId,
}

impl BufferedMessage {
    fn size(&self) -> usize {
        self.payload.len() + 32 + 8 // payload + sender pubkey + timestamp
    }
}

/// Buffer for a single topic/room.
struct TopicBuffer {
    /// Ordered list of messages (oldest first).
    messages: Vec<BufferedMessage>,
    /// Total bytes used by this topic.
    total_bytes: usize,
    /// Subscribers currently connected.
    subscribers: HashSet<EndpointId>,
    /// Last activity timestamp.
    last_activity: Instant,
}

impl TopicBuffer {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            total_bytes: 0,
            subscribers: HashSet::new(),
            last_activity: Instant::now(),
        }
    }

    fn push(&mut self, msg: BufferedMessage) {
        self.total_bytes += msg.size();
        self.messages.push(msg);
        self.last_activity = Instant::now();

        // Evict oldest if over per-topic limit
        while self.total_bytes > PER_TOPIC_MAX_BYTES && !self.messages.is_empty() {
            let removed = self.messages.remove(0);
            self.total_bytes -= removed.size();
        }
    }
}

/// Sweep statistics.
pub struct SweepStats {
    pub evicted: usize,
    pub topics: usize,
    pub memory_bytes: usize,
}

/// The Giant Hall — in-memory PubSub for the relay.
pub struct GiantHall {
    inner: Mutex<HallInner>,
    max_memory: usize,
    max_age_secs: u64,
}

struct HallInner {
    topics: HashMap<TopicHash, TopicBuffer>,
    total_bytes: usize,
}

impl GiantHall {
    pub fn new(max_memory: usize, max_age_secs: u64) -> Self {
        Self {
            inner: Mutex::new(HallInner {
                topics: HashMap::new(),
                total_bytes: 0,
            }),
            max_memory,
            max_age_secs,
        }
    }

    /// Subscribe a peer to a topic.
    pub fn subscribe(&self, topic: TopicHash, peer: EndpointId) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .topics
            .entry(topic)
            .or_insert_with(TopicBuffer::new)
            .subscribers
            .insert(peer);
    }

    /// Publish a message to a topic.
    ///
    /// The message is buffered for offline subscribers.
    pub fn publish(&self, topic: TopicHash, sender: EndpointId, payload: Vec<u8>) {
        let mut inner = self.inner.lock().unwrap();
        let msg_size = payload.len() + 32 + 8;
        inner.total_bytes += msg_size;

        let buf = inner.topics.entry(topic).or_insert_with(TopicBuffer::new);
        buf.push(BufferedMessage {
            timestamp: now_millis(),
            payload,
            sender,
        });

        // Global eviction if over total memory budget
        if inner.total_bytes > self.max_memory {
            evict_oldest(&mut inner, self.max_memory);
        }
    }

    /// Drain buffered messages for a specific peer on a topic.
    ///
    /// Returns messages that the peer hasn't seen (i.e., not sent by them).
    pub fn drain_for_peer(
        &self,
        topic: &TopicHash,
        peer: &EndpointId,
    ) -> Vec<Vec<u8>> {
        let inner = self.inner.lock().unwrap();
        match inner.topics.get(topic) {
            Some(buf) => buf
                .messages
                .iter()
                .filter(|m| m.sender != *peer)
                .map(|m| m.payload.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get all buffered payloads for a topic (for initial sync).
    pub fn get_all_for_topic(&self, topic: &TopicHash) -> Vec<u8> {
        let inner = self.inner.lock().unwrap();
        match inner.topics.get(topic) {
            Some(buf) => {
                // Concatenate all payloads with length prefixes
                let mut result = Vec::new();
                for msg in &buf.messages {
                    let len = msg.payload.len() as u32;
                    result.extend_from_slice(&len.to_be_bytes());
                    result.extend_from_slice(&msg.payload);
                }
                result
            }
            None => Vec::new(),
        }
    }

    /// Sweep expired messages and enforce memory limits.
    ///
    /// Called periodically by the background sweep task.
    pub fn sweep(&self) -> SweepStats {
        let mut inner = self.inner.lock().unwrap();
        let now = now_millis();
        let max_age_ms = self.max_age_secs * 1000;
        let mut evicted = 0;

        // Remove expired messages in each topic
        for buf in inner.topics.values_mut() {
            let before_len = buf.messages.len();
            buf.messages.retain(|msg| {
                let age = now.saturating_sub(msg.timestamp);
                age < max_age_ms
            });
            let removed = before_len - buf.messages.len();
            evicted += removed;

            // Recalculate topic bytes
            buf.total_bytes = buf.messages.iter().map(|m| m.size()).sum();
        }

        // Remove empty topics
        inner.topics.retain(|_, buf| {
            !buf.messages.is_empty() || !buf.subscribers.is_empty()
        });

        // Recalculate total bytes
        inner.total_bytes = inner.topics.values().map(|b| b.total_bytes).sum();

        // If still over budget, evict more
        if inner.total_bytes > self.max_memory {
            evicted += evict_oldest(&mut inner, self.max_memory);
        }

        SweepStats {
            evicted,
            topics: inner.topics.len(),
            memory_bytes: inner.total_bytes,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Evict oldest messages globally until under the memory budget.
fn evict_oldest(inner: &mut HallInner, max_memory: usize) -> usize {
    let mut evicted = 0;

    while inner.total_bytes > max_memory {
        // Find the topic with the oldest message
        let oldest_topic = inner
            .topics
            .iter()
            .filter(|(_, buf)| !buf.messages.is_empty())
            .min_by_key(|(_, buf)| buf.messages[0].timestamp)
            .map(|(topic, _)| *topic);

        match oldest_topic {
            Some(topic) => {
                if let Some(buf) = inner.topics.get_mut(&topic) {
                    if let Some(removed) = buf.messages.first() {
                        let size = removed.size();
                        inner.total_bytes = inner.total_bytes.saturating_sub(size);
                        buf.total_bytes = buf.total_bytes.saturating_sub(size);
                    }
                    if !buf.messages.is_empty() {
                        buf.messages.remove(0);
                    }
                    evicted += 1;
                }
            }
            None => break,
        }
    }

    if evicted > 0 {
        debug!(evicted, "evicted messages to stay under memory budget");
    }
    evicted
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh_base::SecretKey;

    fn test_peer() -> EndpointId {
        SecretKey::generate(&mut rand::rng()).public()
    }

    #[test]
    fn publish_and_drain() {
        let hall = GiantHall::new(1024 * 1024, 3600);
        let topic = [42u8; 32];
        let peer_a = test_peer();
        let peer_b = test_peer();

        hall.subscribe(topic, peer_a);
        hall.subscribe(topic, peer_b);

        hall.publish(topic, peer_a, b"hello from A".to_vec());
        hall.publish(topic, peer_b, b"hello from B".to_vec());

        // peer_b should see A's message but not their own
        let msgs = hall.drain_for_peer(&topic, &peer_b);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], b"hello from A");

        // peer_a should see B's message but not their own
        let msgs = hall.drain_for_peer(&topic, &peer_a);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], b"hello from B");
    }

    #[test]
    fn memory_eviction() {
        // 100 bytes max
        let hall = GiantHall::new(100, 3600);
        let topic = [1u8; 32];
        let peer = test_peer();

        // Each message is ~50+ bytes, so after 3 we should evict
        for i in 0..5 {
            hall.publish(topic, peer, vec![i; 30]);
        }

        let stats = hall.sweep();
        assert!(stats.memory_bytes <= 100);
    }
}
