use crate::core::buffer::FileBuffer;
use crate::core::history::History;

/// Represents a document processing unit that bundles a file buffer and its edit history.
pub struct Document {
    pub buffer: FileBuffer,
    pub history: History,
}

impl Document {
    pub fn new(buffer: FileBuffer) -> Self {
        Self {
            buffer,
            history: History::new(),
        }
    }
}
