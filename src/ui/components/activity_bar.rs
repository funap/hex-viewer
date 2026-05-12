use crate::actions::{ShowFilesTab, ShowStructureTab};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{ActiveTheme, Icon, IconName};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Activity {
    Files,
    Structure,
}

pub enum ActivityBarEvent {
    Select(Activity),
}

pub struct ActivityBar {
    pub active_activity: Option<Activity>,
}

impl EventEmitter<ActivityBarEvent> for ActivityBar {}

impl ActivityBar {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            active_activity: Some(Activity::Files),
        }
    }

    pub fn set_activity(&mut self, activity: Option<Activity>, cx: &mut Context<Self>) {
        if self.active_activity != activity {
            self.active_activity = activity;
            cx.notify();
        }
    }
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .flex()
            .flex_col()
            .w(px(42.0))
            .h_full()
            .bg(theme.background)
            .border_r_1()
            .border_color(theme.border)
            .items_center()
            .py_4()
            .gap_2()
            .child(self.render_icon(Activity::Files, IconName::File, "Files", cx))
            .child(self.render_icon(Activity::Structure, IconName::Search, "Structure", cx))
    }
}

impl ActivityBar {
    fn render_icon(&self, activity: Activity, icon: IconName, _tooltip: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_active = self.active_activity == Some(activity);

        div()
            .id(("activity", activity as u32))
            .cursor_pointer()
            .p_2()
            .text_color(if is_active { theme.foreground } else { theme.muted_foreground })
            .relative()
            .hover(|style| style.text_color(theme.foreground))
            .on_click(cx.listener(move |_, _, _window, cx| {
                cx.emit(ActivityBarEvent::Select(activity));
            }))
            .child(Icon::new(icon).size(px(24.0)))
            .when(is_active, |this| {
                this.child(div().absolute().left_0().top_2().bottom_2().w_0p5().bg(theme.accent))
            })
    }
}
