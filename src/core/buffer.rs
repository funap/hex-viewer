use std::path::{Path, PathBuf};

/// A buffer to hold the contents of a file.
#[allow(dead_code)]
pub struct FileBuffer {
    path: PathBuf,
    data: Vec<u8>,
}

#[allow(dead_code)]
impl FileBuffer {
    /// Creates a new FileBuffer with the given path and data.
    pub fn new(path: PathBuf, data: Vec<u8>) -> Self {
        Self { path, data }
    }

    /// Creates an empty FileBuffer with no file path.
    pub fn empty() -> Self {
        Self {
            path: PathBuf::from("Untitled"),
            data: Vec::new(),
        }
    }

    /// Returns the path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a slice of the buffer's data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns a slice of the buffer's data in the given range.
    /// If the range is out of bounds, it is clamped to the valid range.
    pub fn get_range(&self, start: usize, len: usize) -> &[u8] {
        let start = start.min(self.data.len());
        let end = (start + len).min(self.data.len());
        &self.data[start..end]
    }
}
