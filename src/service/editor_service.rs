use crate::model::file_buffer::FileBuffer;
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
        let data = tokio::fs::read(&path).await?;
        let new_buffer = Arc::new(FileBuffer::new(path.clone(), data));

        // Acquire a write lock to insert the new buffer into the cache.
        let mut buffers = self.buffers.write().unwrap();

        // Before inserting, check again if another thread has inserted it in the meantime.
        if let Some(buffer) = buffers.get(&path) {
            return Ok(buffer.clone());
        }

        buffers.insert(path, new_buffer.clone());
        Ok(new_buffer)
    }

    /// Searches for a query in the given buffer based on the search options.
    /// Returns a Task that executes the search in the background.
    pub fn search(&self, buffer: Arc<FileBuffer>, query: String, options: crate::model::search::SearchOptions, cx: &gpui::App) -> gpui::Task<Vec<usize>> {
        cx.background_executor().spawn(async move {
            if query.is_empty() {
                return Vec::new();
            }

            match options.mode {
                crate::model::search::SearchMode::Text => buffer.search_text(&query, options.limit, options.range.clone()),
                crate::model::search::SearchMode::Hex => {
                    // Parse hex string (remove spaces and keep only valid hex characters)
                    let hex_str: String = query.chars().filter(|c| c.is_ascii_hexdigit()).collect();

                    if hex_str.is_empty() || hex_str.len() % 2 != 0 {
                        // Invalid or empty hex string
                        Vec::new()
                    } else {
                        let bytes: Result<Vec<u8>, _> = (0..hex_str.len())
                            .step_by(2)
                            .map(|i| {
                                // Safe to use byte indexing since we filtered to ASCII only
                                u8::from_str_radix(&hex_str[i..i + 2], 16)
                            })
                            .collect();

                        match bytes {
                            Ok(pattern) => buffer.search_bytes(&pattern, options.limit, options.range.clone()),
                            Err(_) => Vec::new(),
                        }
                    }
                }
            }
        })
    }

    /// Performs a search and updates the provided Editor entity with the results.
    pub fn perform_search(
        &self,
        editor: gpui::Entity<crate::model::editor::Editor>,
        query: String,
        options: crate::model::search::SearchOptions,
        is_full: bool,
        cx: &gpui::App,
    ) -> gpui::Task<()> {
        let buffer = editor.read(cx).buffer.clone();
        let query_clone = query.clone();
        let search_task = self.search(buffer, query, options, cx);
        let editor_weak = editor.downgrade();

        cx.spawn(move |cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let results = search_task.await;
                if let Some(editor) = editor_weak.upgrade() {
                    editor
                        .update(&mut cx, |editor, cx| {
                            editor.set_search_query(query_clone);
                            editor.set_search_results(results, is_full);
                            cx.notify();
                        })
                        .ok();
                }
            }
        })
    }

    pub fn compute_diff(&self, left: Arc<FileBuffer>, right: Arc<FileBuffer>, cx: &gpui::App) -> gpui::Task<crate::model::diff::DiffResult> {
        cx.background_executor().spawn(async move {
            let left_data = left.data();
            let right_data = right.data();
            crate::model::diff::compute_simple_diff(left_data, right_data)
        })
    }
}
impl Default for EditorService {
    fn default() -> Self {
        Self::new()
    }
}
