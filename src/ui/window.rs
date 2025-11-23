use gpui::prelude::*;
use gpui::*;

use crate::actions::*;

use crate::ui::editor_panel::EditorPanel;

use crate::ui::toolbar::AppTitleBar;

use crate::app_state::AppState;
use gpui_component::dock::{DockArea, DockAreaState, DockPlacement};
use std::sync::Arc;

pub struct WindowRoot {
    pub dock_area: Entity<DockArea>,

    pub title_bar: Entity<AppTitleBar>,

    pub last_layout_state: Option<DockAreaState>,

    pub _save_layout_task: Option<Task<()>>,
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
            let app = cx.update(|cx| AppState::global(cx).clone()).ok().unwrap();
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

    #[allow(dead_code)]
    fn save_layout(
        &mut self,
        dock_area: &Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dock_area = dock_area.clone();
        self._save_layout_task = Some(cx.spawn_in(window, async move |story, window| {
            Timer::after(std::time::Duration::from_secs(10)).await;

            _ = story.update_in(window, move |this, _, cx| {
                let dock_area = dock_area.read(cx);
                let state = dock_area.dump(cx);

                let last_layout_state = this.last_layout_state.clone();
                if Some(&state) == last_layout_state.as_ref() {
                    return;
                }

                Self::save_state(&state).ok();
                this.last_layout_state = Some(state);
            });
        }));
    }

    #[allow(dead_code)]
    fn save_state(state: &DockAreaState) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write("dock_layout.json", json)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn load_layout(
        dock_area: Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let json = std::fs::read_to_string("dock_layout.json")?;
        let state = serde_json::from_str::<DockAreaState>(&json)?;

        dock_area.update(cx, |dock_area, cx| {
            dock_area.load(state, window, cx)?;
            dock_area.set_dock_collapsible(
                Edges {
                    left: true,
                    bottom: true,
                    right: true,
                    ..Default::default()
                },
                window,
                cx,
            );

            Ok::<(), anyhow::Error>(())
        })
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
