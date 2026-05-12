use gpui::*;
use crate::core::editor::Editor;
use crate::core::structure::ParsedField;
use gpui_component::theme::Theme;

pub struct StructTreeView {
    pub fields: Vec<crate::core::structure::ParsedField>,
    pub flattened_fields: Vec<FlattenedField>,
    pub editor: Option<Entity<Editor>>,
    pub list_state: ListState,
    last_parse_id: Option<String>,
    _editor_subscription: Option<Subscription>,
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
    pub fn new(fields: Vec<crate::core::structure::ParsedField>, editor: Option<Entity<Editor>>, cx: &mut Context<Self>) -> Self {
        let mut flattened = Vec::new();
        Self::flatten_fields(&fields, 0, &mut flattened);
        let list_state = ListState::new(
            flattened.len(),
            ListAlignment::Top,
            px(24.0),
        );

        let mut this = Self { 
            fields, 
            flattened_fields: flattened,
            editor: editor.clone(),
            list_state,
            last_parse_id: None,
            _editor_subscription: None,
        };

        if let Some(ed) = editor {
            this._editor_subscription = Some(cx.observe(&ed, |this, editor, cx| {
                this.sync_fields(&editor, cx);
            }));
            this.sync_fields(&ed, cx);
        }

        this
    }

    pub fn set_editor(&mut self, editor: Option<Entity<Editor>>, cx: &mut Context<Self>) {
        self._editor_subscription = None;
        self.editor = editor.clone();
        self.last_parse_id = None;
        
        self.set_fields(Vec::new(), cx);
        
        if let Some(ed) = editor {
            self._editor_subscription = Some(cx.observe(&ed, |this, editor, cx| {
                this.sync_fields(&editor, cx);
            }));
            self.sync_fields(&ed, cx);
        }
        cx.notify();
    }

    fn sync_fields(&mut self, editor: &Entity<Editor>, cx: &mut Context<Self>) {
        let editor_lock = editor.read(cx);
        let current_parse_id = editor_lock.parse_result.as_ref().map(|r| {
            format!("{}-{}-{}", r.definition_id, r.total_parsed_bytes, r.fields.len())
        });
        
        if current_parse_id != self.last_parse_id {
            let fields = editor_lock.parse_result.as_ref()
                .map(|res| res.fields.clone())
                .unwrap_or_default();
            
            self.set_fields(fields, cx);
            self.last_parse_id = current_parse_id;
            cx.notify();
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
        
        let padding_left = 8.0 + field.depth as f32 * 16.0;
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
                    .flex_shrink_0()
                    .w(px(12.0))
                    .h(px(12.0))
                    .mr(px(8.0))
                    .bg(field.color)
                    .border_1()
                    .border_color(border_color)
            )
            .child(
                div()
                    .flex_shrink_0()
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
        
        let is_empty = self.fields.is_empty();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .child(if is_empty {
                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.foreground)
                    .child("No structure loaded")
                    .into_any_element()
            } else {
                list(self.list_state.clone(), {
                    let view = cx.entity().clone();
                    move |ix, _window, cx| {
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
                    }
                })
                .w_full()
                .h_full()
                .into_any_element()
            })
    }
}
