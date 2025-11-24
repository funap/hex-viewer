use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// A buffer to hold the contents of a file.
#[allow(dead_code)]
pub struct FileBuffer {
    pub path: PathBuf,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
impl FileBuffer {
    /// Creates a new FileBuffer by asynchronously reading the contents of the file at the given path.
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        let mut file = File::open(path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        Ok(Self {
            path: path.to_path_buf(),
            data,
        })
    }

    /// Creates an empty FileBuffer with no file path.
    pub fn empty() -> Self {
        Self {
            path: PathBuf::from("Untitled"),
            data: Vec::new(),
        }
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

    /// Searches for a byte pattern in the buffer and returns all matching offsets.
    /// Uses a simple but efficient search algorithm.
    pub fn search_bytes(&self, pattern: &[u8]) -> Vec<usize> {
        if pattern.is_empty() || pattern.len() > self.data.len() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let pattern_len = pattern.len();
        let data_len = self.data.len();

        for i in 0..=(data_len - pattern_len) {
            if &self.data[i..i + pattern_len] == pattern {
                results.push(i);
            }
        }

        results
    }

    /// Searches for a UTF-8 text string in the buffer and returns all matching offsets.
    pub fn search_text(&self, text: &str) -> Vec<usize> {
        self.search_bytes(text.as_bytes())
    }
}
