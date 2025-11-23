use gpui::{AppContext, Application, WindowOptions};
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockItem};
use gpui_component_assets::Assets;
use std::sync::Arc;

mod actions;
mod app_state;
mod data;
mod keybindings;
mod service;
mod theme;
mod ui;

use gpui_component::menu::AppMenuBar;
use ui::editor_panel::EditorPanel;
use ui::file_tree_panel::FileTreePanel;
use ui::toolbar::AppTitleBar;
use ui::window::WindowRoot;

// Application constants
const MAIN_DOCK_AREA_ID: &str = "main_dock_area";
const MAIN_DOCK_AREA_VERSION: usize = 1;
const FILE_TREE_PANEL_TITLE: &str = "File Tree";

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let path_arg = args.get(1).map(|s| std::path::PathBuf::from(s));

    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        app_state::AppState::init(cx);

        keybindings::init(cx);
        gpui_component::init(cx);
        theme::init(cx);
        ui::file_tree_panel::init(cx);
        ui::editor_panel::init(cx);

        cx.spawn(async move |cx| {
            // Determine what to open based on command line argument
            let (file_to_open, folder_to_open) = if let Some(path) = path_arg {
                if path.is_file() {
                    (Some(path), None)
                } else if path.is_dir() {
                    (None, Some(path))
                } else {
                    eprintln!("Warning: Path does not exist: {}", path.display());
                    (None, None)
                }
            } else {
                // No argument: open empty buffer
                (None, None)
            };

            let app = cx
                .update(|cx| app_state::AppState::global(cx).clone())
                .ok()
                .unwrap();

            let buffer = if let Some(file_path) = file_to_open {
                app.editor_service.open_file(file_path).await.unwrap()
            } else {
                // Open empty buffer
                std::sync::Arc::new(crate::data::file_buffer::FileBuffer::empty())
            };

            let _ = cx
                .update(|cx| {
                    let _ = cx.open_window(WindowOptions::default(), |window_ctx, cx| {
                        let dock_area_entity = cx.new(|cx| {
                            DockArea::new(
                                MAIN_DOCK_AREA_ID,
                                Some(MAIN_DOCK_AREA_VERSION),
                                window_ctx,
                                cx,
                            )
                        });

                        let app_menu_bar = AppMenuBar::new(window_ctx, cx);
                        let app_title_bar = cx.new(|_cx| AppTitleBar { app_menu_bar });

                        let file_tree_panel =
                            cx.new(|cx| FileTreePanel::new(FILE_TREE_PANEL_TITLE, cx));

                        // Clone for later use
                        let file_tree_panel_clone = file_tree_panel.clone();

                        dock_area_entity.update(cx, |dock_area, cx| {
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

                            // Enable drag and drop for dock panels
                            dock_area.set_dock_collapsible(
                                gpui::Edges {
                                    left: true,
                                    bottom: true,
                                    right: true,
                                    ..Default::default()
                                },
                                window_ctx,
                                cx,
                            );
                        });

                        // Set folder if specified in command line
                        if let Some(folder_path) = folder_to_open {
                            file_tree_panel_clone.update(cx, |panel, cx| {
                                panel.set_root_path(folder_path, cx);
                            });
                        }

                        let view = cx.new(|_cx| WindowRoot {
                            dock_area: dock_area_entity,
                            title_bar: app_title_bar,
                            last_layout_state: None,
                            _save_layout_task: None,
                        });
                        cx.new(|cx| Root::new(view, window_ctx, cx))
                    });
                })
                .ok();
        })
        .detach();
    });
}
