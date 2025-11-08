use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div,
};
use gpui_component::{TitleBar, button::Button, menu::AppMenuBar, IconName};
use gpui_component::button::ButtonVariants;

pub struct AppTitleBar {
    pub app_menu_bar: Entity<AppMenuBar>,
}

impl Render for AppTitleBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(self.app_menu_bar.clone())
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("settings")
                            .ghost()
                            .icon(IconName::Settings)
                    )
                    .child(
                        Button::new("help")
                            .ghost()
                    )
            )
    }
}
