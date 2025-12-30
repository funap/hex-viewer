use crate::core::buffer::FileBuffer;
use crate::core::command::Command;
use crate::core::document::Document;
use std::cmp;
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
}

impl Editor {
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        Self {
            document,
            cursor_offset: 0,
            selection_start: None,
            selection_end: None,
            search_state: SearchState::default(),
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
        if self.cursor_offset >= BYTES_PER_ROW {
            self.cursor_offset -= BYTES_PER_ROW;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_down(&mut self) {
        let buffer_len = self.total_size();
        let new_offset = self.cursor_offset + BYTES_PER_ROW;
        if new_offset <= buffer_len {
            self.cursor_offset = new_offset;
            self.selection_start = None;
            self.selection_end = None;
        } else {
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
        if self.cursor_offset >= BYTES_PER_ROW {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset -= BYTES_PER_ROW;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_down(&mut self) {
        let buffer_len = self.total_size();
        let new_offset = self.cursor_offset + BYTES_PER_ROW;
        if new_offset <= buffer_len {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset = new_offset;
            self.selection_end = Some(self.cursor_offset);
        } else {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
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
        let move_amount = visible_rows * BYTES_PER_ROW;
        self.selection_start = None;
        self.selection_end = None;
        if self.cursor_offset >= move_amount {
            self.cursor_offset -= move_amount;
        } else {
            self.cursor_offset = 0;
        }
    }

    pub fn page_down(&mut self, visible_rows: usize) {
        let buffer_len = self.total_size();
        let move_amount = visible_rows * BYTES_PER_ROW;
        let new_offset = self.cursor_offset + move_amount;
        self.selection_start = None;
        self.selection_end = None;
        if new_offset <= buffer_len {
            self.cursor_offset = new_offset;
        } else {
            self.cursor_offset = buffer_len;
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
        let move_amount = visible_rows * BYTES_PER_ROW;
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        if self.cursor_offset >= move_amount {
            self.cursor_offset -= move_amount;
        } else {
            self.cursor_offset = 0;
        }
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_page_down(&mut self, visible_rows: usize) {
        let buffer_len = self.total_size();
        let move_amount = visible_rows * BYTES_PER_ROW;
        let new_offset = self.cursor_offset + move_amount;
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        if new_offset <= buffer_len {
            self.cursor_offset = new_offset;
        } else {
            self.cursor_offset = buffer_len;
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

        let mut command = {
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
        let mut command = {
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

    fn create_editor_with_content(content: &[u8]) -> Editor {
        let buffer = FileBuffer::new(std::path::PathBuf::from("test"), content.to_vec());
        let document = Arc::new(RwLock::new(Document::new(buffer)));
        Editor::new(document)
    }

    #[test]
    fn test_initialization() {
        let editor = create_editor_with_content(b"test");
        assert_eq!(editor.cursor_offset, 0);
        assert!(editor.selection_start.is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = create_editor_with_content(b"12345678901234567890"); // > 16 bytes for row testing

        // Right
        editor.move_right();
        assert_eq!(editor.cursor_offset, 1);
        editor.move_right();
        assert_eq!(editor.cursor_offset, 2);

        // Left
        editor.move_left();
        assert_eq!(editor.cursor_offset, 1);

        // Down (BYTES_PER_ROW = 16)
        editor.cursor_offset = 0;
        editor.move_down();
        assert_eq!(editor.cursor_offset, 16);

        // Up
        editor.move_up();
        assert_eq!(editor.cursor_offset, 0);

        // Boundaries
        editor.move_left();
        assert_eq!(editor.cursor_offset, 0); // Should stick at 0

        editor.end(); // Move to end
        let end_pos = editor.cursor_offset;
        editor.move_right();
        assert_eq!(editor.cursor_offset, end_pos); // Should stick at end
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
        assert_eq!(editor.selection_end, Some(5));
    }

    #[test]
    fn test_search_navigation() {
        let mut editor = create_editor_with_content(b"AABBCCAA");
        let results = vec![0, 6]; // Matches for "AA"
        editor.set_search_results(results.clone(), true);

        // Initial state
        assert_eq!(editor.search_state.results, results);
        assert_eq!(editor.search_state.current_result_index, Some(0));

        // Next result
        assert_eq!(editor.next_search_result(), Some(6));
        assert_eq!(editor.search_state.current_result_index, Some(1));

        // Wrap around
        assert_eq!(editor.next_search_result(), Some(0));
        assert_eq!(editor.search_state.current_result_index, Some(0));

        // Previous result
        assert_eq!(editor.prev_search_result(), Some(6));
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = create_editor_with_content(b"");

        // Execute Insert Command
        let cmd = Box::new(InsertCharCommand::new(0, b'A'));
        editor.execute_command(cmd);

        assert_eq!(editor.total_size(), 1);
        assert_eq!(editor.value_at_cursor(), None); // Cursor moved to 1

        // Undo
        editor.undo();
        assert_eq!(editor.total_size(), 0);

        // Redo
        editor.redo();
        assert_eq!(editor.total_size(), 1);
    }

    #[test]
    fn test_shared_document() {
        let buffer = FileBuffer::new(std::path::PathBuf::from("test"), b"".to_vec());
        let document = Arc::new(RwLock::new(Document::new(buffer)));
        let mut editor1 = Editor::new(document.clone());
        let mut editor2 = Editor::new(document.clone());

        // Execute Insert Command on editor1
        let cmd = Box::new(InsertCharCommand::new(0, b'A'));
        editor1.execute_command(cmd);

        // Verify editor2 sees the change
        assert_eq!(editor2.total_size(), 1);

        // Undo on editor2
        editor2.undo();

        // Verify editor1 sees the undo
        assert_eq!(editor1.total_size(), 0);
    }
}
