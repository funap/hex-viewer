use gpui::*;
use gpui_component::dock::{DockArea, Panel, PanelEvent};
use gpui_component::dock::DockPlacement;
use gpui::Bounds; // Boundsをgpui::geometry::Boundsに変更

use crate::app_title_bar::AppTitleBar;
use crate::file_tree_panel::{FileTreePanel, OpenFile};
use crate::editor_panel::EditorPanel;
use gpui_component::input::InputState; // InputStateをインポート
use std::path::PathBuf;
use std::sync::Arc; // Arcをインポート

// ...

impl Workspace {
    // ...

    fn on_action_open_file(
        &mut self,
        action: &OpenFile, // &OpenFileに戻す
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let file_path = &action.path; // &action.pathに変更
        let file_name = PathBuf::from(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        // ファイルの内容を読み込む
        let content = std::fs::read_to_string(file_path).unwrap_or_else(|_| "".to_string());

        // InputStateを作成し、ファイルの内容を設定する
        let editor_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(content)
        });

        // EditorPanelを作成する
        let editor_panel = cx.new(|cx| EditorPanel::new(file_name, editor_input_state, cx));

use std::sync::Arc; // Arcをインポート

// ...

        // DockAreaにEditorPanelを追加する
        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(Arc::new(editor_panel), DockPlacement::Center, None, window, cx); // 引数を修正
        });
    }
}

pub struct Workspace {
    pub dock_area: Entity<DockArea>,
    pub title_bar: Entity<AppTitleBar>,
}

impl Render for Workspace {
    /// タブバーと、選択されたタブに基づいて内容が変化するコンテンツエリア、およびボタンを含む
    /// `Workspace`コンポーネントをレンダリングします。
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_col()
            .size_full()
            .child(self.title_bar.clone())
            .child(
                div()
                    .flex_row() // Arrange horizontally
                    .size_full()
                    .child(self.dock_area.clone()) // Dock area on the right
            .on_action(cx.listener(Self::on_action_open_file))
            )
    }
}
