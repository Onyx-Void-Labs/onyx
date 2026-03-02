// ─── Cursor ────────────────────────────────────────────────────────
// Multi-cursor / selection state for the editor.
// ────────────────────────────────────────────────────────────────────

/// Represents a single cursor position + optional selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Cursor position as a character offset into the buffer.
    pub pos: usize,
    /// Anchor for selection. If `anchor == pos`, nothing is selected.
    pub anchor: usize,
}

impl Cursor {
    pub fn new(pos: usize) -> Self {
        Self { pos, anchor: pos }
    }

    /// Is there an active selection?
    #[inline]
    pub fn has_selection(&self) -> bool {
        self.pos != self.anchor
    }

    /// Ordered selection range `(start, end)`.
    #[inline]
    pub fn selection_range(&self) -> (usize, usize) {
        if self.pos <= self.anchor {
            (self.pos, self.anchor)
        } else {
            (self.anchor, self.pos)
        }
    }

    /// Move cursor to `new_pos`, collapsing the selection.
    pub fn move_to(&mut self, new_pos: usize) {
        self.pos = new_pos;
        self.anchor = new_pos;
    }

    /// Extend selection to `new_pos` (shift-move).
    pub fn select_to(&mut self, new_pos: usize) {
        self.pos = new_pos;
    }

    /// Move right by `n` characters (capped at `max`).
    pub fn move_right(&mut self, n: usize, max: usize) {
        let new = (self.pos + n).min(max);
        self.move_to(new);
    }

    /// Move left by `n` characters.
    pub fn move_left(&mut self, n: usize) {
        let new = self.pos.saturating_sub(n);
        self.move_to(new);
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new(0)
    }
}
