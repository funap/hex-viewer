use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

#[derive(Clone, Debug, Default)]
pub struct EditorStatus {
    pub cursor_offset: usize,
    pub total_size: usize,
    pub value_at_cursor: Option<u8>,
    pub selection_count: Option<usize>,
}

pub struct StatusBar {
    status: Entity<EditorStatus>,
}

impl StatusBar {
    pub fn new(status: Entity<EditorStatus>, cx: &mut Context<Self>) -> Self {
        cx.observe(&status, |_, _, cx| {
            cx.notify();
        })
        .detach();

        Self { status }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.status.read(cx);
        let theme = cx.theme();

        let cursor_val = if let Some(byte) = status.value_at_cursor {
            let ch = byte as char;
            let char_display = if ch.is_ascii_graphic() || ch == ' ' {
                format!("'{}'", ch)
            } else {
                ".".to_string()
            };
            format!("0x{:02X} ({}) {}", byte, byte, char_display)
        } else {
            "--".to_string()
        };

        div()
            .h_8()
            .flex()
            .items_center()
            .px_4()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .text_sm()
            .font_family("Menlo")
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .w(px(240.))
                            .child(format!("Offset: 0x{:08X} ({})", status.cursor_offset, status.cursor_offset)),
                    )
                    .child(div().w(px(220.)).child(format!("Value: {}", cursor_val)))
                    .child(
                        div()
                            .w(px(150.))
                            .when_some(status.selection_count, |this, count| this.child(format!("Sel: {} bytes", count))),
                    )
                    .child(div().w(px(200.)).child(format!("Size: {} bytes", status.total_size))),
            )
    }
}
