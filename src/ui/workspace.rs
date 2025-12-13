use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

use crate::actions::*;

use crate::ui::editor_panel::EditorPanel;
use crate::ui::file_tree_panel::FileTreePanel;

use crate::ui::toolbar::AppTitleBar;

use crate::app_state::AppState;
use crate::ui::status_bar::StatusBar;
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockItem, DockPlacement};
use gpui_component::menu::AppMenuBar;
use gpui_component::resizable::{h_resizable, resizable_panel};
use std::path::PathBuf;
use std::sync::Arc;

pub struct Workspace {
    pub dock_area: Entity<DockArea>,
    pub file_tree: Entity<FileTreePanel>,
    pub is_file_tree_visible: bool,
    pub title_bar: Entity<AppTitleBar>,
    pub status_bar: Entity<StatusBar>,
}

const MAIN_DOCK_AREA_ID: &str = "main_dock_area";
const MAIN_DOCK_AREA_VERSION: usize = 1;
const FILE_TREE_PANEL_TITLE: &str = "FILES";

pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", gpui_component::dock::ToggleZoom, None),
        KeyBinding::new("ctrl-w", CloseActiveTab, None),
    ]);

    cx.activate(true);
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dock_area = cx.new(|cx| DockArea::new(MAIN_DOCK_AREA_ID, Some(MAIN_DOCK_AREA_VERSION), window, cx));
        let weak_dock_area = dock_area.downgrade();

        cx.observe(&dock_area, |_, _, cx| cx.notify()).detach();

        let app_menu_bar = AppMenuBar::new(window, cx);
        let title_bar = cx.new(|_cx| AppTitleBar { app_menu_bar });

        let editor_status = AppState::global(cx).editor_status.clone();
        let status_bar = cx.new(|cx| StatusBar::new(editor_status, cx));
        cx.subscribe(&status_bar, |this, _, event, cx| match event {
            crate::ui::status_bar::StatusBarEvent::ToggleFileTree => {
                this.is_file_tree_visible = !this.is_file_tree_visible;
                cx.notify();
            }
        })
        .detach();

        let file_tree = cx.new(|cx| FileTreePanel::new(FILE_TREE_PANEL_TITLE, cx));
        cx.subscribe(&file_tree, |_, _, event, cx| match event {
            crate::ui::file_tree_panel::FileTreeEvent::OpenFile(path) => {
                cx.dispatch_action(&crate::actions::OpenFile {
                    path: path.to_string_lossy().to_string(),
                });
            }
        })
        .detach();

        Self::reset_default_layout(weak_dock_area, window, cx);

        Self {
            dock_area,
            file_tree,
            is_file_tree_visible: true,
            title_bar,
            status_bar,
        }
    }

    fn reset_default_layout(dock_area: WeakEntity<DockArea>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dock_area_entity) = dock_area.upgrade() {
            dock_area_entity.update(cx, |dock_area_view, cx| {
                // Center dock starts empty
                dock_area_view.set_center(DockItem::split(Axis::Vertical, vec![], &dock_area, window, cx), window, cx);
            });
        }
    }

    fn new_local(cx: &mut App) -> Task<anyhow::Result<WindowHandle<Root>>> {
        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                #[cfg(not(target_os = "linux"))]
                titlebar: Some(gpui_component::TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(640.),
                    height: px(480.),
                }),
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                kind: WindowKind::Normal,
                ..Default::default()
            };

            let window = cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| Self::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            window
                .update(cx, |_, window, cx| {
                    window.activate_window();
                    window.set_window_title("XVI");
                    cx.on_release(|_, cx| {
                        cx.quit();
                    })
                    .detach();
                })
                .expect("failed to update window");

            Ok(window)
        })
    }

    fn on_action_add_editor_panel(&mut self, action: &AddEditorPanel, window: &mut Window, cx: &mut Context<Self>) {
        let buffer = action.0.clone();
        let editor_panel = cx.new(|cx| EditorPanel::new(buffer, window, cx));
        let panel = Arc::new(editor_panel);

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(panel, DockPlacement::Center, None, window, cx);
        });
    }

    fn on_action_open_file(&mut self, action: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        let file_path = action.path.clone();
        let path = std::path::PathBuf::from(&file_path);

        if let Some(focus_handle) = self.find_existing_panel(&path, cx) {
            focus_handle.focus(window);
            return;
        }

        cx.spawn(async move |this, cx| {
            let app = cx.update(|cx| AppState::global(cx).clone()).ok().unwrap();

            if let Some(add_editor_panel) = this.upgrade() {
                if let Ok(buffer) = app.editor_service.open_file(std::path::PathBuf::from(file_path)).await {
                    let _ = add_editor_panel.update(cx, |_, cx| {
                        cx.dispatch_action(&AddEditorPanel(buffer));
                    });
                }
            }
        })
        .detach();
    }

    fn on_action_open_diff(&mut self, action: &OpenDiff, window: &mut Window, cx: &mut Context<Self>) {
        let left_path = action.left_path.clone();
        let right_path = action.right_path.clone();

        cx.spawn_in(window, async move |this, window| {
            let app = this.update(window, |_, cx| AppState::global(cx).clone()).ok().unwrap();

            if let Some(workspace) = this.upgrade() {
                let left_result = app.editor_service.open_file(std::path::PathBuf::from(left_path)).await;
                let right_result = app.editor_service.open_file(std::path::PathBuf::from(right_path)).await;

                if let (Ok(left_buffer), Ok(right_buffer)) = (left_result, right_result) {
                    let _ = workspace.update_in(window, |_, window, cx| {
                        let app = AppState::global(cx).clone();
                        let diff_result_task = app.editor_service.compute_diff(left_buffer.clone(), right_buffer.clone(), cx);

                        cx.spawn_in(window, async move |workspace, window| {
                            let diff_result = diff_result_task.await;

                            let _ = workspace.update_in(window, |_workspace, window, cx| {
                                use crate::ui::diff_panel::DiffPanel;
                                let diff_view = cx.new(|cx| {
                                    let mut view = DiffPanel::new(left_buffer, right_buffer, window, cx);
                                    view.set_diff_result(diff_result, cx);
                                    view
                                });
                                let panel = Arc::new(diff_view);

                                _workspace.dock_area.update(cx, |dock_area, cx| {
                                    dock_area.add_panel(panel, DockPlacement::Center, None, window, cx);
                                });
                            });
                        })
                        .detach();
                    });
                }
            }
        })
        .detach();
    }

    fn on_action_toggle_file_tree(&mut self, _: &ToggleFileTree, _: &mut Window, cx: &mut Context<Self>) {
        self.is_file_tree_visible = !self.is_file_tree_visible;
        cx.notify();
    }

    fn find_existing_panel(&self, path: &std::path::Path, cx: &App) -> Option<FocusHandle> {
        let dock_area = self.dock_area.read(cx);
        Self::find_panel_in_items(dock_area.items(), path, cx)
    }

    fn find_panel_in_items(dock_item: &DockItem, path: &std::path::Path, cx: &App) -> Option<FocusHandle> {
        match dock_item {
            DockItem::Tabs { items, .. } => {
                for item in items {
                    if let Ok(panel) = item.view().downcast::<EditorPanel>() {
                        if panel.read(cx).path() == path {
                            return Some(panel.read(cx).focus_handle(cx));
                        }
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if let Some(handle) = Self::find_panel_in_items(item, path, cx) {
                        return Some(handle);
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Opens a new workspace window with the specified files and folder.
    /// This is the main public API for creating workspace windows.
    pub fn open_window(cx: &mut App, initial_files: Vec<PathBuf>, initial_folder: Option<PathBuf>) -> Task<()> {
        let task = Self::new_local(cx);
        cx.spawn(async move |cx| {
            if let Ok(window) = task.await {
                // Open all initial files if provided
                if !initial_files.is_empty() {
                    if let Ok(app) = cx.update(|cx| AppState::global(cx).clone()) {
                        for file_path in initial_files {
                            if let Ok(buffer) = app.editor_service.open_file(file_path).await {
                                let _ = window.update(cx, |_root, _window, cx| {
                                    cx.dispatch_action(&AddEditorPanel(buffer));
                                });
                            }
                        }
                    }
                }

                // Set initial folder if provided
                if let Some(folder_path) = initial_folder {
                    let _ = window.update(cx, |_root, _window, cx| {
                        cx.dispatch_action(&SetFileTreeFolder {
                            path: folder_path.to_string_lossy().to_string(),
                        });
                    });
                }
            }
        })
    }

    fn on_action_close_panel_by_id(&mut self, action: &ClosePanelById, window: &mut Window, cx: &mut Context<Self>) {
        println!("on_action_close_panel_by_id: view_id={}", action.view_id);
        let dock_area = self.dock_area.read(cx);
        if let Some(panel) = Self::find_panel_by_id(dock_area.items(), action.view_id) {
            println!("Panel found, attempting to remove: {:?}", panel.panel_name());
            self.dock_area.update(cx, |dock_area, cx| {
                dock_area.remove_panel(panel, gpui_component::dock::DockPlacement::Center, window, cx);
            });
        } else {
            println!("Panel not found for id: {}", action.view_id);
        }
    }

    fn find_panel_by_id(dock_item: &DockItem, view_id: u64) -> Option<std::sync::Arc<dyn gpui_component::dock::PanelView>> {
        match dock_item {
            DockItem::Tabs { items, .. } => {
                for item in items {
                    // println!("Checking item: {:?} id={}", item.panel_name(), item.view().entity_id().as_u64());
                    if item.view().entity_id().as_u64() == view_id {
                        return Some(item.clone());
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if let Some(found) = Self::find_panel_by_id(item, view_id) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn on_action_close_active_tab(&mut self, _: &CloseActiveTab, window: &mut Window, cx: &mut Context<Self>) {
        let dock_area = self.dock_area.read(cx);
        if let Some(panel) = Self::find_active_panel(dock_area.items(), window, cx) {
            self.dock_area.update(cx, |dock_area, cx| {
                dock_area.remove_panel(panel, gpui_component::dock::DockPlacement::Center, window, cx);
            });
        }
    }

    fn find_active_panel(dock_item: &DockItem, window: &Window, cx: &App) -> Option<std::sync::Arc<dyn gpui_component::dock::PanelView>> {
        match dock_item {
            DockItem::Tabs { items, .. } => {
                for item in items {
                    if let Ok(panel) = item.view().downcast::<EditorPanel>() {
                        if panel.read(cx).focus_handle(cx).is_focused(window) {
                            return Some(item.clone());
                        }
                    }
                    if let Ok(panel) = item.view().downcast::<crate::ui::diff_panel::DiffPanel>() {
                        if panel.read(cx).focus_handle(cx).is_focused(window) {
                            return Some(item.clone());
                        }
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if let Some(found) = Self::find_active_panel(item, window, cx) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn check_has_panels(&self, cx: &App) -> bool {
        let dock_area = self.dock_area.read(cx);
        Self::has_panels_recursive(dock_area.items())
    }

    fn has_panels_recursive(dock_item: &DockItem) -> bool {
        match dock_item {
            DockItem::Tabs { items, .. } => {
                for item in items {
                    if item.view().downcast::<EditorPanel>().is_ok() {
                        return true;
                    }
                    if item.view().downcast::<crate::ui::diff_panel::DiffPanel>().is_ok() {
                        return true;
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if Self::has_panels_recursive(item) {
                        return true;
                    }
                }
            }
            _ => {}
        }
        false
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .on_action(cx.listener(Self::on_action_open_file))
            .on_action(cx.listener(Self::on_action_add_editor_panel))
            .on_action(cx.listener(Self::on_action_open_diff))
            .on_action(cx.listener(Self::on_action_toggle_file_tree))
            .on_action(cx.listener(Self::on_action_close_panel_by_id))
            .on_action(cx.listener(Self::on_action_close_active_tab))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(
                h_resizable("workspace-h-resize")
                    .when(self.is_file_tree_visible, |this| {
                        this.child(resizable_panel().size(px(250.)).child(self.file_tree.clone()))
                    })
                    .child(
                        resizable_panel().child(
                            div()
                                .flex_1()
                                .relative()
                                .child(self.dock_area.clone())
                                .when(!self.check_has_panels(cx), |this| {
                                    this.child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .size_full()
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .bg(cx.theme().background)
                                            .child(div().text_xl().text_color(cx.theme().muted_foreground).child("Nothing is open")),
                                    )
                                }),
                        ),
                    ),
            )
            .child(self.status_bar.clone())
    }
}
