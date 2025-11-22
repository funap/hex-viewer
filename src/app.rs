use crate::data::file_buffer::FileBuffer;
use crate::service::editor_service::EditorService;
use gpui::{Action, Global};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = app)]
#[serde(deny_unknown_fields)]
pub struct OpenFile {
    pub path: String,
}

#[derive(Clone, PartialEq, Action)]
pub struct Rename;

#[derive(Clone, PartialEq, Action)]
pub struct SelectItem;

#[derive(Clone, PartialEq, Action)]
pub struct OpenFolder;

#[derive(Clone, PartialEq, Action)]
pub struct CloseFolder;

#[derive(Clone)]
pub struct AddEditorPanel(pub Arc<FileBuffer>);

impl Action for AddEditorPanel {
    fn name(&self) -> &'static str {
        "AddEditorPanel"
    }

    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }

    fn partial_eq(&self, other: &dyn Action) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |other| self == other)
    }

    fn name_for_type() -> &'static str {
        "AddEditorPanel"
    }

    fn build(_: serde_json::Value) -> Result<Box<dyn Action>, anyhow::Error> {
        todo!()
    }
}

impl PartialEq for AddEditorPanel {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct App {
    pub editor_service: EditorService,
}

impl Global for App {}

impl App {
    pub fn new() -> Self {
        Self {
            editor_service: EditorService::new(),
        }
    }
}

#[allow(dead_code)]
pub struct AppState;
