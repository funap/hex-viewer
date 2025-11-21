use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, SharedString,
    Window,
};
use gpui_component::dock::{Panel, PanelEvent};
use gpui_component::input::{Input, InputState};

/// A panel for displaying a code editor.
#[allow(dead_code)]
pub struct Pane {
    title: SharedString,
    editor: Entity<InputState>,
    focus_handle: FocusHandle,
}

impl Pane {
    #[allow(dead_code)]
    pub fn new(
        title: impl Into<SharedString>,
        editor: Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
            editor,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for Pane {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.editor).h_full() // Full height
    }
}

impl EventEmitter<PanelEvent> for Pane {}

impl Focusable for Pane {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for Pane {
    fn panel_name(&self) -> &'static str {
        "Pane"
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
}
