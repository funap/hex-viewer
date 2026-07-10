use crate::core::appearance::Appearance;
use crate::core::editor::Editor;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

pub enum StatusBarEvent {
    #[allow(dead_code)]
    ToggleLeftPanel,
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

        let (cursor_offset, total_size) = if let Some(editor) = &active_editor {
            let editor = editor.read(cx);
            (
                editor.cursor_offset,
                editor.total_size(),
            )
        } else {
            (0, 0)
        };

        let has_custom_layout = if let Some(editor) = &active_editor {
            let editor = editor.read(cx);
            editor.has_custom_layout()
        } else {
            false
        };

        let custom_layout_count = if let Some(editor) = &active_editor {
            let editor = editor.read(cx);
            editor.custom_layout_count()
        } else {
            0
        };

        let encoding_name = if let Some(editor) = &active_editor {
            let editor = editor.read(cx);
            format!("{:?}", editor.encoding)
        } else {
            "--".to_string()
        };

        div()
            .flex()
            .items_center()
            .h_8()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .font_family(cx.global::<Appearance>().font_family.clone())
            .px_4()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .text_sm()
                    .child(format!("Offset: 0x{:08X} ({})", cursor_offset, cursor_offset))
                    .child(format!("Size: {} bytes", total_size))
                    .when(has_custom_layout, |el| {
                        el.child(
                            div()
                                .px_2()
                                .rounded_md()
                                .bg(theme.yellow.opacity(0.2))
                                .text_color(theme.yellow)
                                .child(format!("Layout: {} breaks", custom_layout_count)),
                        )
                    }),
            )
            .child(div().w_px().h_4().bg(theme.border))
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format!("Encoding: {}", encoding_name))
            )
    }
}
