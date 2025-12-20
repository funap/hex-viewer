use crate::appearance::Appearance;
use gpui::prelude::*;
use gpui::{Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render, Subscription, Window, div, px};
use gpui_component::{
    ActiveTheme, Icon, IconName, PixelsExt,
    dock::{Panel, PanelEvent},
    h_flex,
    input::{self, Input, InputState},
};

#[derive(Clone, PartialEq, Action)]
pub struct UpdateSettingInput;

pub struct SettingsPanel {
    focus_handle: FocusHandle,
    font_family_input: Entity<InputState>,
    font_size_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let font_family_input = cx.new(|cx| InputState::new(window, cx));
        let font_size_input = cx.new(|cx| InputState::new(window, cx));

        // The global must be retrieved after the entities have been created.
        let (family, size) = {
            let appearance = cx.global::<Appearance>();
            (appearance.font_family.to_string(), appearance.font_size.as_f32().to_string())
        };

        // Set initial values
        font_family_input.update(cx, |input: &mut InputState, cx| {
            input.set_value(family, window, cx);
        });

        font_size_input.update(cx, |input: &mut InputState, cx| {
            input.set_value(size, window, cx);
        });

        let mut subscriptions = Vec::new();

        subscriptions.push(cx.observe_global::<Appearance>(|_, cx| {
            cx.dispatch_action(&UpdateSettingInput);
        }));

        subscriptions.push(cx.subscribe(&font_family_input, |_, input: Entity<InputState>, event: &input::InputEvent, cx| {
            if let input::InputEvent::Change = event {
                let value = input.read(cx).value().to_string();
                cx.update_global::<Appearance, _>(|appearance, _| {
                    appearance.font_family = value.into();
                });
            }
        }));

        subscriptions.push(cx.subscribe(&font_size_input, |_, input: Entity<InputState>, event: &input::InputEvent, cx| {
            if let input::InputEvent::Change = event {
                let value = input.read(cx).value().to_string();
                if let Ok(size) = value.parse::<f32>() {
                    cx.update_global::<Appearance, _>(|appearance, _| {
                        appearance.font_size = px(size);
                    });
                }
            }
        }));

        Self {
            focus_handle,
            font_family_input,
            font_size_input,
            _subscriptions: subscriptions,
        }
    }

    fn on_action_update_setting_input(&mut self, _action: &UpdateSettingInput, window: &mut Window, cx: &mut Context<Self>) {
        let appearance = cx.global::<Appearance>();
        let family = appearance.font_family.to_string();
        let size_str = appearance.font_size.as_f32().to_string();

        self.font_family_input.update(cx, |input, cx| {
            if input.value() != family.as_str() {
                input.set_value(family, window, cx);
            }
        });
        self.font_size_input.update(cx, |input, cx| {
            if input.value() != size_str.as_str() {
                input.set_value(size_str, window, cx);
            }
        });
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .on_action(cx.listener(Self::on_action_update_setting_input))
            .child(div().child("Editor").font_weight(gpui::FontWeight::BOLD).mb_2())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().child("Font Family"))
                    .child(div().w_48().child(Input::new(&self.font_family_input))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().child("Font Size"))
                    .child(div().w_48().child(Input::new(&self.font_size_input))),
            )
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
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex().gap_2().items_center().child("Settings").child(
            div()
                .id("close-icon")
                .cursor_pointer()
                .rounded_md()
                .hover(|style| style.bg(theme.accent).text_color(theme.accent_foreground))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus_handle(cx).focus(window);
                    window.dispatch_action(Box::new(gpui_component::dock::ClosePanel), cx);
                }))
                .child(Icon::new(IconName::Close).size(px(14.0))),
        )
    }
    fn closable(&self, _: &App) -> bool {
        true
    }
    fn zoomable(&self, _: &App) -> Option<gpui_component::dock::PanelControl> {
        None
    }
    fn visible(&self, _: &App) -> bool {
        true
    }
    fn set_active(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) {}
    fn set_zoomed(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) {}
}
