use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

use crate::actions::*;

use crate::ui::components::activity_bar::{Activity, ActivityBar, ActivityBarEvent};
use crate::ui::components::file_tree_view::{FileTreeView, FileTreeViewEvent};
use crate::ui::components::title_bar::AppTitleBar;
use crate::ui::panels::editor_panel::EditorPanel;
use crate::ui::panels::left_panel::{LeftPanel, LeftPanelTab};

use crate::app_state::AppState;
use crate::core::editor::Editor;
use crate::ui::components::status_bar::StatusBar;
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockItem, DockPlacement, Panel, PanelView, TabPanel};
use gpui_component::menu::AppMenuBar;
use gpui_component::resizable::{h_resizable, resizable_panel};
use std::path::PathBuf;
use std::sync::Arc;

pub struct Workspace {
    pub dock_area: Entity<DockArea>,
    pub title_bar: Entity<AppTitleBar>,
    pub status_bar: Entity<StatusBar>,
    pub active_editor: Option<Entity<Editor>>,
    pub active_panel: Option<Arc<dyn PanelView>>,
    pub left_panel: Entity<LeftPanel>,
    pub activity_bar: Entity<ActivityBar>,
    pub ksy_definition: Option<Arc<crate::core::structure::KsyDefinition>>,
    pub is_left_panel_visible: bool,
}

const MAIN_DOCK_AREA_ID: &str = "main_dock_area";
const MAIN_DOCK_AREA_VERSION: usize = 1;

pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", gpui_component::dock::ToggleZoom, None),
        KeyBinding::new("ctrl-w", crate::actions::CloseActivePanel, None),
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
            crate::ui::components::title_bar::AppTitleBarEvent::OpenSettings => {
                this.open_settings_panel(window, cx);
            }
        })
        .detach();

        let file_tree = cx.new(|cx| FileTreeView::new("FILES", cx));
        let left_panel = cx.new(|cx| LeftPanel::new(file_tree.clone(), cx));
        let activity_bar = cx.new(|cx| ActivityBar::new(cx));

        cx.subscribe_in(&activity_bar, window, |this, _, event: &ActivityBarEvent, window, cx| match event {
            ActivityBarEvent::Select(activity) => {
                this.select_activity(*activity, window, cx);
            }
            ActivityBarEvent::OpenSettings => {
                this.open_settings_panel(window, cx);
            }
        })
        .detach();

        cx.observe(&left_panel, |this, _, cx| {
            this.sync_activity_bar(cx);
        })
        .detach();

        let status_bar = cx.new(|cx| StatusBar::new(cx));
        cx.subscribe(&status_bar, |this, _, event, cx| match event {
            crate::ui::components::status_bar::StatusBarEvent::ToggleLeftPanel => {
                this.is_left_panel_visible = !this.is_left_panel_visible;
                cx.notify();
            }
        })
        .detach();

        cx.on_focus_in(&file_tree.read(cx).focus_handle(cx), window, {
            move |this, _, cx| {
                // Keep active_editor and active_panel reference to support global/menu actions when file tree is focused.
                this.on_focus_changed(cx);
                cx.notify();
            }
        })
        .detach();

        let struct_tree = left_panel.read(cx).struct_tree.clone();
        cx.on_focus_in(&struct_tree.read(cx).focus_handle(cx), window, |this, _, cx| {
            this.on_focus_changed(cx);
            cx.notify();
        })
        .detach();

        let data_inspector = left_panel.read(cx).data_inspector.clone();
        cx.on_focus_in(&data_inspector.read(cx).focus_handle(cx), window, |this, _, cx| {
            this.on_focus_changed(cx);
            cx.notify();
        })
        .detach();

        let visual_map = left_panel.read(cx).visual_map.clone();
        cx.on_focus_in(&visual_map.read(cx).focus_handle(cx), window, |this, _, cx| {
            this.on_focus_changed(cx);
            cx.notify();
        })
        .detach();

        cx.subscribe(&left_panel, |_, _, event, cx| match event {
            FileTreeViewEvent::OpenFile(path) => {
                cx.dispatch_action(&crate::actions::OpenFile {
                    path: path.to_string_lossy().to_string(),
                });
            }
        })
        .detach();

        Self::reset_default_layout(weak_dock_area, window, cx);
        Self {
            dock_area,
            title_bar,
            status_bar,
            active_editor: None,
            active_panel: None,
            left_panel,
            activity_bar,
            ksy_definition: None,
            is_left_panel_visible: true,
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
                    window.set_window_title("XVW");
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

        if let Some(ksy) = &self.ksy_definition {
            let ksy = ksy.clone();
            editor.update(cx, |editor, cx| {
                editor.set_kaitai_definition(ksy);
                cx.notify();
            });
        }

        let editor_panel = cx.new(|cx| EditorPanel::new(editor, window, cx));
        cx.on_focus_in(&editor_panel.read(cx).focus_handle(cx), window, {
            let editor_panel = editor_panel.clone();
            move |this, _window, cx| {
                let editor = editor_panel.read(cx).editor();
                this.active_editor = Some(editor.clone());
                this.active_panel = Some(Arc::new(editor_panel.clone()));

                // Apply workspace-wide definition to the newly active editor
                if let Some(ksy) = &this.ksy_definition {
                    let ksy = ksy.clone();
                    editor.update(cx, |editor, cx| {
                        editor.set_kaitai_definition(ksy);
                        cx.notify();
                    });
                }

                this.status_bar.update(cx, |status_bar, _| {
                    status_bar.set_active_editor(Some(editor.clone()));
                });
                this.left_panel.update(cx, |panel, cx| {
                    panel.set_editor(Some(editor.clone()), cx);
                });
                this.on_focus_changed(cx);
                cx.notify();
            }
        })
        .detach();
        let panel = Arc::new(editor_panel);
        self.add_panel_to_center_dock(panel, window, cx);
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
                window
                    .update(|window, cx| {
                        println!("Directly calling OpenFile handler for {:?}", path);
                        view.update(cx, |this, cx| {
                            let action = crate::actions::OpenFile {
                                path: path.to_string_lossy().to_string(),
                            };
                            this.on_action_open_file(&action, window, cx);
                        });
                    })
                    .ok();
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

        if let Some((tab_panel, panel)) = self.find_existing_panel_and_tab_panel(&path, cx) {
            println!("Found existing panel for {:?}", path);
            let index_opt = {
                let mirror = unsafe { &*(tab_panel.read(cx) as *const TabPanel as *const TabPanelMirror) };
                mirror.panels.iter().position(|p| p.view().entity_id() == panel.view().entity_id())
            };

            if let Some(ix) = index_opt {
                Self::activate_tab_via_mirror(&tab_panel, ix, window, cx);
            } else {
                panel.focus_handle(cx).focus(window);
            }
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
                        window
                            .update(|window, cx| {
                                view.update(cx, |this, cx| {
                                    let action = AddEditorPanel(document);
                                    this.on_action_add_editor_panel(&action, window, cx);
                                });
                            })
                            .ok();
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

                                let diff_view_clone = diff_view.clone();
                                cx.on_focus_in(&diff_view.read(cx).focus_handle(cx), window, move |this, _, cx| {
                                    this.active_editor = None;
                                    this.status_bar.update(cx, |status_bar, _| status_bar.set_active_editor(None));
                                    this.active_panel = Some(Arc::new(diff_view_clone.clone()));
                                    this.on_focus_changed(cx);
                                    cx.notify();
                                })
                                .detach();

                                let panel = Arc::new(diff_view);
                                workspace_view.add_panel_to_center_dock(panel, window, cx);
                            });
                        })
                        .detach();
                    });
                }
            }
        })
        .detach();
    }

    fn on_action_toggle_left_panel(&mut self, _: &ToggleLeftPanel, _: &mut Window, cx: &mut Context<Self>) {
        self.is_left_panel_visible = !self.is_left_panel_visible;
        self.sync_activity_bar(cx);
        cx.notify();
    }

    fn on_action_show_files_tab(&mut self, _: &ShowFilesTab, window: &mut Window, cx: &mut Context<Self>) {
        self.select_activity(Activity::Files, window, cx);
    }

    fn on_action_show_structure_tab(&mut self, _: &ShowStructureTab, window: &mut Window, cx: &mut Context<Self>) {
        self.select_activity(Activity::Structure, window, cx);
    }

    fn select_activity(&mut self, activity: Activity, window: &mut Window, cx: &mut Context<Self>) {
        let tab = match activity {
            Activity::Files => LeftPanelTab::Files,
            Activity::Structure => LeftPanelTab::Structure,
            Activity::Inspector => LeftPanelTab::Inspector,
            Activity::Map => LeftPanelTab::Map,
        };

        let current_tab = self.left_panel.read(cx).active_tab;

        // If the same tab is already active and the panel is visible, hide it.
        // Otherwise, switch to the tab and ensure it's visible.
        if self.is_left_panel_visible && current_tab == tab {
            self.is_left_panel_visible = false;
        } else {
            self.is_left_panel_visible = true;
            self.left_panel.update(cx, |p, cx| {
                p.set_tab(tab, cx);
            });
            let focus_handle = self.left_panel.read(cx).focus_handle(cx);
            focus_handle.focus(window);
        }

        // Ensure the activity bar reflects the new state immediately
        self.sync_activity_bar(cx);
        cx.notify();
    }

    fn on_action_load_structure_definition(&mut self, _: &LoadStructureDefinition, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().clone();

        cx.spawn_in(window, async move |_, window| {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Kaitai Struct Definitions", &["ksy", "yaml"])
                .pick_file()
                .await;

            if let Some(handle) = file {
                let path = handle.path().to_path_buf();
                match std::fs::read_to_string(&path) {
                    Ok(contents) => match serde_yaml::from_str::<crate::core::structure::KsyDefinition>(&contents) {
                        Ok(ksy) => {
                            window
                                .update(|_window, cx| {
                                    view.update(cx, |this, cx| {
                                        let ksy_arc = Arc::new(ksy);
                                        this.ksy_definition = Some(ksy_arc.clone());

                                        if let Some(editor_entity) = &this.active_editor {
                                            editor_entity.update(cx, |editor, cx| {
                                                editor.set_kaitai_definition(ksy_arc.clone());
                                                cx.notify();
                                            });
                                        }

                                        this.left_panel.update(cx, |p, cx| {
                                            if let Some(editor_entity) = &this.active_editor {
                                                p.set_editor(Some(editor_entity.clone()), cx);
                                            }
                                            p.set_tab(crate::ui::panels::left_panel::LeftPanelTab::Structure, cx);
                                        });
                                        this.is_left_panel_visible = true;
                                        cx.notify();
                                    });
                                })
                                .ok();
                        }
                        Err(e) => {
                            eprintln!("Failed to parse KSY definition: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read KSY file at {:?}: {}", path, e);
                    }
                }
            }
        })
        .detach();
    }

    fn on_action_clear_structure_definition(&mut self, _: &ClearStructureDefinition, _: &mut Window, cx: &mut Context<Self>) {
        self.ksy_definition = None;
        if let Some(editor_entity) = self.active_editor.as_ref() {
            editor_entity.update(cx, |editor, cx| {
                editor.clear_structure_definition();
                cx.notify();
            });
            self.left_panel.update(cx, |p, cx| {
                p.set_editor(Some(editor_entity.clone()), cx);
            });
        }
        cx.notify();
    }

    fn on_action_open_folder(&mut self, _: &OpenFolder, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select a folder".into()),
        });

        let left_panel = self.left_panel.clone();
        cx.spawn_in(window, async move |_, window| {
            if let Some(path) = path.await.ok().and_then(|r| r.ok()).flatten().and_then(|mut p| p.pop()) {
                window
                    .update(|_, cx| {
                        left_panel.update(cx, |p, cx| {
                            p.file_tree.update(cx, |ft, cx| {
                                ft.set_root_path(path, cx);
                            });
                        });
                    })
                    .ok();
            }
        })
        .detach();
    }

    fn on_action_close_folder(&mut self, _: &CloseFolder, _: &mut Window, cx: &mut Context<Self>) {
        self.left_panel.update(cx, |p, cx| {
            p.file_tree.update(cx, |ft, cx| {
                ft.close_folder(cx);
            });
        });
    }

    fn on_action_close_active_panel(&mut self, _: &CloseActivePanel, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(panel) = self.active_panel.take() {
            self.dock_area.update(cx, |dock_area, cx| {
                dock_area.remove_panel_from_all_docks(panel, window, cx);
            });
            if !self.check_has_panels(cx) {
                let weak_dock_area = self.dock_area.downgrade();
                Self::reset_default_layout(weak_dock_area, window, cx);
            }
            cx.notify();
        }
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
        let settings_panel_clone = settings_panel.clone();
        cx.on_focus_in(&settings_panel.read(cx).focus_handle(cx), window, {
            let settings_panel_clone = settings_panel_clone.clone();
            move |this, _, cx| {
                this.active_editor = None;
                this.status_bar.update(cx, |status_bar, _| status_bar.set_active_editor(None));
                this.left_panel.update(cx, |panel, cx| {
                    panel.set_editor(None, cx);
                });
                this.active_panel = Some(Arc::new(settings_panel_clone.clone()));
                this.on_focus_changed(cx);
                cx.notify();
            }
        })
        .detach();
        let panel = Arc::new(settings_panel);
        self.add_panel_to_center_dock(panel, window, cx);
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

    fn on_action_open_visual_map(&mut self, _: &OpenVisualMap, window: &mut Window, cx: &mut Context<Self>) {
        self.select_activity(Activity::Map, window, cx);
    }

    fn add_panel_to_center_dock(&self, panel: Arc<dyn PanelView>, window: &mut Window, cx: &mut Context<Self>) {
        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(panel.clone(), DockPlacement::Center, None, window, cx);
            let mut items = dock_area.items().clone();
            if Self::add_panel_to_dock_item_tree(&mut items, panel) {
                dock_area.set_center(items, window, cx);
            }
        });
    }

    fn add_panel_to_dock_item_tree(dock_item: &mut DockItem, panel: Arc<dyn PanelView>) -> bool {
        match dock_item {
            DockItem::Tabs { items, .. } => {
                if !items.iter().any(|item| item.view().entity_id() == panel.view().entity_id()) {
                    items.push(panel);
                    return true;
                }
                false
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if Self::add_panel_to_dock_item_tree(item, panel.clone()) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn find_existing_panel_and_tab_panel(&self, path: &std::path::Path, cx: &App) -> Option<(Entity<TabPanel>, Arc<dyn PanelView>)> {
        let dock_area = self.dock_area.read(cx);
        Self::find_panel_and_tab_panel_in_items(dock_area.items(), path, cx)
    }

    fn find_panel_and_tab_panel_in_items(dock_item: &DockItem, path: &std::path::Path, cx: &App) -> Option<(Entity<TabPanel>, Arc<dyn PanelView>)> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        match dock_item {
            DockItem::Tabs { items, view, .. } => {
                for item in items {
                    if let Ok(panel) = item.view().downcast::<EditorPanel>() {
                        let panel_path = panel.read(cx).path(cx);
                        let canonical_panel_path = panel_path.canonicalize().unwrap_or(panel_path.clone());
                        if canonical_panel_path == canonical_path {
                            return Some((view.clone(), item.clone()));
                        }
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if let Some(res) = Self::find_panel_and_tab_panel_in_items(item, path, cx) {
                        return Some(res);
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
        Self::has_panels_recursive(dock_area.items(), cx)
    }

    fn has_panels_recursive(dock_item: &DockItem, cx: &App) -> bool {
        match dock_item {
            DockItem::Tabs { view, .. } => {
                let state = view.read(cx).dump(cx);
                for child in state.children {
                    if child.panel_name == "EditorPanel" {
                        return true;
                    }
                    if child.panel_name == "DiffPanel" {
                        return true;
                    }
                    if child.panel_name == "SettingsPanel" {
                        return true;
                    }
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    if Self::has_panels_recursive(item, cx) {
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
                let state = view.dump(cx);
                if state.panel_name == "EditorPanel" {
                    return true;
                }
                if state.panel_name == "DiffPanel" {
                    return true;
                }
                if state.panel_name == "SettingsPanel" {
                    return true;
                }
            }
        }
        false
    }

    fn on_focus_changed(&self, cx: &mut Context<Self>) {
        self.left_panel.update(cx, |panel, cx| {
            panel.file_tree.update(cx, |_, cx| cx.notify());
            panel.struct_tree.update(cx, |_, cx| cx.notify());
            panel.data_inspector.update(cx, |_, cx| cx.notify());
            panel.visual_map.update(cx, |_, cx| cx.notify());
        });

        // Clone the item to release the immutable borrow on cx
        let item = self.dock_area.read(cx).items().clone();
        Self::notify_panels_recursive(&item, cx);
    }

    fn notify_panels_recursive(item: &gpui_component::dock::DockItem, cx: &mut Context<Self>) {
        match item {
            gpui_component::dock::DockItem::Tabs { items, .. } => {
                for panel in items {
                    if let Ok(p) = panel.view().downcast::<EditorPanel>() {
                        let _ = p.update(cx, |_, cx| cx.notify());
                    } else if let Ok(p) = panel.view().downcast::<crate::ui::panels::diff_panel::DiffPanel>() {
                        let _ = p.update(cx, |_, cx| cx.notify());
                    } else if let Ok(p) = panel.view().downcast::<crate::ui::panels::settings_panel::SettingsPanel>() {
                        let _ = p.update(cx, |_, cx| cx.notify());
                    }
                }
            }
            gpui_component::dock::DockItem::Split { items, .. } => {
                for sub_item in items {
                    Self::notify_panels_recursive(sub_item, cx);
                }
            }
            gpui_component::dock::DockItem::Panel { view, .. } => {
                if let Ok(p) = view.view().downcast::<EditorPanel>() {
                    let _ = p.update(cx, |_, cx| cx.notify());
                } else if let Ok(p) = view.view().downcast::<crate::ui::panels::diff_panel::DiffPanel>() {
                    let _ = p.update(cx, |_, cx| cx.notify());
                } else if let Ok(p) = view.view().downcast::<crate::ui::panels::settings_panel::SettingsPanel>() {
                    let _ = p.update(cx, |_, cx| cx.notify());
                }
            }
            _ => {}
        }
    }

    fn sync_activity_bar(&self, cx: &mut Context<Self>) {
        let is_visible = self.is_left_panel_visible;
        let active_tab = self.left_panel.read(cx).active_tab;
        self.activity_bar.update(cx, |activity_bar, cx| {
            if is_visible {
                match active_tab {
                    LeftPanelTab::Files => activity_bar.set_activity(Some(Activity::Files), cx),
                    LeftPanelTab::Structure => activity_bar.set_activity(Some(Activity::Structure), cx),
                    LeftPanelTab::Inspector => activity_bar.set_activity(Some(Activity::Inspector), cx),
                    LeftPanelTab::Map => activity_bar.set_activity(Some(Activity::Map), cx),
                }
            } else {
                activity_bar.set_activity(None, cx);
            }
        });
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .on_action(cx.listener(Self::on_action_open_file))
            .on_action(cx.listener(Self::on_action_close_active_panel))
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
            .on_action(cx.listener(Self::on_action_toggle_left_panel))
            .on_action(cx.listener(Self::on_action_open_settings))
            .on_action(cx.listener(Self::on_action_open_visual_map))
            .on_action(cx.listener(Self::on_action_show_files_tab))
            .on_action(cx.listener(Self::on_action_show_structure_tab))
            .on_action(cx.listener(Self::on_action_load_structure_definition))
            .on_action(cx.listener(Self::on_action_clear_structure_definition))
            .on_action(cx.listener(Self::on_action_open_folder))
            .on_action(cx.listener(Self::on_action_close_folder))
            .on_drop(cx.listener(move |this, external_paths: &gpui::ExternalPaths, window, cx| {
                for path in external_paths.paths() {
                    if path.is_file() {
                        let action = crate::actions::OpenFile {
                            path: path.to_string_lossy().to_string(),
                        };
                        this.on_action_open_file(&action, window, cx);
                    } else if path.is_dir() {
                        this.is_left_panel_visible = true;
                        this.left_panel.update(cx, |p, cx| {
                            p.set_tab(crate::ui::panels::left_panel::LeftPanelTab::Files, cx);
                            p.file_tree.update(cx, |ft, cx| {
                                ft.set_root_path(path.clone(), cx);
                            });
                        });
                        this.sync_activity_bar(cx);
                    }
                }
                cx.notify();
            }))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.activity_bar.clone())
                    .child(
                        h_resizable("workspace-h-resize")
                            .child(
                                resizable_panel()
                                    .visible(self.is_left_panel_visible)
                                    .size(px(250.))
                                    .child(self.left_panel.clone()),
                            )
                            .child(
                                resizable_panel().child(div().relative().size_full().flex().flex_col().child(self.dock_area.clone()).when(
                                    !self.check_has_panels(cx),
                                    |this| {
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
                    ),
            )
            .child(self.status_bar.clone())
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

#[allow(dead_code)]
pub struct TabPanelMirror {
    focus_handle: gpui::FocusHandle,
    dock_area: gpui::WeakEntity<gpui_component::dock::DockArea>,
    stack_panel: Option<gpui::WeakEntity<gpui_component::dock::StackPanel>>,
    pub panels: Vec<std::sync::Arc<dyn gpui_component::dock::PanelView>>,
    pub active_ix: usize,
    pub closable: bool,
    tab_bar_scroll_handle: gpui::ScrollHandle,
    zoomed: bool,
    collapsed: bool,
    will_split_placement: Option<gpui_component::Placement>,
    in_tiles: bool,
}

impl Workspace {
    fn activate_tab_via_mirror(tab_panel_entity: &Entity<TabPanel>, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        tab_panel_entity.update(cx, |tab_panel, cx| {
            let mirror = unsafe { &mut *(tab_panel as *mut TabPanel as *mut TabPanelMirror) };

            if ix == mirror.active_ix {
                if let Some(active_panel) = mirror.panels.get(ix) {
                    active_panel.focus_handle(cx).focus(window);
                }
                return;
            }

            let last_active_ix = mirror.active_ix;
            mirror.active_ix = ix;
            mirror.tab_bar_scroll_handle.scroll_to_item(ix);

            if let Some(active_panel) = mirror.panels.get(ix) {
                active_panel.focus_handle(cx).focus(window);
            }

            cx.spawn_in(window, async move |view, cx| {
                _ = cx.update(|window, cx| {
                    _ = view.update(cx, |view, cx| {
                        let mirror = unsafe { &mut *(view as *mut TabPanel as *mut TabPanelMirror) };
                        if let Some(last_active) = mirror.panels.get(last_active_ix) {
                            last_active.set_active(false, window, cx);
                        }
                        if let Some(active) = mirror.panels.get(mirror.active_ix) {
                            active.set_active(true, window, cx);
                        }
                    });
                });
            })
            .detach();

            cx.emit(gpui_component::dock::PanelEvent::LayoutChanged);
            cx.notify();
        });
    }
}
