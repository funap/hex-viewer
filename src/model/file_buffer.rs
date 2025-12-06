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

    /// Searches for a byte pattern in the buffer and returns all matching offsets.
    /// The limit parameter controls how many results to return.
    pub fn search_bytes(&self, pattern: &[u8], limit: crate::model::search::SearchLimit, range: Option<std::ops::Range<usize>>) -> Vec<usize> {
        if pattern.is_empty() || pattern.len() > self.data.len() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let pattern_len = pattern.len();
        let data_len = self.data.len();

        // Determine search range
        let (start, end) = if let Some(r) = range {
            (r.start.min(data_len), r.end.min(data_len))
        } else {
            (0, data_len)
        };

        if start >= end || end < pattern_len {
            return Vec::new();
        }

        let search_end = end - pattern_len;
        let mut first_match: Option<usize> = None;

        // Ensure start is within bounds for the loop
        let start = start.min(search_end + 1);

        for i in start..=search_end {
            if &self.data[i..i + pattern_len] == pattern {
                results.push(i);

                // Track first match for range-based limiting
                if first_match.is_none() {
                    first_match = Some(i);
                }

                // Check limit
                match limit {
                    crate::model::search::SearchLimit::Count(max) => {
                        if results.len() >= max {
                            break;
                        }
                    }
                    crate::model::search::SearchLimit::Range(range_bytes) => {
                        if let Some(first) = first_match {
                            if i >= first + range_bytes {
                                break;
                            }
                        }
                    }
                    crate::model::search::SearchLimit::Unlimited => {
                        // Continue searching
                    }
                }
            }
        }

        results
    }

    /// Searches for a UTF-8 text string in the buffer and returns all matching offsets.
    /// The limit parameter controls how many results to return.
    pub fn search_text(&self, text: &str, limit: crate::model::search::SearchLimit, range: Option<std::ops::Range<usize>>) -> Vec<usize> {
        self.search_bytes(text.as_bytes(), limit, range)
    }
}
