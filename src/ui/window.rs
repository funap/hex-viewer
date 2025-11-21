use gpui::prelude::*;
use gpui::*;

use crate::app::{AddEditorPanel, OpenFile};

use crate::ui::editor_panel::EditorPanel;

use crate::ui::toolbar::AppTitleBar;

use gpui_component::dock::{DockArea, DockPlacement};

use std::sync::Arc;

pub struct WindowRoot {
    pub dock_area: Entity<DockArea>,

    pub title_bar: Entity<AppTitleBar>,
}

impl WindowRoot {
    fn on_action_add_editor_panel(
        &mut self,

        action: &AddEditorPanel,

        window: &mut Window,

        cx: &mut Context<Self>,
    ) {
        let buffer = action.0.clone();

        let editor_panel = cx.new(|cx| EditorPanel::new(buffer, cx));

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(
                Arc::new(editor_panel),
                DockPlacement::Center,
                None,
                window,
                cx,
            );
        });
    }

    fn on_action_open_file(
        &mut self,
        action: &OpenFile,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let file_path = action.path.clone();

        cx.spawn(async move |this, cx| {
            let app = cx
                .update(|cx| cx.global::<crate::app::App>().clone())
                .ok()
                .unwrap();
            if let Some(add_editor_panel) = this.upgrade() {
                if let Ok(buffer) = app
                    .editor_service
                    .open_file(std::path::PathBuf::from(file_path))
                    .await
                {
                    let _ = add_editor_panel.update(cx, |_, cx| {
                        cx.dispatch_action(&AddEditorPanel(buffer));
                    });
                }
            }
        })
        .detach();
    }
}

impl Render for WindowRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_col()
            .size_full()
            .child(self.title_bar.clone())
            .child(div().flex_row().size_full().child(self.dock_area.clone()))
            .on_action(cx.listener(Self::on_action_open_file))
            .on_action(cx.listener(Self::on_action_add_editor_panel))
    }
}
