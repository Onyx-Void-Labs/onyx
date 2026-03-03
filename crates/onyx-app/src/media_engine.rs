// ─── Media Engine ──────────────────────────────────────────────────
// Phase 3: The Senses — MoQ Voice Chat
//
// Architecture:
//   MediaEngine runs on a dedicated tokio thread to ensure audio
//   processing never blocks the Makepad UI or Loro CRDT sync.
//
//   Capture Pipeline:
//     Mic (cpal) → PCM frames → Opus encoder → QUIC Datagrams
//
//   Playback Pipeline:
//     QUIC Datagrams → Opus decoder → PCM → Speaker (cpal)
//
//   Transport:
//     Audio frames are sent as QUIC Unreliable Datagrams (0-RTT)
//     over the existing Iroh endpoint. We deliberately avoid
//     reliable streams to prevent head-of-line blocking lag.
//
//   Relay Integration:
//     The Onyx Relay acts as a stateless MoQ reflector: when it
//     receives a media datagram destined for a room, it broadcasts
//     it to all other peers without decoding the Opus payload.
//
// Wire Format (per datagram):
//   [32B topic_hash] [8B sender_id_prefix] [2B sequence] [N bytes opus frame]
//
// Dependencies (gated behind `voice` feature):
//   • cpal     — cross-platform audio capture/playback
//   • (future) audiopus — Opus encoding/decoding bindings
// ────────────────────────────────────────────────────────────────────

use std::sync::mpsc;
use std::thread;
#[allow(unused_imports)]
use tracing::{info, warn};

/// Commands from the UI thread to the MediaEngine.
#[derive(Debug)]
pub enum MediaCommand {
    /// Start capturing + transmitting audio.
    StartCapture,
    /// Stop capturing + transmitting audio.
    StopCapture,
    /// Received an audio datagram from a peer — decode and play.
    IncomingAudio {
        from: String,
        data: Vec<u8>,
    },
    /// Shut down the media engine entirely.
    Shutdown,
}

/// Events from the MediaEngine back to the UI thread.
#[derive(Debug, Clone)]
pub enum MediaEvent {
    /// Audio capture started successfully.
    CaptureStarted,
    /// Audio capture stopped.
    CaptureStopped,
    /// An encoded audio frame ready to broadcast.
    AudioFrame(Vec<u8>),
    /// An error occurred in the media pipeline.
    Error(String),
}

/// Audio configuration for the voice engine.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz (default: 48000 for Opus).
    pub sample_rate: u32,
    /// Number of channels (default: 1 = mono for voice).
    pub channels: u16,
    /// Frame duration in milliseconds (default: 20ms for Opus).
    pub frame_duration_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            frame_duration_ms: 20,
        }
    }
}

/// The UI-side handle to the Media Engine.
///
/// All audio processing happens on a background thread.
/// The UI communicates via channels, never blocking the render loop.
pub struct MediaEngine {
    cmd_tx: mpsc::Sender<MediaCommand>,
    pub evt_rx: mpsc::Receiver<MediaEvent>,
}

impl MediaEngine {
    /// Spawn the media engine on a dedicated background thread.
    pub fn spawn(config: AudioConfig) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<MediaCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<MediaEvent>();

        thread::Builder::new()
            .name("onyx-media".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .thread_name("media-worker")
                    .build()
                    .expect("failed to create media tokio runtime");

                rt.block_on(async move {
                    media_loop(config, cmd_rx, evt_tx).await;
                });
            })
            .expect("failed to spawn media thread");

        Self { cmd_tx, evt_rx }
    }

    /// Start audio capture and transmission.
    pub fn start(&self) {
        let _ = self.cmd_tx.send(MediaCommand::StartCapture);
    }

    /// Stop audio capture.
    pub fn stop(&self) {
        let _ = self.cmd_tx.send(MediaCommand::StopCapture);
    }

    /// Feed an incoming audio datagram for playback.
    pub fn receive_audio(&self, from: String, data: Vec<u8>) {
        let _ = self.cmd_tx.send(MediaCommand::IncomingAudio { from, data });
    }

    /// Shut down the engine.
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(MediaCommand::Shutdown);
    }

    /// Drain all pending events from the media thread.
    pub fn drain_events(&self) -> Vec<MediaEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.evt_rx.try_recv() {
            events.push(evt);
        }
        events
    }
}

// ─── Background Media Loop ──────────────────────────────────────

async fn media_loop(
    config: AudioConfig,
    cmd_rx: mpsc::Receiver<MediaCommand>,
    evt_tx: mpsc::Sender<MediaEvent>,
) {
    info!(
        sample_rate = config.sample_rate,
        channels = config.channels,
        frame_ms = config.frame_duration_ms,
        "media engine started"
    );

    let mut capturing = false;

    // ── Audio device enumeration (requires `voice` feature + cpal) ──
    #[cfg(feature = "voice")]
    {
        match enumerate_audio_devices() {
            Ok(info) => info!("audio devices: {info}"),
            Err(e) => warn!(%e, "failed to enumerate audio devices"),
        }
    }

    loop {
        // Block on the command channel
        let cmd = match cmd_rx.recv() {
            Ok(cmd) => cmd,
            Err(_) => {
                info!("media command channel closed, shutting down");
                break;
            }
        };

        match cmd {
            MediaCommand::StartCapture => {
                if !capturing {
                    info!("starting audio capture");
                    capturing = true;

                    // TODO (Phase 3 full impl):
                    // 1. Open cpal input stream (microphone)
                    // 2. Create Opus encoder (48kHz, mono, 20ms frames)
                    // 3. Feed PCM samples → Opus → evt_tx.send(AudioFrame)
                    //
                    // For now, signal readiness to the UI.
                    let _ = evt_tx.send(MediaEvent::CaptureStarted);
                }
            }
            MediaCommand::StopCapture => {
                if capturing {
                    info!("stopping audio capture");
                    capturing = false;

                    // TODO: Close cpal stream, drop encoder
                    let _ = evt_tx.send(MediaEvent::CaptureStopped);
                }
            }
            MediaCommand::IncomingAudio { from, data } => {
                if data.is_empty() {
                    continue;
                }
                tracing::trace!(
                    from = %from,
                    bytes = data.len(),
                    "received audio frame for playback"
                );

                // TODO (Phase 3 full impl):
                // 1. Decode Opus frame → PCM samples
                // 2. Mix into playback buffer (per-peer jitter buffer)
                // 3. Feed to cpal output stream
            }
            MediaCommand::Shutdown => {
                info!("media engine shutting down");
                let _ = capturing;
                break;
            }
        }
    }

    info!("media engine exited");
}

// ─── Audio Device Enumeration (cpal) ────────────────────────────

#[cfg(feature = "voice")]
fn enumerate_audio_devices() -> Result<String, String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let mut info = String::new();

    if let Some(device) = host.default_input_device() {
        let name = device.name().unwrap_or_else(|_| "unknown".into());
        info.push_str(&format!("input: {name}"));
    } else {
        info.push_str("input: none");
    }

    if let Some(device) = host.default_output_device() {
        let name = device.name().unwrap_or_else(|_| "unknown".into());
        info.push_str(&format!(", output: {name}"));
    } else {
        info.push_str(", output: none");
    }

    Ok(info)
}
