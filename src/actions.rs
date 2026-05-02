use crate::core::document::Document;
use gpui::Action;
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = app)]
#[serde(deny_unknown_fields)]
pub struct OpenFile {
    pub path: String,
}

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = app)]
#[serde(deny_unknown_fields)]
pub struct SetFileTreeFolder {
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

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = app)]
#[serde(deny_unknown_fields)]
pub struct LoadChildren {
    pub path: String,
}

#[derive(Clone, PartialEq, Action)]
pub struct ToggleSearch;

#[derive(Clone, PartialEq, Action)]
pub struct SearchNext;

#[derive(Clone, PartialEq, Action)]
pub struct SearchPrev;

#[derive(Clone, PartialEq, Action)]
pub struct FocusHexView;

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = app)]
#[serde(deny_unknown_fields)]
pub struct OpenDiff {
    pub left_path: String,
    pub right_path: String,
}

#[derive(Clone, PartialEq, Action)]
pub struct NextDifference;

#[derive(Clone, PartialEq, Action)]
pub struct PrevDifference;

#[derive(Clone, PartialEq, Action)]
pub struct ToggleSyncScroll;

#[derive(Clone, PartialEq, Action)]
pub struct ToggleFileTree;

#[derive(Clone, PartialEq, Action)]
pub struct OpenSettings;

#[derive(Clone, PartialEq, Action)]
pub struct MoveLeft;

#[derive(Clone, PartialEq, Action)]
pub struct MoveRight;

#[derive(Clone, PartialEq, Action)]
pub struct MoveUp;

#[derive(Clone, PartialEq, Action)]
pub struct MoveDown;

#[derive(Clone, PartialEq, Action)]
pub struct SelectLeft;

#[derive(Clone, PartialEq, Action)]
pub struct SelectRight;

#[derive(Clone, PartialEq, Action)]
pub struct SelectUp;

#[derive(Clone, PartialEq, Action)]
pub struct SelectDown;

#[derive(Clone, PartialEq, Action)]
pub struct SelectAll;

#[derive(Clone, PartialEq, Action)]
pub struct PageUp;

#[derive(Clone, PartialEq, Action)]
pub struct PageDown;

#[derive(Clone, PartialEq, Action)]
pub struct Home;

#[derive(Clone, PartialEq, Action)]
pub struct End;

#[derive(Clone, PartialEq, Action)]
pub struct SelectPageUp;

#[derive(Clone, PartialEq, Action)]
pub struct SelectPageDown;

#[derive(Clone, PartialEq, Action)]
pub struct SelectHome;

#[derive(Clone, PartialEq, Action)]
pub struct SelectEnd;

#[derive(Clone, PartialEq, Action)]
pub struct AddCustomBreak;

#[derive(Clone, PartialEq, Action)]
pub struct RemoveCustomBreak;

#[derive(Clone)]
pub struct AddEditorPanel(pub Arc<RwLock<Document>>);

impl Action for AddEditorPanel {
    fn name(&self) -> &'static str {
        "AddEditorPanel"
    }

    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }

    fn partial_eq(&self, other: &dyn Action) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |other| self == other)
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
