use crate::data::file_buffer::FileBuffer;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// A service for managing file buffers.
/// It caches open files to avoid redundant reads and ensures thread-safe access.
#[allow(dead_code)]
#[derive(Clone)]
pub struct EditorService {
    buffers: Arc<RwLock<HashMap<PathBuf, Arc<FileBuffer>>>>,
}

#[allow(dead_code)]
impl EditorService {
    /// Creates a new, empty EditorService.
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Opens a file asynchronously.
    /// If the file is already in the cache, it returns the cached buffer.
    /// Otherwise, it reads the file from disk, adds it to the cache, and returns it.
    /// This operation is thread-safe.
    pub async fn open_file(&self, path: PathBuf) -> anyhow::Result<Arc<FileBuffer>> {
        // First, check if the buffer is already in the cache with a read lock.
        if let Some(buffer) = self.buffers.read().unwrap().get(&path) {
            return Ok(buffer.clone());
        }

        // If not in the cache, read the file without holding any lock.
        let new_buffer = Arc::new(FileBuffer::new(&path).await?);

        // Acquire a write lock to insert the new buffer into the cache.
        let mut buffers = self.buffers.write().unwrap();

        // Before inserting, check again if another thread has inserted it in the meantime.
        if let Some(buffer) = buffers.get(&path) {
            return Ok(buffer.clone());
        }

        buffers.insert(path, new_buffer.clone());
        Ok(new_buffer)
    }
}
impl Default for EditorService {
    fn default() -> Self {
        Self::new()
    }
}
