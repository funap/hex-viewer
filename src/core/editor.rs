use crate::core::command::Command;
use crate::core::document::Document;
use std::cmp;
use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;
use std::sync::RwLock;

pub const BYTES_PER_ROW: usize = 16;

#[derive(Default, Clone)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<usize>,
    pub current_result_index: Option<usize>,
    pub is_full_search_complete: bool,
}

/// Represents the editor.
pub struct Editor {
    // Shared document containing buffer and history
    pub document: Arc<RwLock<Document>>,
    pub cursor_offset: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub search_state: SearchState,
    pub custom_breaks: BTreeSet<usize>,
}

impl Editor {
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        Self {
            document,
            cursor_offset: 0,
            selection_start: None,
            selection_end: None,
            search_state: SearchState::default(),
            custom_breaks: BTreeSet::new(),
        }
    }

    pub fn total_size(&self) -> usize {
        self.document.read().unwrap().buffer.len()
    }

    pub fn value_at_cursor(&self) -> Option<u8> {
        let binding = self.document.read().unwrap();
        let buffer = &binding.buffer;
        buffer.data().get(self.cursor_offset).copied()
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let min = cmp::min(start, end);
            let max = cmp::max(start, end);
            Some(min..max)
        } else {
            None
        }
    }

    pub fn set_cursor_offset(&mut self, offset: usize) {
        let buffer_len = self.total_size();
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = offset.min(buffer_len);
    }

    pub fn move_left(&mut self) {
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_right(&mut self) {
        let buffer_len = self.total_size();
        if self.cursor_offset < buffer_len {
            self.cursor_offset += 1;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_up(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        if current_line_idx > 0 {
            let current_line_start = line_starts[current_line_idx];
            let offset_in_line = self.cursor_offset - current_line_start;
            let prev_line_start = line_starts[current_line_idx - 1];
            let prev_line_len = current_line_start - prev_line_start;

            self.cursor_offset = prev_line_start + cmp::min(offset_in_line, prev_line_len - 1);
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_down(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        if current_line_idx + 1 < line_starts.len() {
            let current_line_start = line_starts[current_line_idx];
            let offset_in_line = self.cursor_offset - current_line_start;
            let next_line_start = line_starts[current_line_idx + 1];
            let next_line_end = if current_line_idx + 2 < line_starts.len() {
                line_starts[current_line_idx + 2]
            } else {
                self.total_size()
            };
            let next_line_len = next_line_end - next_line_start;

            if next_line_len > 0 {
                self.cursor_offset = next_line_start + cmp::min(offset_in_line, next_line_len - 1);
            } else {
                self.cursor_offset = next_line_start;
            }
            self.selection_start = None;
            self.selection_end = None;
        } else {
            let buffer_len = self.total_size();
            self.cursor_offset = buffer_len;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn select_left(&mut self) {
        if self.cursor_offset > 0 {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset -= 1;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_right(&mut self) {
        let buffer_len = self.total_size();
        if self.cursor_offset < buffer_len {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset += 1;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_up(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        if current_line_idx > 0 {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            let current_line_start = line_starts[current_line_idx];
            let offset_in_line = self.cursor_offset - current_line_start;
            let prev_line_start = line_starts[current_line_idx - 1];
            let prev_line_len = current_line_start - prev_line_start;

            self.cursor_offset = prev_line_start + cmp::min(offset_in_line, prev_line_len - 1);
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_down(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }

        if current_line_idx + 1 < line_starts.len() {
            let current_line_start = line_starts[current_line_idx];
            let offset_in_line = self.cursor_offset - current_line_start;
            let next_line_start = line_starts[current_line_idx + 1];
            let next_line_end = if current_line_idx + 2 < line_starts.len() {
                line_starts[current_line_idx + 2]
            } else {
                self.total_size()
            };
            let next_line_len = next_line_end - next_line_start;

            if next_line_len > 0 {
                self.cursor_offset = next_line_start + cmp::min(offset_in_line, next_line_len - 1);
            } else {
                self.cursor_offset = next_line_start;
            }
            self.selection_end = Some(self.cursor_offset);
        } else {
            let buffer_len = self.total_size();
            self.cursor_offset = buffer_len;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_all(&mut self) {
        let buffer_len = self.total_size();
        self.selection_start = Some(0);
        self.selection_end = Some(buffer_len);
        self.cursor_offset = buffer_len;
    }

    pub fn page_up(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        self.selection_start = None;
        self.selection_end = None;

        let target_line_idx = current_line_idx.saturating_sub(visible_rows);
        let current_line_start = line_starts[current_line_idx];
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts[target_line_idx];
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts[target_line_idx + 1]
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
    }

    pub fn page_down(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        self.selection_start = None;
        self.selection_end = None;

        let target_line_idx = cmp::min(current_line_idx + visible_rows, line_starts.len() - 1);
        let current_line_start = line_starts[current_line_idx];
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts[target_line_idx];
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts[target_line_idx + 1]
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        if target_line_idx == line_starts.len() - 1 && target_line_len == 0 {
            self.cursor_offset = self.total_size();
        } else {
            self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
        }
    }

    pub fn home(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = 0;
    }

    pub fn end(&mut self) {
        let buffer_len = self.total_size();
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = buffer_len;
    }

    pub fn select_page_up(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }

        let target_line_idx = current_line_idx.saturating_sub(visible_rows);
        let current_line_start = line_starts[current_line_idx];
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts[target_line_idx];
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts[target_line_idx + 1]
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_page_down(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = match line_starts.binary_search(&self.cursor_offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };

        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }

        let target_line_idx = cmp::min(current_line_idx + visible_rows, line_starts.len() - 1);
        let current_line_start = line_starts[current_line_idx];
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts[target_line_idx];
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts[target_line_idx + 1]
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        if target_line_idx == line_starts.len() - 1 && target_line_len == 0 {
            self.cursor_offset = self.total_size();
        } else {
            self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
        }
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_home(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = 0;
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_end(&mut self) {
        let buffer_len = self.total_size();
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = buffer_len;
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn start_drag(&mut self, byte_pos: usize) {
        self.cursor_offset = byte_pos;
        self.selection_start = Some(byte_pos);
        self.selection_end = Some(byte_pos);
    }

    pub fn continue_drag(&mut self, byte_pos: usize) {
        self.selection_end = Some(byte_pos);
    }

    pub fn set_search_query(&mut self, query: String) {
        if self.search_state.query != query {
            self.search_state.query = query;
            self.search_state.results.clear();
            self.search_state.current_result_index = None;
            self.search_state.is_full_search_complete = false;
        }
    }

    pub fn set_search_results(&mut self, results: Vec<usize>, is_full: bool) {
        self.search_state.results = results;
        if is_full {
            self.search_state.is_full_search_complete = true;
        }
        if !self.search_state.results.is_empty() && self.search_state.current_result_index.is_none() {
            self.search_state.current_result_index = Some(0);
        }
    }

    pub fn clear_search(&mut self) {
        self.search_state.query.clear();
        self.search_state.results.clear();
        self.search_state.current_result_index = None;
        self.search_state.is_full_search_complete = false;
    }

    pub fn next_search_result(&mut self) -> Option<usize> {
        if self.search_state.results.is_empty() {
            return None;
        }

        let next_index = if let Some(index) = self.search_state.current_result_index {
            (index + 1) % self.search_state.results.len()
        } else {
            0
        };

        self.search_state.current_result_index = Some(next_index);
        Some(self.search_state.results[next_index])
    }

    pub fn line_starts(&self) -> Vec<usize> {
        let total_size = self.total_size();
        let mut starts = Vec::new();
        let mut current = 0;

        while current < total_size {
            starts.push(current);
            // Find the next custom break after current
            let next_custom = self.custom_breaks.range((current + 1)..).next().copied();
            // Default break after BYTES_PER_ROW
            let next_default = current + BYTES_PER_ROW;

            match next_custom {
                Some(break_pos) if break_pos < next_default => {
                    current = break_pos;
                }
                _ => {
                    current = next_default;
                }
            }
        }

        // If the buffer is empty, or the last byte was a break, add one more empty line if needed?
        // Actually, HexView handles empty buffer by showing at least one line.
        if starts.is_empty() {
            starts.push(0);
        }

        starts
    }

    pub fn add_custom_break(&mut self, offset: usize) {
        if offset > 0 && offset < self.total_size() {
            self.custom_breaks.insert(offset);
        }
    }

    pub fn remove_custom_break(&mut self, offset: usize) {
        self.custom_breaks.remove(&offset);
    }

    pub fn toggle_custom_break(&mut self, offset: usize) {
        if self.custom_breaks.contains(&offset) {
            self.remove_custom_break(offset);
        } else {
            self.add_custom_break(offset);
        }
    }

    pub fn prev_search_result(&mut self) -> Option<usize> {
        if self.search_state.results.is_empty() {
            return None;
        }

        let prev_index = if let Some(index) = self.search_state.current_result_index {
            if index == 0 { self.search_state.results.len() - 1 } else { index - 1 }
        } else {
            self.search_state.results.len() - 1
        };

        self.search_state.current_result_index = Some(prev_index);
        Some(self.search_state.results[prev_index])
    }

    pub fn current_search_result(&self) -> Option<usize> {
        self.search_state.current_result_index.and_then(|i| self.search_state.results.get(i).copied())
    }

    pub fn execute_command(&mut self, mut command: Box<dyn Command>) {
        command.execute(self);
        self.document.write().unwrap().history.push(command);
    }

    pub fn undo(&mut self) {
        // Need to acquire a write lock on the document to access history
        // And also we need to pop from history, then call command.undo(self)
        // command.undo might need to access document.buf, which is in the same lock if we are not careful
        // The current History implementation stores Box<dyn Command>, which is fine.
        // But if I hold the lock while calling command.undo(self), and command.undo tries to lock document again... deadlock.

        let command = {
            let mut doc = self.document.write().unwrap();
            doc.history.pop_undo()
        };

        if let Some(mut cmd) = command {
            cmd.undo(self);

            // Re-acquire lock to push redo
            self.document.write().unwrap().history.push_redo(cmd);
        }
    }

    pub fn redo(&mut self) {
        let command = {
            let mut doc = self.document.write().unwrap();
            doc.history.pop_redo()
        };

        if let Some(mut cmd) = command {
            cmd.execute(self);

            // Re-acquire lock to push undo
            self.document.write().unwrap().history.push_undo(cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::command::InsertCharCommand;
    use std::sync::Arc;

    fn create_editor_with_content(content: &[u8]) -> Editor {
        let buffer = crate::core::buffer::Buffer::new(content.to_vec());
        let document = Arc::new(RwLock::new(Document::new(std::path::PathBuf::from("test"), buffer)));
        Editor::new(document)
    }

    #[test]
    fn test_initialization() {
        let mut editor = create_editor_with_content(b"Hello");
        assert_eq!(editor.total_size(), 5);
        assert_eq!(editor.cursor_offset, 0);
        assert!(editor.selection_start.is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = create_editor_with_content(b"123");

        // Move right
        editor.move_right();
        assert_eq!(editor.cursor_offset, 1);

        // Move left
        editor.move_left();
        assert_eq!(editor.cursor_offset, 0);

        // Boundary checks
        editor.move_left();
        assert_eq!(editor.cursor_offset, 0);

        editor.end();
        assert_eq!(editor.cursor_offset, 3);
        editor.move_right();
        assert_eq!(editor.cursor_offset, 3);
    }

    #[test]
    fn test_selection() {
        let mut editor = create_editor_with_content(b"12345");

        // Select Right
        editor.select_right();
        assert_eq!(editor.selection_start, Some(0));
        assert_eq!(editor.selection_end, Some(1));
        assert_eq!(editor.cursor_offset, 1);

        // Clear selection on move
        editor.move_right();
        assert!(editor.selection_start.is_none());
        assert!(editor.selection_end.is_none());

        // Select All
        editor.select_all();
        assert_eq!(editor.selection_start, Some(0));
        assert_eq!(editor.selection_end, Some(5)); // Corrected expectation
    }

    #[test]
    fn test_search_navigation() {
        let mut editor = create_editor_with_content(b"test match test");
        editor.search_state.results = vec![0, 11];

        // Ensure we handle no current index gracefully
        assert_eq!(editor.current_search_result(), None);

        // Next: 0 -> 11
        editor.next_search_result();
        assert_eq!(editor.current_search_result(), Some(0));
        assert_eq!(editor.cursor_offset, 0);

        editor.next_search_result();
        assert_eq!(editor.current_search_result(), Some(11));

        // Wrap around
        editor.next_search_result();
        assert_eq!(editor.current_search_result(), Some(0));

        // Prev
        editor.prev_search_result();
        assert_eq!(editor.current_search_result(), Some(11));
    }

    #[test]
    fn test_shared_document() {
        let buffer = crate::core::buffer::Buffer::new(b"".to_vec());
        let document = Arc::new(RwLock::new(Document::new(std::path::PathBuf::from("test"), buffer)));
        let mut editor1 = Editor::new(document.clone());
        let mut editor2 = Editor::new(document.clone());

        // Insert in editor1
        let cmd1 = Box::new(InsertCharCommand::new(0, b'A'));
        editor1.execute_command(cmd1);

        // Verify editor2 sees change
        assert_eq!(editor2.total_size(), 1);

        // Undo in editor2
        editor2.undo();
        assert_eq!(editor1.total_size(), 0);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = create_editor_with_content(b"");

        // Insert 'A'
        let cmd = Box::new(InsertCharCommand::new(0, b'A'));
        editor.execute_command(cmd);
        assert_eq!(editor.total_size(), 1);

        // Undo
        editor.undo();
        assert_eq!(editor.total_size(), 0);

        // Redo
        editor.redo();
        assert_eq!(editor.total_size(), 1);
    }

    #[test]
    fn test_line_starts_with_custom_breaks() {
        let mut editor = create_editor_with_content(&[0; 32]);
        // Default: 0, 16
        assert_eq!(editor.line_starts(), vec![0, 16]);

        // Add custom break at 10
        editor.add_custom_break(10);
        // Should be 0, 10, 26
        // Wait, current logic:
        // current=0 -> push 0. next_custom=10, next_default=16. 10 < 16, so current=10.
        // current=10 -> push 10. next_custom=None, next_default=26. current=26.
        // current=26 -> push 26. next_custom=None, next_default=42. current=42 (>= 32, loop ends).
        assert_eq!(editor.line_starts(), vec![0, 10, 26]);

        // Add custom break at 5
        editor.add_custom_break(5);
        // 0, 5, 10, 26
        assert_eq!(editor.line_starts(), vec![0, 5, 10, 26]);
    }

    #[test]
    fn test_move_up_down_with_custom_breaks() {
        let mut editor = create_editor_with_content(&[0; 32]);
        editor.add_custom_break(10); // Lines: [0..10], [10..26], [26..32]

        editor.set_cursor_offset(5);
        editor.move_down();
        // Move from line 0 pos 5 to line 1 pos 5 (offset 10 + 5 = 15)
        assert_eq!(editor.cursor_offset, 15);

        editor.move_down();
        // Move from line 1 pos 5 to line 2 pos 5 (offset 26 + 5 = 31)
        assert_eq!(editor.cursor_offset, 31);

        editor.move_up();
        assert_eq!(editor.cursor_offset, 15);

        editor.move_up();
        assert_eq!(editor.cursor_offset, 5);

        // Test clamping to line length
        editor.set_cursor_offset(28); // Line 2, pos 2 (28-26)
        editor.move_up();
        // Line 1 is 16 bytes long. pos 2 is valid. 10 + 2 = 12.
        assert_eq!(editor.cursor_offset, 12);

        editor.set_cursor_offset(20); // Line 1, pos 10
        editor.move_down();
        // Line 2 is 6 bytes long. pos 10 is too far. Clamp to 5. 26 + 5 = 31.
        assert_eq!(editor.cursor_offset, 31);
    }
}
