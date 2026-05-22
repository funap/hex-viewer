use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyDefinition {
    pub meta: KsyMeta,
    #[serde(default)]
    pub seq: Vec<KsyAttr>,
    #[serde(default)]
    pub types: HashMap<String, KsyType>,
    #[serde(default)]
    pub enums: HashMap<String, HashMap<String, serde_yaml::Value>>,
    #[serde(default)]
    pub instances: HashMap<String, KsyAttr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyMeta {
    #[serde(default)]
    pub id: String,
    pub endian: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyAttr {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub attr_type: Option<serde_yaml::Value>,
    pub size: Option<KsyValue>,
    #[serde(rename = "size-eos", default)]
    pub size_eos: bool,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub repeat: Option<String>,
    #[serde(rename = "repeat-expr")]
    pub repeat_expr: Option<String>,
    #[serde(rename = "repeat-until")]
    pub repeat_until: Option<String>,
    #[serde(rename = "pos")]
    pub pos: Option<KsyValue>,
    #[serde(rename = "enum")]
    pub enum_ref: Option<String>,
    pub io: Option<String>,
    pub contents: Option<serde_yaml::Value>,
    pub encoding: Option<String>,
    pub doc: Option<String>,
    #[serde(rename = "doc-ref")]
    pub doc_ref: Option<serde_yaml::Value>,
    pub value: Option<String>,
    pub valid: Option<serde_yaml::Value>,
    pub process: Option<serde_yaml::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum KsyValue {
    Int(usize),
    Expr(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyType {
    #[serde(default)]
    pub seq: Vec<KsyAttr>,
    #[serde(default)]
    pub types: HashMap<String, KsyType>,
    #[serde(default)]
    pub instances: HashMap<String, KsyAttr>,
    #[serde(default)]
    pub enums: HashMap<String, HashMap<String, serde_yaml::Value>>,
}
