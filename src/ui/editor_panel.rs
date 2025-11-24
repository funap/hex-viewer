use crate::data::file_buffer::FileBuffer;
use gpui::prelude::*;
use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use std::sync::Arc;

use crate::ui::component::hex_view::{self, HexView};

const CONTEXT: &str = "EditorPanel";

pub(crate) fn init(cx: &mut App) {
    // Initialize HexView actions and keybindings
    hex_view::init(cx);
}

pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    focus_handle: FocusHandle,
    hex_view: Entity<HexView>,
}

impl EditorPanel {
    pub fn new(buffer: Arc<FileBuffer>, cx: &mut Context<Self>) -> Self {
        let hex_view = cx.new(|cx| HexView::new(cx).buffer(buffer.clone()));
        Self {
            buffer,
            focus_handle: cx.focus_handle(),
            hex_view,
        }
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}

impl Focusable for EditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        "EditorPanel"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        let title = self
            .buffer
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "(untitled)".to_string());
        title.into_any_element()
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

impl Render for EditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .child(self.hex_view.clone())
    }
}
