use crate::model::file_buffer::FileBuffer;
use std::cmp;
use std::ops::Range;
use std::sync::Arc;

/// Represents the editor.
pub struct Editor {
    pub buffer: Arc<FileBuffer>,
    pub cursor_offset: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
}

impl Editor {
    pub fn new(buffer: Arc<FileBuffer>) -> Self {
        Self {
            buffer,
            cursor_offset: 0,
            selection_start: None,
            selection_end: None,
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
}
