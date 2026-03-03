// ─── Media Engine ──────────────────────────────────────────────────
// Phase 3: The Senses — MoQ Voice Chat
//
// Architecture:
//   MediaEngine runs on a dedicated background thread.  Audio
//   processing never blocks the Makepad UI or the Loro CRDT sync.
//
//   Capture Pipeline:
//     Mic (cpal 48kHz mono) → accumulate 960 samples (20ms)
//       → Opus encode → AudioFrame event → NetBridge → QUIC datagram
//
//   Playback Pipeline:
//     QUIC datagram → NetBridge → IncomingAudio command
//       → per-peer Opus decoder → f32 PCM → playback VecDeque
//       → cpal output stream
//
//   Transport (handled by NetBridge, not here):
//     Opus frames travel as QUIC Unreliable Datagrams with header:
//       [32B topic_hash] [32B sender_node_id] [N bytes opus frame]
//
// Dependencies (gated behind `voice` feature):
//   • cpal      — cross-platform audio capture/playback
//   • audiopus  — safe Opus encoding/decoding bindings
// ────────────────────────────────────────────────────────────────────

use std::sync::mpsc;
use std::thread;
use tracing::{info, warn, error, trace};

/// Opus frame size: 20ms at 48 kHz mono = 960 samples.
const FRAME_SIZE: usize = 960;

/// Maximum encoded Opus frame (bytes).  Opus rarely exceeds 500 B
/// for mono voice at 48 kHz, but we keep headroom.
const MAX_OPUS_BYTES: usize = 4000;

// ── Commands (UI → MediaEngine) ─────────────────────────────────

/// Commands from the UI/App thread into the media engine.
#[derive(Debug)]
pub enum MediaCommand {
    /// Start capturing microphone + playing back remote audio.
    StartCapture,
    /// Stop capturing + playback.
    StopCapture,
    /// An Opus-encoded frame arrived from a remote peer — decode & play.
    IncomingAudio { from: String, data: Vec<u8> },
    /// Shut down the engine thread.
    Shutdown,
}

// ── Events (MediaEngine → UI) ───────────────────────────────────

/// Events emitted by the media engine back to the UI thread.
#[derive(Debug, Clone)]
pub enum MediaEvent {
    /// Audio capture started successfully.
    CaptureStarted,
    /// Audio capture stopped.
    CaptureStopped,
    /// An Opus-encoded frame ready for network broadcast.
    AudioFrame(Vec<u8>),
    /// A non-fatal error in the pipeline.
    Error(String),
}

// ── UI-side handle ──────────────────────────────────────────────

/// The UI-side handle to the Media Engine.
///
/// All heavy lifting happens on the background thread.
/// The UI communicates via channels — never blocking the render loop.
pub struct MediaEngine {
    cmd_tx: mpsc::Sender<MediaCommand>,
    pub evt_rx: mpsc::Receiver<MediaEvent>,
}

impl MediaEngine {
    /// Spawn the media engine on a dedicated background thread.
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<MediaCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<MediaEvent>();

        thread::Builder::new()
            .name("onyx-media".into())
            .spawn(move || {
                media_loop(cmd_rx, evt_tx);
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

    /// Feed an incoming Opus frame for playback.
    pub fn receive_audio(&self, from: String, data: Vec<u8>) {
        let _ = self.cmd_tx.send(MediaCommand::IncomingAudio { from, data });
    }

    /// Shut down the engine.
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(MediaCommand::Shutdown);
    }

    /// Drain all pending events (non-blocking).
    pub fn drain_events(&self) -> Vec<MediaEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.evt_rx.try_recv() {
            events.push(evt);
        }
        events
    }
}

// ═══════════════════════════════════════════════════════════════════
// Background media loop — voice feature ENABLED
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "voice")]
fn media_loop(
    cmd_rx: mpsc::Receiver<MediaCommand>,
    evt_tx: mpsc::Sender<MediaEvent>,
) {
    use std::collections::HashMap;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use audiopus::coder::{Decoder as OpusDecoder, Encoder as OpusEncoder};
    use audiopus::{Application, Channels, MutSignals, SampleRate};
    use audiopus::packet::Packet as OpusPacket;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    info!("media engine started (voice feature enabled)");

    // ── Audio device discovery ───────────────────────────────────
    let host = cpal::default_host();

    let input_device = host.default_input_device();
    let output_device = host.default_output_device();

    if let Some(ref d) = input_device {
        info!(name = %d.name().unwrap_or_default(), "input device");
    } else {
        warn!("no input audio device available");
    }
    if let Some(ref d) = output_device {
        info!(name = %d.name().unwrap_or_default(), "output device");
    } else {
        warn!("no output audio device available");
    }

    // ── Mutable state ────────────────────────────────────────────
    let mut capturing = false;

    // cpal streams — dropping them stops audio.
    let mut _input_stream: Option<cpal::Stream> = None;
    let mut _output_stream: Option<cpal::Stream> = None;

    // Opus encoder (created on StartCapture).
    let mut encoder: Option<OpusEncoder> = None;

    // Per-peer Opus decoders (keyed by peer-id string).
    let mut decoders: HashMap<String, OpusDecoder> = HashMap::new();

    // Playback ring: the cpal output callback pops from the front;
    // decoded audio is pushed to the back.  Protected by a Mutex
    // because cpal callbacks run on a real-time audio thread.
    let playback_buf: Arc<Mutex<VecDeque<f32>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(FRAME_SIZE * 10)));

    // Internal channel: cpal input callback → this loop.
    // Each message is one 20ms frame of f32 PCM (960 samples).
    let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<f32>>();

    // ── Main loop ────────────────────────────────────────────────
    loop {
        // ── 1. Process commands ──────────────────────────────────
        match cmd_rx.try_recv() {
            Ok(MediaCommand::StartCapture) => {
                if capturing {
                    continue;
                }
                info!("starting audio capture + playback");

                // — Opus Encoder —
                match OpusEncoder::new(
                    SampleRate::Hz48000,
                    Channels::Mono,
                    Application::Voip,
                ) {
                    Ok(enc) => encoder = Some(enc),
                    Err(e) => {
                        let msg = format!("Opus encoder creation failed: {e}");
                        error!("{msg}");
                        let _ = evt_tx.send(MediaEvent::Error(msg));
                        continue;
                    }
                }

                // — cpal input stream (microphone) —
                if let Some(ref device) = input_device {
                    let cfg = cpal::StreamConfig {
                        channels: 1,
                        sample_rate: cpal::SampleRate(48000),
                        buffer_size: cpal::BufferSize::Default,
                    };
                    let tx = pcm_tx.clone();
                    let mut acc: Vec<f32> = Vec::with_capacity(FRAME_SIZE * 2);

                    match device.build_input_stream(
                        &cfg,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            acc.extend_from_slice(data);
                            while acc.len() >= FRAME_SIZE {
                                let frame: Vec<f32> =
                                    acc.drain(..FRAME_SIZE).collect();
                                let _ = tx.send(frame);
                            }
                        },
                        |err| error!("cpal input error: {err}"),
                        None,
                    ) {
                        Ok(stream) => {
                            let _ = stream.play();
                            _input_stream = Some(stream);
                            info!("cpal input stream started");
                        }
                        Err(e) => {
                            let msg = format!("input stream open failed: {e}");
                            error!("{msg}");
                            let _ = evt_tx.send(MediaEvent::Error(msg));
                        }
                    }
                }

                // — cpal output stream (speaker) —
                if let Some(ref device) = output_device {
                    let cfg = cpal::StreamConfig {
                        channels: 1,
                        sample_rate: cpal::SampleRate(48000),
                        buffer_size: cpal::BufferSize::Default,
                    };
                    let pb = Arc::clone(&playback_buf);

                    match device.build_output_stream(
                        &cfg,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            let mut buf = pb.lock().unwrap();
                            for sample in data.iter_mut() {
                                *sample = buf.pop_front().unwrap_or(0.0);
                            }
                        },
                        |err| error!("cpal output error: {err}"),
                        None,
                    ) {
                        Ok(stream) => {
                            let _ = stream.play();
                            _output_stream = Some(stream);
                            info!("cpal output stream started");
                        }
                        Err(e) => {
                            let msg = format!("output stream open failed: {e}");
                            error!("{msg}");
                            let _ = evt_tx.send(MediaEvent::Error(msg));
                        }
                    }
                }

                capturing = true;
                let _ = evt_tx.send(MediaEvent::CaptureStarted);
            }

            Ok(MediaCommand::StopCapture) => {
                if !capturing {
                    continue;
                }
                info!("stopping audio capture + playback");
                _input_stream = None; // drop → stops capture
                _output_stream = None; // drop → stops playback
                encoder = None;
                decoders.clear();
                if let Ok(mut buf) = playback_buf.lock() {
                    buf.clear();
                }
                capturing = false;
                let _ = evt_tx.send(MediaEvent::CaptureStopped);
            }

            Ok(MediaCommand::IncomingAudio { from, data }) => {
                if data.is_empty() {
                    continue;
                }
                trace!(from = %from, bytes = data.len(), "decoding incoming audio");

                // Get or create a per-peer Opus decoder.
                let decoder = decoders
                    .entry(from.clone())
                    .or_insert_with(|| {
                        OpusDecoder::new(SampleRate::Hz48000, Channels::Mono)
                            .expect("failed to create Opus decoder")
                    });

                // Decode Opus → i16 PCM.
                let mut pcm = vec![0i16; FRAME_SIZE];
                let packet = match OpusPacket::try_from(&data[..]) {
                    Ok(p) => p,
                    Err(e) => {
                        warn!(peer = %from, %e, "invalid Opus packet");
                        continue;
                    }
                };
                let output = match MutSignals::try_from(&mut pcm[..]) {
                    Ok(o) => o,
                    Err(e) => {
                        warn!(peer = %from, %e, "MutSignals creation failed");
                        continue;
                    }
                };
                match decoder.decode(Some(packet), output, false) {
                    Ok(samples) => {
                        // Convert i16 → f32 and push into playback ring.
                        if let Ok(mut buf) = playback_buf.lock() {
                            for &s in &pcm[..samples] {
                                buf.push_back(s as f32 / 32768.0);
                            }
                            // Cap to ~200ms to prevent unbounded growth.
                            while buf.len() > FRAME_SIZE * 10 {
                                buf.pop_front();
                            }
                        }
                    }
                    Err(e) => {
                        warn!(peer = %from, %e, "Opus decode error");
                    }
                }
            }

            Ok(MediaCommand::Shutdown) => {
                info!("media engine shutting down");
                _input_stream = None;
                _output_stream = None;
                break;
            }

            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                info!("media command channel closed");
                break;
            }
        }

        // ── 2. Encode captured PCM frames ────────────────────────
        if capturing {
            while let Ok(pcm_f32) = pcm_rx.try_recv() {
                if let Some(ref mut enc) = encoder {
                    // Convert f32 → i16 for Opus encoder.
                    let pcm_i16: Vec<i16> = pcm_f32
                        .iter()
                        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                        .collect();

                    let mut opus_buf = vec![0u8; MAX_OPUS_BYTES];
                    match enc.encode(&pcm_i16[..], &mut opus_buf[..]) {
                        Ok(len) => {
                            opus_buf.truncate(len);
                            trace!(opus_bytes = len, "encoded audio frame");
                            let _ = evt_tx.send(MediaEvent::AudioFrame(opus_buf));
                        }
                        Err(e) => {
                            warn!(%e, "Opus encode error");
                        }
                    }
                }
            }
        }

        // Brief sleep to avoid busy-spinning (audio arrives every 20ms).
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    info!("media engine exited");
}

// ═══════════════════════════════════════════════════════════════════
// Background media loop — voice feature DISABLED (stub)
// ═══════════════════════════════════════════════════════════════════

#[cfg(not(feature = "voice"))]
fn media_loop(
    cmd_rx: mpsc::Receiver<MediaCommand>,
    evt_tx: mpsc::Sender<MediaEvent>,
) {
    info!("media engine started (voice feature DISABLED — stub only)");

    loop {
        match cmd_rx.recv() {
            Ok(MediaCommand::StartCapture) => {
                let _ = evt_tx.send(MediaEvent::Error(
                    "voice feature not enabled at compile time".into(),
                ));
            }
            Ok(MediaCommand::Shutdown) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    info!("media engine exited");
}
