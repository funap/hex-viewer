use crate::core::appearance::Appearance;
use crate::core::editor::Editor;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

pub enum StatusBarEvent {
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

        let (cursor_offset, total_size, _value_at_cursor, bytes_at_cursor) = if let Some(editor) = &active_editor {
            let editor = editor.read(cx);
            (editor.cursor_offset, editor.total_size(), editor.value_at_cursor(), editor.read_bytes_at_cursor(8))
        } else {
            (0, 0, None, Vec::new())
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

        let mut i8_val = "--".to_string();
        let mut u8_val = "--".to_string();
        let mut i16_val = "--".to_string();
        let mut u16_val = "--".to_string();
        let mut i32_val = "--".to_string();
        let mut u32_val = "--".to_string();
        let mut i64_val = "--".to_string();
        let mut u64_val = "--".to_string();
        let mut f32_val = "--".to_string();
        let mut f64_val = "--".to_string();
        let mut ascii_val = "--".to_string();
        let mut utf8_val = "--".to_string();
        let mut utf16_val = "--".to_string();

        if bytes_at_cursor.len() >= 1 {
            let b = bytes_at_cursor[0];
            i8_val = format!("{}", b as i8);
            u8_val = format!("{}", b);

            let ch = b as char;
            if ch.is_ascii_graphic() || ch == ' ' {
                ascii_val = format!("'{}'", ch);
            } else {
                ascii_val = ".".to_string();
            }
        }

        if bytes_at_cursor.len() >= 2 {
            let arr: [u8; 2] = bytes_at_cursor[0..2].try_into().unwrap();
            i16_val = format!("{}", i16::from_le_bytes(arr));
            u16_val = format!("{}", u16::from_le_bytes(arr));

            let ch = u16::from_le_bytes(arr);
            if let Some(c) = char::from_u32(ch as u32) {
                if !c.is_control() {
                    utf16_val = format!("'{}'", c);
                } else {
                    utf16_val = ".".to_string();
                }
            } else {
                utf16_val = ".".to_string();
            }
        }

        if bytes_at_cursor.len() >= 4 {
            let arr: [u8; 4] = bytes_at_cursor[0..4].try_into().unwrap();
            i32_val = format!("{}", i32::from_le_bytes(arr));
            u32_val = format!("{}", u32::from_le_bytes(arr));
            f32_val = format!("{:.4}", f32::from_le_bytes(arr));
        }

        if bytes_at_cursor.len() >= 8 {
            let arr: [u8; 8] = bytes_at_cursor[0..8].try_into().unwrap();
            i64_val = format!("{}", i64::from_le_bytes(arr));
            u64_val = format!("{}", u64::from_le_bytes(arr));
            f64_val = format!("{:.4}", f64::from_le_bytes(arr));
        }

        if !bytes_at_cursor.is_empty() {
            let mut decoded = false;
            for len in (1..=std::cmp::min(4, bytes_at_cursor.len())).rev() {
                if let Ok(s) = std::str::from_utf8(&bytes_at_cursor[0..len]) {
                    if let Some(c) = s.chars().next() {
                        if !c.is_control() {
                            utf8_val = format!("'{}'", c);
                        } else {
                            utf8_val = ".".to_string();
                        }
                        decoded = true;
                        break;
                    }
                }
            }
            if !decoded {
                utf8_val = ".".to_string();
            }
        }

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
                div().flex().items_center().gap_1().child(
                    div()
                        .id("toggle-sidebar")
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.accent).text_color(theme.accent_foreground))
                        .on_click(cx.listener(|_, _, _window, cx| {
                            cx.emit(StatusBarEvent::ToggleLeftPanel);
                        }))
                        .child(gpui_component::Icon::new(gpui_component::IconName::Folder).size(px(14.0))),
                ),
            )
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
                    .gap_4()
                    .text_color(theme.muted_foreground)
                    .when(!bytes_at_cursor.is_empty(), |el| {
                        el.child(
                            div().flex().gap_2()
                                .child(div().child(format!("i8: {}", i8_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("u8: {}", u8_val)))
                        )
                        .child(
                            div().flex().gap_2()
                                .child(div().child(format!("i16: {}", i16_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("u16: {}", u16_val)))
                        )
                        .child(
                            div().flex().gap_2()
                                .child(div().child(format!("i32: {}", i32_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("u32: {}", u32_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("f32: {}", f32_val)))
                        )
                        .child(
                            div().flex().gap_2()
                                .child(div().child(format!("i64: {}", i64_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("u64: {}", u64_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("f64: {}", f64_val)))
                        )
                        .child(
                            div().flex().gap_2()
                                .child(div().child(format!("ASCII: {}", ascii_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("UTF-8: {}", utf8_val)))
                                .child(div().text_color(theme.border).child("|"))
                                .child(div().child(format!("UTF-16: {}", utf16_val)))
                        )
                    }),
            )
    }
}
