# ONYX Notes

A modular, local-only note-taking application designed to fit every user's needs. Built with Tauri, React, and TypeScript for a high-performance, native-like experience.

## 🚀 Features

### ✅ Implemented
- **Premium UI/UX**: Custom window frame with Discord-like aesthetics, seamless titlebar integration, and glassmorphism effects.
- **Fluid Tab System**:
  - Browser-grade tab dragging and reordering.
  - Smooth "settle" animations (FLIP) for a natural feel.
  - Smart positioning and visual feedback.
- **Modern Sidebar**: Collapsible navigation with smooth animations and custom styling.
- **Fast Search**: 
  - `Ctrl+P` Quick Open modal for files.
  - Keyboard-centric navigation.
- **Block-Based Editor**: Foundation for a modular content editing experience.
- **Local-First**: All data is stored locally for maximum privacy and speed.

### 🚧 Roadmap & Upcoming
- [ ] **Rich Text Support**: Bold, Italic, Underline, and more inline styles.
- [ ] **Mathematical Notation**: Inline LaTeX/KaTeX support.
- [ ] **Plugin System**: Modular architecture to allow community extensions.
- [ ] **In-File Search**: `Ctrl+F` to find text within the active note.
- [ ] **Media Support**: Drag & drop images and file attachments.
- [ ] **Export Options**: Export notes to Markdown, PDF, or HTML.

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
   This command starts the React frontend and the Tauri native window handler with hot-reload enabled.

### Building for Production
To build the application for your OS:
```bash
npm run tauri build
```
The output binaries will be located in `src-tauri/target/release/`.

## 🤝 Contributing
ONYX is currently in active development. Usage and testing feedback is highly appreciated!

---
*Created by [Omar Itani](https://github.com/omar.itaniau)*
