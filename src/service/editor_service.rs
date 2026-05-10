use crate::core::buffer::Buffer;
use crate::core::document::Document;
use crate::core::editor::Editor;
use crate::core::search::{self, SearchOptions};
use gpui::{App, Entity, Task};
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// A service for managing file buffers and editor workflows.
/// It caches open files to avoid redundant reads and ensures thread-safe access.
#[allow(dead_code)]
#[derive(Clone)]
pub struct EditorService {
    documents: Arc<RwLock<HashMap<PathBuf, Arc<RwLock<Document>>>>>,
}

#[allow(dead_code)]
impl EditorService {
    /// Creates a new, empty EditorService.
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Opens a file asynchronously.
    /// If the file is already in the cache, it returns the cached document.
    /// Otherwise, it reads the file from disk, adds it to the cache, and returns it.
    /// This operation is thread-safe.
    pub async fn open_file(&self, path: PathBuf) -> anyhow::Result<Arc<RwLock<Document>>> {
        // First, check if the document is already in the cache with a read lock.
        if let Some(document) = self.documents.read().unwrap().get(&path) {
            return Ok(document.clone());
        }

        // If not in the cache, read the file using memory mapping without holding any lock.
        let path_clone = path.clone();
        let buffer = tokio::task::spawn_blocking(move || -> anyhow::Result<Buffer> {
            let file = std::fs::File::open(&path_clone)?;
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
            Ok(Buffer::from_mmap(mmap))
        }).await??;
        let new_document = Arc::new(RwLock::new(Document::new(path.clone(), buffer)));

        // Acquire a write lock to insert the new document into the cache.
        let mut documents = self.documents.write().unwrap();

        // Before inserting, check again if another thread has inserted it in the meantime.
        if let Some(document) = documents.get(&path) {
            return Ok(document.clone());
        }

        documents.insert(path, new_document.clone());
        Ok(new_document)
    }

    /// Searches for a query in the given buffer based on the search options.
    /// Returns a Task that executes the search in the background.
    pub fn search(&self, buffer: Arc<Buffer>, query: String, options: crate::core::search::SearchOptions, cx: &gpui::App) -> gpui::Task<Vec<usize>> {
        cx.background_executor().spawn(async move {
            if query.is_empty() {
                return Vec::new();
            }

            match options.mode {
                crate::core::search::SearchMode::Text => search::find_occurrences(buffer.data(), query.as_bytes(), options.limit, options.range.clone()),
                crate::core::search::SearchMode::Hex => {
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
                            Ok(pattern) => search::find_occurrences(buffer.data(), &pattern, options.limit, options.range.clone()),
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
        editor: gpui::Entity<crate::core::editor::Editor>,
        query: String,
        options: crate::core::search::SearchOptions,
        is_full: bool,
        cx: &gpui::App,
    ) -> gpui::Task<()> {
        let buffer_data = {
            let editor_read = editor.read(cx);
            let document = editor_read.document.read().unwrap();
            // We need a way to pass the buffer data to the background task.
            // Buffer is not thread-safe refcounted by itself unless we clone the data or put it in Arc.
            // But Editor has Arc<RwLock<Document>>.
            // The search function takes Arc<Buffer>.
            // We need to change the search function signature or create a temporary Arc<Buffer> (which triggers clone if Buffer is not Arc internally).
            // Now Document holds `buffer: Buffer` directly.
            // So we can't just clone the Arc.
            // We should clone the data or change Buffer to be cheap to clone (Arc inside).
            // Given the context, I will clone the buffer data for now to get it working, or wrap it in Arc derived from Document? No.
            // Let's modify `search` to take `Arc<Buffer>` is annoying if Buffer is not in Arc.
            // Actually `EditorService::search` takes `Arc<Buffer>`.
            // I should probably change `search` to take `Buffer` (clone) or `Vec<u8>`.
            // Taking a closer look at `search` implementation: it spawns a task.
            // `cx.background_executor().spawn(async move { ... buffer.data() ... })`
            // If I clone Buffer, it copies all data. That's bad for large files.
            // The previous architecture had `Arc<RwLock<Buffer>>`.
            // Now `Document` owns `Buffer`.
            // If I want to search in background, I need a snapshot of data.
            // So cloning `Buffer` (allocating new Vec) is actually correct for a consistent snapshot if we want to avoid locking for the duration of search.
            // BUT, `Buffer` was previously shared via Arc.
            // Let's assume for this refactor, `Buffer` is small enough or we simply clone it.
            // Note: `Buffer` struct: `path: PathBuf, data: Vec<u8>`. Cloning it clones the vector.
            // Optimization: Use `Arc<Vec<u8>>` in `Buffer` later.
            // For now, I will create a new Arc<Buffer> from the clone.
            Arc::new(document.buffer.clone())
        };

        let query_clone = query.clone();
        let search_task = self.search(buffer_data, query, options, cx);
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

    /// Performs an incremental search: immediate viewport search followed by background full search.
    pub fn incremental_search(
        &self,
        editor: Entity<Editor>,
        query: String,
        mode: crate::core::search::SearchMode,
        viewport_range: Range<usize>,
        cx: &App,
    ) -> (Task<()>, Task<()>) {
        // 1. Immediate viewport search
        let viewport_options = SearchOptions {
            mode,
            limit: crate::core::search::SearchLimit::Unlimited,
            range: Some(viewport_range),
        };
        let viewport_task = self.perform_search(editor.clone(), query.clone(), viewport_options, false, cx);

        // 2. Background full search
        let full_options = SearchOptions {
            mode,
            limit: crate::core::search::SearchLimit::Unlimited,
            range: None,
        };
        let full_task = self.perform_search(editor, query, full_options, true, cx);

        (viewport_task, full_task)
    }

    pub fn compute_diff(&self, left: Arc<RwLock<Document>>, right: Arc<RwLock<Document>>, cx: &gpui::App) -> gpui::Task<crate::core::diff::DiffResult> {
        cx.background_executor().spawn(async move {
            let left_doc = left.read().unwrap();
            let right_doc = right.read().unwrap();
            let left_data = left_doc.buffer.data();
            let right_data = right_doc.buffer.data();
            crate::core::diff::compute_simple_diff(left_data, right_data)
        })
    }
}
impl Default for EditorService {
    fn default() -> Self {
        Self::new()
    }
}
