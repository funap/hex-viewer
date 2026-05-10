use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDefinition {
    pub meta: MetaDef,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub types: HashMap<String, TypeDef>,
    #[serde(default)]
    pub enums: HashMap<String, EnumDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDef {
    pub id: String,
    pub title: Option<String>,
    pub endian: Option<String>, // "little" or "big"
    pub file_extension: Option<String>,
    pub magic: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub id: String,
    #[serde(rename = "type")]
    pub field_type: String, // e.g. "u32", "str", "bytes", or custom type name
    pub size: Option<usize>,
    pub size_ref: Option<String>,
    pub encoding: Option<String>, // e.g. "ascii" for string types
    pub color: Option<String>,
    pub description: Option<String>,
    pub display: Option<String>, // e.g. "hex", "binary"
    pub endian: Option<String>,
    #[serde(rename = "enum")]
    pub enum_ref: Option<String>,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub repeat: Option<String>,
    pub repeat_expr: Option<String>,
    pub repeat_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef(pub HashMap<String, String>);
