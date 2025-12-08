use gpui::{App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render, SharedString, Window, div};
use gpui_component::dock::{Panel, PanelEvent};

/// A simple panel for demonstration purposes.
#[allow(dead_code)]
pub struct SettingsPanel {
    title: SharedString,
    focus_handle: FocusHandle,
}

impl SettingsPanel {
    #[allow(dead_code)]
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            title: title.into(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(format!("Content for {}", self.title))
    }
}

impl EventEmitter<PanelEvent> for SettingsPanel {}

impl Focusable for SettingsPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for SettingsPanel {
    fn panel_name(&self) -> &'static str {
        "SettingsPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {}
}
