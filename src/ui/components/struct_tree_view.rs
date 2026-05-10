use gpui::*;
use crate::core::editor::Editor;
use crate::core::structure::ParsedField;
use gpui_component::theme::Theme;

pub struct StructTreeView {
    pub fields: Vec<ParsedField>,
    pub editor: Option<Entity<Editor>>,
}

impl StructTreeView {
    pub fn new(fields: Vec<ParsedField>, editor: Option<Entity<Editor>>) -> Self {
        Self { fields, editor }
    }

    fn on_field_click(&self, offset: usize, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.set_cursor_offset(offset);
                cx.notify();
            });
        }
    }

    fn render_field(&self, field: &ParsedField, depth: usize, colors: (Hsla, Hsla, Hsla), cx: &mut Context<Self>) -> impl IntoElement {
        let padding_left = depth as f32 * 16.0;
        let offset = field.offset;
        let (selection_color, border_color, foreground_color) = colors;

        let mut row = div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .pl(px(padding_left))
            .py(px(2.0))
            .hover(|style| style.bg(selection_color))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.on_field_click(offset, cx);
            }));

        // Color indicator
        row = row.child(
            div()
                .w(px(12.0))
                .h(px(12.0))
                .mr(px(8.0))
                .bg(field.color)
                .border_1()
                .border_color(border_color),
        );

        // ID and Type
        row = row.child(
            div()
                .flex()
                .flex_row()
                .w(px(200.0))
                .child(div().text_color(foreground_color).child(field.id.clone()))
                .child(
                    div()
                        .ml(px(8.0))
                        .text_color(foreground_color)
                        .child(field.field_type.clone()),
                ),
        );

        // Value
        let val_str = if let Some(label) = &field.enum_label {
            format!("{} ({})", field.value, label)
        } else {
            format!("{}", field.value)
        };

        row = row.child(div().text_color(foreground_color).child(val_str));

        let mut container = div().flex().flex_col().w_full().child(row);

        // Children recursively
        if !field.children.is_empty() {
            for child in &field.children {
                container = container.child(self.render_field(child, depth + 1, colors, cx));
            }
        }

        container
    }
}

impl Render for StructTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let colors = (theme.selection, theme.border, theme.foreground);
        
        let mut list = div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.foreground);

        let fields = self.fields.clone();
        for field in &fields {
            list = list.child(self.render_field(field, 0, colors, cx));
        }

        list
    }
}
