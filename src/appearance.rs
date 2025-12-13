use gpui::{App, Global, Pixels, SharedString};

#[derive(Clone)]
pub struct Appearance {
    pub font_family: SharedString,
    pub font_size: Pixels,
}

impl Global for Appearance {}

impl Appearance {
    pub fn default() -> Self {
        let font_family = if cfg!(target_os = "macos") {
            "Menlo"
        } else if cfg!(target_os = "windows") {
            "Consolas"
        } else {
            "DejaVu Sans Mono"
        };

        Self {
            font_family: font_family.into(),
            font_size: gpui::px(14.0),
        }
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(Appearance::default());
}
