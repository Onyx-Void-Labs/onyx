# ✅ Onyx Verification Checklist

Use this checklist to test all recently implemented features.

## 🧮 Smart Math Suite
In a Math Block (`$$`):
- [ ] **Fractions**: Type `1/2` -> `\frac{1}{2}`
- [ ] **Variables**: Type `a/b` -> `\frac{a}{b}`
- [ ] **Nested**: Type `(x/y)/z` -> `\frac{\frac{x}{y}}{z}`
- [ ] **Symbols**: Type `alpha` -> `\alpha`, `beta` -> `\beta`
- [ ] **Flexible Prefixes**: Type `3alpha` -> `3\alpha`, `xsqrt` -> `x\sqrt`
- [ ] **Backspace Safety**: Type `\alpha`, then backspace. It should NOT re-trigger logic loops.
- [ ] **Menu Logic**: Type `\`, menu opens. Backspace `\`, menu closes.

## 📝 Core Editor
- [ ] **Headings**: `# h1`, `## h2`, `### h3` shortcuts work.
- [ ] **Selection**: Select text across multiple blocks.
- [ ] **Enter**: Pressing Enter splits blocks correctly.
- [ ] **Merge**: Backspace at start of block merges up correctly.
- [ ] **Undo/Redo**: `Ctrl+Z` and `Ctrl+Shift+Z` work reliably.

## 🎨 Visuals
- [ ] **Zoom**: `Ctrl + Scroll` zooms the entire interface.
- [ ] **Window**: Review Minimize/Maximize/Close buttons behavior.
- [ ] **Scroll**: Context menus scroll to selection on `ArrowUp/Down`.

## 🔜 Next Up (Phase 6)
- [ ] **Inline Math**: Typing `$ E=mc^2` inside a paragraph.
- [ ] **Inline Code**: Typing `code` inside a paragraph.
- [ ] **Rich Text**: **Bold** and *Italics* shortcuts.
