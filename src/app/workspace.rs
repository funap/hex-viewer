use gpui::*;
use gpui_component::dock::DockArea;
use gpui_component::dock::DockPlacement;

use crate::ui::components::app_title_bar::AppTitleBar;
use crate::app::actions::OpenFile;
use crate::ui::components::editor_panel::EditorPanel;
use gpui_component::input::InputState;
use std::path::PathBuf;
use std::sync::Arc;

impl Workspace {
    fn on_action_open_file(
        &mut self,
        action: &OpenFile,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> anyhow::Result<()> {
        let file_path = &action.path;
        let file_name = PathBuf::from(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let content = std::fs::read_to_string(file_path)?;

        let editor_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .searchable(true)
                .default_value(content)
        });

        let editor_panel = cx.new(|cx| EditorPanel::new(file_name, editor_input_state, cx));

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(Arc::new(editor_panel), DockPlacement::Center, None, window, cx);
        });
        Ok(())
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
            .on_action(cx.listener(|this, action, window, cx| {
                if let Err(error) = this.on_action_open_file(action, window, cx) {
                    eprintln!("Error opening file: {:?}", error);
                }
            }))
            )
    }
}
