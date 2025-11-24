use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName,
    button::{Button, ButtonVariants},
    input::{self, Input, InputState},
};

#[derive(Clone, Copy, PartialEq)]
pub enum SearchMode {
    Text,
    Hex,
}

pub enum SearchBarEvent {
    Search(String, SearchMode),
    Next,
    Prev,
    Dismiss,
}

pub struct SearchBar {
    input: Entity<InputState>,
    mode: SearchMode,
    result_count: usize,
    current_index: Option<usize>,
}

impl EventEmitter<SearchBarEvent> for SearchBar {}

impl SearchBar {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Find...")
                .clean_on_escape()
        });

        // Subscribe to input changes
        cx.subscribe(&input, |this, input, event: &input::InputEvent, cx| {
            if let input::InputEvent::Change = event {
                let query = input.read(cx).value().to_string();
                cx.emit(SearchBarEvent::Search(query, this.mode));
            }
        })
        .detach();

        Self {
            input,
            mode: SearchMode::Hex,
            result_count: 0,
            current_index: None,
        }
    }

    pub fn set_results(&mut self, count: usize, current: Option<usize>, cx: &mut Context<Self>) {
        self.result_count = count;
        self.current_index = current;
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
    }

    pub fn query(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    fn on_mode_change(&mut self, mode: SearchMode, cx: &mut Context<Self>) {
        self.mode = mode;
        let query = self.input.read(cx).value().to_string();
        cx.emit(SearchBarEvent::Search(query, self.mode));
        cx.notify();
    }
}

impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let result_info = if self.result_count > 0 {
            if let Some(index) = self.current_index {
                format!("{}/{}", index + 1, self.result_count)
            } else {
                format!("{} results", self.result_count)
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
            .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, _window, cx| {
                if event.keystroke.key == "enter" {
                    if event.keystroke.modifiers.shift {
                        cx.emit(SearchBarEvent::Prev);
                    } else {
                        cx.emit(SearchBarEvent::Next);
                    }
                }
            }))
            .child(
                div()
                    .flex()
                    .child(
                        if self.mode == SearchMode::Hex {
                            Button::new("hex_mode").label("Hex").primary()
                        } else {
                            Button::new("hex_mode").label("Hex").ghost()
                        }
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.on_mode_change(SearchMode::Hex, cx);
                        })),
                    )
                    .child(
                        if self.mode == SearchMode::Text {
                            Button::new("text_mode").label("Text").primary()
                        } else {
                            Button::new("text_mode").label("Text").ghost()
                        }
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.on_mode_change(SearchMode::Text, cx);
                        })),
                    ),
            )
            .child(
                div().flex_1().child(
                    Input::new(&self.input)
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
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.emit(SearchBarEvent::Prev);
                    })),
            )
            .child(
                Button::new("next")
                    .ghost()
                    .icon(IconName::ChevronDown)
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.emit(SearchBarEvent::Next);
                    })),
            )
            .child(
                Button::new("close")
                    .ghost()
                    .icon(IconName::Close)
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.emit(SearchBarEvent::Dismiss);
                    })),
            )
    }
}
