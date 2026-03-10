# Onyx Void — The Master Plan v2
## Revised After Your Feedback

> **The Rule**: You do NOT skip milestones. Each one builds on the last.

---

## 📊 CODEBASE AUDIT — What's Built vs What's Missing

### ✅ DONE (onyx-core, ~4500 lines)

| Module | What It Does |
|--------|-------------|
| [document.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/document.rs) | LoroTree CRDT workspace (voids, notes, blocks, layout trees, flashcards, canvas, vectors, schemas) |
| [blocks.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/blocks.rs) | 12+ block types with semantic attributes (Sentiment, ClozeGap, VoiceSync, LaTeX) |
| [fsrs.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/fsrs.rs) | FSRS-6 scheduler |
| [search.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/search.rs) | Tantivy full-text search |
| [neural.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/neural.rs) | Candle BERT semantic embeddings + cosine search |
| [canvas.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/canvas.rs) | Canvas elements (Rect, Line, Arrow, Freehand) + hit-testing |
| [grid_layout.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/grid_layout.rs) | 12-column newspaper grid engine |
| [persistence.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/persistence.rs) | Encrypted save/load + autosave |
| [blob.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/blob.rs) | SHA256 content-addressable blob store |
| [graph.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/graph.rs) | Backlink index |
| [crypto.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/crypto.rs) | XChaCha20-Poly1305 + Argon2 |
| [history.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/history.rs) | Undo/redo via snapshot stack |
| [query.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/query.rs) | Property-filtered queries |
| [import.rs](file:///c:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-core/src/import.rs) | Markdown import/export |
| Others | settings, templates, learning (Feynman grader), math, media, model, diffing, scheduler |

### ❌ NOT STARTED

Real text editing, block manipulation UI, navigation (tabs/breadcrumbs/back-forward), homepage, void linking, toolbar/ribbon, flashcard review UI, canvas drawing tools, question banks, slides, calendar/email/messaging wrappers, password vault, P2P sync.

---

## 🎨 THE DESIGN SYSTEM

### The Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│ 🌌 Onyx Void               [ Search... ]      │ Tab1 │ Tab2 │ Tab3 │ + │ ← HEADER (48px)
├──────────────────────────────────────────────────────────────────────┤ 
│ B  I  U  H1  H2  •  1.  ☑  ∑  🔗  +note  🃏  📊  🖼  ≡  ⊞      │ ← TOP RIBBON (40px)
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ← → │ Root / Networks / OSI Layers                                  │ ← NAV STRIP (32px)
│                                                                      │    Above the title
│  Title of Note                                                       │ ← FULL-WIDTH EDITOR
│  ───────────                                                         │
│                                                                      │
│  Block 1 text flows the full width of the window...              [+]│
│                                                                      │     [+] = add grid col
│  ┌─────────────────────┐ ┌─────────────────────┐                     │     [-] = remove col
│  │  Callout: Warning   │ │  Callout: Info       │                 [-]│  ← Full-width grid row
│  └─────────────────────┘ └─────────────────────┘                     │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### Why This Is Different From Everything Else

| Decision | Why |
|----------|-----|
| **Toolbar at TOP** | Your eyes naturally look up. Top ribbon, always visible, Word-style familiarity |
| **Full-width editor** | No 850px centered cage. Text flows left-to-right using the whole window |
| **Nav Strip above Title** | Back/Forward arrows + Breadcrumbs live directly above the note title |
| **Tabs in Header** | Tabs stay at the very top for rapid switching |
| **Block grid via +/- buttons** | No drag handles. `[+]` on right edge adds a column, `[-]` removes. |
| **Block Movement** | **Alt+↑/↓** (VSCode-style) OR **Alt+Click** to "pick up" and move blocks |

### The Void System — Everything Is a Void

```
                        ┌──────────┐
                        │   ROOT   │ (your workspace)
                        └──┬───┬───┘
                           │   │
                    ┌──────┘   └──────┐
                    ▼                 ▼
              ┌──────────┐     ┌──────────┐
              │  Maths   │     │CompSci   │
              └──┬───────┘     └──┬───┬───┘
                 │                │   │
           ┌─────┘          ┌────┘   └────┐
           ▼                ▼             ▼
     ┌──────────┐    ┌──────────┐   ┌──────────┐
     │ Algebra  │    │Networks  │   │EEET2634  │
     └──────────┘    └──┬───┬──┘   └──┬───────┘
                        │   │         │
                  ┌─────┘   │    ┌────┘
                  ▼         ▼    ▼
            ┌──────────┐  ┌──────────┐
            │OSI Layers│  │OSI Layers│ ← SAME void, linked in 2 places
            └──┬───┬───┘  └──────────┘   (alias: "Week 1" in EEET2634)
               │   │
          ┌────┘   └────┐
          ▼             ▼
    ┌──────────┐  ┌──────────┐
    │ Layer 1  │  │ Layer 2  │  ← These are also voids (containing notes)
    └──────────┘  └──────────┘
```

**Rules:**
- **Everything is a Void** — a topic, a class, a sub-topic, a layer. Voids contain other voids and/or notes
- **Notes live inside voids** — notes are the actual content. Voids are containers
- **Void Links** — a void can appear in multiple parents (OSI Layers lives in Networks AND EEET2634)
- **Properties are LOCAL** — in EEET2634, OSI Layers has a "Week" property. In Networks, it doesn't
- **Out of sight, out of mind** — you only see the voids inside your current void. Clean. No mess

### Homepage View — The Launchpad

```
┌──────────────────────────────────────────────────────────────────────┐
│ 🌌 Onyx Void                                         🔍 Search...  │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  YOUR VOIDS                                                          │
│                                                                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐    │
│  │  🧮        │  │  💻        │  │  📡        │  │     ＋     │    │
│  │  Maths     │  │  CompSci   │  │  EEET2634  │  │  New Void  │    │
│  │            │  │            │  │            │  │            │    │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘    │
│                                                                      │
│  SCHEDULE (Due Today)                                                │
│  ├─ 🃏 12 flashcards across 3 voids      [Review All →]             │
│  ├─ 📅 Assignment: Layer 2 Summary       (in EEET2634)              │
│  └─ 📅 Quiz: Network Topology            (in Networks)              │
│                                                                      │
│  RECENT                                                              │
│  ├─ OSI Layer 1                          5 min ago                  │
│  ├─ Algebra Homework 3                   2 hours ago                │
│  └─ EEET2634 Week 1 Notes                yesterday                  │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

- **Cards** for top-level voids
- **Schedule**: Unified view of flashcards and items with a "Due Date" property
- **Recent** list for quick access


### Color Tokens

```rust
const BG_VOID:       Color = Color::from_rgba8(9,   9,  11, 255);  // deepest bg
const BG_SURFACE:    Color = Color::from_rgba8(18, 18,  22, 255);  // header, ribbon
const BG_RAISED:     Color = Color::from_rgba8(28, 28,  34, 255);  // cards, panels
const BG_OVERLAY:    Color = Color::from_rgba8(40, 40,  48, 200);  // dropdowns
const TEXT_PRIMARY:   Color = Color::from_rgba8(220, 220, 230, 255);
const TEXT_SECONDARY: Color = Color::from_rgba8(150, 150, 160, 255);
const TEXT_MUTED:     Color = Color::from_rgba8(100, 100, 110, 255);
const ACCENT_BLUE:   Color = Color::from_rgba8(96, 165, 250, 255);
const ACCENT_PURPLE: Color = Color::from_rgba8(168, 130, 255, 255);
const ACCENT_GREEN:  Color = Color::from_rgba8(74,  222, 128, 255);
const ACCENT_RED:    Color = Color::from_rgba8(248, 113, 113, 255);
const BORDER_SUBTLE: Color = Color::from_rgba8(40, 40, 45, 255);
const BORDER_FOCUS:  Color = Color::from_rgba8(96, 165, 250, 128);
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+B/I/U` | Bold / Italic / Underline |
| `Ctrl+Z/Y` | Undo / Redo |
| `Ctrl+S` | Force save |
| `Ctrl+F` | Search in current note |
| `Ctrl+Shift+F` | Global search |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+N` | New note in current void |
| `Ctrl+Shift+N` | New void |
| `Alt+←/→` | Back / Forward navigation |
| `Alt+↑/↓` | Move block up / down |
| `Enter` | New paragraph block |
| `Shift+Enter` | Soft line break |
| `Tab / Shift+Tab` | Indent / Outdent |
| `Ctrl+Shift+M` | Insert math block |
| `Escape` | Dismiss panel, deselect |

---

## 🗺️ THE 10 MILESTONES

---

### MILESTONE 1: The Editor — Real Text Editing + Blocks
**Time**: 2-3 weeks

#### 1.1 Cursor Model — [NEW] `onyx-app/src/cursor.rs`
- `CursorState`: `block_index`, `byte_offset`, `selection_anchor`
- Arrow key navigation using Parley's `cursor_for_position()` / `cursor_for_offset()`
- Blinking 2px cursor rectangle (no pipe hack)
- Selection highlight: semi-transparent blue rectangles from Parley geometry

#### 1.2 Block Mutation Engine — [NEW] `onyx-core/src/editing.rs`
- `insert_text()` → insert at cursor, shift attribute spans
- `delete_text()` → remove range, adjust spans
- `split_block()` → Enter key splits block in two
- `merge_blocks()` → Backspace at start merges with previous
- `apply_attribute()` → Bold/Italic/Underline on selection
- All mutations go through LoroTree CRDT

#### 1.3 Block Movement — No Drag Handles
- `Alt+↑` / `Alt+↓` moves the current block up/down (VSCode-style)
- Clean, no visible drag handles cluttering the UI

#### 1.4 Block Grid System — The +/- Buttons
- `[+]` button appears on right edge of any block on hover → splits block into 2-column grid row
- `[-]` removes a column (merges back to single column)
- Underlying engine: existing 12-column `grid_layout.rs`
- A "grid row" is a container block holding child blocks in columns
- Each column can hold any block type (text, callout, image, canvas)

#### 1.5 Full-Width Editor — [MODIFY] `editor_renderer.rs`
- Remove 850px constraint — content spans from left padding (24px) to right padding (24px)
- Wire `live_text_buffer` to CRDT block mutations
- `Ctrl+B/I/U` apply attributes, `Ctrl+Z/Y` trigger undo/redo

#### 1.6 Top Ribbon — [MODIFY] `app.rs`
- Replace current header with two-row layout:
  - **Row 1 (Header, 48px)**: Back/Forward arrows + Breadcrumbs + Tabs + Search
  - **Row 2 (Ribbon, 40px)**: B, I, U, H1, H2, •, 1., ☑, ∑, 🔗, +note, 🃏, 📊, 🖼, ≡, ⊞
- Ribbon buttons trigger formatting/block-type commands
- Active formatting state highlighted (bold button lit when cursor is in bold text)

## Ribbon UI Overhaul (Refinement Phase)

### [MODIFY] [ribbon.rs](file:///C:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-app/src/ribbon.rs)

#### Visual Fixes
- **Doubled Borders**: Use inset strokes for `BtnStyle::Dropdown` and `BtnStyle::Active`. `BtnStyle::Default` and `BtnStyle::Heading` will be fill-only with a top highlight.
- **Text Glyphs as Icons**: Replace "hand-drawn" `BezPath` icons for B, I, U, S with actual font glyphs (Inter Bold/Italic) for pixel-perfect hinting.
- **Compact Layout**: Reduce total ribbon height to 82px. Adjust `TOP_ROW` (12px) and `BOT_ROW` (46px).
- **Group Backgrounds**: Remove the stroke from `draw_group_bg` to stop container outlines from doubling with button borders.

#### Verification Plan
- `cargo run --release` to verify the "raised" effect and icon crispness.
- Confirm active state highlights look centered and clean.
- Check that adjacent buttons don't have overlapping borders.
- Manual: type text, move cursor, select text, Ctrl+B to bold, Enter to split blocks, Alt+↑ to move blocks, click [+] to create grid columns

---

### MILESTONE 2: Navigation — Tabs, Breadcrumbs, Homepage
**Time**: 2-3 weeks

#### 2.1 Tab System — [NEW] `onyx-app/src/tabs.rs`
- `TabBar`: list of `Tab { id, node_id, title, is_active, scroll_y, cursor }`
- `Ctrl+T` = new tab, `Ctrl+W` = close tab, `Ctrl+Tab` = cycle tabs
- Tabs render in header row, scrollable if too many
- Drag a tab to the right → side-by-side split view (like VSCode)

#### 2.2 Navigation History — [NEW] `onyx-app/src/navigation.rs`
- `NavigationHistory`: stack-based back/forward
- `Alt+←` = go back, `Alt+→` = go forward
- Back/Forward arrow buttons in header (left side)
- Each tab has its own navigation history

#### 2.3 Breadcrumb Bar
- Dynamic breadcrumbs: `Root / Networks / OSI Layers / Layer 1`
- Click any segment → zoom out to that level
- Current title is inline-editable (click to rename)

#### 2.4 Homepage (The Launchpad) — [NEW] `onyx-app/src/homepage.rs`
- Shown when no note/void is focused (or via clicking 🌌 in breadcrumb)
- **Your Spaces**: card grid of top-level voids (icon, name, note count, due count)
- **Recent**: last 10 opened notes with timestamps
- **Due Today**: flashcard summary with "Review All" button
- `Ctrl+Shift+N` from homepage creates a new top-level void

#### 2.5 Void Dashboard View
- When you zoom into a Void (not a Note), show a customizable dashboard:
  - Children voids as cards (drag to reorder)
  - Notes list (grouped by user-defined groups)
  - Query widgets (configured per dashboard)
- Click a child void card → zoom into it
- Click a note → opens in current tab

#### 2.6 Void Linking — [MODIFY] `onyx-core/src/document.rs`
- `link_void(void_id, parent_void_id, alias)` → creates a Void Link (not a move)
- A void can appear in multiple parents via links
- Each link has an optional alias (e.g. "Week 1" in EEET2634)
- Properties are stored per-link-context (local, not global)
- `unlink_void()` removes the link (not the void itself)
- Drag a void card into another void → creates a link

#### Verification
- `cargo test -p onyx-app -- tabs navigation`
- `cargo test -p onyx-core -- document` for void linking
- Manual: open multiple notes in tabs, use Alt+← to go back, click breadcrumbs, visit homepage, create voids, link a void into two parents, verify alias + local properties

---

### MILESTONE 3: Properties & Queries — The Database
**Time**: 2 weeks

#### 3.1 Property Panel
- Ribbon button `≡` or `Ctrl+P` opens property panel (slide-up, bottom 200px)
- Shows void-scoped schema properties
- Inline editing: Text input, Select dropdown, Date picker, Checkbox toggle
- Auto-save on blur

#### 3.2 Query Block — [MODIFY] `blocks.rs`
- Add `BlockType::Query { config: String }` 
- Query block renders as a live view inside the note
- 5 view modes (togglable via icons in the query block header):

| View | Description |
|------|-------------|
| **Table** | Spreadsheet-like rows with property columns |
| **List** | Simple title list with property badges |
| **Cards** | Card grid (like homepage void cards but for notes) |
| **Kanban** | Swim lanes grouped by a Select property |
| **Graph** | Node-link visualization of void/note relationships |

## Proposed Changes

### Document-Centric Navigation
The navigation arrows and breadcrumbs are moving from the fixed app header to the document content area, sitting right above the note title.
#### [MODIFY] [app.rs](file:///C:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-app/src/app.rs)
- **Header Clean-up**: Update `draw_header` to remove navigation elements, leaving only the global shelf toggle.
- **Nav Strip**: Implement `draw_navigation_strip` to render arrows and breadcrumbs within the document flow.
- **Title Area**: Add a dedicated rendering section for the note title using a large, premium font weight before the editor blocks.
- **Layout Adjustments**: Recalculate `content_y` offsets to stack Ribbon > Nav Strip > Title > Editor.

### High DPI & 4K Support
The UI currently renders in physical pixels without accounting for scale factor, causing it to look tiny on 4K displays.
#### [MODIFY] [app.rs](file:///C:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-app/src/app.rs)
- Store `scale_factor` in `OnyxApp`.
- Apply a root scale transform to the scene in `draw()` to map logical coordinates to physical pixels.
- Update `handle_click` to reverse-scale mouse coordinates.

### Ribbon UI Polish
#### [MODIFY] [ribbon.rs](file:///C:/Users/omar_/Documents/Onyx%20Development/onyx/crates/onyx-app/src/ribbon.rs)
- **Centering**: Implement precise vertical centering for text/icons within buttons.
- **Icons**: Replace "Left/Center/Right" text with line-based alignment icons.
- **Dropdowns**: Add a subtle border and shadow-like depth to dropdown boxes.
- **Micro-centering**: Ensure all labels/icons are "middle-middle" aligned.

## Verification Plan
### Manual Verification
- Run the app on different monitors (or simulate DPI changes).
- Visually inspect ribbon buttons for perfect centering and correct icon rendering.

#### 3.3 Void Dashboard Queries
- On a void's dashboard, add query widgets that filter/sort child notes
- Grouping: user creates named groups (e.g. "Week 1", "Week 2") and assigns voids/notes to them
- Groups are just a property value on the void link

#### Verification
- `cargo test -p onyx-core -- query blocks`
- Manual: add properties to notes, create a query block with Table view, switch to Kanban view, verify filtering and sorting work

---

### MILESTONE 4: Canvas — draw.io + tldraw + Excalidraw Powerhouse
**Time**: 3-4 weeks

#### 4.1 Canvas as a Block AND as a Page
- **Embedded canvas**: a block type (`BlockType::Canvas`) renders an inline canvas area within a note
- **Full-page canvas**: a Void can be set to "Canvas" mode — the entire page is a canvas
- Both use the same rendering/interaction engine

#### 4.2 Drawing Tools
- **Pointer** (`V`): select, multi-select (Shift+click), box-select (drag on empty space), move elements
- **Rectangle** (`R`): click+drag → rectangle with rounded corners, resizable
- **Ellipse** (`E`): click+drag → oval/circle
- **Line/Arrow** (`A`): click start → click end, Bézier control points for curves
- **Freehand Pen** (`P`): click+drag for ink strokes
- **Text** (`T`): click to place text box, inline editing
- **Eraser** (`X`): click on elements to delete
- **Pan** (`H` or middle mouse): drag to pan viewport
- **Zoom**: Ctrl+scroll, or pinch on trackpad

#### 4.3 Pen Customization
- Color picker (8 preset colors + custom hex)
- Width slider (1px–10px)
- Pen types: Solid, Dashed, Dotted

#### 4.4 Shape Library (draw.io power)
- **Conditional diagram shapes**: Decision diamonds, process rectangles, start/end ovals, connectors
- **UML shapes**: Class boxes, actor stickmen, lifeline bars, arrows (association, aggregation, composition, inheritance)
- Shapes snap to grid, arrows auto-route between shapes
- Shape palette accessible from ribbon when canvas is focused

#### 4.5 Canvas Features
- **Image paste**: Ctrl+V pastes clipboard image onto canvas
- **Infinite pan/zoom** via `Affine` transform
- **Snap-to-grid** (toggle with `G`)
- **Layer ordering**: bring forward/send backward
- **Export**: PNG/SVG of canvas content

#### Verification
- `cargo test -p onyx-core -- canvas` (existing hit-test tests)
- Manual: create embedded canvas block, draw shapes, paste image, create flowchart with connectors, open full-page canvas void, verify same tools work

---

### MILESTONE 5: Flashcards & Question Banks — Anki on Steroids
**Time**: 3-4 weeks

> [!IMPORTANT]
> This is NOT static "flip and check" cards. Every learning mode is designed to force active recall.

#### 5.1 Card Types

| Type | How It Works |
|------|-------------|
| **Classic** | Front + Back, but with 3D card flip animation |
| **Cloze** | Text with blanks. You type the answer, not just reveal it |
| **Matching** | Two columns of items, drag to match pairs |
| **Fill-in-Code** | Code block with function blanks. Type the code. Can execute it (sandboxed) |
| **Ordering** | Drag items into correct sequence |
| **Image Occlusion** | Image with hidden regions, reveal on answer |
| **Audio Recall** | You speak the answer, Whisper transcribes and grades (existing `learning.rs`) |

#### 5.2 Card Creation
- Select text → Ribbon "🃏" → choose card type
- `Ctrl+Enter` on a block → quick-create Classic card
- Cloze: select specific words → "Make Cloze" → wraps in `Attribute::ClozeGap`
- Code fill-in: select a code block → "Make Code Card" → blanks out function bodies

#### 5.3 The 3D Card Deck — [NEW] `onyx-app/src/deck.rs`
- **The Deck View**: cards rendered as a physical stack with perspective transforms (Vello)
- Cards have subtle shadows, rounded corners, thickness illusion
- **Flick physics**: swipe right = correct, swipe left = wrong. Card flies off screen with momentum
- **Peek**: see the next card underneath as you flick
- **Deck stats**: progress ring showing completion, streak counter

#### 5.4 Review Modes
- **Sprint**: timed session (e.g. 10 minutes), rapid-fire cards
- **Chill**: no timer, go at your own pace
- **Exam Prep**: only cards from selected voids, shuffled
- **Feynman**: speak your understanding, Whisper grades you (uses existing `FeynmanAudioGrader`)

#### 5.5 Question Bank — Per Void
- Each void has a "Question Bank" accessible via a button in the void dashboard
- Shows all questions/worked solutions/examples from notes in that void
- **How to mark content**: Ribbon button to tag a block as: 📝 Question, ✅ Answer, 📖 Worked Solution, 💡 Example
- These are stored as block attributes (new `Attribute::ContentTag { tag: String }`)
- **Cross-void review**: toggle which voids to include → questions from selected voids are scrambled together

#### 5.6 Slides — [NEW] `BlockType::SlideBreak`
- Insert a slide break via ribbon button (not `---`)
- Notes split into presentation slides at each break
- **Present mode**: fullscreen one-slide-at-a-time view
- **Self-test**: present + auto-record via Whisper → Feynman grading after each slide
- Slides auto-size content — if too much text, it warns you (doesn't overflow off-screen)
- Can export slides as images

#### Verification
- `cargo test -p onyx-core -- fsrs blocks`
- Manual: create cloze cards, open deck view, flick cards with physics, review in sprint mode, open question bank for a void, toggle multiple voids, present slides

---

### MILESTONE 6: Utility Panels — Calendar, Email, Vault, Messages
**Time**: 3-4 weeks

These features are **hidden by default** and live in their own dedicated areas, accessible from an icon strip or keyboard shortcut.

#### 6.1 App Sidebar — The Feature Strip
- A narrow (48px) icon strip on the far-left edge:
  - 🏠 Homepage
  - 📅 Calendar
  - 📧 Email (future)
  - 💬 Messages (future)
  - 🔐 Vault
  - ⚙ Settings
- Clicking an icon opens that feature as a **full panel** (replaces the editor view)
- Or opens as a **slide-over** panel (35% width from right)
- **Not always visible** — the strip only appears on hover at the left edge (auto-hide)

#### 6.2 Calendar
- **Mini calendar** in sidebar showing current month
- Days with due flashcards highlighted
- Click a day → shows due cards + notes modified that day
- Future: Google Calendar API integration

#### 6.3 Password Vault
- Encrypted key-value store using existing `crypto.rs`
- Entries: site, username, encrypted password, optional notes
- Stored in dedicated LoroMap
- Master password unlock (Argon2)
- Click to copy password to clipboard
- Auto-lock after 5 minutes idle

#### 6.4 Email Wrapper (Placeholder)
- "Connect Email" button for now
- Future: IMAP/SMTP integration

#### 6.5 Messaging (Placeholder)
- Simple message list UI
- Future: rides on Iroh P2P sync mesh

#### 6.6 Settings Panel
- Theme (dark/light/custom)
- Autosave interval
- Flashcard limits
- Font size
- Keyboard shortcut editor

#### Verification
- Manual: hover left edge to see icon strip, open Calendar, check due cards, open Vault, add/retrieve a password entry, verify auto-lock

---

### MILESTONE 7: Media & Math
**Time**: 2-3 weeks

#### 7.1 Image Blocks
- Drag-and-drop image → creates embed block via BlobStore
- Paste from clipboard
- Resize handles on hover
- Full-width by default in single-column, proportional in grid columns

#### 7.2 Math Blocks
- `pulldown-latex` (already in deps) for LaTeX parsing
- Inline math: `$x^2$` renders inline
- Display math: `$$\int_0^1$$` renders centered
- Live preview as you type

#### 7.3 Code Blocks with Highlighting
- Language selector in block header
- Syntax highlighting via regex-based tokenizer
- Monospace font

#### 7.4 PDF Export
- Export note as PDF with proper formatting
- Math rendered as vectors, images embedded

#### Verification
- Manual: drag image, type LaTeX, export PDF

---

### MILESTONE 8: Canvas Advanced — Diagrams & UML
**Time**: 2-3 weeks

This extends Milestone 4 with the heavy-duty diagram features.

#### 8.1 Flowchart Builder
- Predefined shapes: Process, Decision, Start/End, IO, Subprocess
- Smart connectors that route around shapes
- Auto-layout (top-to-bottom, left-to-right)

#### 8.2 UML Diagram Support
- Class diagrams (fields, methods, visibility markers)
- Sequence diagrams (lifelines, messages, activation bars)
- Use case diagrams (actors, use cases, relationships)
- Templates loadable from shape library

#### 8.3 Mind Maps
- Central node + radial branches
- Auto-layout with force simulation
- Collapse/expand branches

#### Verification
- Manual: create a flowchart, verify connector routing, create UML class diagram, build mind map

---

### MILESTONE 9: P2P Sync
**Time**: 3-4 weeks

#### 9.1 Iroh Integration — [NEW] `onyx-core/src/sync.rs`
- QUIC-based P2P via `iroh` crate
- Export Loro deltas on edit → broadcast to peers
- Import deltas → rebuild_id_map → re-render
- Manual peer discovery via connection ticket

#### 9.2 E2EE
- Encrypt deltas before sending (existing crypto)
- Shared passphrase for key exchange

#### 9.3 Sync UI
- Header indicator: "Synced ✓" / "Syncing…" / "Offline"
- Conflict diff viewer (rare with CRDTs)

#### Verification
- Run two app instances, share ticket, type in one, verify appears in other

---

### MILESTONE 10: Polish — Import, Intelligence, Cleanup
**Time**: 2-3 weeks

#### 10.1 Obsidian Import Bridge
- Folders → Voids, `.md` → Notes, `[[links]]` → Link blocks, `![[embeds]]` → Embed blocks

#### 10.2 Semantic Magnetism (Canvas)
- Notes drift toward similar notes based on embedding cosine similarity
- Toggle on/off, adjust force

#### 10.3 Smart Suggestions
- Track open counts → auto-pin frequent notes
- "You always open X when in Y" → suggest mounting

#### 10.4 Fix All Compiler Warnings
- Clean up all `#[allow(dead_code)]` and unused imports
- Wire all stub modules to real implementations

#### Verification
- Import test Obsidian vault, verify structure
- `cargo build --release 2>&1 | grep warning` → zero warnings

---

## ⏱ TIMELINE

| # | Milestone | Time | You Get |
|---|-----------|------|---------|
| 1 | Editor + Ribbon | 2-3 wk | Real typing, formatting, block grid |
| 2 | Navigation | 2-3 wk | Tabs, breadcrumbs, homepage, void linking |
| 3 | Properties & Queries | 2 wk | Database views (table/kanban/graph) |
| 4 | Canvas Basics | 3-4 wk | Drawing tools, shapes, embedded+standalone canvas |
| 5 | Flashcards & Q-Bank | 3-4 wk | 3D deck, all card types, question banks, slides |
| 6 | Utility Panels | 3-4 wk | Calendar, vault, messaging stubs, settings |
| 7 | Media & Math | 2-3 wk | Images, LaTeX, code blocks, PDF export |
| 8 | Canvas Advanced | 2-3 wk | Flowcharts, UML, mind maps |
| 9 | P2P Sync | 3-4 wk | Encrypted peer-to-peer sync |
| 10 | Polish | 2-3 wk | Obsidian import, AI features, zero warnings |

**Total**: ~5-8 months for everything.

---

> [!CAUTION]
> **THE ANTI-JUMPING RULE**: If you want to work on Milestone 5 before Milestone 1 is done — STOP. Open this doc. Work on the current milestone.

**NEXT STEP**: Start Milestone 1.1 — Create `onyx-app/src/cursor.rs`.
