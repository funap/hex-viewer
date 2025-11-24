use crate::data::file_buffer::FileBuffer;
use gpui::prelude::*;
use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use std::sync::Arc;

use crate::actions::{SearchNext, SearchPrev, ToggleSearch};
use crate::ui::component::hex_view::{self, HexView};
use gpui_component::ActiveTheme;
use gpui_component::{
    Icon, IconName,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};

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

#[derive(Clone, Copy, PartialEq)]
enum SearchMode {
    Text,
    Hex,
}

pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    focus_handle: FocusHandle,
    hex_view: Entity<HexView>,
    is_search_visible: bool,
    search_input: Entity<InputState>,
    search_mode: SearchMode,
    search_results: Vec<usize>,
    current_result_index: Option<usize>,
    last_search_query: String,
}

impl EditorPanel {
    pub fn new(buffer: Arc<FileBuffer>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hex_view = cx.new(|cx| HexView::new(cx).buffer(buffer.clone()));
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("検索...")
                .clean_on_escape()
        });
        Self {
            buffer,
            focus_handle: cx.focus_handle(),
            hex_view,
            is_search_visible: false,
            search_input,
            search_mode: SearchMode::Hex,
            search_results: Vec::new(),
            current_result_index: None,
            last_search_query: String::new(),
        }
    }

    fn render_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let result_info = if !self.search_results.is_empty() {
            if let Some(index) = self.current_result_index {
                format!("{}/{}", index + 1, self.search_results.len())
            } else {
                format!("{} results", self.search_results.len())
            }
        } else {
            String::new()
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .p_2()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(cx.theme().border)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "enter" {
                    this.search_next(&SearchNext, window, cx);
                }
            }))
            .child(
                div()
                    .flex()
                    .child(
                        if self.search_mode == SearchMode::Hex {
                            Button::new("hex_mode").label("Hex").primary()
                        } else {
                            Button::new("hex_mode").label("Hex").ghost()
                        }
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.search_mode = SearchMode::Hex;
                            this.perform_search(cx);
                            this.search_input.update(cx, |input, cx| {
                                input.focus(window, cx);
                            });
                        })),
                    )
                    .child(
                        if self.search_mode == SearchMode::Text {
                            Button::new("text_mode").label("Text").primary()
                        } else {
                            Button::new("text_mode").label("Text").ghost()
                        }
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.search_mode = SearchMode::Text;
                            this.perform_search(cx);
                            this.search_input.update(cx, |input, cx| {
                                input.focus(window, cx);
                            });
                        })),
                    ),
            )
            .child(
                div().flex_1().child(
                    Input::new(&self.search_input)
                        .prefix(Icon::new(IconName::Search).size_3p5())
                        .cleanable(true),
                ),
            )
            .when(!result_info.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(result_info.clone()),
                )
            })
            .child(
                Button::new("prev")
                    .ghost()
                    .icon(IconName::ChevronUp)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.search_prev(&SearchPrev, window, cx);
                    })),
            )
            .child(
                Button::new("next")
                    .ghost()
                    .icon(IconName::ChevronDown)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.search_next(&SearchNext, window, cx);
                    })),
            )
            .child(
                Button::new("close")
                    .ghost()
                    .icon(IconName::Close)
                    .on_click(cx.listener(Self::toggle_search_click)),
            )
    }

    fn toggle_search(&mut self, _: &ToggleSearch, window: &mut Window, cx: &mut Context<Self>) {
        self.is_search_visible = !self.is_search_visible;
        if self.is_search_visible {
            // Focus the search input
            self.search_input.update(cx, |input, cx| {
                input.focus(window, cx);
            });
        } else {
            self.hex_view.read(cx).focus_handle(cx).focus(window);
        }
        cx.notify();
    }

    fn toggle_search_click(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_search_visible = !self.is_search_visible;
        if !self.is_search_visible {
            self.hex_view.read(cx).focus_handle(cx).focus(window);
        }
        cx.notify();
    }

    fn perform_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value();

        if query.is_empty() {
            self.search_results.clear();
            self.current_result_index = None;
            self.hex_view.update(cx, |view, cx| {
                view.set_highlights(Vec::new(), cx);
            });
            return;
        }

        self.search_results = match self.search_mode {
            SearchMode::Text => self.buffer.search_text(&query),
            SearchMode::Hex => {
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
                        Ok(pattern) => self.buffer.search_bytes(&pattern),
                        Err(_) => Vec::new(),
                    }
                }
            }
        };

        if !self.search_results.is_empty() {
            self.current_result_index = Some(0);
            self.highlight_current_result(cx);
        } else {
            self.current_result_index = None;
            self.hex_view.update(cx, |view, cx| {
                view.set_highlights(Vec::new(), cx);
            });
        }
    }

    fn highlight_current_result(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.current_result_index {
            if let Some(&offset) = self.search_results.get(index) {
                let query = self.search_input.read(cx).value();
                let pattern_len = match self.search_mode {
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
                let all_highlights: Vec<_> = self
                    .search_results
                    .iter()
                    .map(|&pos| {
                        let end = pos + pattern_len;
                        (pos..end, theme.yellow.opacity(0.4))
                    })
                    .collect();

                // Update highlights and scroll to current result
                self.hex_view.update(cx, |view, cx| {
                    view.set_highlights(all_highlights, cx);
                    view.set_scroll_offset(offset / 16, cx);
                    view.set_cursor_offset(offset, cx);
                });
            }
        }
    }

    fn search_next(&mut self, _: &SearchNext, _window: &mut Window, cx: &mut Context<Self>) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(index) = self.current_result_index {
            self.current_result_index = Some((index + 1) % self.search_results.len());
        } else {
            self.current_result_index = Some(0);
        }

        self.highlight_current_result(cx);
        cx.notify();
    }

    fn search_prev(&mut self, _: &SearchPrev, _window: &mut Window, cx: &mut Context<Self>) {
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

        self.highlight_current_result(cx);
        cx.notify();
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
            .path
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
            let current_query = self.search_input.read(cx).value().to_string();
            if current_query != self.last_search_query {
                self.last_search_query = current_query.clone();
                self.perform_search(cx);
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
            .when(self.is_search_visible, |this| {
                this.child(self.render_search_bar(cx))
            })
            .child(self.hex_view.clone())
    }
}
