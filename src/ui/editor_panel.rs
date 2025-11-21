use crate::data::file_buffer::FileBuffer;
use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use std::sync::Arc;

pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    scroll_offset: usize,
    focus_handle: FocusHandle,
}

impl EditorPanel {
    pub fn new(buffer: Arc<FileBuffer>, cx: &mut Context<Self>) -> Self {
        Self {
            buffer,
            scroll_offset: 0,
            focus_handle: cx.focus_handle(),
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let data = self.buffer.get_range(self.scroll_offset, 16 * 20); // Show 20 lines

        let mut lines = Vec::new();
        for (i, chunk) in data.chunks(16).enumerate() {
            let offset = self.scroll_offset + i * 16;

            let offset_str = format!("{:08x}", offset);

            let mut hex_str = String::new();
            let mut ascii_str = String::new();

            for byte in chunk {
                hex_str.push_str(&format!("{:02x} ", byte));
                if *byte >= 32 && *byte <= 126 {
                    ascii_str.push(*byte as char);
                } else {
                    ascii_str.push('.');
                }
            }

            // Padding for last line
            if chunk.len() < 16 {
                for _ in 0..(16 - chunk.len()) {
                    hex_str.push_str("   ");
                }
            }

            lines.push(
                div()
                    .flex()
                    .child(div().w_24().text_color(gpui::red()).child(offset_str))
                    .child(div().w_96().text_color(gpui::blue()).child(hex_str))
                    .child(div().w_40().text_color(gpui::green()).child(ascii_str)),
            );
        }

        div()
            .flex()
            .flex_col()
            .bg(gpui::white())
            .text_color(gpui::black())
            .font_family("Menlo") // Monospace font
            .children(lines)
    }
}
