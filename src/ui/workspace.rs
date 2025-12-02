use gpui::prelude::*;
use gpui::*;

use crate::actions::*;

use crate::ui::editor_panel::EditorPanel;
use crate::ui::file_tree_panel::FileTreePanel;

use crate::ui::toolbar::AppTitleBar;

use crate::app_state::AppState;
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockAreaState, DockEvent, DockItem, DockPlacement};
use gpui_component::menu::AppMenuBar;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct Workspace {
    pub dock_area: Entity<DockArea>,
    pub title_bar: Entity<AppTitleBar>,
    pub last_layout_state: Option<DockAreaState>,
    pub _save_layout_task: Option<Task<()>>,
}

const MAIN_DOCK_AREA_ID: &str = "main_dock_area";
const MAIN_DOCK_AREA_VERSION: usize = 1;
const FILE_TREE_PANEL_TITLE: &str = "File Tree";
const STATE_FILE: &str = "dock_layout.json";

pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", gpui_component::dock::ToggleZoom, None),
        KeyBinding::new("ctrl-w", gpui_component::dock::ClosePanel, None),
    ]);

    cx.activate(true);
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dock_area =
            cx.new(|cx| DockArea::new(MAIN_DOCK_AREA_ID, Some(MAIN_DOCK_AREA_VERSION), window, cx));
        let weak_dock_area = dock_area.downgrade();

        let app_menu_bar = AppMenuBar::new(window, cx);
        let title_bar = cx.new(|_cx| AppTitleBar { app_menu_bar });

        match Self::load_layout(dock_area.clone(), window, cx) {
            Ok(_) => {
                println!("load layout success");
            }
            Err(err) => {
                eprintln!("load layout error: {:?}", err);
                Self::reset_default_layout(weak_dock_area, window, cx);
            }
        };

        cx.subscribe_in(
            &dock_area,
            window,
            |this, dock_area, ev: &DockEvent, window, cx| match ev {
                DockEvent::LayoutChanged => this.save_layout(dock_area, window, cx),
                _ => {}
            },
        )
        .detach();

        cx.on_app_quit({
            let dock_area = dock_area.clone();
            move |_, cx| {
                let state = dock_area.read(cx).dump(cx);
                cx.background_executor().spawn(async move {
                    Self::save_state(&state).unwrap();
                })
            }
        })
        .detach();

        Self {
            dock_area,
            title_bar,
            last_layout_state: None,
            _save_layout_task: None,
        }
    }

    fn save_layout(
        &mut self,
        dock_area: &Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dock_area = dock_area.clone();
        self._save_layout_task = Some(cx.spawn_in(window, async move |story, window| {
            Timer::after(Duration::from_secs(10)).await;

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

    fn save_state(state: &DockAreaState) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(STATE_FILE, json)?;
        Ok(())
    }

    fn load_layout(
        dock_area: Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let json = std::fs::read_to_string(STATE_FILE)?;
        let state = serde_json::from_str::<DockAreaState>(&json)?;

        // Check version if needed, similar to sample
        if state.version != Some(MAIN_DOCK_AREA_VERSION) {
            // For now, just error out to trigger reset, or handle migration
            return Err(anyhow::anyhow!("Version mismatch"));
        }

        dock_area.update(cx, |dock_area, cx| {
            dock_area.load(state, window, cx)?;
            Ok::<(), anyhow::Error>(())
        })
    }

    fn reset_default_layout(
        dock_area: WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Create empty buffer and file tree panel
        let buffer = Arc::new(crate::model::file_buffer::FileBuffer::empty());
        let file_tree_panel = cx.new(|cx| FileTreePanel::new(FILE_TREE_PANEL_TITLE, cx));
        let editor_panel = cx.new(|cx| EditorPanel::new(buffer.clone(), window, cx));

        if let Some(dock_area_entity) = dock_area.upgrade() {
            dock_area_entity.update(cx, |dock_area_view, cx| {
                let left_dock = DockItem::tab(file_tree_panel, &dock_area, window, cx);

                let center_dock = DockItem::split_with_sizes(
                    Axis::Vertical,
                    vec![DockItem::tab(editor_panel, &dock_area, window, cx)],
                    vec![None],
                    &dock_area,
                    window,
                    cx,
                );

                dock_area_view.set_left_dock(left_dock, None, true, window, cx);
                dock_area_view.set_center(center_dock, window, cx);
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

    fn on_action_add_editor_panel(
        &mut self,
        action: &AddEditorPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let buffer = action.0.clone();
        let editor_panel = cx.new(|cx| EditorPanel::new(buffer, window, cx));
        let panel = Arc::new(editor_panel);

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(panel, DockPlacement::Center, None, window, cx);
        });
    }

    fn on_action_open_file(
        &mut self,
        action: &OpenFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let file_path = action.path.clone();
        let path = std::path::PathBuf::from(&file_path);

        if let Some(focus_handle) = self.find_existing_panel(&path, cx) {
            focus_handle.focus(window);
            return;
        }

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

    fn find_existing_panel(&self, path: &std::path::Path, cx: &App) -> Option<FocusHandle> {
        let dock_area = self.dock_area.read(cx);
        Self::find_panel_in_items(dock_area.items(), path, cx)
    }

    fn find_panel_in_items(
        dock_item: &DockItem,
        path: &std::path::Path,
        cx: &App,
    ) -> Option<FocusHandle> {
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
    pub fn open_window(
        cx: &mut App,
        initial_files: Vec<PathBuf>,
        initial_folder: Option<PathBuf>,
    ) -> Task<()> {
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
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .on_action(cx.listener(Self::on_action_open_file))
            .on_action(cx.listener(Self::on_action_add_editor_panel))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(self.dock_area.clone())
    }
}
