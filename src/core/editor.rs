use crate::core::buffer::FileBuffer;
use std::cmp;
use std::ops::Range;
use std::sync::Arc;

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
    pub buffer: Arc<FileBuffer>,
    pub cursor_offset: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub search_state: SearchState,
}

impl Editor {
    pub fn new(buffer: Arc<FileBuffer>) -> Self {
        Self {
            buffer,
            cursor_offset: 0,
            selection_start: None,
            selection_end: None,
            search_state: SearchState::default(),
        }
    }

    pub fn total_size(&self) -> usize {
        self.buffer.len()
    }

    pub fn value_at_cursor(&self) -> Option<u8> {
        self.buffer.data().get(self.cursor_offset).copied()
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let min = cmp::min(start, end);
            let max = cmp::max(start, end);
            Some(min..max + 1)
        } else {
            None
        }
    }

    pub fn set_cursor_offset(&mut self, offset: usize) {
        let buffer_len = self.buffer.len();
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = offset.min(buffer_len.saturating_sub(1));
    }

    pub fn move_left(&mut self) {
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_right(&mut self) {
        let buffer_len = self.buffer.len();
        if self.cursor_offset < buffer_len.saturating_sub(1) {
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
        let buffer_len = self.buffer.len();
        let new_offset = self.cursor_offset + BYTES_PER_ROW;
        if new_offset < buffer_len {
            self.cursor_offset = new_offset;
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
        let buffer_len = self.buffer.len();
        if self.cursor_offset < buffer_len.saturating_sub(1) {
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
        let buffer_len = self.buffer.len();
        let new_offset = self.cursor_offset + BYTES_PER_ROW;
        if new_offset < buffer_len {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset = new_offset;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_all(&mut self) {
        let buffer_len = self.buffer.len();
        self.selection_start = Some(0);
        self.selection_end = Some(buffer_len.saturating_sub(1));
        self.cursor_offset = buffer_len.saturating_sub(1);
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
        let buffer_len = self.buffer.len();
        let move_amount = visible_rows * BYTES_PER_ROW;
        let new_offset = self.cursor_offset + move_amount;
        self.selection_start = None;
        self.selection_end = None;
        if new_offset < buffer_len {
            self.cursor_offset = new_offset;
        } else {
            self.cursor_offset = buffer_len.saturating_sub(1);
        }
    }

    pub fn home(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = 0;
    }

    pub fn end(&mut self) {
        let buffer_len = self.buffer.len();
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = buffer_len.saturating_sub(1);
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
        let buffer_len = self.buffer.len();
        let move_amount = visible_rows * BYTES_PER_ROW;
        let new_offset = self.cursor_offset + move_amount;
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        if new_offset < buffer_len {
            self.cursor_offset = new_offset;
        } else {
            self.cursor_offset = buffer_len.saturating_sub(1);
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
        let buffer_len = self.buffer.len();
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = buffer_len.saturating_sub(1);
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
}
