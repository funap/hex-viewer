use gpui::*;
use gpui_component::dock::*;

use crate::app_title_bar::AppTitleBar;


// ...

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
            .child(
                div()
                    .flex_row() // Arrange horizontally
                    .size_full()
                    .child(self.dock_area.clone()), // Dock area on the right
            )
    }
}
