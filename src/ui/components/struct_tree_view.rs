use crate::core::editor::Editor;
use crate::core::structure::ParsedField;
use gpui::*;
use gpui_component::{ActiveTheme as _, h_flex, list::ListItem, v_flex};

pub struct StructTreeView {
    pub fields: Vec<crate::core::structure::ParsedField>,
    pub flattened_fields: Vec<FlattenedField>,
    pub editor: Option<Entity<Editor>>,
    pub list_state: ListState,
    pub focus_handle: FocusHandle,
    last_parse_id: Option<String>,
    _editor_subscription: Option<Subscription>,
}

#[derive(Clone)]
pub struct FlattenedField {
    pub id: String,
    pub _field_type: String,
    pub offset: usize,
    pub value_str: String,
    pub color: Hsla,
    pub depth: usize,
}

impl StructTreeView {
    pub fn new(fields: Vec<crate::core::structure::ParsedField>, editor: Option<Entity<Editor>>, cx: &mut Context<Self>) -> Self {
        let mut flattened = Vec::new();
        Self::flatten_fields(&fields, 0, &mut flattened);
        let list_state = ListState::new(flattened.len(), ListAlignment::Top, px(24.0));
        let focus_handle = cx.focus_handle();

        let mut this = Self {
            fields,
            flattened_fields: flattened,
            editor: editor.clone(),
            list_state,
            focus_handle,
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
        let current_parse_id = editor_lock
            .parse_result
            .as_ref()
            .map(|r| format!("{}-{}-{}", r.definition_id, r.total_parsed_bytes, r.fields.len()));

        if current_parse_id != self.last_parse_id {
            let fields = editor_lock.parse_result.as_ref().map(|res| res.fields.clone()).unwrap_or_default();

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
                _field_type: field.field_type.clone(),
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

    fn render_list_item(ix: usize, field: &FlattenedField, editor: Option<Entity<Editor>>, focus_handle: FocusHandle, cx: &mut App) -> AnyElement {
        let padding_left = px(16.0 * field.depth as f32 + 12.0);
        let offset = field.offset;

        ListItem::new(ix)
            .w_full()
            .rounded(cx.theme().radius)
            .px_3()
            .pl(padding_left)
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(px(12.0))
                            .h(px(12.0))
                            .bg(field.color)
                            .border_1()
                            .border_color(cx.theme().border),
                    )
                    .child(div().text_color(cx.theme().foreground).child(field.id.clone()))
                    .child(div().ml_auto().text_color(cx.theme().muted_foreground).child(field.value_str.clone())),
            )
            .on_click(move |_, window, cx| {
                focus_handle.focus(window);
                this_on_field_click(offset, cx, editor.clone());
            })
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let is_empty = self.fields.is_empty();
        let is_focused = self.focus_handle.is_focused(window);
        let theme = cx.theme();

        let container = crate::ui::style::apply_focus_indicator(v_flex(), is_focused, theme)
            .id("struct-tree-view")
            .track_focus(&self.focus_handle)
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, _| {
                this.focus_handle.focus(window);
            }))
            .size_full()
            .flex_shrink_0()
            .bg(theme.sidebar)
            .border_r(px(1.0))
            .border_color(theme.border);

        container
            .child(
                div()
                    .p_2()
                    .text_sm()
                    .text_color(crate::ui::style::header_text_color(is_focused, theme))
                    .child("STRUCTURE"),
            )
            .child(if is_empty {
                v_flex()
                    .size_full()
                    .justify_center()
                    .items_center()
                    .child(div().text_color(theme.muted_foreground).child("No structure loaded"))
                    .into_any_element()
            } else {
                list(self.list_state.clone(), move |ix, _window, cx| {
                    let (item, focus_handle) = {
                        let this = view.read(cx);
                        let item = if ix < this.flattened_fields.len() {
                            Some(this.flattened_fields[ix].clone())
                        } else {
                            None
                        };
                        (item, this.focus_handle.clone())
                    };
                    let editor = view.read(cx).editor.clone();

                    if let Some(field) = item {
                        Self::render_list_item(ix, &field, editor, focus_handle, cx)
                    } else {
                        div().into_any_element()
                    }
                })
                .size_full()
                .into_any_element()
            })
    }
}

impl Focusable for StructTreeView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
