use gpui::{Action, actions};
use schemars::JsonSchema;
use serde::{Deserialize};

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = app)]
#[serde(deny_unknown_fields)]
pub struct OpenFile {
    pub path: String,
}

actions!(
    app,
    [
        Rename,
        SelectItem,
    ]
);
