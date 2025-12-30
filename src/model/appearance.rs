use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Appearance {
    pub font_family: String,
    pub font_size: f32,
}

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
            font_size: 14.0,
        }
    }
}
