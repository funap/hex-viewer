use gpui::*;
use crate::core::editor::Editor;
use crate::core::structure::ParsedField;
use gpui_component::theme::Theme;

pub struct StructTreeView {
    pub fields: Vec<ParsedField>,
    pub flattened_fields: Vec<FlattenedField>,
    pub editor: Option<Entity<Editor>>,
    pub list_state: ListState,
}

#[derive(Clone)]
pub struct FlattenedField {
    pub id: String,
    pub field_type: String,
    pub offset: usize,
    pub value_str: String,
    pub color: Hsla,
    pub depth: usize,
}

impl StructTreeView {
    pub fn new(fields: Vec<ParsedField>, editor: Option<Entity<Editor>>, _cx: &mut Context<Self>) -> Self {
        let mut flattened = Vec::new();
        Self::flatten_fields(&fields, 0, &mut flattened);
        let list_state = ListState::new(
            flattened.len(),
            ListAlignment::Top,
            px(100.0),
        );

        Self { 
            fields, 
            flattened_fields: flattened,
            editor,
            list_state,
        }
    }

    pub fn set_fields(&mut self, fields: Vec<ParsedField>, cx: &mut Context<Self>) {
        let mut flattened = Vec::new();
        Self::flatten_fields(&fields, 0, &mut flattened);
        self.fields = fields;
        self.flattened_fields = flattened;
        self.list_state.reset(self.flattened_fields.len());
        cx.notify();
    }

    fn flatten_fields(fields: &[ParsedField], depth: usize, results: &mut Vec<FlattenedField>) {
        for field in fields {
            let val_str = if let Some(label) = &field.enum_label {
                format!("{} ({})", field.value, label)
            } else {
                format!("{}", field.value)
            };

            results.push(FlattenedField {
                id: field.id.clone(),
                field_type: field.field_type.clone(),
                offset: field.offset,
                value_str: val_str,
                color: field.color,
                depth,
            });

            if !field.children.is_empty() {
                Self::flatten_fields(&field.children, depth + 1, results);
            }
        }
    }

    fn on_field_click(&self, offset: usize, cx: &mut App) {
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.set_cursor_offset(offset);
                cx.notify();
            });
        }
    }

    fn render_list_item(field: &FlattenedField, editor: Option<Entity<Editor>>, theme: &Theme) -> AnyElement {
        let selection_color = theme.selection;
        let border_color = theme.border;
        let foreground_color = theme.foreground;
        
        let padding_left = field.depth as f32 * 16.0;
        let offset = field.offset;

        div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .pl(px(padding_left))
            .py(px(2.0))
            .hover(|style| style.bg(selection_color))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                this_on_field_click(offset, cx, editor.clone());
            })
            .child(
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .mr(px(8.0))
                    .bg(field.color)
                    .border_1()
                    .border_color(border_color)
            )
            .child(
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
                    )
            )
            .child(div().text_color(foreground_color).child(field.value_str.clone()))
            .into_any_element()
    }
}

fn this_on_field_click(offset: usize, cx: &mut App, editor: Option<Entity<Editor>>) {
    if let Some(editor) = editor {
        editor.update(cx, |editor, cx| {
            editor.set_cursor_offset(offset);
            cx.notify();
        });
    }
}

impl Render for StructTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let view = cx.entity().clone();
        
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .child(
                list(self.list_state.clone(), move |ix, _window, cx| {
                    let item = {
                        let this = view.read(cx);
                        if ix < this.flattened_fields.len() {
                            Some((this.flattened_fields[ix].clone(), this.editor.clone()))
                        } else {
                            None
                        }
                    };
                    
                    if let Some((field, editor)) = item {
                        Self::render_list_item(&field, editor, &theme)
                    } else {
                        div().into_any_element()
                    }
                })
                .w_full()
                .h_full()
            )
    }
}
