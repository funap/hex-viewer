use gpui::prelude::*;
use gpui::{App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, KeyBinding, Subscription, Task, Window, div, px};
use gpui_component::dock::{Panel, PanelEvent};

use crate::actions::{FocusHexView, SearchNext, SearchPrev, ToggleSearch};
use crate::app_state::AppState;
use crate::appearance::Appearance;
use crate::model::search::SearchMode;
use crate::ui::component::hex_view::{self, HexView};
use crate::ui::component::search_bar::{SearchBar, SearchBarEvent};
use gpui_component::{ActiveTheme, Icon, IconName, h_flex};

const CONTEXT: &str = "EditorPanel";

pub(crate) fn init(cx: &mut App) {
    // Initialize HexView actions and keybindings
    hex_view::init(cx);
    cx.bind_keys([
        KeyBinding::new("ctrl-f", ToggleSearch, Some(CONTEXT)),
        KeyBinding::new("cmd-f", ToggleSearch, Some(CONTEXT)),
        KeyBinding::new("f3", SearchNext, Some(CONTEXT)),
        KeyBinding::new("ctrl-g", SearchNext, Some(CONTEXT)),
        KeyBinding::new("cmd-g", SearchNext, Some(CONTEXT)),
        KeyBinding::new("shift-f3", SearchPrev, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-g", SearchPrev, Some(CONTEXT)),
        KeyBinding::new("cmd-shift-g", SearchPrev, Some(CONTEXT)),
    ]);
}

use crate::model::editor::Editor;

pub struct EditorPanel {
    editor: Entity<Editor>,
    focus_handle: FocusHandle,
    hex_view: Entity<HexView>,
    is_search_visible: bool,
    search_bar: Entity<SearchBar>,
    search_task: Option<Task<()>>,
    viewport_search_task: Option<Task<()>>,
    _appearance_subscription: Subscription,
    _editor_subscription: Subscription,
}

impl EditorPanel {
    pub fn new(editor: Entity<Editor>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = cx.global::<Appearance>().clone();
        let hex_view = cx.new(|cx| {
            HexView::new(editor.clone(), cx)
                .font_family(appearance.font_family.clone())
                .font_size(appearance.font_size)
        });
        let search_bar = cx.new(|cx| SearchBar::new(window, cx));

        cx.subscribe(&search_bar, |this, _, event: &SearchBarEvent, cx| match event {
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
        })
        .detach();

        let hex_focus_handle = hex_view.read(cx).focus_handle(cx);
        cx.on_focus_in(&hex_focus_handle, window, |this: &mut Self, _window: &mut Window, cx: &mut Context<Self>| {
            this.update_global_status(cx);
        })
        .detach();

        // Subscribe to HexView scroll events to update highlights when scrolling
        cx.subscribe(&hex_view, |this, _, event: &crate::ui::component::hex_view::HexViewEvent, cx| {
            if let crate::ui::component::hex_view::HexViewEvent::Scrolled(_) = event {
                // Update highlights if there's an active search
                if this.is_search_visible {
                    let editor = this.editor.read(cx);
                    if !editor.search_state.is_full_search_complete {
                        this.perform_viewport_search(cx);
                    } else if !editor.search_state.results.is_empty() {
                        this.update_viewport_highlights(cx);
                    }
                }
            }
        })
        .detach();

        let _appearance_subscription = cx.observe_global::<Appearance>(|this, cx| {
            let appearance = cx.global::<Appearance>();
            let font_family = appearance.font_family.clone();
            let font_size = appearance.font_size;
            this.hex_view.update(cx, |this_hex_view, cx| {
                this_hex_view.set_font_family(font_family, cx);
                this_hex_view.set_font_size(font_size, cx);
            });
        });

        let _editor_subscription = cx.observe(&editor, |this, _, cx| {
            this.update_search_bar_results(cx);
        });

        // Observe search bar for incremental search
        cx.observe(&search_bar, |this, search_bar, cx| {
            if this.is_search_visible {
                let query = search_bar.read(cx).query(cx);
                let mode = search_bar.read(cx).mode();
                if query != this.editor.read(cx).search_state.query {
                    this.perform_incremental_search(&query, mode, cx);
                }
            }
        })
        .detach();

        Self {
            editor,
            focus_handle: cx.focus_handle(),
            hex_view,
            is_search_visible: false,
            search_bar,
            search_task: None,
            viewport_search_task: None,
            _appearance_subscription,
            _editor_subscription,
        }
    }

    fn update_global_status(&self, cx: &mut Context<Self>) {
        // Update active editor in AppState
        let editor_weak = self.editor.downgrade();

        cx.update_global::<AppState, _>(|state, _cx| {
            state.active_editor = Some(editor_weak);
        });
    }

    pub fn path(&self, cx: &App) -> std::path::PathBuf {
        self.editor.read(cx).buffer.path().to_path_buf()
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

    fn perform_incremental_search(&mut self, query: &str, mode: SearchMode, cx: &mut Context<Self>) {
        if query.is_empty() {
            self.editor.update(cx, |editor, cx| {
                editor.clear_search();
                cx.notify();
            });
            self.hex_view.update(cx, |view, cx| {
                view.set_highlights(Vec::new(), cx);
            });
            return;
        }

        self.editor.update(cx, |editor, cx| {
            editor.set_search_query(query.to_string());
            cx.notify();
        });

        // 1. Start viewport search for immediate feedback
        self.perform_viewport_search(cx);

        // 2. Start full search for complete results
        self.perform_full_search(query, mode, cx);
    }

    fn perform_viewport_search(&mut self, cx: &mut Context<Self>) {
        let (start, end) = self.hex_view.read(cx).viewport_byte_range(cx);
        let query = self.editor.read(cx).search_state.query.clone();
        if query.is_empty() {
            return;
        }

        let mode = self.search_bar.read(cx).mode();
        let app_state = AppState::global(cx);

        let (_, viewport_task) = app_state.editor_service.incremental_search(self.editor.clone(), query, mode, start..end, cx);
        self.viewport_search_task = Some(viewport_task);
    }

    fn perform_full_search(&mut self, query: &str, mode: SearchMode, cx: &mut Context<Self>) {
        let (start, end) = self.hex_view.read(cx).viewport_byte_range(cx);
        let app_state = AppState::global(cx);

        let (viewport_task, full_task) = app_state
            .editor_service
            .incremental_search(self.editor.clone(), query.to_string(), mode, start..end, cx);
        self.viewport_search_task = Some(viewport_task);
        self.search_task = Some(full_task);
    }

    fn update_search_bar_results(&mut self, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        let count = editor.search_state.results.len();
        let current = editor.search_state.current_result_index;
        self.search_bar.update(cx, |bar, cx| {
            bar.set_results(count, current, cx);
        });
    }

    fn highlight_current_result(&mut self, preserve_scroll: bool, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        if let Some(offset) = editor.current_search_result() {
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

            let theme = cx.theme();

            // If preserving scroll, filter to viewport range
            let all_highlights: Vec<_> = if preserve_scroll {
                let (viewport_start, viewport_end) = self.hex_view.read(cx).viewport_byte_range(cx);
                editor
                    .search_state
                    .results
                    .iter()
                    .filter(|&&pos| pos >= viewport_start && pos < viewport_end)
                    .map(|&pos| {
                        let end = pos + pattern_len;
                        (pos..end, theme.yellow.opacity(0.4))
                    })
                    .collect()
            } else {
                editor
                    .search_state
                    .results
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

    fn update_viewport_highlights(&mut self, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
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
        let (viewport_start, viewport_end) = self.hex_view.read(cx).viewport_byte_range(cx);
        let theme = cx.theme();
        let viewport_highlights: Vec<_> = editor
            .search_state
            .results
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
        self.editor.update(cx, |editor, _| {
            editor.next_search_result();
        });
        self.highlight_current_result(false, cx);
        cx.notify();
    }

    fn search_prev(&mut self, _: &SearchPrev, _window: &mut Window, cx: &mut Context<Self>) {
        self.perform_search_prev(cx);
    }

    fn perform_search_prev(&mut self, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.prev_search_result();
        });
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

    fn title(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .editor
            .read(cx)
            .buffer
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "(untitled)".to_string());

        let theme = cx.theme();

        h_flex().gap_2().items_center().child(title).child(
            div()
                .id("close-icon")
                .cursor_pointer()
                .rounded_md()
                .hover(|style| style.bg(theme.accent).text_color(theme.accent_foreground))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus_handle(cx).focus(window);
                    window.dispatch_action(Box::new(gpui_component::dock::ClosePanel), cx);
                }))
                .child(Icon::new(IconName::Close).size(px(14.0))),
        )
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

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn dump(&self, cx: &App) -> gpui_component::dock::PanelState {
        let mut state = gpui_component::dock::PanelState::new(self);
        let panel_state = EditorPanelState {
            path: Some(self.editor.read(cx).buffer.path().to_path_buf()),
        };
        state.info = gpui_component::dock::PanelInfo::panel(panel_state.to_value());
        state
    }
}

impl Render for EditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .when(self.is_search_visible, |el| el.child(self.search_bar.clone()))
            .child(self.hex_view.clone())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EditorPanelState {
    pub path: Option<std::path::PathBuf>,
}

impl EditorPanelState {
    #[allow(dead_code)]
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    #[allow(dead_code)]
    pub fn from_value(value: serde_json::Value) -> Option<Self> {
        serde_json::from_value(value).ok()
    }
}
