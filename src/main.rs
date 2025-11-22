use gpui::{AppContext, Application, WindowOptions};
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockItem};
use std::sync::Arc;

mod app;
mod data;
mod service;
mod ui;
mod util;

use gpui_component::menu::AppMenuBar;
use ui::editor_panel::EditorPanel;
use ui::file_tree_panel::FileTreePanel;
use ui::settings_panel::SettingsPanel;
use ui::toolbar::AppTitleBar;
use ui::window::WindowRoot;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    let app = Application::new();

    app.run(move |cx| {
        cx.set_global(app::App::new());

        gpui_component::init(cx);
        ui::file_tree_panel::init(cx);
        ui::editor_panel::init(cx);

        cx.spawn(async move |cx| {
            let app = cx
                .update(|cx| cx.global::<app::App>().clone())
                .ok()
                .unwrap();
            let buffer = app
                .editor_service
                .open_file("Cargo.toml".into())
                .await
                .unwrap();

            let _ = cx
                .update(|cx| {
                    let _ = cx.open_window(WindowOptions::default(), |window_ctx, cx| {
                        use crate::util::constants;

                        // ...

                        let dock_area_entity = cx.new(|cx| {
                            DockArea::new(constants::MAIN_DOCK_AREA_ID, None, window_ctx, cx)
                        });

                        let app_menu_bar = AppMenuBar::new(window_ctx, cx);
                        let app_title_bar = cx.new(|_cx| AppTitleBar { app_menu_bar });

                        let file_tree_panel =
                            cx.new(|cx| FileTreePanel::new(constants::FILE_TREE_PANEL_TITLE, cx));

                        dock_area_entity.update(cx, |dock_area, cx| {
                            let panel2 = cx.new(|cx| {
                                SettingsPanel::new(constants::SETTINGS_PANEL_TITLE_2, cx)
                            });
                            let panel3 = cx.new(|cx| {
                                SettingsPanel::new(constants::SETTINGS_PANEL_TITLE_3, cx)
                            });

                            let editor_panel = cx.new(|cx| EditorPanel::new(buffer, cx));

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
                                    vec![Arc::new(editor_panel)],
                                    None,
                                    &dock_area_entity.downgrade(),
                                    window_ctx,
                                    cx,
                                ),
                                window_ctx,
                                cx,
                            );
                        });

                        let view = cx.new(|_cx| WindowRoot {
                            dock_area: dock_area_entity,
                            title_bar: app_title_bar,
                        });
                        cx.new(|cx| Root::new(view, window_ctx, cx))
                    });
                })
                .ok();
        })
        .detach();
    });
}
