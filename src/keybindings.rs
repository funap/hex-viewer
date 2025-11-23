use gpui::KeyBinding;
use gpui_component::dock::{ClosePanel, ToggleZoom};

pub fn init(cx: &mut gpui::App) {
    cx.bind_keys(vec![
        KeyBinding::new("shift-escape", ToggleZoom, None),
        KeyBinding::new("ctrl-w", ClosePanel, None),
    ]);

    cx.activate(true);
}
