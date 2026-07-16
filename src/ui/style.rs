use gpui::{Div, Hsla, ParentElement, Styled, div, px};
use gpui_component::theme::Theme;

pub trait StyleExt {
    fn focus_indicator(self, focused: bool, theme: &Theme) -> Div;
}

impl StyleExt for Div {
    fn focus_indicator(self, focused: bool, theme: &Theme) -> Div {
        self.relative().child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(1.0))
                .bg(if focused { theme.accent } else { theme.accent.opacity(0.0) }),
        )
    }
}

/// Returns the header text color based on the focus state.
/// When focused, it returns `theme.foreground`. When not focused, it returns `theme.muted_foreground`.
pub fn header_text_color(focused: bool, theme: &Theme) -> Hsla {
    if focused { theme.foreground } else { theme.muted_foreground }
}
