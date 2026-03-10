// ─── Onyx Void — Cursor Model ─────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorState {
    /// The index of the block the cursor is currently in.
    pub block_index: usize,
    /// The byte offset within the text of that block.
    pub byte_offset: usize,
    /// The starting point of a selection, if any.
    pub selection_anchor: Option<(usize, usize)>, // (block_index, byte_offset)
    /// Whether the cursor is currently visible (blinking state).
    pub is_visible: bool,
    /// Last time the cursor blinked (for timers).
    pub last_blink_time: f64,
}

impl CursorState {
    pub fn new() -> Self {
        Self {
            block_index: 0,
            byte_offset: 0,
            selection_anchor: None,
            is_visible: true,
            last_blink_time: 0.0,
        }
    }

    /// Returns true if the cursor is currently anchoring a selection.
    #[allow(dead_code)]
    pub fn is_selecting(&self) -> bool {
        self.selection_anchor.is_some()
    }

    /// Get the normalized selection range (start_pos, end_pos).
    /// Returns None if no selection is active.
    pub fn get_selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection_anchor.map(|anchor| {
            let current = (self.block_index, self.byte_offset);
            if anchor < current {
                (anchor, current)
            } else {
                (current, anchor)
            }
        })
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Start a selection at the current cursor position.
    pub fn start_selection(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some((self.block_index, self.byte_offset));
        }
    }

    /// Update the cursor position, potentially extending a selection.
    pub fn move_to(&mut self, block_index: usize, byte_offset: usize, shift_pressed: bool) {
        if shift_pressed {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.block_index = block_index;
        self.byte_offset = byte_offset;
        self.reset_blink();
    }

    pub fn reset_blink(&mut self) {
        self.is_visible = true;
        // last_blink_time will be updated by the app loop
    }
}

impl Default for CursorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_initial_state() {
        let cursor = CursorState::new();
        assert_eq!(cursor.block_index, 0);
        assert_eq!(cursor.byte_offset, 0);
        assert!(!cursor.is_selecting());
        assert!(cursor.get_selection_range().is_none());
    }

    #[test]
    fn test_cursor_move_without_selection() {
        let mut cursor = CursorState::new();
        cursor.move_to(1, 5, false);
        assert_eq!(cursor.block_index, 1);
        assert_eq!(cursor.byte_offset, 5);
        assert!(!cursor.is_selecting());
    }

    #[test]
    fn test_cursor_selection() {
        let mut cursor = CursorState::new();
        cursor.move_to(0, 5, false);
        cursor.move_to(1, 10, true);
        assert!(cursor.is_selecting());
        assert_eq!(cursor.selection_anchor, Some((0, 5)));
        assert_eq!(cursor.get_selection_range(), Some(((0, 5), (1, 10))));
        
        cursor.move_to(0, 2, true);
        assert_eq!(cursor.get_selection_range(), Some(((0, 2), (0, 5))));
    }

    #[test]
    fn test_cursor_clear_selection() {
        let mut cursor = CursorState::new();
        cursor.move_to(0, 5, false);
        cursor.move_to(1, 10, true);
        assert!(cursor.is_selecting());
        cursor.clear_selection();
        assert!(!cursor.is_selecting());
    }
}
