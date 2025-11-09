use gpui::{
    Application, AppContext, WindowOptions,
};
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockItem};
use gpui_component::input::InputState;
use std::sync::Arc;

mod workspace;
mod editor_panel;
mod settings_panel;
mod app_title_bar;
mod file_tree_panel;
mod app_actions;

use workspace::Workspace;
use editor_panel::EditorPanel;
use settings_panel::SettingsPanel;
use app_title_bar::AppTitleBar;
use file_tree_panel::FileTreePanel;
use gpui_component::menu::AppMenuBar;


fn main() {
    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);
        file_tree_panel::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window_ctx, cx| {
                let dock_area_entity =
                    cx.new(|cx| DockArea::new("main_dock_area", None, window_ctx, cx));

                let app_menu_bar = AppMenuBar::new(window_ctx, cx);
                let app_title_bar = cx.new(|_cx| AppTitleBar { app_menu_bar });

                let file_tree_panel = cx.new(|cx| FileTreePanel::new("File Tree", cx));

                dock_area_entity.update(cx, |dock_area, cx| {
                    let panel2 = cx.new(|cx| SettingsPanel::new("Panel 2", cx));
                    let panel3 = cx.new(|cx| SettingsPanel::new("Panel 3", cx));

                    let code_editor_state1 = cx.new(|cx| {
                        InputState::new(window_ctx, cx)
                            .code_editor("rust")
                            .line_number(true)
                            .searchable(true)
                            .default_value("fn main() {\n    println!(\"Hello, from editor 1!\");\n}")
                    });
                    let editor_panel1 =
                        cx.new(|cx| EditorPanel::new("Editor 1", code_editor_state1, cx));

                    let code_editor_state2 = cx.new(|cx| {
                        InputState::new(window_ctx, cx)
                            .code_editor("rust")
                            .line_number(true)
                            .searchable(true)
                            .default_value("fn main() {\n    println!(\"Hello, from editor 2!\");\n}")
                    });
                    let editor_panel2 =
                        cx.new(|cx| EditorPanel::new("Editor 2", code_editor_state2, cx));

                    dock_area.set_left_dock(
                        DockItem::tabs(
                            vec![Arc::new(file_tree_panel)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        None,
                        true,
                        window_ctx,
                        cx,
                    );

                    dock_area.set_bottom_dock(
                        DockItem::tabs(
                            vec![Arc::new(panel2)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        None,
                        true,
                        window_ctx,
                        cx,
                    );

                    dock_area.set_right_dock(
                        DockItem::tabs(
                            vec![Arc::new(panel3)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        None,
                        true,
                        window_ctx,
                        cx,
                    );

                    dock_area.set_center(
                        DockItem::tabs(
                            vec![Arc::new(editor_panel1), Arc::new(editor_panel2)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        window_ctx,
                        cx,
                    );
                });

                let view = cx.new(|_cx| Workspace {
                    dock_area: dock_area_entity,
                    title_bar: app_title_bar,
                });
                cx.new(|cx| Root::new(view.into(), window_ctx, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}