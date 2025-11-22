use crate::data::file_buffer::FileBuffer;
use gpui::*;
use gpui_component::{
    VirtualListScrollHandle,
    dock::{Panel, PanelEvent},
    h_flex, v_flex, v_virtual_list,
};
use std::rc::Rc;
use std::sync::Arc;

pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    focus_handle: FocusHandle,
    scroll_handle: VirtualListScrollHandle,
}

impl EditorPanel {
    pub fn new(buffer: Arc<FileBuffer>, cx: &mut Context<Self>) -> Self {
        Self {
            buffer,
            focus_handle: cx.focus_handle(),
            scroll_handle: VirtualListScrollHandle::new(),
        }
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
        "Hex Editor".into_any_element()
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
        let line_count = (self.buffer.len() + 15) / 16;
        let buffer = self.buffer.clone();

        // Premium Dark Theme Colors
        let bg_color = rgb(0x1e1e1e);
        let offset_color = rgb(0x858585);
        let hex_byte_color = rgb(0x9cdcfe);
        let hex_null_color = rgb(0x505050);
        let ascii_printable_color = rgb(0xce9178);
        let ascii_non_printable_color = rgb(0x505050);

        v_flex()
            .flex()
            .flex_col()
            .bg(bg_color)
            .font_family("Menlo")
            .size_full()
            .child(
                // Header
                h_flex()
                    .w_full()
                    .h(px(32.))
                    .bg(bg_color)
                    .border_b_1()
                    .border_color(rgb(0x333333)) // Dark border
                    .child(
                        div()
                            .w_24()
                            .mr_4()
                            .flex()
                            .items_center()
                            .text_color(offset_color)
                            .font_weight(FontWeight::BOLD)
                            .child("Offset"),
                    )
                    .child(
                        div()
                            .w(px(400.))
                            .flex()
                            .items_center()
                            .gap_1()
                            .mr_4()
                            .children((0..16).map(|i| {
                                div()
                                    .w(px(22.))
                                    .flex()
                                    .justify_center()
                                    .text_color(hex_byte_color)
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("+{:X}", i))
                            })),
                    )
                    .child(
                        div()
                            .w_40()
                            .flex()
                            .items_center()
                            .text_color(ascii_printable_color)
                            .font_weight(FontWeight::BOLD)
                            .child("ASCII"),
                    ),
            )
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "hex_list",
                    Rc::new(vec![size(px(600.), px(24.)); line_count]),
                    move |_, visible_range, _window, _cx| {
                        let mut items = Vec::new();
                        for i in visible_range {
                            let offset = i * 16;
                            let chunk = buffer.get_range(offset, 16);

                            let offset_str = format!("{:08x}", offset);

                            // Build Hex View
                            let mut hex_elements = h_flex().gap_1();
                            for byte in chunk.iter() {
                                let color = if *byte == 0 {
                                    hex_null_color
                                } else {
                                    hex_byte_color
                                };
                                hex_elements = hex_elements.child(
                                    div()
                                        .w(px(22.))
                                        .flex()
                                        .justify_center()
                                        .text_color(color)
                                        .child(format!("{:02x}", byte)),
                                );
                            }

                            if chunk.len() < 16 {
                                for _ in 0..(16 - chunk.len()) {
                                    hex_elements = hex_elements.child(div().w(px(22.)));
                                }
                            }

                            // Build ASCII View
                            let mut ascii_elements = h_flex().gap_0();
                            for byte in chunk.iter() {
                                let (char_str, color) = if *byte >= 32 && *byte <= 126 {
                                    ((*byte as char).to_string(), ascii_printable_color)
                                } else {
                                    (".".to_string(), ascii_non_printable_color)
                                };
                                ascii_elements =
                                    ascii_elements.child(div().text_color(color).child(char_str));
                            }

                            items.push(
                                div()
                                    .h(px(24.))
                                    .flex()
                                    .items_center()
                                    .child(
                                        div()
                                            .w_24()
                                            .text_color(offset_color)
                                            .mr_4()
                                            .child(offset_str),
                                    )
                                    .child(div().w(px(400.)).mr_4().child(hex_elements))
                                    .child(div().w_40().child(ascii_elements))
                                    .into_any_element(),
                            );
                        }
                        items
                    },
                )
                .track_scroll(&self.scroll_handle)
                .flex_1(),
            )
    }
}
