use gpui::{Div, Hsla, ParentElement, Styled, div, px};
use gpui_component::theme::Theme;

/// Returns the border color based on the focus state.
/// When focused, it returns `theme.accent`. When not focused, it returns a transparent border (or transparent color).
#[allow(dead_code)]
pub fn focus_border_color(focused: bool, theme: &Theme) -> Hsla {
    if focused {
        theme.accent
    } else {
        // Use transparent color so the layout does not shift
        theme.border.opacity(0.0)
    }
}

/// Applies a focus indicator (2px left border-like line) to a container.
#[allow(dead_code)]
pub fn apply_focus_indicator(element: Div, focused: bool, theme: &Theme) -> Div {
    if focused {
        element
            .relative()
            .child(div().absolute().left_0().top_0().bottom_0().w(px(2.0)).bg(theme.accent))
    } else {
        element
    }
}

/// Returns the header text color based on the focus state.
/// When focused, it returns `theme.foreground`. When not focused, it returns `theme.muted_foreground`.
pub fn header_text_color(focused: bool, theme: &Theme) -> Hsla {
    if focused { theme.foreground } else { theme.muted_foreground }
}
