use gpui::*;
use gpui_component::dock::{DockArea, Panel, PanelEvent};
use gpui_component::dock::DockPlacement;
use gpui::Bounds;

use crate::app_title_bar::AppTitleBar;
use crate::file_tree_panel::{FileTreePanel, OpenFile};
use crate::editor_panel::EditorPanel;
use gpui_component::input::InputState;
use std::path::PathBuf;
use std::sync::Arc;

impl Workspace {
    fn on_action_open_file(
        &mut self,
        action: &OpenFile,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let file_path = &action.path;
        let file_name = PathBuf::from(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let content = std::fs::read_to_string(file_path).unwrap_or_else(|_| "".to_string());

        let editor_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(content)
        });

        let editor_panel = cx.new(|cx| EditorPanel::new(file_name, editor_input_state, cx));

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(Arc::new(editor_panel), DockPlacement::Center, None, window, cx);
        });
    }
}

pub struct Workspace {
    pub dock_area: Entity<DockArea>,
    pub title_bar: Entity<AppTitleBar>,
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_col()
            .size_full()
            .child(self.title_bar.clone())
            .child(
                div()
                    .flex_row()
                    .size_full()
                    .child(self.dock_area.clone())
            .on_action(cx.listener(Self::on_action_open_file))
            )
    }
}
