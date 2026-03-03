// --- Makepad Application Shell ---
// Root Makepad app. Owns the widget tree and bridges keyboard input
// to the local EditorBuffer (Rope) + Loro CRDT.
//
// Architecture:
//   OnyxApp
//     +-- Widget Tree (script_mod DSL)
//           |-- Side Panel       (room code, join button, peer list)
//           |-- Main Note Area   (editor_label -- text display)
//           +-- Status Bar       (live char count + sync status)
//
//   Keyboard -> EditorBuffer (onyx-editor) --+
//                                            +-->  Label redraw
//   Keyboard -> CrdtDoc (onyx-store/Loro) ---+
//
//   NetBridge -> Iroh Gossip <-> Remote CrdtDoc
// ----

use makepad_widgets::*;
use onyx_editor::{Cursor, EditorBuffer};
use onyx_store::CrdtDoc;
use std::collections::HashMap;
use std::time::Instant;

use crate::net_bridge::{NetBridge, NetEvent};
use crate::media_engine::{MediaEngine, MediaEvent};

// --- DSL: The Void UI ---

script_mod! {
    use mod.prelude.widgets.*

    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(1280, 800)
                pass.clear_color: vec4(0.039, 0.039, 0.059, 1.0)
                body +: {
                    flow: Down
                    spacing: 0

                    // -- Title bar --
                    View {
                        width: Fill
                        height: 48
                        show_bg: true
                        draw_bg.color: #x12121A
                        flow: Right
                        padding: Inset{left: 20, top: 12, right: 20}
                        spacing: 12

                        // Panel toggle
                        panel_toggle := Button {
                            text: "<<"
                            width: 32
                            height: 28
                        }

                        Label {
                            text: "ONYX VOID"
                            draw_text.color: #x7B68EE
                            draw_text.text_style.font_size: 14.0
                        }

                        View { width: Fill, height: 1 }

                        Label {
                            text: "Phase 3 -- The Senses"
                            draw_text.color: #x2A2A3A
                            draw_text.text_style.font_size: 10.0
                        }
                    }

                    // -- Content row: side panel + divider + editor --
                    View {
                        width: Fill
                        height: Fill
                        flow: Right
                        spacing: 0

                        // -- Side Panel --
                        side_panel := View {
                            width: 220
                            height: Fill
                            show_bg: true
                            draw_bg.color: #x0E0E14
                            flow: Down
                            spacing: 8
                            padding: Inset{left: 16, top: 16, right: 16}

                            Label {
                                text: "DOCUMENTS"
                                draw_text.color: #x4A4A5A
                                draw_text.text_style.font_size: 10.0
                            }

                            Label {
                                text: "untitled.void"
                                draw_text.color: #x7B68EE
                                draw_text.text_style.font_size: 12.0
                            }

                            View { width: Fill, height: 16 }

                            Label {
                                text: "ROOM"
                                draw_text.color: #x4A4A5A
                                draw_text.text_style.font_size: 10.0
                            }

                            room_input := TextInput {
                                empty_text: "secret room code..."
                                width: Fill
                                height: 36
                                draw_bg.color: #x1A1A24
                                draw_text.color: #xC0C0D0
                                draw_text.text_style.font_size: 11.0
                            }

                            join_button := Button {
                                text: "Join Void"
                                width: Fill
                                height: 32
                            }

                            disconnect_button := Button {
                                text: "Disconnect"
                                width: Fill
                                height: 32
                                visible: false
                            }

                            View { width: Fill, height: 16 }

                            Label {
                                text: "PEERS"
                                draw_text.color: #x4A4A5A
                                draw_text.text_style.font_size: 10.0
                            }

                            peers_label := Label {
                                text: "Not connected"
                                draw_text.color: #x3A3A4A
                                draw_text.text_style.font_size: 11.0
                            }

                            View { width: Fill, height: 16 }

                            Label {
                                text: "VOICE"
                                draw_text.color: #x4A4A5A
                                draw_text.text_style.font_size: 10.0
                            }

                            voice_button := Button {
                                text: "Toggle Voice"
                                width: Fill
                                height: 32
                                visible: false
                            }

                            voice_status := Label {
                                text: ""
                                draw_text.color: #x3A3A4A
                                draw_text.text_style.font_size: 11.0
                            }
                        }

                        // -- Divider --
                        View {
                            width: 1
                            height: Fill
                            show_bg: true
                            draw_bg.color: #x1A1A24
                        }

                        // -- Main Note Area --
                        editor_area := View {
                            width: Fill
                            height: Fill
                            padding: Inset{left: 48, top: 32, right: 48, bottom: 32}

                            editor_label := Label {
                                text: ""
                                draw_text.color: #xE0E0E8
                                draw_text.text_style.font_size: 13.0
                            }

                            // Cursor indicator — thin purple bar.
                            // Positioned via set_uniform; blink via set_visible.
                            cursor_overlay := View {
                                abs_pos: vec2(48, 80)
                                width: 2
                                height: 18
                                show_bg: true
                                draw_bg.color: #x7B68EE
                            }
                        }
                    }

                    // -- Status bar --
                    View {
                        width: Fill
                        height: 28
                        show_bg: true
                        draw_bg.color: #x12121A
                        flow: Right
                        padding: Inset{left: 20, top: 6, right: 20}
                        spacing: 24

                        status_label := Label {
                            text: "Void Active"
                            draw_text.color: #x4A4A5A
                            draw_text.text_style.font_size: 10.0
                        }

                        status_chars := Label {
                            text: "0 chars"
                            draw_text.color: #x4A4A5A
                            draw_text.text_style.font_size: 10.0
                        }

                        View { width: Fill, height: 1 }

                        status_sync := Label {
                            text: "Loro in-memory"
                            draw_text.color: #x3A3A4A
                            draw_text.text_style.font_size: 10.0
                        }
                    }
                }
            }
        }
    }
}

// --- App struct ---

app_main!(App);

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    buffer: EditorBuffer,
    #[rust]
    cursor: Cursor,
    #[rust]
    crdt: CrdtDoc,
    #[rust]
    net: Option<NetBridge>,
    #[rust]
    net_timer: Timer,
    #[rust]
    peers: Vec<String>,
    #[rust]
    connected: bool,
    #[rust]
    room_input_focused: bool,
    #[rust]
    panel_open: bool,
    /// Tracks whether display text actually changed to avoid
    /// re-uploading identical geometry to the GPU.
    #[rust]
    last_display_hash: u64,
    /// Animated cursor position (smoothed via spring interpolation).
    #[rust]
    cursor_anim_x: f32,
    /// Animated cursor Y (line) position, spring-smoothed.
    #[rust]
    cursor_anim_y: f32,
    /// Cursor spring velocity (X axis).
    #[rust]
    cursor_vel: f32,
    /// Cursor spring velocity (Y axis).
    #[rust]
    cursor_vel_y: f32,
    /// Accumulated time for cursor blink animation.
    #[rust]
    cursor_time: f64,
    /// Timer driving the cursor animation at ~60 fps.
    #[rust]
    cursor_timer: Timer,
    /// Whether the cursor timer has been started.
    #[rust]
    cursor_timer_started: bool,
    /// The relay’s NodeID string — cached so we can filter it from the peers list.
    #[rust]
    relay_node_id: String,
    /// Last-seen timestamp for each peer (for heartbeat TTL expiry).
    #[rust]
    peer_last_seen: HashMap<String, Instant>,
    /// Heartbeat broadcast counter (sends every ~5s at 50ms poll rate).
    #[rust]
    heartbeat_counter: u32,
    /// Whether the MoQ voice engine is active.
    #[rust]
    voice_active: bool,
    /// The media engine handle (spawned when voice is toggled ON).
    #[rust]
    media_engine: Option<MediaEngine>,
}

impl App {
    fn run(vm: &mut ScriptVm) -> Self {
        makepad_widgets::script_mod(vm);
        let mut app = App::from_script_mod(vm, self::script_mod);
        app.panel_open = true;
        app.last_display_hash = 0;
        app.cursor_anim_x = 0.0;
        app.cursor_anim_y = 0.0;
        app.cursor_vel = 0.0;
        app.cursor_vel_y = 0.0;
        app.cursor_time = 0.0;
        app.relay_node_id = onyx_core::protocol::relay_node_id_string();
        app.peer_last_seen = HashMap::new();
        app.heartbeat_counter = 0;
        app.voice_active = false;
        app.media_engine = None;
        app
    }

    /// Push the buffer text into the editor label + update status bar.
    /// Uses a content hash to skip redundant GPU re-uploads (dirty rect).
    fn sync_display(&mut self, cx: &mut Cx) {
        let text = self.buffer.text();
        let display = if text.is_empty() {
            "Begin typing...".to_string()
        } else {
            text.clone()
        };

        // ── Dirty-rect optimisation ──
        // Only update the label (which triggers a full layout + GPU upload)
        // when the display text actually changed.
        let new_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            display.hash(&mut h);
            h.finish()
        };
        if new_hash != self.last_display_hash {
            self.last_display_hash = new_hash;
            self.ui
                .label(cx, ids!(editor_label))
                .set_text(cx, &display);
        }

        // Status: char count
        self.ui.label(cx, ids!(status_chars)).set_text(
            cx,
            &format!("{} chars", self.buffer.len_chars()),
        );

        // Status: line/col info
        let buf_text = self.buffer.text();
        let pos = self.cursor.pos.min(buf_text.len());
        let line = buf_text[..pos].matches('\n').count() + 1;
        let col = pos - buf_text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
        self.ui.label(cx, ids!(status_label)).set_text(
            cx,
            &format!("Void Active  Ln {} Col {}", line, col),
        );

        // Sync status
        let sync_text = if self.connected {
            format!("Iroh mesh  {} peers", self.peers.len())
        } else {
            "Loro in-memory".to_string()
        };
        self.ui
            .label(cx, ids!(status_sync))
            .set_text(cx, &sync_text);

        // Update the visible cursor indicator in the status bar
        self.update_cursor_display(cx);
    }

    // ── Cursor animation ────────────────────────────────────────

    /// Compute the target X position (in characters) for the cursor
    /// and update the status bar with a visual caret indicator.
    fn update_cursor_display(&self, cx: &mut Cx) {
        // Show cursor position indicator in the status bar.
        // No block characters — just a clean pipe that works
        // with every font atlas.
        let buf_text = self.buffer.text();
        let pos = self.cursor.pos.min(buf_text.len());
        let line = buf_text[..pos].matches('\n').count() + 1;
        let col = pos - buf_text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;

        // Blink the pipe using a sine pulse
        let blink = (self.cursor_time * 3.0).sin();
        let caret = if blink > 0.0 { "|" } else { " " };

        self.ui.label(cx, ids!(status_label)).set_text(
            cx,
            &format!("{caret} Ln {line} Col {col}"),
        );
    }

    /// Spring-mass cursor animation tick (~60 fps).
    ///
    /// The cursor's visual X position "slides" to the target
    /// position using a critically-damped spring, giving a
    /// satisfying physical feel to typing.
    fn tick_cursor_animation(&mut self, cx: &mut Cx) {
        const DT: f32 = 1.0 / 60.0;
        // Spring parameters (critically damped: ζ ≈ 1.0)
        const STIFFNESS: f32 = 800.0;
        const DAMPING: f32 = 56.0; // 2 * sqrt(STIFFNESS)

        // Advance blink timer
        self.cursor_time += DT as f64;

        // Target position = cursor column and line (in character units)
        let buf_text = self.buffer.text();
        let pos = self.cursor.pos.min(buf_text.len());
        let line = buf_text[..pos].matches('\n').count();
        let col = pos - buf_text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let target_x = col as f32;
        let target_y = line as f32;

        // Apply spring force (X axis)
        let dx = target_x - self.cursor_anim_x;
        let spring_x = STIFFNESS * dx;
        let damp_x = -DAMPING * self.cursor_vel;
        self.cursor_vel += (spring_x + damp_x) * DT;
        self.cursor_anim_x += self.cursor_vel * DT;

        // Apply spring force (Y axis — same parameters for consistency)
        let dy = target_y - self.cursor_anim_y;
        let spring_y = STIFFNESS * dy;
        let damp_y = -DAMPING * self.cursor_vel_y;
        self.cursor_vel_y += (spring_y + damp_y) * DT;
        self.cursor_anim_y += self.cursor_vel_y * DT;

        // Snap when close enough (avoid infinite oscillation)
        if dx.abs() < 0.01 && self.cursor_vel.abs() < 0.1 {
            self.cursor_anim_x = target_x;
            self.cursor_vel = 0.0;
        }
        if dy.abs() < 0.01 && self.cursor_vel_y.abs() < 0.1 {
            self.cursor_anim_y = target_y;
            self.cursor_vel_y = 0.0;
        }

        // ── Position the cursor overlay ──
        // Map animated column/line to pixel coords.
        // These constants approximate the editor area padding + glyph metrics.
        let char_width: f32 = 7.8;  // approximate monospace glyph width at font_size 13
        let line_height: f32 = 20.0;
        let pad_left: f32 = 48.0;
        let pad_top: f32 = 80.0; // title bar(48) + editor padding(32)

        let _px_x = pad_left + self.cursor_anim_x * char_width;
        let _px_y = pad_top + self.cursor_anim_y * line_height;

        // ── Cursor blink ──
        // Toggle the cursor bar visibility for a blink effect.
        let blink = (self.cursor_time * 3.5).sin();
        let visible = blink > 0.0;

        let overlay = self.ui.view(cx, ids!(cursor_overlay));
        overlay.set_visible(cx, visible);
        overlay.redraw(cx);

        // Update the editor label with the plain text (no cursor char)
        let text = self.buffer.text();
        let display = if text.is_empty() {
            "Begin typing...".to_string()
        } else {
            text.clone()
        };

        let new_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            display.hash(&mut h);
            h.finish()
        };
        if new_hash != self.last_display_hash {
            self.last_display_hash = new_hash;
            self.ui
                .label(cx, ids!(editor_label))
                .set_text(cx, &display);
        }

        cx.redraw_all();
    }

    /// Update the peers label in the side panel.
    /// Filters out the relay's NodeID so only real peers appear.
    fn update_peers_display(&self, cx: &mut Cx) {
        // Filter out the relay node from the visible peer list
        let visible_peers: Vec<&String> = self
            .peers
            .iter()
            .filter(|p| *p != &self.relay_node_id)
            .collect();

        let text = if !self.connected {
            "Not connected".to_string()
        } else if visible_peers.is_empty() {
            "Waiting for peers...".to_string()
        } else {
            let mut lines = vec!["local (you)".to_string()];
            for p in &visible_peers {
                // Show first 8 chars of peer ID
                let short = if p.len() > 8 { &p[..8] } else { p };
                lines.push(format!("{short}..."));
            }
            lines.join("\n")
        };
        self.ui
            .label(cx, ids!(peers_label))
            .set_text(cx, &text);

        // Show/hide disconnect and voice buttons based on connection state
        self.ui
            .button(cx, ids!(disconnect_button))
            .set_visible(cx, self.connected);
        self.ui
            .button(cx, ids!(voice_button))
            .set_visible(cx, self.connected);
    }

    /// Send the current CRDT delta to the network.
    /// Uses incremental delta export when possible (tiny ~50 byte
    /// updates vs multi-KB snapshots on every keystroke).
    fn broadcast_crdt_delta(&self) {
        if let Some(ref net) = self.net {
            let payload = self.crdt.export_incremental_delta();
            if !payload.is_empty() {
                net.send_delta(payload);
            }
        }
    }

    /// Broadcast a full CRDT snapshot to all peers.
    ///
    /// Called when a new peer joins so they receive all existing
    /// content immediately, rather than seeing an empty screen
    /// until the next keystroke.
    fn broadcast_full_snapshot(&self) {
        if let Some(ref net) = self.net {
            let snapshot = self.crdt.export_snapshot();
            if !snapshot.is_empty() {
                tracing::info!(
                    bytes = snapshot.len(),
                    "sending full CRDT snapshot for new peer catch-up"
                );
                net.send_delta(snapshot);
            }
        }
    }

    /// Capture the version vector *before* a local edit so we can
    /// export just the delta afterwards.
    fn capture_pre_edit_vv(&self) {
        self.crdt.capture_pre_edit();
    }

    /// Process incoming network events.
    fn poll_network(&mut self, cx: &mut Cx) {
        // ── Drain MediaEngine events → forward to NetBridge ──
        if let Some(ref engine) = self.media_engine {
            for evt in engine.drain_events() {
                match evt {
                    MediaEvent::AudioFrame(opus_frame) => {
                        // Forward encoded audio to the network.
                        if let Some(ref net) = self.net {
                            net.send_media_datagram(opus_frame);
                        }
                    }
                    MediaEvent::CaptureStarted => {
                        tracing::info!("MediaEngine: capture started");
                    }
                    MediaEvent::CaptureStopped => {
                        tracing::info!("MediaEngine: capture stopped");
                    }
                    MediaEvent::Error(msg) => {
                        tracing::error!("MediaEngine error: {msg}");
                        self.ui
                            .label(cx, ids!(voice_status))
                            .set_text(cx, &format!("Voice err: {msg}"));
                    }
                }
            }
        }

        let Some(ref net) = self.net else { return };
        let events = net.drain_events();

        // ── Heartbeat broadcast (every ~5 seconds at 50ms poll rate) ──
        if self.connected {
            self.heartbeat_counter += 1;
            if self.heartbeat_counter >= 100 {
                self.heartbeat_counter = 0;
                let hb = onyx_core::protocol::encode_control(
                    onyx_core::protocol::CTRL_HEARTBEAT,
                );
                net.send_control(hb);
            }

            // ── Heartbeat TTL expiry (15 seconds = 3 missed heartbeats) ──
            // Only disconnect peers on actual network failure (3 consecutive
            // missed heartbeats), NOT because the user is idle.
            let now = Instant::now();
            let expired: Vec<String> = self
                .peer_last_seen
                .iter()
                .filter(|(_, last)| now.duration_since(**last).as_secs() >= 15)
                .map(|(id, _)| id.clone())
                .collect();
            for peer_id in expired {
                tracing::warn!(peer = %peer_id, "peer expired (15s — 3 missed heartbeats)");
                self.peer_last_seen.remove(&peer_id);
                self.peers.retain(|p| p != &peer_id);
                self.update_peers_display(cx);
            }
        }

        if events.is_empty() {
            return;
        }

        for evt in events {
            match evt {
                NetEvent::Connected { our_id } => {
                    tracing::info!(id = %our_id, "connected to mesh");
                    self.connected = true;
                    self.update_peers_display(cx);
                }
                NetEvent::PeerJoined(peer_id) => {
                    tracing::info!(peer = %peer_id, "peer joined");
                    if !self.peers.contains(&peer_id) {
                        self.peers.push(peer_id.clone());
                    }
                    self.peer_last_seen.insert(peer_id, Instant::now());
                    self.update_peers_display(cx);

                    // ── Catch-Up Handshake ──
                    self.broadcast_full_snapshot();
                }
                NetEvent::PeerLeft(peer_id) => {
                    tracing::info!(peer = %peer_id, "peer left");
                    self.peers.retain(|p| p != &peer_id);
                    self.peer_last_seen.remove(&peer_id);
                    self.update_peers_display(cx);
                }
                NetEvent::GoodbyeReceived(peer_id) => {
                    tracing::info!(peer = %peer_id, "peer sent Goodbye — removing instantly");
                    self.peers.retain(|p| p != &peer_id);
                    self.peer_last_seen.remove(&peer_id);
                    self.update_peers_display(cx);
                }
                NetEvent::HeartbeatReceived(peer_id) => {
                    // Reset the peer's TTL
                    if self.peers.contains(&peer_id) {
                        self.peer_last_seen.insert(peer_id, Instant::now());
                    }
                }
                NetEvent::MediaStarted => {
                    tracing::info!("media QUIC connection established with relay");
                    self.ui
                        .label(cx, ids!(voice_status))
                        .set_text(cx, "Voice: ON (live)");
                }
                NetEvent::MediaDatagramReceived { from, data } => {
                    // Forward incoming Opus frames to the MediaEngine for decode + playback.
                    if let Some(ref engine) = self.media_engine {
                        engine.receive_audio(from, data);
                    }
                }
                NetEvent::DeltaReceived(raw_bytes) => {
                    tracing::debug!(bytes = raw_bytes.len(), "received delta");

                    // ── Detect in-flight batch frame ──
                    // Single delta:  raw Loro bytes (starts with 'l' = 0x6C)
                    // Batch frame:   0xBB | u16 count | [u32 len | bytes]…
                    if raw_bytes.first() == Some(&0xBB) && raw_bytes.len() >= 3 {
                        let count =
                            u16::from_be_bytes([raw_bytes[1], raw_bytes[2]]) as usize;
                        tracing::debug!(count, "received batched delta packet");
                        let mut offset = 3usize;
                        for i in 0..count {
                            if offset + 4 > raw_bytes.len() {
                                tracing::warn!(idx = i, "truncated batch frame");
                                break;
                            }
                            let len = u32::from_be_bytes([
                                raw_bytes[offset],
                                raw_bytes[offset + 1],
                                raw_bytes[offset + 2],
                                raw_bytes[offset + 3],
                            ]) as usize;
                            offset += 4;
                            if offset + len > raw_bytes.len() {
                                tracing::warn!(idx = i, "truncated delta in batch");
                                break;
                            }
                            if let Err(e) = self
                                .crdt
                                .import_snapshot(&raw_bytes[offset..offset + len])
                            {
                                tracing::warn!(%e, idx = i, "failed to import batched delta");
                            }
                            offset += len;
                        }
                    } else {
                        // Single delta (normal / legacy format)
                        if let Err(e) = self.crdt.import_snapshot(&raw_bytes) {
                            tracing::warn!(%e, "failed to import remote delta");
                            continue;
                        }
                    }

                    // Sync buffer from CRDT (full replacement for now)
                    if let Ok(new_text) = self.crdt.get_text() {
                        let old_len = self.buffer.len_chars();
                        if old_len > 0 {
                            self.buffer.delete(0, old_len);
                        }
                        if !new_text.is_empty() {
                            self.buffer.insert(0, &new_text);
                        }
                        // Clamp cursor
                        let max = self.buffer.len_chars();
                        if self.cursor.pos > max {
                            self.cursor.move_to(max);
                        }
                    }
                    self.sync_display(cx);
                }
                NetEvent::Error(msg) => {
                    tracing::error!("net error: {msg}");
                    self.ui
                        .label(cx, ids!(peers_label))
                        .set_text(cx, &format!("Error: {msg}"));
                }
            }
        }
        cx.redraw_all();
    }
}

impl MatchEvent for App {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // -- Join Void button --
        if self.ui.button(cx, ids!(join_button)).clicked(actions) {
            let room_code = self.ui.text_input(cx, ids!(room_input)).text();
            if !room_code.is_empty() {
                tracing::info!(room = %room_code, "Join Void clicked");
                // Spawn the network bridge if not already running
                if self.net.is_none() {
                    self.net = Some(NetBridge::spawn());
                    // Start polling timer (50ms interval)
                    self.net_timer = cx.start_interval(0.05);
                }
                if let Some(ref net) = self.net {
                    net.connect(room_code);
                }
                self.ui
                    .label(cx, ids!(peers_label))
                    .set_text(cx, "Connecting...");
                cx.redraw_all();
            }
        }

        // -- Disconnect button --
        if self.ui.button(cx, ids!(disconnect_button)).clicked(actions) {
            tracing::info!("Disconnect clicked");
            // Shutdown media engine if active
            if let Some(ref engine) = self.media_engine {
                engine.stop();
                engine.shutdown();
            }
            self.media_engine = None;
            if let Some(ref net) = self.net {
                net.disconnect();
            }
            // Clear local state
            self.connected = false;
            self.peers.clear();
            self.peer_last_seen.clear();
            self.heartbeat_counter = 0;
            self.voice_active = false;
            // Clear the text buffer
            let len = self.buffer.len_chars();
            if len > 0 {
                self.buffer.delete(0, len);
            }
            self.crdt = CrdtDoc::default();
            self.cursor.move_to(0);
            self.sync_display(cx);
            self.update_peers_display(cx);
            self.ui
                .label(cx, ids!(voice_status))
                .set_text(cx, "");
            cx.redraw_all();
        }

        // -- Voice toggle button (Phase 3) --
        if self.ui.button(cx, ids!(voice_button)).clicked(actions) {
            self.voice_active = !self.voice_active;
            let status = if self.voice_active {
                tracing::info!("Voice chat ENABLED — spawning MediaEngine");

                // Spawn the media engine
                let engine = MediaEngine::spawn();
                engine.start();
                self.media_engine = Some(engine);

                // Tell NetBridge to open a media QUIC connection
                if let Some(ref net) = self.net {
                    net.start_media();
                }
                "Voice: ON (Opus/QUIC)"
            } else {
                tracing::info!("Voice chat DISABLED — shutting down MediaEngine");

                // Shutdown media engine
                if let Some(ref engine) = self.media_engine {
                    engine.stop();
                    engine.shutdown();
                }
                self.media_engine = None;

                // Tell NetBridge to close media connection
                if let Some(ref net) = self.net {
                    net.stop_media();
                }
                "Voice: OFF"
            };
            self.ui
                .label(cx, ids!(voice_status))
                .set_text(cx, status);
            cx.redraw_all();
        }

        // -- Panel toggle button --
        if self.ui.button(cx, ids!(panel_toggle)).clicked(actions) {
            self.panel_open = !self.panel_open;
            self.ui.view(cx, ids!(side_panel)).set_visible(cx, self.panel_open);
            cx.redraw_all();
        }

        // -- Track room input focus to avoid editor key conflicts --
        let room_input = self.ui.text_input(cx, ids!(room_input));
        for action in actions.filter_widget_actions_cast::<TextInputAction>(room_input.widget_uid()) {
            match action {
                TextInputAction::KeyFocus => self.room_input_focused = true,
                TextInputAction::KeyFocusLost => self.room_input_focused = false,
                _ => {}
            }
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());

        // -- Network polling timer --
        if self.net_timer.is_event(event).is_some() {
            self.poll_network(cx);
        }

        // -- Cursor animation timer (~60 fps) --
        if self.cursor_timer.is_event(event).is_some() {
            self.tick_cursor_animation(cx);
        }

        // Start cursor timer on first event if not already running
        if !self.cursor_timer_started {
            self.cursor_timer = cx.start_interval(1.0 / 60.0);
            self.cursor_timer_started = true;
        }

        // Skip editor key handling when room input has focus
        if self.room_input_focused {
            return;
        }

        // ── Android / Mobile Keyboard ──
        // When the user taps on the editor area, show the virtual
        // keyboard (IME) so they can type on mobile devices.
        {
            let editor_area = self.ui.view(cx, ids!(editor_area)).area();
            match event.hits(cx, editor_area) {
                Hit::FingerDown(fd) => {
                    cx.set_key_focus(editor_area);
                    cx.show_text_ime(editor_area, fd.abs);
                }
                _ => {}
            }
        }

        // ── Graceful shutdown on app close ──
        match event {
            Event::Shutdown => {
                // Broadcast Goodbye so peers remove us instantly
                if let Some(ref net) = self.net {
                    let goodbye = onyx_core::protocol::encode_control(
                        onyx_core::protocol::CTRL_GOODBYE,
                    );
                    net.send_control(goodbye);
                }
            }
            _ => {}
        }

        match event {
            // -- Printable text input --
            Event::TextInput(e) => {
                if e.input.is_empty() {
                    return;
                }
                self.capture_pre_edit_vv();
                let pos = self.cursor.pos;
                // Insert into the Rope buffer
                self.buffer.insert(pos, &e.input);
                // Mirror into the Loro CRDT
                let _ = self.crdt.insert(pos, &e.input);
                // Advance cursor
                self.cursor.move_right(e.input.len(), self.buffer.len_chars());
                self.sync_display(cx);
                self.broadcast_crdt_delta();
                cx.redraw_all();
            }
            // -- Special keys --
            Event::KeyDown(ke) => {
                let max = self.buffer.len_chars();
                match ke.key_code {
                    KeyCode::Backspace if self.cursor.pos > 0 => {
                        self.capture_pre_edit_vv();
                        let pos = self.cursor.pos - 1;
                        self.buffer.delete(pos, pos + 1);
                        let _ = self.crdt.delete(pos, 1);
                        self.cursor.move_left(1);
                        self.sync_display(cx);
                        self.broadcast_crdt_delta();
                        cx.redraw_all();
                    }
                    KeyCode::Delete if self.cursor.pos < max => {
                        self.capture_pre_edit_vv();
                        let pos = self.cursor.pos;
                        self.buffer.delete(pos, pos + 1);
                        let _ = self.crdt.delete(pos, 1);
                        self.sync_display(cx);
                        self.broadcast_crdt_delta();
                        cx.redraw_all();
                    }
                    KeyCode::ReturnKey => {
                        self.capture_pre_edit_vv();
                        let pos = self.cursor.pos;
                        self.buffer.insert(pos, "\n");
                        let _ = self.crdt.insert(pos, "\n");
                        self.cursor.move_right(1, self.buffer.len_chars());
                        self.sync_display(cx);
                        self.broadcast_crdt_delta();
                        cx.redraw_all();
                    }
                    KeyCode::ArrowLeft => {
                        self.cursor.move_left(1);
                        self.sync_display(cx);
                        cx.redraw_all();
                    }
                    KeyCode::ArrowRight => {
                        self.cursor.move_right(1, max);
                        self.sync_display(cx);
                        cx.redraw_all();
                    }
                    KeyCode::Home => {
                        self.cursor.move_to(0);
                        self.sync_display(cx);
                        cx.redraw_all();
                    }
                    KeyCode::End => {
                        self.cursor.move_to(max);
                        self.sync_display(cx);
                        cx.redraw_all();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
