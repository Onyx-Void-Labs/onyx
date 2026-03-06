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
use onyx_core::id::OnyxId;
use onyx_editor::{Cursor, EditorBuffer};
use onyx_store::CrdtDoc;
use std::collections::HashMap;

use crate::aero_hud::AeroHudAction;
use crate::cosmos::Cosmos;
use crate::cosmos_view::{CosmosViewAction, CosmosViewWidgetRefExt, NodeDrawData};
use crate::media_engine::{MediaEngine, MediaEvent};
use crate::net_bridge::{NetBridge, NetEvent};

// ── Cosmos Camera ───────────────────────────────────────────────

/// Camera state for navigating the spatial knowledge universe.
///
/// Zoom levels:
///   0 — Multiverse (distant vault glows)
///   1 — Constellation (topic clusters)
///   2 — Planet (individual note)
///   3 — Surface (inline editing)
pub struct CameraState {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub zoom_level: u8,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            zoom_level: 2, // start at Planet level
        }
    }
}

// --- DSL: The Void UI ---

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.RemoteCursorWidget
    use mod.widgets.AeroHud
    use mod.widgets.CosmosView

    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(1280, 800)
                pass.clear_color: vec4(0.039, 0.039, 0.059, 1.0)
                body +: {
                    View {
                        width: Fill
                        height: Fill
                        flow: Overlay

                        cosmos_view := CosmosView {
                            width: Fill
                            height: Fill
                        }
                        // Inline editor overlay — fades in on "Dive" (double-click)
                        dive_editor := View {
                            visible: false
                            width: Fill
                            height: Fill
                            show_bg: true
                            draw_bg.color: #x080808
                            flow: Overlay

                            // Centered editor paper
                            View {
                                width: Fill
                                height: Fill
                                flow: Down
                                align: {x: 0.5, y: 0.5}
                                padding: {top: 50.0, left: 50.0, right: 50.0, bottom: 50.0}

                                View {
                                    width: 800.0
                                    height: Fill
                                    flow: Down
                                    spacing: 20.0

                                    dive_title := Label {
                                        text: "ENTERING ORBIT..."
                                        draw_text.color: #x666666
                                        draw_text.text_style: {font_size: 10.0}
                                    }

                                    dive_text_input := TextInput {
                                        width: Fill
                                        height: Fill
                                        is_read_only: true
                                        empty_message: "Write to ignite the star..."
                                        draw_text.color: #xE0E0E0
                                        draw_bg.color: #x101010
                                        text_style: {font_size: 14.0}
                                    }
                                }
                            }

                            // Close button (floating top-right)
                            View {
                                width: Fill
                                height: Fit
                                align: {x: 1.0, y: 0.0}
                                padding: {top: 15.0, right: 15.0}
                                eject_button := Button {
                                    text: "EJECT [ESC]"
                                    draw_text.color: #xDD0000
                                    draw_bg.color: vec4(0.0, 0.0, 0.0, 0.0)
                                }
                            }
                        }
                        editor_area := View {
                            width: Fill
                            height: Fill
                            visible: false
                        }
                        // Wrapper View to position HUD at bottom-center
                        View {
                            width: Fill
                            height: Fill
                            align: Align{x: 0.5, y: 1.0}
                            padding: Inset{bottom: 40.0}
                            aero_hud := AeroHud {
                                width: 400.0
                                height: 60.0
                            }
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
    /// Whether the MoQ voice engine is active.
    #[rust]
    voice_active: bool,
    /// The media engine handle (spawned when voice is toggled ON).
    #[rust]
    media_engine: Option<MediaEngine>,
    /// Our own NodeID string — cached to filter self-sent media datagrams.
    #[rust]
    our_node_id: String,
    /// Remote peer cursor positions (peer_id → char offset).
    #[rust]
    remote_cursors: HashMap<String, u32>,
    /// Counter for cursor broadcast debouncing (every ~500ms = 10 polls).
    #[rust]
    cursor_broadcast_counter: u32,
    /// Cosmos camera state for spatial navigation.
    #[rust]
    camera: CameraState,
    /// The spatial universe — owns all VoidNodes and drives physics.
    #[rust]
    cosmos: Cosmos,
    /// Whether the cosmos canvas is the active view (vs. editor).
    #[rust]
    cosmos_active: bool,
    /// Timer driving the physics simulation at ~60 fps.
    #[rust]
    cosmos_timer: Timer,
    /// The OnyxId of the node currently being edited (\"dived into\").
    /// When Some, the editor reads/writes to `crdt.get_text_for(id)`.
    #[rust]
    active_node_id: Option<OnyxId>,
}

impl App {
    fn run(vm: &mut ScriptVm) -> Self {
        makepad_widgets::script_mod(vm);
        // Register the remote cursor shader + widget BEFORE the app's own
        // script_mod so the DSL can reference RemoteCursorWidget.
        crate::remote_cursor::script_mod(vm);
        // Register the Aero-HUD widget for the Singularity Engine UI.
        crate::aero_hud::script_mod(vm);
        // Register the CosmosView spatial canvas widget.
        crate::cosmos_view::script_mod(vm);
        let mut app = App::from_script_mod(vm, self::script_mod);
        app.panel_open = true;
        app.last_display_hash = 0;
        app.cursor_anim_x = 0.0;
        app.cursor_anim_y = 0.0;
        app.cursor_vel = 0.0;
        app.cursor_vel_y = 0.0;
        app.cursor_time = 0.0;
        app.relay_node_id = onyx_core::protocol::relay_node_id_string();
        app.voice_active = false;
        app.media_engine = None;
        app.our_node_id = String::new();
        app.remote_cursors = HashMap::new();
        app.cursor_broadcast_counter = 0;
        app.camera = CameraState::default();

        // ── Cosmos initialisation ──
        let mut cosmos = Cosmos::new();
        // Spawn demo nodes with simulated content for Ignition Protocol.
        // Taxonomy is emergent: mass determines type automatically.
        let idx = cosmos.spawn_node();
        cosmos.nodes[idx].calculate_mass_and_type(10000, 8); // Sun (≥ 50.0)
        let idx = cosmos.spawn_node();
        cosmos.nodes[idx].calculate_mass_and_type(3000, 2); // GasGiant
        let idx = cosmos.spawn_node();
        cosmos.nodes[idx].calculate_mass_and_type(1500, 1); // RockyPlanet
        let idx = cosmos.spawn_node();
        cosmos.nodes[idx].calculate_mass_and_type(800, 0); // Asteroid
        let idx = cosmos.spawn_node();
        cosmos.nodes[idx].calculate_mass_and_type(200, 0); // Asteroid
        let idx = cosmos.spawn_node();
        cosmos.nodes[idx].calculate_mass_and_type(5000, 3); // GasGiant
        app.cosmos = cosmos;
        app.cosmos_active = true; // start in cosmos view
        app.active_node_id = None;
        app
    }

    /// Tick the cosmos physics and feed draw data to the CosmosView widget.
    fn tick_cosmos(&mut self, cx: &mut Cx) {
        if !self.cosmos_active {
            return;
        }

        let dt = 1.0 / 60.0_f32; // fixed timestep at 60 fps
        self.cosmos.tick(dt);

        // Build draw data from current cosmos state
        let draw_data: Vec<NodeDrawData> = self
            .cosmos
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| NodeDrawData {
                x: node.spatial.pos[0],
                y: node.spatial.pos[1],
                radius: crate::physics::node_radius(node),
                heat: node.spatial.heat,
                node_type: node.node_type,
                selected: self.cosmos.selected == Some(i),
                label: self.cosmos.node_label(i),
            })
            .collect();

        // Feed draw data to the CosmosView widget
        let cosmos_view = self.ui.cosmos_view(cx, ids!(cosmos_view));
        cosmos_view.set_draw_data(draw_data);
        cosmos_view.set_camera(self.camera.x, self.camera.y, self.camera.z);

        // Update status bar with cosmos info
        self.ui
            .label(cx, ids!(status_label))
            .set_text(cx, &format!("Cosmos — {} nodes", self.cosmos.len()));

        cx.redraw_all();
    }

    /// Close the dive editor overlay and return to the cosmos view.
    fn close_editor(&mut self, cx: &mut Cx) {
        self.ui.view(cx, ids!(dive_editor)).set_visible(cx, false);
        let dive_input = self.ui.text_input(cx, ids!(dive_text_input));
        dive_input.set_is_read_only(cx, true);
        self.cosmos_active = true;
        self.active_node_id = None;
        let cv = self.ui.cosmos_view(cx, ids!(cosmos_view));
        cv.set_visible(cx, true);
        self.ui.view(cx, ids!(editor_area)).set_visible(cx, false);
        self.ui
            .button(cx, ids!(hud_view_toggle))
            .set_text(cx, "\u{27C1} Editor");
        cx.redraw_all();
    }

    /// Map HUD button clicks to a typed AeroHudAction.
    fn poll_hud_action(&self, cx: &Cx, actions: &Actions) -> AeroHudAction {
        if self.ui.button(cx, ids!(hud_spawn)).clicked(actions) {
            AeroHudAction::SpawnNode
        } else if self.ui.button(cx, ids!(hud_view_toggle)).clicked(actions) {
            AeroHudAction::ToggleView
        } else if self.ui.button(cx, ids!(hud_delete)).clicked(actions) {
            AeroHudAction::DeleteSelected
        } else {
            AeroHudAction::None
        }
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
            self.ui.label(cx, ids!(editor_label)).set_text(cx, &display);
        }

        // Status: char count
        self.ui
            .label(cx, ids!(status_chars))
            .set_text(cx, &format!("{} chars", self.buffer.len_chars()));

        // Status: line/col info
        let buf_text = self.buffer.text();
        let pos = self.cursor.pos.min(buf_text.len());
        let line = buf_text[..pos].matches('\n').count() + 1;
        let col = pos - buf_text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
        self.ui
            .label(cx, ids!(status_label))
            .set_text(cx, &format!("Void Active  Ln {} Col {}", line, col));

        // Sync status — reports TEXT mesh peers only (decoupled from voice state)
        let visible_peer_count = self
            .peers
            .iter()
            .filter(|p| *p != &self.relay_node_id)
            .count();
        let sync_text = if self.connected {
            format!("Iroh mesh  {} text peers", visible_peer_count)
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

        self.ui
            .label(cx, ids!(status_label))
            .set_text(cx, &format!("{caret} Ln {line} Col {col}"));
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
        // These constants approximate glyph metrics at font_size 13.
        let char_width: f32 = 7.8; // approximate monospace glyph width at font_size 13
        let line_height: f32 = 20.0;

        // Derive the editor text origin from the editor_area's actual
        // screen position.  `abs_pos` in Makepad is window-absolute, so
        // we must account for the side panel, divider, title bar, AND
        // the editor area's own padding (left:48, top:32).
        let editor_rect = self.ui.view(cx, ids!(editor_area)).area().rect(cx);
        let pad_left: f32 = editor_rect.pos.x as f32 + 48.0; // editor view X + left padding
        let pad_top: f32 = editor_rect.pos.y as f32 + 32.0; // editor view Y + top padding

        let px_x = pad_left + self.cursor_anim_x * char_width;
        let px_y = pad_top + self.cursor_anim_y * line_height;

        // ── Cursor blink ──
        // Toggle the cursor bar visibility for a blink effect.
        let blink = (self.cursor_time * 3.5).sin();
        let visible = blink > 0.0;

        let overlay = self.ui.view(cx, ids!(cursor_overlay));
        // Reposition the cursor overlay to follow the animated position
        if let Some(mut inner) = overlay.borrow_mut() {
            inner.walk.abs_pos = Some(Vec2d {
                x: px_x as f64,
                y: px_y as f64,
            });
        }
        overlay.set_visible(cx, visible);
        overlay.redraw(cx);

        // ── Position remote peer cursors ──
        // Map remote peer cursor character offsets → pixel coords and
        // position the RemoteCursorWidget overlays.
        let remote_cursor_ids = [
            ids!(remote_cursor_0),
            ids!(remote_cursor_1),
            ids!(remote_cursor_2),
            ids!(remote_cursor_3),
        ];

        // Collect peer positions (sorted for stable assignment).
        let mut remote_peers: Vec<(&String, &u32)> = self
            .remote_cursors
            .iter()
            .filter(|(id, _)| *id != &self.relay_node_id && *id != &self.our_node_id)
            .collect();
        remote_peers.sort_by_key(|(id, _)| (*id).clone());

        for (i, rc_id) in remote_cursor_ids.iter().enumerate() {
            let rc_view = self.ui.view(cx, *rc_id);
            if let Some((_, &char_pos)) = remote_peers.get(i) {
                // Compute line/col from character offset.
                let buf_text = self.buffer.text();
                let cpos = (char_pos as usize).min(buf_text.len());
                let rline = buf_text[..cpos].matches('\n').count();
                let rcol = cpos - buf_text[..cpos].rfind('\n').map(|x| x + 1).unwrap_or(0);

                let rpx_x = pad_left + rcol as f32 * char_width - 12.0; // centre the 24px-wide widget
                let rpx_y = pad_top + rline as f32 * line_height;

                if let Some(mut inner) = rc_view.borrow_mut() {
                    inner.walk.abs_pos = Some(Vec2d {
                        x: rpx_x as f64,
                        y: rpx_y as f64,
                    });
                }
                rc_view.set_visible(cx, true);
                rc_view.redraw(cx);
            } else {
                // No peer for this slot — hide it.
                rc_view.set_visible(cx, false);
            }
        }

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
            self.ui.label(cx, ids!(editor_label)).set_text(cx, &display);
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
        self.ui.label(cx, ids!(peers_label)).set_text(cx, &text);

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
                        // Graceful degradation: shut down the failed media
                        // engine but do NOT touch mesh peers or connection
                        // state — text sync is independent of voice.
                        self.voice_active = false;
                        self.ui
                            .label(cx, ids!(voice_status))
                            .set_text(cx, &format!("Voice err: {msg}"));
                        // Clean up the broken engine so the user can retry.
                        // (We can't borrow `self.media_engine` mutably here
                        //  because we're iterating its events, so flag for
                        //  cleanup after the drain loop.)
                    }
                }
            }
        }

        // Post-drain cleanup: if a MediaEvent::Error set voice_active = false,
        // tear down the engine without disturbing the mesh peer state.
        if !self.voice_active && self.media_engine.is_some() {
            if let Some(ref engine) = self.media_engine {
                engine.stop();
                engine.shutdown();
            }
            self.media_engine = None;
            tracing::info!("MediaEngine cleaned up after error — mesh peers unaffected");
        }

        let Some(ref net) = self.net else { return };
        let events = net.drain_events();

        // ── Cursor position broadcast (every ~500ms = 10 polls) ──
        if self.connected {
            self.cursor_broadcast_counter += 1;
            if self.cursor_broadcast_counter >= 10 {
                self.cursor_broadcast_counter = 0;
                let cursor_msg = onyx_core::protocol::encode_cursor_control(self.cursor.pos as u32);
                net.send_control(cursor_msg);
            }
        }

        if events.is_empty() {
            return;
        }

        for evt in events {
            match evt {
                NetEvent::Connected { our_id } => {
                    tracing::info!(id = %our_id, "connected to mesh");
                    self.our_node_id = our_id;
                    self.connected = true;
                    self.update_peers_display(cx);
                }
                NetEvent::PeerJoined(peer_id) => {
                    tracing::info!(peer = %peer_id, "peer joined");
                    if !self.peers.contains(&peer_id) {
                        self.peers.push(peer_id.clone());
                    }
                    self.update_peers_display(cx);

                    // ── Catch-Up Handshake ──
                    self.broadcast_full_snapshot();
                }
                NetEvent::PeerLeft(peer_id) => {
                    tracing::info!(peer = %peer_id, "peer left");
                    self.peers.retain(|p| p != &peer_id);
                    self.remote_cursors.remove(&peer_id);
                    self.update_peers_display(cx);
                }
                NetEvent::GoodbyeReceived(peer_id) => {
                    tracing::info!(peer = %peer_id, "peer sent Goodbye — removing instantly");
                    self.peers.retain(|p| p != &peer_id);
                    self.remote_cursors.remove(&peer_id);
                    self.update_peers_display(cx);
                }
                NetEvent::CursorReceived { from, pos } => {
                    // Store the remote peer's cursor position for rendering.
                    if from != self.our_node_id && from != self.relay_node_id {
                        self.remote_cursors.insert(from, pos);
                    }
                }
                NetEvent::MediaStarted => {
                    tracing::info!("media QUIC connection established with relay");
                    self.ui
                        .label(cx, ids!(voice_status))
                        .set_text(cx, "Voice: ON (live)");
                }
                NetEvent::MediaDatagramReceived { from, data } => {
                    // Filter out our own datagrams (relay reflects them back).
                    if from == self.our_node_id {
                        continue;
                    }
                    // Forward incoming Opus frames to the MediaEngine for decode + playback.
                    tracing::info!(
                        from = %from,
                        bytes = data.len(),
                        "received media datagram — forwarding to MediaEngine"
                    );
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
                        let count = u16::from_be_bytes([raw_bytes[1], raw_bytes[2]]) as usize;
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
                            if let Err(e) =
                                self.crdt.import_snapshot(&raw_bytes[offset..offset + len])
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
            self.remote_cursors.clear();
            self.cursor_broadcast_counter = 0;
            self.voice_active = false;
            self.our_node_id.clear();
            // Clear the text buffer
            let len = self.buffer.len_chars();
            if len > 0 {
                self.buffer.delete(0, len);
            }
            self.crdt = CrdtDoc::default();
            self.cursor.move_to(0);
            self.sync_display(cx);
            self.update_peers_display(cx);
            self.ui.label(cx, ids!(voice_status)).set_text(cx, "");
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
            self.ui.label(cx, ids!(voice_status)).set_text(cx, status);
            cx.redraw_all();
        }

        // -- Panel toggle button --
        if self.ui.button(cx, ids!(panel_toggle)).clicked(actions) {
            self.panel_open = !self.panel_open;
            self.ui
                .view(cx, ids!(side_panel))
                .set_visible(cx, self.panel_open);
            cx.redraw_all();
        }

        // -- EJECT button (close dive editor) --
        if self.ui.button(cx, ids!(eject_button)).clicked(actions) {
            if !self.cosmos_active {
                self.close_editor(cx);
            }
        }

        // ── Aero-HUD controls (dispatched via AeroHudAction) ──────
        {
            match self.poll_hud_action(cx, actions) {
                AeroHudAction::SpawnNode => {
                    let idx = self.cosmos.spawn_node();
                    tracing::info!("spawned node {idx} — {} nodes total", self.cosmos.len());
                    cx.redraw_all();
                }
                AeroHudAction::ToggleView => {
                    self.cosmos_active = !self.cosmos_active;
                    self.ui
                        .view(cx, ids!(editor_area))
                        .set_visible(cx, !self.cosmos_active);
                    let cv = self.ui.cosmos_view(cx, ids!(cosmos_view));
                    cv.set_visible(cx, self.cosmos_active);
                    let label = if self.cosmos_active {
                        "⟁ Editor"
                    } else {
                        "⟁ Cosmos"
                    };
                    self.ui
                        .button(cx, ids!(hud_view_toggle))
                        .set_text(cx, label);
                    if self.cosmos_active {
                        // Returning to cosmos — clear active node binding
                        self.active_node_id = None;
                        // Hide the dive editor overlay
                        self.ui.view(cx, ids!(dive_editor)).set_visible(cx, false);
                        let dive_input = self.ui.text_input(cx, ids!(dive_text_input));
                        dive_input.set_is_read_only(cx, true);
                    } else {
                        self.sync_display(cx);
                    }
                    cx.redraw_all();
                }
                AeroHudAction::DeleteSelected => {
                    if let Some(idx) = self.cosmos.selected {
                        tracing::info!("deleting node {idx}");
                        self.cosmos.remove_node(idx);
                        cx.redraw_all();
                    }
                }
                AeroHudAction::None => {}
            }
        }

        // -- Track room input focus to avoid editor key conflicts --
        let room_input = self.ui.text_input(cx, ids!(room_input));
        for action in actions.filter_widget_actions_cast::<TextInputAction>(room_input.widget_uid())
        {
            match action {
                TextInputAction::KeyFocus => self.room_input_focused = true,
                TextInputAction::KeyFocusLost => self.room_input_focused = false,
                _ => {}
            }
        }

        // -- CosmosView actions --
        if self.cosmos_active {
            let cosmos_view_ref = self.ui.cosmos_view(cx, ids!(cosmos_view));
            for action in
                actions.filter_widget_actions_cast::<CosmosViewAction>(cosmos_view_ref.widget_uid())
            {
                match action {
                    CosmosViewAction::NodeClicked(idx) => {
                        self.cosmos.selected = Some(idx);
                    }
                    CosmosViewAction::NodeDoubleClicked(idx) => {
                        // ── "The Dive" — transition into the editor for this node ──
                        self.cosmos.selected = Some(idx);
                        self.cosmos_active = false;

                        // Hide cosmos, show editor
                        self.ui.view(cx, ids!(editor_area)).set_visible(cx, true);
                        let cv = self.ui.cosmos_view(cx, ids!(cosmos_view));
                        cv.set_visible(cx, false);
                        self.ui
                            .button(cx, ids!(hud_view_toggle))
                            .set_text(cx, "⟁ Cosmos");

                        // ── Show dive editor overlay with fade-in ──
                        self.ui.view(cx, ids!(dive_editor)).set_visible(cx, true);
                        // Make text visible (opacity → 1.0)
                        let dive_input = self.ui.text_input(cx, ids!(dive_text_input));
                        dive_input.set_text(cx, "");
                        // Enable editing
                        dive_input.set_is_read_only(cx, false);
                        // Request focus so the user can start typing
                        let area = dive_input.area();
                        cx.set_key_focus(area);

                        // ── The Anvil: Load this VoidNode's Loro text ──
                        let node_id = self.cosmos.nodes[idx].id;
                        self.active_node_id = Some(node_id);
                        let key = node_id.to_string();

                        // Read existing text from the CRDT (or empty for new nodes)
                        let text = self.crdt.get_text_for(&key).unwrap_or_default();

                        // Pre-fill the dive editor
                        dive_input.set_text(cx, &text);

                        // Sync the editor buffer
                        let old_len = self.buffer.len_chars();
                        if old_len > 0 {
                            self.buffer.delete(0, old_len);
                        }
                        if !text.is_empty() {
                            self.buffer.insert(0, &text);
                        }
                        self.cursor.move_to(0);

                        self.sync_display(cx);
                        cx.redraw_all();
                    }
                    CosmosViewAction::Deselect => {
                        self.cosmos.selected = None;
                    }
                    CosmosViewAction::NodeDragStart(idx) => {
                        self.cosmos.dragged = Some(idx);
                        self.cosmos.selected = Some(idx);
                    }
                    CosmosViewAction::NodeDragging { x, y } => {
                        self.cosmos.drag_to(x, y);
                    }
                    CosmosViewAction::NodeDragEnd { throw_vx, throw_vy } => {
                        self.cosmos.release_throw(throw_vx, throw_vy);
                    }
                    CosmosViewAction::CameraPanned { x, y } => {
                        self.camera.x = x;
                        self.camera.y = y;
                    }
                    CosmosViewAction::CameraZoomed(z) => {
                        self.camera.z = z;
                        self.camera.zoom_level = match z {
                            z if z < 0.4 => 0, // Multiverse
                            z if z < 0.8 => 1, // Constellation
                            z if z < 2.0 => 2, // Planet
                            _ => 3,            // Surface
                        };
                    }
                    CosmosViewAction::NodeHovered(idx) => {
                        self.cosmos.clear_all_hovers();
                        self.cosmos.set_hovered(idx, true);
                    }
                    CosmosViewAction::NodeUnhovered => {
                        self.cosmos.clear_all_hovers();
                    }
                    CosmosViewAction::None => {}
                }
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

        // -- Cosmos physics timer (~60 fps) --
        if self.cosmos_timer.is_event(event).is_some() {
            self.tick_cosmos(cx);
            self.ui.redraw(cx);
        }

        // -- Cursor animation timer (~60 fps) --
        if self.cursor_timer.is_event(event).is_some() {
            self.tick_cursor_animation(cx);
            // Always request next frame for smooth Cosmos animation
            cx.new_next_frame();
        }

        // Start cursor timer on first event if not already running
        if !self.cursor_timer_started {
            self.cursor_timer = cx.start_interval(1.0 / 60.0);
            self.cursor_timer_started = true;

            // Toggle initial view visibility — cosmos starts visible
            self.ui
                .view(cx, ids!(editor_area))
                .set_visible(cx, !self.cosmos_active);
            let cv = self.ui.cosmos_view(cx, ids!(cosmos_view));
            cv.set_visible(cx, self.cosmos_active);
            // Start cosmos timer for physics loop
            self.cosmos_timer = cx.start_interval(0.016); // ~60fps

            // Seed initial draw data so the first frame has nodes
            self.tick_cosmos(cx);
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
                    let goodbye =
                        onyx_core::protocol::encode_control(onyx_core::protocol::CTRL_GOODBYE);
                    net.send_control(goodbye);
                }
            }
            _ => {}
        }

        match event {
            // -- Printable text input --
            Event::TextInput(e) => {
                // Block all typing if in Cosmos mode (editor hidden)
                if self.cosmos_active {
                    return;
                }
                // ── Android IME full-state sync ──
                if let Some(ref full_state) = e.full_state_sync {
                    let new_text = &full_state.text;
                    let text_changed = self.buffer.text() != *new_text;
                    if text_changed {
                        self.capture_pre_edit_vv();
                        self.buffer.set_text(new_text);
                        let old_crdt_text = self.crdt.get_text().unwrap_or_default();
                        let old_len = old_crdt_text.chars().count();
                        if old_len > 0 {
                            let _ = self.crdt.delete(0, old_len);
                        }
                        if !new_text.is_empty() {
                            let _ = self.crdt.insert(0, new_text);
                        }
                        self.broadcast_crdt_delta();
                    }
                    let char_pos = full_state.selection.end.0.min(self.buffer.len_chars());
                    self.cursor.move_to(char_pos);
                    self.sync_display(cx);
                    cx.redraw_all();
                    return;
                }
                if e.input.is_empty() {
                    return;
                }
                self.capture_pre_edit_vv();
                let pos = self.cursor.pos;
                self.buffer.insert(pos, &e.input);
                // Write to the active node's CRDT text container
                if let Some(node_id) = self.active_node_id {
                    let _ = self.crdt.insert_for(&node_id.to_string(), pos, &e.input);
                } else {
                    let _ = self.crdt.insert(pos, &e.input);
                }
                self.cursor
                    .move_right(e.input.len(), self.buffer.len_chars());
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
                        if let Some(node_id) = self.active_node_id {
                            let _ = self.crdt.delete_for(&node_id.to_string(), pos, 1);
                        } else {
                            let _ = self.crdt.delete(pos, 1);
                        }
                        self.cursor.move_left(1);
                        self.sync_display(cx);
                        self.broadcast_crdt_delta();
                        cx.redraw_all();
                    }
                    KeyCode::Delete if self.cursor.pos < max => {
                        self.capture_pre_edit_vv();
                        let pos = self.cursor.pos;
                        self.buffer.delete(pos, pos + 1);
                        if let Some(node_id) = self.active_node_id {
                            let _ = self.crdt.delete_for(&node_id.to_string(), pos, 1);
                        } else {
                            let _ = self.crdt.delete(pos, 1);
                        }
                        self.sync_display(cx);
                        self.broadcast_crdt_delta();
                        cx.redraw_all();
                    }
                    KeyCode::ReturnKey => {
                        self.capture_pre_edit_vv();
                        let pos = self.cursor.pos;
                        self.buffer.insert(pos, "\n");
                        if let Some(node_id) = self.active_node_id {
                            let _ = self.crdt.insert_for(&node_id.to_string(), pos, "\n");
                        } else {
                            let _ = self.crdt.insert(pos, "\n");
                        }
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
                    KeyCode::Escape if !self.cosmos_active => {
                        self.close_editor(cx);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
