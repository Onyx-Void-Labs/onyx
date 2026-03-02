// ─── Editor Buffer ─────────────────────────────────────────────────
// Rope-backed text buffer for O(log n) insert/delete/slice ops.
// Designed for documents with 10k+ lines.
// ────────────────────────────────────────────────────────────────────

use ropey::Rope;
use tracing::trace;

/// Core text buffer.
#[derive(Debug, Clone)]
pub struct EditorBuffer {
    rope: Rope,
    /// Dirty flag — set on any mutation, cleared by the renderer.
    dirty: bool,
}

impl EditorBuffer {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            dirty: false,
        }
    }

    /// Create a buffer seeded with text.
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            dirty: true,
        }
    }

    // ── Queries ──────────────────────────────────────────────────

    /// Total number of characters.
    #[inline]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Total number of lines.
    #[inline]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Return text of a single line (0-indexed), **without** trailing newline.
    pub fn line(&self, idx: usize) -> String {
        let line = self.rope.line(idx);
        let s = line.to_string();
        s.trim_end_matches('\n').trim_end_matches('\r').to_string()
    }

    /// Full text snapshot (allocates).
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Is the buffer empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Has the buffer been mutated since last `clear_dirty()`?
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag (call after re-layout / save).
    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    // ── Mutations ────────────────────────────────────────────────

    /// Insert `text` at character offset `pos`.
    pub fn insert(&mut self, pos: usize, text: &str) {
        let pos = pos.min(self.rope.len_chars());
        trace!(pos, text, "buffer insert");
        self.rope.insert(pos, text);
        self.dirty = true;
    }

    /// Delete the range `start..end` (character offsets).
    pub fn delete(&mut self, start: usize, end: usize) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars());
        if start >= end {
            return;
        }
        trace!(start, end, "buffer delete");
        self.rope.remove(start..end);
        self.dirty = true;
    }

    /// Replace the full contents.
    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.dirty = true;
    }

    /// Convert a (line, col) pair to a character offset.
    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        if line >= self.len_lines() {
            return self.len_chars();
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        line_start + col.min(line_len)
    }

    /// Convert a character offset to (line, col).
    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let idx = char_idx.min(self.len_chars());
        let line = self.rope.char_to_line(idx);
        let line_start = self.rope.line_to_char(line);
        (line, idx - line_start)
    }
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_delete() {
        let mut buf = EditorBuffer::new();
        buf.insert(0, "Hello World");
        assert_eq!(buf.text(), "Hello World");

        buf.delete(5, 11);
        assert_eq!(buf.text(), "Hello");
    }

    #[test]
    fn line_queries() {
        let buf = EditorBuffer::from_str("line1\nline2\nline3");
        assert_eq!(buf.len_lines(), 3);
        assert_eq!(buf.line(1), "line2");
    }

    #[test]
    fn line_col_conversion() {
        let buf = EditorBuffer::from_str("abc\ndef\nghi");
        assert_eq!(buf.line_col_to_char(1, 2), 6); // 'd','e','f' → char 4,5,6 → col 2 of line 1
        let (l, c) = buf.char_to_line_col(6);
        assert_eq!((l, c), (1, 2));
    }
}
