use gpui::*;
use gpui_component::theme::Theme;

use crate::core::editor::Editor;
use crate::ui::components::struct_tree_view::StructTreeView;
use crate::ui::panels::file_tree_panel::{FileTreePanel, FileTreeEvent};

#[derive(Clone, Copy, PartialEq)]
pub enum LeftPanelTab {
    Files,
    Structure,
}

pub struct StructTreePanel {
    pub editor: Option<Entity<Editor>>,
    pub tree_view: Entity<StructTreeView>,
    last_parse_id: Option<String>,
    _editor_subscription: Option<Subscription>,
}

impl StructTreePanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tree_view = cx.new(|cx| StructTreeView::new(Vec::new(), None, cx));
        Self {
            editor: None,
            tree_view,
            last_parse_id: None,
            _editor_subscription: None,
        }
    }

    pub fn set_editor(&mut self, editor: Option<Entity<Editor>>, cx: &mut Context<Self>) {
        self._editor_subscription = None;
        self.editor = editor.clone();
        
        if let Some(ed) = editor {
            self._editor_subscription = Some(cx.observe(&ed, |this, editor, cx| {
                this.sync_fields(&editor, cx);
            }));
            self.sync_fields(&ed, cx);
        } else {
            self.tree_view.update(cx, |view, cx| {
                view.set_fields(Vec::new(), cx);
                view.editor = None;
            });
            self.last_parse_id = None;
        }
        cx.notify();
    }

    fn sync_fields(&mut self, editor: &Entity<Editor>, cx: &mut Context<Self>) {
        let editor_lock = editor.read(cx);
        let current_parse_id = editor_lock.parse_result.as_ref().map(|r| {
            // Use a combination of definition ID, parsed bytes, and field count to ensure uniqueness
            format!("{}-{}-{}", r.definition_id, r.total_parsed_bytes, r.fields.len())
        });
        
        if current_parse_id != self.last_parse_id {
            let fields = editor_lock.parse_result.as_ref()
                .map(|res| res.fields.clone())
                .unwrap_or_default();
            
            self.tree_view.update(cx, |view, cx| {
                view.set_fields(fields, cx);
                view.editor = Some(editor.clone());
            });
            self.last_parse_id = current_parse_id;
            cx.notify();
        }
    }
}

impl Render for StructTreePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_empty = self.tree_view.read(cx).fields.is_empty();

        let theme = cx.global::<Theme>();
        let content = if is_empty {
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
            self.tree_view.clone().into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .child(content)
    }
}

pub struct LeftPanel {
    pub file_tree: Entity<FileTreePanel>,
    pub struct_tree: Entity<StructTreePanel>,
    pub active_tab: LeftPanelTab,
}

impl EventEmitter<FileTreeEvent> for LeftPanel {}

impl LeftPanel {
    pub fn new(file_tree: Entity<FileTreePanel>, cx: &mut Context<Self>) -> Self {
        let struct_tree = cx.new(|cx| StructTreePanel::new(cx));

        cx.subscribe(&file_tree, |_, _, event: &FileTreeEvent, cx| {
            match event {
                FileTreeEvent::OpenFile(path) => cx.emit(FileTreeEvent::OpenFile(path.clone())),
            }
        }).detach();

        Self {
            file_tree,
            struct_tree,
            active_tab: LeftPanelTab::Files,
        }
    }

    pub fn set_editor(&mut self, editor: Option<Entity<Editor>>, cx: &mut Context<Self>) {
        self.struct_tree.update(cx, |panel, cx| {
            panel.set_editor(editor, cx);
        });
    }

    pub fn set_tab(&mut self, tab: LeftPanelTab, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }
}

impl Render for LeftPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            // Tabs
            .child(
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .h(px(28.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(if self.active_tab == LeftPanelTab::Files { theme.foreground } else { theme.muted_foreground })
                            .bg(if self.active_tab == LeftPanelTab::Files { theme.background } else { theme.background })
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.set_tab(LeftPanelTab::Files, cx);
                            }))
                            .child("Files"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(if self.active_tab == LeftPanelTab::Structure { theme.foreground } else { theme.muted_foreground })
                            .bg(if self.active_tab == LeftPanelTab::Structure { theme.background } else { theme.background })
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.set_tab(LeftPanelTab::Structure, cx);
                            }))
                            .child("Structure"),
                    ),
            )
            // Content
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(if self.active_tab == LeftPanelTab::Files {
                        self.file_tree.clone().into_any_element()
                    } else {
                        self.struct_tree.clone().into_any_element()
                    }),
            )
    }
}
