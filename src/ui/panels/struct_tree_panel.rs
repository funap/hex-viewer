use gpui::*;

use crate::core::editor::Editor;
use crate::ui::components::struct_tree_view::StructTreeView;
use gpui_component::theme::Theme;

pub struct StructTreePanel {
    pub editor: Option<Entity<Editor>>,
    pub tree_view: Entity<StructTreeView>,
}

impl StructTreePanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tree_view = cx.new(|_| StructTreeView::new(Vec::new()));
        Self {
            editor: None,
            tree_view,
        }
    }

    pub fn set_editor(&mut self, editor: Entity<Editor>, cx: &mut Context<Self>) {
        self.editor = Some(editor.clone());
        // No event emitter for editor, just use notify
        cx.notify();
        cx.notify();
    }
}

impl Render for StructTreePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = if let Some(editor) = &self.editor {
            let editor_lock = editor.read(cx);
            if let Some(res) = &editor_lock.parse_result {
                res.fields.clone()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Update tree view fields
        self.tree_view.update(cx, |view, cx| {
            view.fields = fields;
            cx.notify();
        });

        let theme = cx.global::<Theme>();
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .child(
                div()
                    .w_full()
                    .py(px(4.0))
                    .px(px(8.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .text_color(theme.foreground)
                    .child("Structure Definition")
            )
            .child(self.tree_view.clone())
    }
}
