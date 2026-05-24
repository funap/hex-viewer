use gpui::*;

use crate::core::editor::Editor;
use crate::ui::components::file_tree_view::{FileTreeView, FileTreeViewEvent};
use crate::ui::components::struct_tree_view::StructTreeView;

#[derive(Clone, Copy, PartialEq)]
pub enum LeftPanelTab {
    Files,
    Structure,
}

pub struct LeftPanel {
    pub file_tree: Entity<FileTreeView>,
    pub struct_tree: Entity<StructTreeView>,
    pub active_tab: LeftPanelTab,
}

impl EventEmitter<FileTreeViewEvent> for LeftPanel {}

impl LeftPanel {
    pub fn new(file_tree: Entity<FileTreeView>, cx: &mut Context<Self>) -> Self {
        let struct_tree = cx.new(|cx| StructTreeView::new(Vec::new(), None, cx));

        cx.subscribe(&file_tree, |_, _, event: &FileTreeViewEvent, cx| match event {
            FileTreeViewEvent::OpenFile(path) => cx.emit(FileTreeViewEvent::OpenFile(path.clone())),
        })
        .detach();

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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().w_full().h_full().child(if self.active_tab == LeftPanelTab::Files {
            self.file_tree.clone().into_any_element()
        } else {
            self.struct_tree.clone().into_any_element()
        })
    }
}

impl Focusable for LeftPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self.active_tab {
            LeftPanelTab::Files => self.file_tree.read(cx).focus_handle(cx),
            LeftPanelTab::Structure => self.struct_tree.read(cx).focus_handle(cx),
        }
    }
}
