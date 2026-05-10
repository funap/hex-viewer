use std::collections::HashMap;
use std::io::{Cursor, Read};
use serde::{Deserialize, Serialize};

pub trait KaitaiStream {
    fn pos(&self) -> u64;
    fn set_pos(&mut self, pos: u64);
    fn is_eof(&mut self) -> bool;
    fn read_u1(&mut self) -> Option<u8>;
    fn read_u2le(&mut self) -> Option<u16>;
    fn read_u2be(&mut self) -> Option<u16>;
    fn read_u4le(&mut self) -> Option<u32>;
    fn read_u4be(&mut self) -> Option<u32>;
    fn read_u8le(&mut self) -> Option<u64>;
    fn read_u8be(&mut self) -> Option<u64>;
    fn read_s1(&mut self) -> Option<i8>;
    fn read_s2le(&mut self) -> Option<i16>;
    fn read_s2be(&mut self) -> Option<i16>;
    fn read_s4le(&mut self) -> Option<i32>;
    fn read_s4be(&mut self) -> Option<i32>;
    fn read_s8le(&mut self) -> Option<i64>;
    fn read_s8be(&mut self) -> Option<i64>;
    fn read_bytes(&mut self, size: usize) -> Option<Vec<u8>>;
    fn read_bytes_term(&mut self, term: u8, include: bool, consume: bool, eos_error: bool) -> Option<Vec<u8>>;
}

impl<'a> KaitaiStream for Cursor<&'a [u8]> {
    fn pos(&self) -> u64 { self.position() }
    fn set_pos(&mut self, pos: u64) { self.set_position(pos); }
    fn is_eof(&mut self) -> bool { self.position() as usize >= self.get_ref().len() }
    fn read_u1(&mut self) -> Option<u8> {
        let mut b = [0u8; 1];
        self.read_exact(&mut b).ok()?;
        Some(b[0])
    }
    fn read_u2le(&mut self) -> Option<u16> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b).ok()?;
        Some(u16::from_le_bytes(b))
    }
    fn read_u2be(&mut self) -> Option<u16> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b).ok()?;
        Some(u16::from_be_bytes(b))
    }
    fn read_u4le(&mut self) -> Option<u32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).ok()?;
        Some(u32::from_le_bytes(b))
    }
    fn read_u4be(&mut self) -> Option<u32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).ok()?;
        Some(u32::from_be_bytes(b))
    }
    fn read_u8le(&mut self) -> Option<u64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b).ok()?;
        Some(u64::from_le_bytes(b))
    }
    fn read_u8be(&mut self) -> Option<u64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b).ok()?;
        Some(u64::from_be_bytes(b))
    }
    fn read_s1(&mut self) -> Option<i8> { Some(self.read_u1()? as i8) }
    fn read_s2le(&mut self) -> Option<i16> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b).ok()?;
        Some(i16::from_le_bytes(b))
    }
    fn read_s2be(&mut self) -> Option<i16> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b).ok()?;
        Some(i16::from_be_bytes(b))
    }
    fn read_s4le(&mut self) -> Option<i32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).ok()?;
        Some(i32::from_le_bytes(b))
    }
    fn read_s4be(&mut self) -> Option<i32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).ok()?;
        Some(i32::from_be_bytes(b))
    }
    fn read_s8le(&mut self) -> Option<i64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b).ok()?;
        Some(i64::from_le_bytes(b))
    }
    fn read_s8be(&mut self) -> Option<i64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b).ok()?;
        Some(i64::from_be_bytes(b))
    }
    fn read_bytes(&mut self, size: usize) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; size];
        self.read_exact(&mut buf).ok()?;
        Some(buf)
    }
    fn read_bytes_term(&mut self, term: u8, include: bool, consume: bool, _eos_error: bool) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        let ref_data = self.get_ref();
        let mut pos = self.position() as usize;
        while pos < ref_data.len() {
            let b = ref_data[pos];
            if b == term {
                if include {
                    buf.push(b);
                }
                if consume {
                    pos += 1;
                }
                self.set_position(pos as u64);
                return Some(buf);
            }
            buf.push(b);
            pos += 1;
        }
        self.set_position(pos as u64);
        Some(buf)
    }
}

/// Additional stream methods for float and remaining-bytes support
pub trait KaitaiStreamExt: KaitaiStream {
    fn read_f4le(&mut self) -> Option<f32>;
    fn read_f4be(&mut self) -> Option<f32>;
    fn read_f8le(&mut self) -> Option<f64>;
    fn read_f8be(&mut self) -> Option<f64>;
    fn read_bytes_remaining(&mut self) -> Option<Vec<u8>>;
    fn size(&self) -> u64;
}

impl<'a> KaitaiStreamExt for Cursor<&'a [u8]> {
    fn read_f4le(&mut self) -> Option<f32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).ok()?;
        Some(f32::from_le_bytes(b))
    }
    fn read_f4be(&mut self) -> Option<f32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).ok()?;
        Some(f32::from_be_bytes(b))
    }
    fn read_f8le(&mut self) -> Option<f64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b).ok()?;
        Some(f64::from_le_bytes(b))
    }
    fn read_f8be(&mut self) -> Option<f64> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b).ok()?;
        Some(f64::from_be_bytes(b))
    }
    fn read_bytes_remaining(&mut self) -> Option<Vec<u8>> {
        let pos = self.position() as usize;
        let data = self.get_ref();
        if pos >= data.len() {
            return Some(Vec::new());
        }
        let buf = data[pos..].to_vec();
        self.set_position(data.len() as u64);
        Some(buf)
    }
    fn size(&self) -> u64 {
        self.get_ref().len() as u64
    }
}

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
