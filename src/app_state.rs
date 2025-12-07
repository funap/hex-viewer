use crate::service::editor_service::EditorService;
use crate::ui::status_bar::EditorStatus;
use gpui::prelude::*;
use gpui::{App, Entity, Global};

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub editor_service: EditorService,
    pub editor_status: Entity<EditorStatus>,
}

impl Global for AppState {}

impl AppState {
    pub fn init(cx: &mut App) {
        let state = Self {
            editor_service: EditorService::new(),
            editor_status: cx.new(|_| EditorStatus::default()),
        };
        cx.set_global::<AppState>(state);
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    #[allow(dead_code)]
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}
