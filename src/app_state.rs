use crate::data::editor::Editor;
use crate::service::editor_service::EditorService;
use gpui::{App, Entity, Global, WeakEntity};

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub editor_service: EditorService,
    pub editors: Vec<Entity<Editor>>,
    pub active_editor: Option<WeakEntity<Editor>>,
}

impl Global for AppState {}

impl AppState {
    pub fn init(cx: &mut App) {
        let state = Self {
            editor_service: EditorService::new(),
            editors: Vec::new(),
            active_editor: None,
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
