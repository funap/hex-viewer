use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

use crate::actions::*;

use crate::ui::panels::editor_panel::EditorPanel;
use crate::ui::panels::file_tree_panel::FileTreePanel;
use crate::ui::panels::struct_tree_panel::StructTreePanel;

use crate::ui::components::toolbar::AppTitleBar;

use crate::app_state::AppState;
use crate::core::editor::Editor;
use crate::ui::components::status_bar::StatusBar;
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
    pub active_editor: Option<Entity<Editor>>,
    pub struct_tree: Entity<StructTreePanel>,
    pub is_struct_tree_visible: bool,
}

const MAIN_DOCK_AREA_ID: &str = "main_dock_area";
const MAIN_DOCK_AREA_VERSION: usize = 1;
const FILE_TREE_PANEL_TITLE: &str = "FILES";

pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", gpui_component::dock::ToggleZoom, None),
        KeyBinding::new("ctrl-w", gpui_component::dock::ClosePanel, None),
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

        cx.subscribe_in(&title_bar, window, |this, _, event, window, cx| match event {
            crate::ui::components::toolbar::AppTitleBarEvent::OpenSettings => {
                this.open_settings_panel(window, cx);
            }
        })
        .detach();

        let struct_tree = cx.new(|cx| StructTreePanel::new(cx));

        let status_bar = cx.new(|cx| StatusBar::new(cx));
        cx.subscribe(&status_bar, |this, _, event, cx| match event {
            crate::ui::components::status_bar::StatusBarEvent::ToggleFileTree => {
                this.is_file_tree_visible = !this.is_file_tree_visible;
                cx.notify();
            }
        })
        .detach();

        let file_tree = cx.new(|cx| FileTreePanel::new(FILE_TREE_PANEL_TITLE, cx));
        cx.on_focus_in(&file_tree.read(cx).focus_handle(cx), window, |this, _, cx| {
            this.active_editor = None;
            this.status_bar
                .update(cx, |status_bar, _| status_bar.set_active_editor(None));
            cx.notify();
        })
        .detach();
        cx.subscribe(&file_tree, |_, _, event, cx| match event {
            crate::ui::panels::file_tree_panel::FileTreeEvent::OpenFile(path) => {
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
            active_editor: None,
            struct_tree,
            is_struct_tree_visible: true,
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
        println!("Workspace::on_action_add_editor_panel triggered");
        let document = action.0.clone();
        let editor = cx.new(|_| Editor::new(document));

        let editor_panel = cx.new(|cx| EditorPanel::new(editor, window, cx));
        cx.on_focus_in(&editor_panel.read(cx).focus_handle(cx), window, {
            let editor_panel = editor_panel.clone();
            move |this, _window, cx| {
                let editor = editor_panel.read(cx).editor();
                this.active_editor = Some(editor.clone());
                this.status_bar.update(cx, |status_bar, _| {
                    status_bar.set_active_editor(Some(editor));
                });
                cx.notify();
            }
        })
        .detach();
        let panel = Arc::new(editor_panel);

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(panel, DockPlacement::Center, None, window, cx);
        });
    }

    fn on_action_open_file_dialog(&mut self, _: &OpenFileDialog, window: &mut Window, cx: &mut Context<Self>) {
        println!("OpenFileDialog triggered");
        let path = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select a file".into()),
        });

        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            println!("OpenFileDialog prompt returned");
            if let Some(path) = path.await.ok().and_then(|r| r.ok()).flatten().and_then(|mut v| v.pop()) {
                println!("Selected path: {:?}", path);
                window.update(|window, cx| {
                    println!("Directly calling OpenFile handler for {:?}", path);
                    view.update(cx, |this, cx| {
                        let action = crate::actions::OpenFile { path: path.to_string_lossy().to_string() };
                        this.on_action_open_file(&action, window, cx);
                    });
                }).ok();
            } else {
                println!("No path selected or error occurred");
            }
        })
        .detach();
    }

    fn on_action_quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn on_action_select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.select_all();
            });
        }
    }

    fn on_action_go_to_beginning(&mut self, _: &GoToBeginning, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.go_to_beginning();
            });
        }
    }

    fn on_action_go_to_end(&mut self, _: &GoToEnd, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.go_to_end();
            });
        }
    }

    fn on_action_set_encoding_ascii(&mut self, _: &SetEncodingAscii, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.set_encoding(crate::core::encoding::Encoding::Ascii);
            });
        }
    }

    fn on_action_set_encoding_utf8(&mut self, _: &SetEncodingUtf8, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.set_encoding(crate::core::encoding::Encoding::Utf8);
            });
        }
    }

    fn on_action_set_encoding_utf16le(&mut self, _: &SetEncodingUtf16Le, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.set_encoding(crate::core::encoding::Encoding::Utf16Le);
            });
        }
    }

    fn on_action_set_encoding_utf16be(&mut self, _: &SetEncodingUtf16Be, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.active_editor {
            editor.update(cx, |editor, _cx| {
                editor.set_encoding(crate::core::encoding::Encoding::Utf16Be);
            });
        }
    }

    fn on_action_open_file(&mut self, action: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        println!("Workspace::on_action_open_file triggered for {}", action.path);
        let file_path = action.path.clone();
        let path = std::path::PathBuf::from(&file_path);

        if let Some(focus_handle) = self.find_existing_panel(&path, cx) {
            println!("Found existing panel for {:?}", path);
            focus_handle.focus(window);
            return;
        }

        println!("Spawning task to open file {:?}", path);
        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let editor_service_opt = window.update(|_, cx| AppState::global(cx).editor_service.clone()).ok();
            
            if let Some(editor_service) = editor_service_opt {
                println!("Opening file through editor service...");
                match editor_service.open_file(std::path::PathBuf::from(&file_path)).await {
                    Ok(document) => {
                        println!("File opened successfully. Adding EditorPanel directly");
                        window.update(|window, cx| {
                            view.update(cx, |this, cx| {
                                let action = AddEditorPanel(document);
                                this.on_action_add_editor_panel(&action, window, cx);
                            });
                        }).ok();
                    }
                    Err(e) => {
                        println!("Failed to open file: {:?}", e);
                    }
                }
            } else {
                println!("Failed to access window state");
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

                if let (Ok(left_document), Ok(right_document)) = (left_result, right_result) {
                    let _ = workspace.update_in(window, |_, window, cx| {
                        let app = AppState::global(cx).clone();
                        let diff_result_task = app.editor_service.compute_diff(left_document.clone(), right_document.clone(), cx);

                        cx.spawn_in(window, async move |workspace, window| {
                            let diff_result = diff_result_task.await;

                            let _ = workspace.update_in(window, |workspace_view, window, cx| {
                                use crate::ui::panels::diff_panel::DiffPanel;
                                let diff_view = cx.new(|cx| {
                                    let mut view = DiffPanel::new(left_document.clone(), right_document.clone(), window, cx);
                                    view.set_diff_result(diff_result.clone(), cx);
                                    view
                                });

                                cx.on_focus_in(
                                    &diff_view.read(cx).focus_handle(cx),
                                    window,
                                    |this, _, cx| {
                                        this.active_editor = None;
                                        this.status_bar.update(cx, |status_bar, _| {
                                            status_bar.set_active_editor(None)
                                        });
                                        cx.notify();
                                    },
                                )
                                .detach();

                                let panel = Arc::new(diff_view);

                                workspace_view.dock_area.update(cx, |dock_area, cx| {
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

    fn on_action_toggle_struct_tree(&mut self, _: &ToggleStructTree, _: &mut Window, cx: &mut Context<Self>) {
        self.is_struct_tree_visible = !self.is_struct_tree_visible;
        cx.notify();
    }

    fn on_action_open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
        self.open_settings_panel(window, cx);
    }

    fn open_settings_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use crate::ui::panels::settings_panel::SettingsPanel;

        let dock_area = self.dock_area.read(cx);
        let existing_panel = Self::check_has_settings_panel(dock_area.items());

        if let Some(panel) = existing_panel {
            let focus_handle = panel.read(cx).focus_handle(cx);
            focus_handle.focus(window);
            return;
        }

        let settings_panel = cx.new(|cx| SettingsPanel::new(window, cx));
        cx.on_focus_in(
            &settings_panel.read(cx).focus_handle(cx),
            window,
            |this, _, cx| {
                this.active_editor = None;
                this.status_bar
                    .update(cx, |status_bar, _| status_bar.set_active_editor(None));
                cx.notify();
            },
        )
        .detach();
        let panel = Arc::new(settings_panel);

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(panel, DockPlacement::Center, None, window, cx);
        });
    }

    fn check_has_settings_panel(dock_item: &DockItem) -> Option<Entity<crate::ui::panels::settings_panel::SettingsPanel>> {
        match dock_item {
            DockItem::Tabs { items, .. } => {
                for item in items {
                    if let Ok(panel) = item.view().downcast::<crate::ui::panels::settings_panel::SettingsPanel>() {
                        return Some(panel);
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if let Some(panel) = Self::check_has_settings_panel(item) {
                        return Some(panel);
                    }
                }
            }
            _ => {}
        }
        None
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
                        if panel.read(cx).path(cx) == path {
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
                            if let Ok(document) = app.editor_service.open_file(file_path).await {
                                let _ = window.update(cx, |_root, _window, cx| {
                                    cx.dispatch_action(&AddEditorPanel(document));
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
                    if item.view().downcast::<crate::ui::panels::diff_panel::DiffPanel>().is_ok() {
                        return true;
                    }
                    if item.view().downcast::<crate::ui::panels::settings_panel::SettingsPanel>().is_ok() {
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
            DockItem::Tiles { items, .. } => {
                if !items.is_empty() {
                    return true;
                }
            }
            DockItem::Panel { view, .. } => {
                if view.view().downcast::<EditorPanel>().is_ok() {
                    return true;
                }
                if view.view().downcast::<crate::ui::panels::diff_panel::DiffPanel>().is_ok() {
                    return true;
                }
                if view.view().downcast::<crate::ui::panels::settings_panel::SettingsPanel>().is_ok() {
                    return true;
                }
            }
        }
        false
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .on_action(cx.listener(Self::on_action_open_file))
            .on_action(cx.listener(Self::on_action_open_file_dialog))
            .on_action(cx.listener(Self::on_action_quit))
            .on_action(cx.listener(Self::on_action_select_all))
            .on_action(cx.listener(Self::on_action_go_to_beginning))
            .on_action(cx.listener(Self::on_action_go_to_end))
            .on_action(cx.listener(Self::on_action_set_encoding_ascii))
            .on_action(cx.listener(Self::on_action_set_encoding_utf8))
            .on_action(cx.listener(Self::on_action_set_encoding_utf16le))
            .on_action(cx.listener(Self::on_action_set_encoding_utf16be))
            .on_action(cx.listener(Self::on_action_add_editor_panel))
            .on_action(cx.listener(Self::on_action_open_diff))
            .on_action(cx.listener(Self::on_action_toggle_file_tree))
            .on_action(cx.listener(Self::on_action_open_settings))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(
                h_resizable("workspace-h-resize")
                    .child(
                        resizable_panel()
                            .visible(self.is_file_tree_visible)
                            .size(px(250.))
                            .child(self.file_tree.clone()),
                    )
                    .child(
                        resizable_panel()
                            .child(
                                div()
                                .relative()
                                .size_full()
                                .flex()
                                .flex_col()
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
                                },
                            )),
                    ),
            )
            .child(self.status_bar.clone())
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
