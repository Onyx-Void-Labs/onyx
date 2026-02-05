# 💎 ONYX Notes

A modular, local-first note-taking application designed for speed, privacy, and aesthetics. Built with **Tauri**, **React**, and **CodeMirror 6** for a high-performance, native-like experience.

> **Current Version**: `v0.0.2-alpha`

## 🚀 Features

### ✅ Implemented
- **Core Engine (CodeMirror 6)**:
  - Replaced legacy block editor with a high-performance text engine.
  - **Hybrid Markdown**: Markdown syntax is hidden while you type, giving a rich-text feel while keeping plain-text portability.
  - **Math Support**: Full LaTeX support using KaTeX. Use `$$` for block math and `$` for inline.
  - **Smart Lists**: Auto-indenting bullets and numbered lists.
  - **Code Folding**: Collapse headers and sections for focused editing.

- **Strict Search Engine**:
  - Zero-latency "Live Indexing" search.
  - **Strict Matching**: No fuzzy guessing. Exact matches only.
  - **Native Highlights**: Search matches look and feel like native text selection.

- **Security & Privacy**:
  - **Local-First**: Data lives on your disk, not our cloud.
  - **Encryption**: AES-256-GCM encryption for "Locked Notes". Password-protected at rest.

- **Premium UI/UX**:
  - "Discord-like" dark aesthetic.
  - **Fluid Tabs**: Browser-grade dragging (dnd-kit) with FLIP animations.
  - **Glassmorphism**: Subtle translucent effects and smooth transitions.

- **Infrastructure**:
  - **Auto-Updater**: Built-in support for background updates.
  - **Cross-Platform**: Builds native binaries for **Windows** (`.exe`), **macOS** (`.dmg`), and **Linux** (`.deb`).

### 🚧 Roadmap
- [ ] **Mobile App**: Native Android port (Tablet/Phone support).
- [ ] **Cloud Sync**: Optional encrypted sync (Git/S3/WebDAV).
- [ ] **Command Palette**: `Ctrl+Shift+P` for keyboard-only control.
- [ ] **Plugin System**: API for community extensions.
- [ ] **Export**: PDF and HTML export with themes.

## 🛠️ Setup & Development

### Prerequisites
- [Node.js](https://nodejs.org/) (Latest LTS)
- [Rust](https://www.rust-lang.org/tools/install) (Required for Tauri)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/om-itani/onyx.git
   cd onyx
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Run in Development Mode**
   ```bash
   npm run tauri dev
   ```
   *Starts the React frontend and Tauri native host with hot-reload.*

### Building for Production
To build individual binaries:
```bash
npm run tauri build
```
*Artifacts will be in `src-tauri/target/release/bundle`*

## 🤝 Contributing
ONYX is open source and active. Feel free to open issues or PRs!

---
*Created by [Omar Itani](https://github.com/om-itani)*
