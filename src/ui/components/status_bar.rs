use crate::core::appearance::Appearance;
use crate::core::editor::Editor;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

pub enum StatusBarEvent {
    ToggleFileTree,
}

pub struct StatusBar {
    active_editor: Option<WeakEntity<Editor>>,
}

impl EventEmitter<StatusBarEvent> for StatusBar {}

impl StatusBar {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self { active_editor: None }
    }

    pub fn set_active_editor(&mut self, editor: Option<Entity<Editor>>) {
        self.active_editor = editor.map(|e| e.downgrade());
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active_editor = self.active_editor.as_ref().and_then(|e| e.upgrade());

        let (cursor_offset, total_size, value_at_cursor) = if let Some(editor) = active_editor {
            let editor = editor.read(cx);
            (editor.cursor_offset, editor.total_size(), editor.value_at_cursor())
        } else {
            (0, 0, None)
        };

        let cursor_val = if let Some(byte) = value_at_cursor {
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
            .font_family(cx.global::<Appearance>().font_family.clone())
            .gap_2()
            .child(
                div().flex().items_center().gap_1().child(
                    div()
                        .id("toggle-sidebar")
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.accent).text_color(theme.accent_foreground))
                        .on_click(cx.listener(|_, _, _window, cx| {
                            cx.emit(StatusBarEvent::ToggleFileTree);
                        }))
                        .child(gpui_component::Icon::new(gpui_component::IconName::Folder).size(px(14.0))),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(div().w(px(240.)).child(format!("Offset: 0x{:08X} ({})", cursor_offset, cursor_offset)))
                    .child(div().w(px(220.)).child(format!("Value: {}", cursor_val)))
                    .child(div().w(px(200.)).child(format!("Size: {} bytes", total_size))),
            )
    }
}
