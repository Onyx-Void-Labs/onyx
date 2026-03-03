// ─── Media Engine ──────────────────────────────────────────────────
// Phase 3: The Senses — MoQ Voice Chat
//
// Architecture:
//   MediaEngine runs on a dedicated background thread.  Audio
//   processing never blocks the Makepad UI or the Loro CRDT sync.
//
//   Capture Pipeline:
//     Mic (cpal native rate) → accumulate N samples (20ms native)
//       → downmix stereo→mono → resample to 48kHz (if needed)
//       → VAD gate (200ms rolling buffer, zero syllable cutoff)
//       → Opus encode → AudioFrame event → NetBridge → QUIC datagram
//
//   Playback Pipeline:
//     QUIC datagram → NetBridge → IncomingAudio command
//       → per-peer Opus decoder → resample 48kHz→native (if needed)
//       → f32 PCM → playback VecDeque → cpal output stream
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
use tracing::info;
#[cfg(feature = "voice")]
use tracing::{warn, error, trace};

#[cfg(feature = "voice")]
/// Opus always operates at 48 kHz internally.
const OPUS_RATE: u32 = 48000;

#[cfg(feature = "voice")]
/// Opus frame size: 20ms at 48 kHz mono = 960 samples.
const FRAME_SIZE: usize = 960;

#[cfg(feature = "voice")]
/// Maximum encoded Opus frame (bytes).  Opus rarely exceeds 500 B
/// for mono voice at 48 kHz, but we keep headroom.
const MAX_OPUS_BYTES: usize = 4000;

#[cfg(feature = "voice")]
/// VAD: RMS energy threshold for voice detection.
/// Typical ambient noise is ~0.002–0.005 RMS; voice starts ~0.01+.
const VAD_THRESHOLD: f32 = 0.008;

#[cfg(feature = "voice")]
/// VAD: Number of 20ms frames to keep in the rolling pre-buffer (200ms).
const VAD_RING_FRAMES: usize = 10;

#[cfg(feature = "voice")]
/// VAD: Number of 20ms trailing frames to keep sending after voice stops (200ms).
const VAD_TRAILING_FRAMES: u32 = 10;

#[cfg(feature = "voice")]
/// Keep-alive: send a silence frame every N silent frames (~5s = 250 × 20ms).
/// This prevents the QUIC media connection from timing out during long silence.
/// We use a very long interval so we don't leak bandwidth during muted periods.
const KEEPALIVE_INTERVAL_FRAMES: u64 = 250;

// ── Commands (UI → MediaEngine) ─────────────────────────────────

/// Commands from the UI/App thread into the media engine.
#[derive(Debug)]
#[allow(dead_code)] // Fields read only in #[cfg(feature = "voice")] build
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
#[allow(dead_code)] // Variants constructed only in #[cfg(feature = "voice")] build
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

// -- Resampling --

/// Linear interpolation resampler.  Good enough for voice (Opus is
/// lossy anyway). Converts `input` from `in_rate` Hz to `out_rate` Hz
/// producing exactly `out_len` samples.
#[cfg(feature = "voice")]
fn resample(input: &[f32], in_rate: u32, out_rate: u32, out_len: usize) -> Vec<f32> {
    if in_rate == out_rate {
        return input.to_vec();
    }
    let ratio = in_rate as f64 / out_rate as f64;
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = input.get(idx).copied().unwrap_or(0.0);
        let s1 = input.get(idx + 1).copied().unwrap_or(s0);
        output.push(s0 * (1.0 - frac) + s1 * frac);
    }
    output
}

/// Compute RMS (root-mean-square) energy of a PCM frame.
#[cfg(feature = "voice")]
fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

// ── VAD state machine ───────────────────────────────────────────

#[cfg(feature = "voice")]
#[derive(Debug, Clone, Copy, PartialEq)]
enum VadState {
    /// Below threshold — buffering into ring, not sending.
    Silent,
    /// Above threshold — sending live frames.
    Speaking,
    /// Voice just stopped — keep sending for `remaining` more frames.
    Trailing { remaining: u32 },
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

    // Query the native input config (sample rate the hardware actually wants).
    let (input_native_rate, input_channels, input_sample_format) = if let Some(ref dev) = input_device {
        match dev.default_input_config() {
            Ok(cfg) => {
                let rate = cfg.sample_rate().0;
                let ch = cfg.channels();
                let fmt = cfg.sample_format();
                info!(
                    name = %dev.name().unwrap_or_default(),
                    sample_rate = rate,
                    channels = ch,
                    sample_format = ?fmt,
                    "input device (native config)"
                );
                (rate, ch, fmt)
            }
            Err(e) => {
                warn!(%e, "failed to query input config, falling back to 48kHz mono F32");
                (OPUS_RATE, 1, cpal::SampleFormat::F32)
            }
        }
    } else {
        warn!("no input audio device available");
        (OPUS_RATE, 1, cpal::SampleFormat::F32)
    };

    // Query the native output config.
    let (output_native_rate, output_channels) = if let Some(ref dev) = output_device {
        match dev.default_output_config() {
            Ok(cfg) => {
                let rate = cfg.sample_rate().0;
                let ch = cfg.channels();
                info!(
                    name = %dev.name().unwrap_or_default(),
                    sample_rate = rate,
                    channels = ch,
                    "output device (native config)"
                );
                (rate, ch)
            }
            Err(e) => {
                warn!(%e, "failed to query output config, falling back to 48kHz mono");
                (OPUS_RATE, 1)
            }
        }
    } else {
        warn!("no output audio device available");
        (OPUS_RATE, 1)
    };

    // Calculate the native frame size for 20ms of audio at the input rate.
    // e.g. at 44100 Hz: 44100 * 20 / 1000 = 882 samples.
    let input_native_frame_size = (input_native_rate as usize * 20) / 1000;
    info!(
        input_native_rate,
        input_native_frame_size,
        opus_rate = OPUS_RATE,
        opus_frame_size = FRAME_SIZE,
        needs_input_resample = (input_native_rate != OPUS_RATE),
        "audio config resolved"
    );

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
    // Each message is one 20ms frame of MONO f32 PCM at the INPUT native rate.
    let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<f32>>();

    // ── VAD state ────────────────────────────────────────────────
    let mut vad_state = VadState::Silent;
    // Rolling ring buffer: stores the last VAD_RING_FRAMES frames for
    // zero-syllable-cutoff prepend.
    let mut vad_ring: VecDeque<Vec<f32>> = VecDeque::with_capacity(VAD_RING_FRAMES + 1);

    // Packet send counter for logging.
    let mut opus_packets_sent: u64 = 0;

    // Keep-alive: count silent frames to know when to send a silence packet.
    let mut silent_frame_counter: u64 = 0;

    // ── Main loop ────────────────────────────────────────────────
    // CRITICAL FIX: drain ALL pending commands per iteration instead
    // of processing only one.  This prevents IncomingAudio frames
    // from piling up in the channel and starving the playback buffer.
    'outer: loop {
        // ── 1. Drain ALL pending commands ────────────────────────
        loop {
            match cmd_rx.try_recv() {
                Ok(MediaCommand::StartCapture) => {
                    if capturing {
                        continue; // inner loop — drain next command
                    }
                    info!("starting audio capture + playback");

                    // — Opus Encoder (always 48kHz, Voip for low latency) —
                    match OpusEncoder::new(
                        SampleRate::Hz48000,
                        Channels::Mono,
                        Application::Voip,
                    ) {
                        Ok(enc) => {
                            info!("Opus encoder created (48kHz, mono, VOIP mode)");
                            encoder = Some(enc);
                        }
                        Err(e) => {
                            let msg = format!("Opus encoder creation failed: {e}");
                            error!("{msg}");
                            let _ = evt_tx.send(MediaEvent::Error(msg));
                            continue; // inner loop — try next command
                        }
                    }

                    // — cpal input stream (microphone) —
                    if let Some(ref device) = input_device {
                        let cfg = cpal::StreamConfig {
                            channels: input_channels,
                            sample_rate: cpal::SampleRate(input_native_rate),
                            buffer_size: cpal::BufferSize::Default,
                        };

                        let build_result = match input_sample_format {
                            cpal::SampleFormat::I16 => {
                                info!("building i16 input stream (native format)");
                                let tx = pcm_tx.clone();
                                let ch = input_channels;
                                let native_frame = input_native_frame_size;
                                let mut acc: Vec<f32> = Vec::with_capacity(native_frame * 2);

                                device.build_input_stream(
                                    &cfg,
                                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                        if ch > 1 {
                                            for chunk in data.chunks(ch as usize) {
                                                let mono: f32 = chunk
                                                    .iter()
                                                    .map(|&s| s as f32 / 32768.0)
                                                    .sum::<f32>()
                                                    / ch as f32;
                                                acc.push(mono);
                                            }
                                        } else {
                                            for &s in data {
                                                acc.push(s as f32 / 32768.0);
                                            }
                                        }
                                        while acc.len() >= native_frame {
                                            let frame: Vec<f32> =
                                                acc.drain(..native_frame).collect();
                                            let _ = tx.send(frame);
                                        }
                                    },
                                    |err| error!("cpal input error: {err}"),
                                    None,
                                )
                            }
                            _ => {
                                info!(
                                    format = ?input_sample_format,
                                    "building f32 input stream"
                                );
                                let tx = pcm_tx.clone();
                                let ch = input_channels;
                                let native_frame = input_native_frame_size;
                                let mut acc: Vec<f32> = Vec::with_capacity(native_frame * 2);

                                device.build_input_stream(
                                    &cfg,
                                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                        if ch > 1 {
                                            for chunk in data.chunks(ch as usize) {
                                                let mono: f32 =
                                                    chunk.iter().sum::<f32>() / ch as f32;
                                                acc.push(mono);
                                            }
                                        } else {
                                            acc.extend_from_slice(data);
                                        }
                                        while acc.len() >= native_frame {
                                            let frame: Vec<f32> =
                                                acc.drain(..native_frame).collect();
                                            let _ = tx.send(frame);
                                        }
                                    },
                                    |err| error!("cpal input error: {err}"),
                                    None,
                                )
                            }
                        };

                        match build_result {
                            Ok(stream) => {
                                match stream.play() {
                                    Ok(()) => info!("cpal input stream: play() succeeded"),
                                    Err(e) => error!("cpal input stream: play() FAILED: {e}"),
                                }
                                _input_stream = Some(stream);
                                info!(
                                    sample_rate = input_native_rate,
                                    channels = input_channels,
                                    frame_size = input_native_frame_size,
                                    sample_format = ?input_sample_format,
                                    "cpal input stream started (native config)"
                                );
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
                            channels: output_channels,
                            sample_rate: cpal::SampleRate(output_native_rate),
                            buffer_size: cpal::BufferSize::Default,
                        };
                        let pb = Arc::clone(&playback_buf);
                        let out_ch = output_channels;

                        match device.build_output_stream(
                            &cfg,
                            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                let mut buf = pb.lock().unwrap();
                                if out_ch > 1 {
                                    for frame_samples in data.chunks_mut(out_ch as usize) {
                                        let sample = buf.pop_front().unwrap_or(0.0);
                                        for s in frame_samples.iter_mut() {
                                            *s = sample;
                                        }
                                    }
                                } else {
                                    for sample in data.iter_mut() {
                                        *sample = buf.pop_front().unwrap_or(0.0);
                                    }
                                }
                            },
                            |err| error!("cpal output error: {err}"),
                            None,
                        ) {
                            Ok(stream) => {
                                match stream.play() {
                                    Ok(()) => info!("cpal output stream: play() succeeded"),
                                    Err(e) => error!("cpal output stream: play() FAILED: {e}"),
                                }
                                _output_stream = Some(stream);
                                info!(
                                    sample_rate = output_native_rate,
                                    channels = output_channels,
                                    "cpal output stream started (native config)"
                                );
                            }
                            Err(e) => {
                                let msg = format!("output stream open failed: {e}");
                                error!("{msg}");
                                let _ = evt_tx.send(MediaEvent::Error(msg));
                            }
                        }
                    }

                    // Reset VAD state.
                    vad_state = VadState::Silent;
                    vad_ring.clear();
                    opus_packets_sent = 0;
                    silent_frame_counter = 0;

                    capturing = true;
                    let _ = evt_tx.send(MediaEvent::CaptureStarted);
                }

                Ok(MediaCommand::StopCapture) => {
                    if !capturing {
                        continue; // inner loop — drain next command
                    }
                    info!(
                        total_opus_packets = opus_packets_sent,
                        "stopping audio capture + playback"
                    );
                    _input_stream = None;
                    _output_stream = None;
                    encoder = None;
                    decoders.clear();
                    if let Ok(mut buf) = playback_buf.lock() {
                        buf.clear();
                    }
                    vad_state = VadState::Silent;
                    vad_ring.clear();
                    capturing = false;
                    let _ = evt_tx.send(MediaEvent::CaptureStopped);
                }

                Ok(MediaCommand::IncomingAudio { from, data }) => {
                    if data.is_empty() {
                        continue; // inner loop — drain next command
                    }

                    // INFO-level to help debug the playback pipeline.
                    info!(
                        from = %from,
                        bytes = data.len(),
                        "decoding incoming audio frame for playback"
                    );

                    // Get or create a per-peer Opus decoder.
                    let decoder = decoders
                        .entry(from.clone())
                        .or_insert_with(|| {
                            info!(peer = %from, "creating new Opus decoder for peer");
                            OpusDecoder::new(SampleRate::Hz48000, Channels::Mono)
                                .expect("failed to create Opus decoder")
                        });

                    // Decode Opus → i16 PCM at 48kHz.
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
                            // Convert i16 → f32.
                            let pcm_f32: Vec<f32> = pcm[..samples]
                                .iter()
                                .map(|&s| s as f32 / 32768.0)
                                .collect();

                            // Resample 48kHz → native output rate if needed.
                            let output_samples = if output_native_rate != OPUS_RATE {
                                let out_len = (samples as u64
                                    * output_native_rate as u64
                                    / OPUS_RATE as u64)
                                    as usize;
                                resample(&pcm_f32, OPUS_RATE, output_native_rate, out_len)
                            } else {
                                pcm_f32
                            };

                            // Push into playback ring.
                            if let Ok(mut buf) = playback_buf.lock() {
                                let before = buf.len();
                                for s in &output_samples {
                                    buf.push_back(*s);
                                }
                                info!(
                                    before_samples = before,
                                    added_samples = output_samples.len(),
                                    total_samples = buf.len(),
                                    "pushed decoded audio to playback buffer"
                                );
                                // Cap to ~200ms at native output rate.
                                let max_buf = (output_native_rate as usize * 200) / 1000;
                                while buf.len() > max_buf {
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
                    info!(
                        total_opus_packets = opus_packets_sent,
                        "media engine shutting down"
                    );
                    _input_stream = None;
                    _output_stream = None;
                    break 'outer;
                }

                Err(mpsc::TryRecvError::Empty) => break, // inner loop → process PCM
                Err(mpsc::TryRecvError::Disconnected) => {
                    info!("media command channel closed");
                    break 'outer;
                }
            }
        }

        // ── 2. Encode captured PCM frames (with VAD) ─────────────
        if capturing {
            while let Ok(pcm_native) = pcm_rx.try_recv() {
                // Resample native rate → 48kHz for Opus encoding.
                let pcm_48k = if input_native_rate != OPUS_RATE {
                    resample(&pcm_native, input_native_rate, OPUS_RATE, FRAME_SIZE)
                } else {
                    pcm_native
                };

                // ── VAD: voice activity detection ──
                let energy = rms_energy(&pcm_48k);
                let voice_detected = energy > VAD_THRESHOLD;

                // Advance VAD state machine.
                let should_send = match vad_state {
                    VadState::Silent => {
                        if voice_detected {
                            // Voice just started — flush the ring buffer
                            // (past 200ms) so no syllables are lost.
                            info!(
                                energy = format!("{energy:.4}"),
                                ring_frames = vad_ring.len(),
                                "VAD: voice started — flushing ring buffer"
                            );
                            vad_state = VadState::Speaking;
                            silent_frame_counter = 0;

                            // Encode and send the buffered frames first.
                            if let Some(ref mut enc) = encoder {
                                for buffered in vad_ring.drain(..) {
                                    let pcm_i16: Vec<i16> = buffered
                                        .iter()
                                        .map(|&s| {
                                            (s * 32767.0).clamp(-32768.0, 32767.0)
                                                as i16
                                        })
                                        .collect();
                                    let mut opus_buf = vec![0u8; MAX_OPUS_BYTES];
                                    if let Ok(len) =
                                        enc.encode(&pcm_i16, &mut opus_buf)
                                    {
                                        opus_buf.truncate(len);
                                        opus_packets_sent += 1;
                                        let _ = evt_tx.send(
                                            MediaEvent::AudioFrame(opus_buf),
                                        );
                                    }
                                }
                            }
                            true // also send current frame
                        } else {
                            // ── SILENT: buffer in ring, do NOT send ──
                            // This is the key fix: during silence, we
                            // skip sending entirely to save bandwidth.
                            vad_ring.push_back(pcm_48k.clone());
                            if vad_ring.len() > VAD_RING_FRAMES {
                                vad_ring.pop_front();
                            }

                            // Rare keep-alive (~5s) to prevent QUIC timeout.
                            silent_frame_counter += 1;
                            if silent_frame_counter >= KEEPALIVE_INTERVAL_FRAMES {
                                silent_frame_counter = 0;
                                trace!("VAD: sending rare keep-alive silence frame");
                                true
                            } else {
                                false // skip — don't send during silence
                            }
                        }
                    }
                    VadState::Speaking => {
                        if voice_detected {
                            true // keep sending
                        } else {
                            // Voice just stopped — enter trailing period.
                            vad_state =
                                VadState::Trailing { remaining: VAD_TRAILING_FRAMES };
                            trace!(
                                energy = format!("{energy:.4}"),
                                "VAD: voice stopped — entering trailing period"
                            );
                            true // send this frame (start of trail)
                        }
                    }
                    VadState::Trailing { remaining } => {
                        if voice_detected {
                            // Voice resumed during trail — back to speaking.
                            vad_state = VadState::Speaking;
                            true
                        } else if remaining > 0 {
                            vad_state =
                                VadState::Trailing { remaining: remaining - 1 };
                            true // still in trail
                        } else {
                            // Trail expired — go silent, start buffering.
                            vad_state = VadState::Silent;
                            vad_ring.clear();
                            vad_ring.push_back(pcm_48k.clone());
                            info!("VAD: trailing ended — returning to silent (no more packets)");
                            false
                        }
                    }
                };

                // ── Encode + send if VAD says go ──
                if should_send {
                    if let Some(ref mut enc) = encoder {
                        let pcm_i16: Vec<i16> = pcm_48k
                            .iter()
                            .map(|&s| {
                                (s * 32767.0).clamp(-32768.0, 32767.0) as i16
                            })
                            .collect();

                        let mut opus_buf = vec![0u8; MAX_OPUS_BYTES];
                        match enc.encode(&pcm_i16, &mut opus_buf) {
                            Ok(len) => {
                                opus_buf.truncate(len);
                                opus_packets_sent += 1;

                                if opus_packets_sent <= 5
                                    || opus_packets_sent % 50 == 0
                                {
                                    info!(
                                        opus_bytes = len,
                                        packet_num = opus_packets_sent,
                                        energy = format!("{energy:.4}"),
                                        vad = ?vad_state,
                                        "Opus packet sent"
                                    );
                                }
                                let _ =
                                    evt_tx.send(MediaEvent::AudioFrame(opus_buf));
                            }
                            Err(e) => {
                                warn!(%e, "Opus encode error");
                            }
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
