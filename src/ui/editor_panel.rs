use crate::model::file_buffer::FileBuffer;
use gpui::prelude::*;
use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use std::sync::Arc;

use crate::actions::{FocusHexView, SearchNext, SearchPrev, ToggleSearch};
use crate::app_state::AppState;
use crate::model::search::SearchMode;
use crate::ui::component::hex_view::{self, HexView};
use crate::ui::component::search_bar::{SearchBar, SearchBarEvent};
use gpui_component::ActiveTheme;

const CONTEXT: &str = "EditorPanel";

pub(crate) fn init(cx: &mut App) {
    // Initialize HexView actions and keybindings
    hex_view::init(cx);
    cx.bind_keys([
        KeyBinding::new("cmd-f", ToggleSearch, Some(CONTEXT)),
        KeyBinding::new("f3", SearchNext, Some(CONTEXT)),
        KeyBinding::new("cmd-g", SearchNext, Some(CONTEXT)),
        KeyBinding::new("shift-f3", SearchPrev, Some(CONTEXT)),
        KeyBinding::new("cmd-shift-g", SearchPrev, Some(CONTEXT)),
    ]);
}

pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    focus_handle: FocusHandle,
    hex_view: Entity<HexView>,
    is_search_visible: bool,
    search_bar: Entity<SearchBar>,
    search_results: Vec<usize>,
    current_result_index: Option<usize>,
    last_search_query: String,
    search_task: Option<Task<()>>,
}

impl EditorPanel {
    pub fn new(buffer: Arc<FileBuffer>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hex_view = cx.new(|cx| HexView::new(cx).buffer(buffer.clone()));
        let search_bar = cx.new(|cx| SearchBar::new(window, cx));

        cx.subscribe(
            &search_bar,
            |this, _, event: &SearchBarEvent, cx| match event {
                SearchBarEvent::IncrementalSearch(query, mode) => {
                    this.perform_incremental_search(query, *mode, cx);
                }
                SearchBarEvent::FullSearch(query, mode) => {
                    this.perform_full_search(query, *mode, cx);
                }
                SearchBarEvent::Next => {
                    this.perform_search_next(cx);
                }
                SearchBarEvent::Prev => {
                    this.perform_search_prev(cx);
                }
                SearchBarEvent::Dismiss => {
                    this.is_search_visible = false;
                    cx.dispatch_action(&FocusHexView);
                    cx.notify();
                }
            },
        )
        .detach();

        // Subscribe to HexView scroll events to update highlights when scrolling
        cx.subscribe(
            &hex_view,
            |this, _, event: &crate::ui::component::hex_view::HexViewEvent, cx| {
                if let crate::ui::component::hex_view::HexViewEvent::Scrolled(_) = event {
                    // Update highlights if there's an active search
                    if this.is_search_visible && !this.search_results.is_empty() {
                        this.update_viewport_highlights(cx);
                    }
                }
            },
        )
        .detach();

        Self {
            buffer,
            focus_handle: cx.focus_handle(),
            hex_view,
            is_search_visible: false,
            search_bar,
            search_results: Vec::new(),
            current_result_index: None,
            last_search_query: String::new(),
            search_task: None,
        }
    }

    fn toggle_search(&mut self, _: &ToggleSearch, window: &mut Window, cx: &mut Context<Self>) {
        self.is_search_visible = !self.is_search_visible;
        if self.is_search_visible {
            // Focus the search input
            self.search_bar.update(cx, |bar, cx| {
                bar.focus(window, cx);
            });
        } else {
            self.hex_view.read(cx).focus_handle(cx).focus(window);
        }
        cx.notify();
    }

    fn perform_incremental_search(
        &mut self,
        query: &str,
        mode: SearchMode,
        cx: &mut Context<Self>,
    ) {
        // Search entire file, then filter to viewport on display
        self.perform_search_with_limit(
            query,
            mode,
            crate::model::search::SearchLimit::Unlimited,
            false, // Reset to first result
            cx,
        );
    }

    fn perform_incremental_search_preserve_index(
        &mut self,
        query: &str,
        mode: SearchMode,
        cx: &mut Context<Self>,
    ) {
        // Calculate viewport range
        let (start_byte, end_byte) = self.hex_view.read(cx).viewport_byte_range();
        let viewport_size = end_byte.saturating_sub(start_byte);

        // Use viewport size as the search range, preserving current index
        self.perform_search_with_limit(
            query,
            mode,
            crate::model::search::SearchLimit::Range(viewport_size),
            true, // Preserve current result index
            cx,
        );
    }

    fn perform_full_search(&mut self, query: &str, mode: SearchMode, cx: &mut Context<Self>) {
        self.perform_search_with_limit(
            query,
            mode,
            crate::model::search::SearchLimit::Unlimited,
            false, // Reset to first result
            cx,
        );
    }

    fn perform_search_with_limit(
        &mut self,
        query: &str,
        mode: SearchMode,
        limit: crate::model::search::SearchLimit,
        preserve_index: bool,
        cx: &mut Context<Self>,
    ) {
        if query.is_empty() {
            self.search_results.clear();
            self.current_result_index = None;
            self.hex_view.update(cx, |view, cx| {
                view.set_highlights(Vec::new(), cx);
            });
            self.update_search_bar_results(cx);
            return;
        }

        self.last_search_query = query.to_string();

        // Cancel previous search task
        self.search_task = None;

        let app_state = AppState::global(cx);
        let options = crate::model::search::SearchOptions { mode, limit };
        let search_task =
            app_state
                .editor_service
                .search(self.buffer.clone(), query.to_string(), options, cx);

        // Spawn task to handle search results
        let task = cx.spawn(async move |this, cx| {
            let results = search_task.await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    // Save current index if preserving
                    let saved_index = if preserve_index {
                        this.current_result_index
                    } else {
                        None
                    };

                    this.search_results = results;

                    if !this.search_results.is_empty() {
                        // Restore saved index or reset to 0
                        this.current_result_index = saved_index.or(Some(0));
                        // Preserve scroll position if requested
                        this.highlight_current_result(preserve_index, cx);
                    } else {
                        this.current_result_index = None;
                        this.hex_view.update(cx, |view, cx| {
                            view.set_highlights(Vec::new(), cx);
                        });
                    }

                    this.update_search_bar_results(cx);
                })
                .ok();
            }
        });
        self.search_task = Some(task);
    }

    fn update_search_bar_results(&mut self, cx: &mut Context<Self>) {
        let count = self.search_results.len();
        let current = self.current_result_index;
        self.search_bar.update(cx, |bar, cx| {
            bar.set_results(count, current, cx);
        });
    }

    fn highlight_current_result(&mut self, preserve_scroll: bool, cx: &mut Context<Self>) {
        if let Some(index) = self.current_result_index {
            if let Some(&offset) = self.search_results.get(index) {
                let bar = self.search_bar.read(cx);
                let query = bar.query(cx);
                let mode = bar.mode();
                let pattern_len = match mode {
                    SearchMode::Text => query.len(),
                    SearchMode::Hex => {
                        let hex_str: String =
                            query.chars().filter(|c| c.is_ascii_hexdigit()).collect();
                        hex_str.len() / 2
                    }
                };

                // Create highlights for all results
                // Note: HexView's highlight implementation adds hex_byte_width to the end,
                // so we need to subtract 1 from the end to get the correct range
                // Use warning color (yellow) to distinguish from cursor (accent/blue)
                let theme = cx.theme();

                // If preserving scroll, filter to viewport range
                let all_highlights: Vec<_> = if preserve_scroll {
                    let (viewport_start, viewport_end) =
                        self.hex_view.read(cx).viewport_byte_range();
                    self.search_results
                        .iter()
                        .filter(|&&pos| pos >= viewport_start && pos < viewport_end)
                        .map(|&pos| {
                            let end = pos + pattern_len;
                            (pos..end, theme.yellow.opacity(0.4))
                        })
                        .collect()
                } else {
                    self.search_results
                        .iter()
                        .map(|&pos| {
                            let end = pos + pattern_len;
                            (pos..end, theme.yellow.opacity(0.4))
                        })
                        .collect()
                };

                // Update highlights and scroll to current result
                self.hex_view.update(cx, |view, cx| {
                    view.set_highlights(all_highlights, cx);
                    // Only update scroll position and cursor if not preserving
                    if !preserve_scroll {
                        view.set_scroll_offset(offset / 16, cx);
                        view.set_cursor_offset(offset, cx);
                    }
                });
            }
        }
    }

    fn update_viewport_highlights(&mut self, cx: &mut Context<Self>) {
        let bar = self.search_bar.read(cx);
        let query = bar.query(cx);
        let mode = bar.mode();
        let pattern_len = match mode {
            SearchMode::Text => query.len(),
            SearchMode::Hex => {
                let hex_str: String = query.chars().filter(|c| c.is_ascii_hexdigit()).collect();
                hex_str.len() / 2
            }
        };

        // Filter search results to viewport range
        let (viewport_start, viewport_end) = self.hex_view.read(cx).viewport_byte_range();
        let theme = cx.theme();
        let viewport_highlights: Vec<_> = self
            .search_results
            .iter()
            .filter(|&&pos| pos >= viewport_start && pos < viewport_end)
            .map(|&pos| {
                let end = pos + pattern_len;
                (pos..end, theme.yellow.opacity(0.4))
            })
            .collect();

        self.hex_view.update(cx, |view, cx| {
            view.set_highlights(viewport_highlights, cx);
        });
    }

    fn search_next(&mut self, _: &SearchNext, _window: &mut Window, cx: &mut Context<Self>) {
        self.perform_search_next(cx);
    }

    fn perform_search_next(&mut self, cx: &mut Context<Self>) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(index) = self.current_result_index {
            self.current_result_index = Some((index + 1) % self.search_results.len());
        } else {
            self.current_result_index = Some(0);
        }

        self.highlight_current_result(false, cx);
        cx.notify();
    }

    fn search_prev(&mut self, _: &SearchPrev, _window: &mut Window, cx: &mut Context<Self>) {
        self.perform_search_prev(cx);
    }

    fn perform_search_prev(&mut self, cx: &mut Context<Self>) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(index) = self.current_result_index {
            self.current_result_index = Some(if index == 0 {
                self.search_results.len() - 1
            } else {
                index - 1
            });
        } else {
            self.current_result_index = Some(0);
        }

        self.highlight_current_result(false, cx);
        cx.notify();
    }

    fn focus_hex_view(&mut self, _: &FocusHexView, window: &mut Window, cx: &mut Context<Self>) {
        self.hex_view.read(cx).focus_handle(cx).focus(window);
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}

impl Focusable for EditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        "EditorPanel"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        let title = self
            .buffer
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "(untitled)".to_string());
        title.into_any_element()
    }

    fn closable(&self, _cx: &App) -> bool {
        true
    }

    fn zoomable(&self, _cx: &App) -> Option<gpui_component::dock::PanelControl> {
        Some(gpui_component::dock::PanelControl::Both)
    }

    fn visible(&self, _cx: &App) -> bool {
        true
    }

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut App) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut App) {}
}

impl Render for EditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check if search query has changed and perform search if needed
        if self.is_search_visible {
            let current_query = self.search_bar.read(cx).query(cx);
            if current_query != self.last_search_query {
                self.last_search_query = current_query.clone();
                let mode = self.search_bar.read(cx).mode();
                self.perform_incremental_search(&current_query, mode, cx);
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::toggle_search))
            .on_action(cx.listener(Self::search_next))
            .on_action(cx.listener(Self::search_prev))
            .on_action(cx.listener(Self::focus_hex_view))
            .when(self.is_search_visible, |this| {
                this.child(self.search_bar.clone())
            })
            .child(self.hex_view.clone())
    }
}
