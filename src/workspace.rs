use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div,
};
use gpui_component::dock::DockArea;

use crate::app_title_bar::AppTitleBar;

/// `Workspace`コンポーネントの主要なアプリケーション状態。
///
/// この構造体は、現在選択されているタブの状態を保持します。
pub struct Workspace {
    pub dock_area: Entity<DockArea>,
    pub title_bar: Entity<AppTitleBar>,
}

impl Render for Workspace {
    /// タブバーと、選択されたタブに基づいて内容が変化するコンテンツエリア、およびボタンを含む
    /// `Workspace`コンポーネントをレンダリングします。
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_col()
            .size_full()
            .child(self.title_bar.clone())
            .child(self.dock_area.clone())
    }
}
