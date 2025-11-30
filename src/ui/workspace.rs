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

impl Workspace {
    pub fn new_local(
        cx: &mut App,
        initial_file: Option<PathBuf>,
        initial_folder: Option<PathBuf>,
    ) -> Task<anyhow::Result<WindowHandle<Root>>> {
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
                let view = cx.new(|cx| Self::new(window, cx, initial_file, initial_folder));
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

    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        initial_file: Option<PathBuf>,
        initial_folder: Option<PathBuf>,
    ) -> Self {
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
                Self::reset_default_layout(
                    weak_dock_area,
                    window,
                    cx,
                    initial_file,
                    initial_folder,
                );
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

    fn reset_default_layout(
        dock_area: WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
        initial_file: Option<PathBuf>,
        initial_folder: Option<PathBuf>,
    ) {
        let app = AppState::global(cx).clone();

        // We need to spawn a task to handle async file opening if needed,
        // but for initial layout setup we might need synchronous creation or placeholders.
        // However, `reset_default_layout` is called from `new` which is synchronous.
        // We can create an empty buffer or load synchronously if possible, but `open_file` is async.
        // For now, let's create an empty buffer and load the file in background if present.

        let buffer = if let Some(_) = initial_file.clone() {
            // Ideally we would wait, but we are in sync context.
            // We'll start with empty and let the file loader update it,
            // OR we can block if we are sure (not recommended in UI thread),
            // OR we can just create a placeholder.
            // Given the structure, let's create an empty buffer and trigger open.
            Arc::new(crate::model::file_buffer::FileBuffer::empty())
        } else {
            Arc::new(crate::model::file_buffer::FileBuffer::empty())
        };

        let file_tree_panel = cx.new(|cx| FileTreePanel::new(FILE_TREE_PANEL_TITLE, cx));
        if let Some(folder) = initial_folder {
            file_tree_panel.update(cx, |panel, cx| {
                panel.set_root_path(folder, cx);
            });
        }

        let editor_panel = cx.new(|cx| EditorPanel::new(buffer.clone(), window, cx));

        // If we have an initial file, we should trigger loading it.
        if let Some(path) = initial_file {
            cx.spawn(async move |this, cx| {
                if let Ok(loaded_buffer) = app.editor_service.open_file(path).await {
                    // We need to update the editor panel with the new buffer.
                    // This might be tricky if the panel is already created with the old buffer.
                    // Alternatively, we can just add a new editor panel.
                    // But we want to replace the initial one.
                    // For simplicity in this step, let's just add a new panel or update if possible.
                    // Since EditorPanel takes buffer in constructor, maybe we can just dispatch AddEditorPanel?
                    // But we want it in the default layout.

                    // Actually, looking at `main.rs` original code, it awaited `open_file`.
                    // Here we are inside `new` which is sync.
                    // We can use `cx.spawn` to update the editor panel later.
                    // But `EditorPanel` might not have a method to swap buffer easily without recreating.
                    // Let's assume for now we just open it in a new tab if it's not empty,
                    // or we can try to make `EditorPanel` updateable.
                    // For now, let's just stick to the structure and maybe dispatch an action.

                    // Wait, `EditorPanel` holds `buffer`.
                    // Let's just dispatch `AddEditorPanel` which adds a NEW panel.
                    // The initial empty panel might be redundant then.

                    // Let's try to load it and replace if possible, or just open it.
                    if let Some(this) = this.upgrade() {
                        let _ = this.update(cx, |_, cx| {
                            cx.dispatch_action(&AddEditorPanel(loaded_buffer));
                        });
                    }
                }
            })
            .detach();
        }

        if let Some(dock_area_entity) = dock_area.upgrade() {
            dock_area_entity.update(cx, |dock_area_view, cx| {
                let left_dock = DockItem::tabs(
                    vec![Arc::new(file_tree_panel)],
                    Some(0),
                    &dock_area,
                    window,
                    cx,
                );

                let center_dock = DockItem::tabs(
                    vec![Arc::new(editor_panel)],
                    Some(0),
                    &dock_area,
                    window,
                    cx,
                );

                dock_area_view.set_left_dock(left_dock, None, true, window, cx);
                dock_area_view.set_center(center_dock, window, cx);
            });
        }
    }

    fn on_action_add_editor_panel(
        &mut self,
        action: &AddEditorPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let buffer = action.0.clone();
        let editor_panel = cx.new(|cx| EditorPanel::new(buffer, window, cx));

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
}

impl Render for Workspace {
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
