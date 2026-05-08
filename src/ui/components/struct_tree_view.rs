use gpui::*;

use crate::core::structure::ParsedField;
use gpui_component::theme::Theme;

pub struct StructTreeView {
    pub fields: Vec<ParsedField>,
}

impl StructTreeView {
    pub fn new(fields: Vec<ParsedField>) -> Self {
        Self { fields }
    }
}

impl Render for StructTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let mut list = div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            ;

        for field in &self.fields {
            list = list.child(render_field(field, 0, theme));
        }

        list
    }
}

fn render_field(field: &ParsedField, depth: usize, theme: &Theme) -> impl IntoElement {
    let padding_left = depth as f32 * 16.0;

    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .pl(px(padding_left))
        .py(px(2.0))
        .hover(|style| style.bg(theme.selection));

    // Color indicator
    row = row.child(
        div()
            .w(px(12.0))
            .h(px(12.0))
            .mr(px(8.0))
            .bg(field.color)
            .border_1()
            .border_color(theme.border)
    );

    // ID and Type
    row = row.child(
        div()
            .flex()
            .flex_row()
            .w(px(200.0))
            .child(
                div().text_color(theme.foreground).child(field.id.clone())
            )
            .child(
                div().ml(px(8.0)).text_color(theme.foreground).child(field.field_type.clone())
            )
    );

    // Value
    let val_str = if let Some(label) = &field.enum_label {
        format!("{} ({})", field.value, label)
    } else {
        format!("{}", field.value)
    };

    row = row.child(
        div().text_color(theme.foreground).child(val_str)
    );

    let mut container = div().flex().flex_col().w_full().child(row);

    // Children recursively
    if !field.children.is_empty() {
        for child in &field.children {
            container = container.child(render_field(child, depth + 1, theme));
        }
    }

    container
}
