use crate::actions::{
    CloseFolder, LoadChildren, OpenDiff, OpenFile, OpenFolder, Rename, SelectItem,
};
use std::collections::HashSet;
use std::path::PathBuf;

use autocorrect::ignorer::Ignorer;
use gpui::{
    App, AppContext, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString, Styled,
    WeakEntity, Window, div, px,
};

use gpui_component::{
    ActiveTheme as _, IconName, StyledExt as _,
    dock::{Panel, PanelEvent},
    h_flex,
    label::Label,
    list::ListItem,
    menu::ContextMenuExt,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};

const CONTEXT: &str = "TreeStory";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Rename, Some(CONTEXT)),
        KeyBinding::new("space", SelectItem, Some(CONTEXT)),
    ]);

    gpui_component::dock::register_panel(cx, "FileTreePanel", |_, _, info, _window, cx| {
        let state = match info {
            gpui_component::dock::PanelInfo::Panel(value) => {
                FileTreePanelState::from_value(value.clone())
            }
            _ => None,
        };

        let view = cx.new(|cx| {
            let mut panel = FileTreePanel::new("File Tree", cx);
            if let Some(state) = state {
                if let Some(path) = state.root_path {
                    panel.set_root_path(path, cx);
                }
            }
            panel
        });
        Box::new(view)
    });
}

pub struct FileTreePanel {
    tree_state: Entity<TreeState>,
    selected_item: Option<TreeItem>,
    selected_items: Vec<TreeItem>, // 複数選択用
    title: SharedString,
    focus_handle: FocusHandle,
    root_path: Option<PathBuf>,
    loaded_paths: HashSet<String>,
    items: Vec<TreeItem>,
}

fn build_file_items(ignorer: &Ignorer, root: &PathBuf, path: &PathBuf) -> Vec<TreeItem> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            if ignorer.is_ignored(&relative_path.to_string_lossy())
                || relative_path.ends_with(".git")
            {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let id = path.to_string_lossy().to_string();
            if path.is_dir() {
                items.push(
                    TreeItem::new(id, file_name).child(TreeItem::new("loading", "Loading...")),
                );
            } else {
                items.push(TreeItem::new(id, file_name));
            }
        }
    }
    items.sort_by(|a, b| {
        b.is_folder()
            .cmp(&a.is_folder())
            .then(a.label.cmp(&b.label))
    });
    items
}

fn update_item_children_recursive(
    items: &mut Vec<TreeItem>,
    target_id: &str,
    children: Vec<TreeItem>,
) -> bool {
    for item in items.iter_mut() {
        if item.id == target_id {
            item.children = children;
            return true;
        }
        if item.is_folder() {
            if update_item_children_recursive(&mut item.children, target_id, children.clone()) {
                return true;
            }
        }
    }
    false
}

impl FileTreePanel {
    // Renamed from TreeStory
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));

        let this = Self {
            tree_state: tree_state.clone(),
            selected_item: None,
            selected_items: Vec::new(),
            title: title.into(),
            focus_handle: cx.focus_handle(),
            root_path: None,
            loaded_paths: HashSet::new(),
            items: Vec::new(),
        };

        this
    }

    fn load_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(|view: WeakEntity<FileTreePanel>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let ignorer = Ignorer::new(&path.to_string_lossy());
                let items = build_file_items(&ignorer, &path, &path);

                view.update(&mut cx, |this, cx| {
                    this.items = items.clone();
                    this.tree_state.update(cx, |state, cx| {
                        state.set_items(items, cx);
                    });
                })
                .ok();
            }
        })
        .detach();
    }

    fn load_children(&mut self, item_id: &str, cx: &mut Context<Self>) {
        if self.loaded_paths.contains(item_id) {
            return;
        }

        let path = PathBuf::from(item_id);
        if path.is_dir() {
            let item_id_clone = item_id.to_string();
            let root_path = self.root_path.clone();

            cx.spawn(|view: WeakEntity<FileTreePanel>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    if let Some(root_path) = root_path {
                        let ignorer = Ignorer::new(&root_path.to_string_lossy());
                        let children =
                            build_file_items(&ignorer, &root_path, &PathBuf::from(&item_id_clone));

                        view.update(&mut cx, |this, cx| {
                            if update_item_children_recursive(
                                &mut this.items,
                                &item_id_clone,
                                children,
                            ) {
                                this.tree_state.update(cx, |state, cx| {
                                    state.set_items(this.items.clone(), cx);
                                });
                            }
                        })
                        .ok();
                    }
                }
            })
            .detach();

            self.loaded_paths.insert(item_id.to_string());
        }
    }

    fn on_action_select_item(
        &mut self,
        _: &SelectItem,
        _: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            self.selected_item = Some(entry.item().clone());
            cx.notify();
        }
    }

    fn on_action_rename(&mut self, _: &Rename, _: &mut Window, cx: &mut gpui::Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            println!("Renaming item: {} ({})", item.label, item.id);
        }
    }

    fn on_action_open_folder(
        &mut self,
        _: &OpenFolder,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let path = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select a folder".into()),
        });

        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??.iter().next()?.clone();

            window
                .update(|_window, cx| {
                    view.update(cx, |this, cx| {
                        this.root_path = Some(path.clone());
                        this.loaded_paths.clear();
                        this.loaded_paths.insert(path.to_string_lossy().to_string());
                        this.load_root(path, cx);
                    });
                })
                .ok()
        })
        .detach();
    }

    fn on_action_close_folder(
        &mut self,
        _: &CloseFolder,
        _: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.root_path = None;
        self.loaded_paths.clear();
        self.items.clear();
        self.tree_state.update(cx, |state, cx| {
            state.set_items(vec![], cx);
        });
        cx.notify();
    }

    pub fn set_root_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.root_path = Some(path.clone());
        self.loaded_paths.clear();
        self.loaded_paths.insert(path.to_string_lossy().to_string());
        self.load_root(path, cx);
        cx.notify();
    }

    fn on_action_set_file_tree_folder(
        &mut self,
        action: &crate::actions::SetFileTreeFolder,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = PathBuf::from(&action.path);
        self.set_root_path(path, cx);
    }

    fn on_action_load_children(
        &mut self,
        action: &LoadChildren,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.load_children(&action.path, cx);
    }

    fn toggle_selection(&mut self, item: TreeItem, cx: &mut Context<Self>) {
        if let Some(pos) = self.selected_items.iter().position(|i| i.id == item.id) {
            self.selected_items.remove(pos);
        } else {
            self.selected_items.push(item);
        }
        cx.notify();
    }
}

impl Render for FileTreePanel {
    // Renamed from TreeStory
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let view = cx.entity();

        if self.root_path.is_none() {
            return v_flex()
                .id("file-tree-panel")
                .key_context(CONTEXT)
                .on_action(cx.listener(Self::on_action_open_folder))
                .size_full()
                .justify_center()
                .items_center()
                .child(
                    div().child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.on_action_open_folder(&OpenFolder, window, cx);
                                }),
                            )
                            .id("open-folder-btn")
                            .p_2()
                            .bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                            .rounded_md()
                            .cursor_pointer()
                            .child("Open Folder"),
                    ),
                );
        }

        v_flex()
            .id("file-tree-panel")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_rename))
            .on_action(cx.listener(Self::on_action_select_item))
            .on_action(cx.listener(Self::on_action_close_folder))
            .on_action(cx.listener(Self::on_action_set_file_tree_folder))
            .on_action(cx.listener(Self::on_action_load_children))
            .gap_5()
            .size_full()
            .child(
                div()
                    .v_flex()
                    .child(
                        tree(&self.tree_state, move |ix, entry, _selected, window, cx| {
                            let item = entry.item();
                            let icon = if !entry.is_folder() {
                                IconName::File
                            } else if entry.is_expanded() {
                                IconName::FolderOpen
                            } else {
                                IconName::Folder
                            };

                            if entry.is_expanded() && entry.is_folder() {
                                let item_id = item.id.to_string();
                                window.dispatch_action(
                                    Box::new(crate::actions::LoadChildren { path: item_id }),
                                    cx,
                                );
                            }

                            ListItem::new(ix)
                                .w_full()
                                .rounded(cx.theme().radius)
                                .px_3()
                                .pl(px(16.) * entry.depth() + px(12.))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(icon)
                                        .child(item.label.clone())
                                        .size_full()
                                        .context_menu({
                                            let view = view.clone();
                                            let item_id = item.id.clone();
                                            move |menu, _window, cx| {
                                                let (can_compare, left_path, right_path) = view
                                                    .update(cx, |this, _cx| {
                                                        let can_compare = this.selected_items.len()
                                                            == 2
                                                            && this
                                                                .selected_items
                                                                .iter()
                                                                .all(|item| !item.is_folder());
                                                        if can_compare {
                                                            (
                                                                true,
                                                                Some(
                                                                    this.selected_items[0]
                                                                        .id
                                                                        .to_string(),
                                                                ),
                                                                Some(
                                                                    this.selected_items[1]
                                                                        .id
                                                                        .to_string(),
                                                                ),
                                                            )
                                                        } else {
                                                            (false, None, None)
                                                        }
                                                    });

                                                let mut menu = menu
                                                    .menu_with_icon(
                                                        "Open",
                                                        IconName::FolderOpen,
                                                        Box::new(OpenFile {
                                                            path: item_id.to_string(),
                                                        }),
                                                    )
                                                    .separator();

                                                if can_compare {
                                                    menu = menu.menu_with_icon(
                                                        "Compare Files",
                                                        IconName::Search,
                                                        Box::new(OpenDiff {
                                                            left_path: left_path
                                                                .unwrap_or_default(),
                                                            right_path: right_path
                                                                .unwrap_or_default(),
                                                        }),
                                                    );
                                                } else {
                                                    menu = menu.menu_with_icon_and_disabled(
                                                        "Compare Files",
                                                        IconName::Search,
                                                        Box::new(OpenDiff {
                                                            left_path: String::new(),
                                                            right_path: String::new(),
                                                        }),
                                                        true,
                                                    );
                                                }

                                                menu.separator().menu("Rename", Box::new(Rename))
                                            }
                                        }),
                                )
                                .on_click(window.listener_for(&view, {
                                    let item = item.clone();
                                    move |this, event: &gpui::ClickEvent, window, cx| {
                                        // Ctrl/Cmdキーで複数選択
                                        if event.modifiers().control || event.modifiers().platform {
                                            this.toggle_selection(item.clone(), cx);
                                        } else {
                                            this.selected_items = vec![item.clone()];
                                            this.selected_item = Some(item.clone());
                                        }

                                        // ファイルを開く処理（単一選択時のみ）
                                        if !item.is_folder() && this.selected_items.len() == 1 {
                                            println!(
                                                "Dispatching OpenFile action for path: {}",
                                                item.id
                                            );
                                            cx.focus_self(window);
                                            window.dispatch_action(
                                                Box::new(OpenFile {
                                                    path: item.id.to_string(),
                                                }),
                                                cx,
                                            );
                                        }
                                        cx.notify();
                                    }
                                }))
                        })
                        .p_1()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .h(px(540.)),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .gap_3()
                            .children(
                                self.tree_state
                                    .read(cx)
                                    .selected_index()
                                    .map(|ix| format!("Selected Index: {}", ix)),
                            )
                            .children(
                                self.selected_item
                                    .as_ref()
                                    .map(|item| Label::new("Selected:").secondary(item.id.clone())),
                            ),
                    ),
            )
    }
}

impl EventEmitter<PanelEvent> for FileTreePanel {}

impl Focusable for FileTreePanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for FileTreePanel {
    fn panel_name(&self) -> &'static str {
        "FileTreePanel"
    }

    fn title(&self, _window: &Window, _cx: &App) -> gpui::AnyElement {
        self.title.clone().into_any_element()
    }

    fn closable(&self, _cx: &App) -> bool {
        true
    }

    fn zoomable(&self, _cx: &App) -> Option<gpui_component::dock::PanelControl> {
        Some(gpui_component::dock::PanelControl::Both)
    }

    fn visible(&self, _cx: &App) -> bool {
        true
    }

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut App) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut App) {}

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        let mut state = gpui_component::dock::PanelState::new(self);
        let panel_state = FileTreePanelState {
            root_path: self.root_path.clone(),
        };
        state.info = gpui_component::dock::PanelInfo::panel(panel_state.to_value());
        state
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileTreePanelState {
    pub root_path: Option<PathBuf>,
}

impl FileTreePanelState {
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    pub fn from_value(value: serde_json::Value) -> Option<Self> {
        serde_json::from_value(value).ok()
    }
}
