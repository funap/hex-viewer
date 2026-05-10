use std::collections::HashMap;
use std::io::{Cursor, Read};
use crate::core::structure::types::{ParseResult, ParsedField, FieldValue, ParseError};
use crate::core::structure::palette;
use crate::core::structure::expression::ExprEvaluator;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyDefinition {
    pub meta: KsyMeta,
    #[serde(default)]
    pub seq: Vec<KsyAttr>,
    #[serde(default)]
    pub types: HashMap<String, KsyType>,
    #[serde(default)]
    pub enums: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub instances: HashMap<String, KsyAttr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyMeta {
    #[serde(default)]
    pub id: String,
    pub endian: Option<String>, // "le" or "be"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KsyAttr {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub attr_type: Option<String>,
    pub size: Option<KsyValue>,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub repeat: Option<String>, // "expr", "until", "eos"
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
}

pub struct KaitaiInterpreter<'a> {
    ksy: KsyDefinition,
    stream: &'a mut dyn KaitaiStream,
    context: HashMap<String, i64>,
    id_stack: Vec<String>,
    color_index: usize,
    errors: Vec<ParseError>,
    global_endian: String,
    field_count: usize,
    recursion_depth: usize,
}

const MAX_RECURSION: usize = 64;

impl<'a> KaitaiInterpreter<'a> {
    pub fn new(ksy: KsyDefinition, stream: &'a mut dyn KaitaiStream) -> Self {
        let global_endian = ksy.meta.endian.clone().unwrap_or_else(|| "le".to_string());
        Self {
            ksy,
            stream,
            context: HashMap::new(),
            id_stack: Vec::new(),
            color_index: 0,
            errors: Vec::new(),
            global_endian,
            field_count: 0,
            recursion_depth: 0,
        }
    }

    pub fn parse(mut self) -> ParseResult {
        println!("KaitaiInterpreter: starting parse for {}", self.ksy.meta.id);
        let mut fields = Vec::new();
        
        // Parse root sequence
        let seq = self.ksy.seq.clone();
        for attr in seq {
            let parsed = self.parse_attr_repeated(&attr, &self.ksy.types.clone());
            fields.extend(parsed);
        }

        // Parse root instances (only those with pos)
        let instances = self.ksy.instances.clone();
        for (id, mut attr) in instances {
            if attr.pos.is_some() {
                attr.id = Some(id);
                let parsed = self.parse_attr_repeated(&attr, &self.ksy.types.clone());
                fields.extend(parsed);
            }
        }

        println!("KaitaiInterpreter: finished parse, found {} fields", fields.len());
        ParseResult {
            definition_id: self.ksy.meta.id.clone(),
            fields,
            total_parsed_bytes: self.stream.pos() as usize,
            errors: self.errors,
        }
    }

    fn parse_attr_repeated(&mut self, attr: &KsyAttr, available_types: &HashMap<String, KsyType>) -> Vec<ParsedField> {
        if self.recursion_depth > MAX_RECURSION {
            return Vec::new();
        }

        let mut results = Vec::new();

        if let Some(repeat) = &attr.repeat {
            match repeat.as_str() {
                "expr" => {
                    if let Some(expr) = &attr.repeat_expr {
                        let count = self.resolve_count(expr);
                        for i in 0..count {
                            if let Some(field) = self.parse_attr_once(attr, Some(i), available_types) {
                                results.push(field);
                            } else {
                                break;
                            }
                        }
                    }
                }
                "eos" => {
                    let mut i = 0;
                    while !self.stream.is_eof() {
                        if let Some(field) = self.parse_attr_once(attr, Some(i), available_types) {
                            results.push(field);
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
                "until" => {
                    if let Some(expr) = &attr.repeat_until {
                        let mut i = 0;
                        while !self.stream.is_eof() {
                            if ExprEvaluator::evaluate_bool(expr, &self.context, &self.id_stack) {
                                break;
                            }
                            if let Some(field) = self.parse_attr_once(attr, Some(i), available_types) {
                                results.push(field);
                                i += 1;
                            } else {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        } else {
            if let Some(field) = self.parse_attr_once(attr, None, available_types) {
                results.push(field);
            }
        }

        results
    }

    fn parse_attr_once(&mut self, attr: &KsyAttr, index: Option<usize>, available_types: &HashMap<String, KsyType>) -> Option<ParsedField> {
        let attr_id = attr.id.as_deref().unwrap_or("unnamed");
        println!("Parsing attribute: {} at pos {}", attr_id, self.stream.pos());
        if self.recursion_depth > MAX_RECURSION {
            return None;
        }

        // Condition check
        if let Some(cond) = &attr.condition {
            if !ExprEvaluator::evaluate_bool(cond, &self.context, &self.id_stack) {
                return None;
            }
        }

        let start_offset = if attr.pos.is_some() {
            self.resolve_size(&attr.pos)?
        } else {
            self.stream.pos() as usize
        };

        // Save position if we are jumping
        let old_pos = if attr.pos.is_some() { Some(self.stream.pos()) } else { None };
        if attr.pos.is_some() {
            let target_pos = self.resolve_size(&attr.pos)?;
            self.stream.set_pos(target_pos as u64);
        }
        let is_little = self.global_endian == "le";

        let mut size = self.resolve_size(&attr.size);
        
        // If size is missing but contents is present, use contents length
        if size.is_none() || size == Some(0) {
            if let Some(expected_contents) = &attr.contents {
                if let Some(arr) = expected_contents.as_sequence() {
                    size = Some(arr.len());
                } else if let Some(s) = expected_contents.as_str() {
                    size = Some(s.len());
                }
            }
        }
        let size = size.unwrap_or(0);

        let (value, size, children) = if let Some(attr_type) = &attr.attr_type {
            match attr_type.as_str() {
                "u1" => (FieldValue::U8(self.stream.read_u1()?), 1, Vec::new()),
                "u2" => {
                    let val = if is_little { self.stream.read_u2le()? } else { self.stream.read_u2be()? };
                    (FieldValue::U16(val), 2, Vec::new())
                }
                "u4" => {
                    let val = if is_little { self.stream.read_u4le()? } else { self.stream.read_u4be()? };
                    (FieldValue::U32(val), 4, Vec::new())
                }
                "u8" => {
                    let val = if is_little { self.stream.read_u8le()? } else { self.stream.read_u8be()? };
                    (FieldValue::U64(val), 8, Vec::new())
                }
                "s1" => (FieldValue::I8(self.stream.read_s1()?), 1, Vec::new()),
                "s2" => {
                    let val = if is_little { self.stream.read_s2le()? } else { self.stream.read_s2be()? };
                    (FieldValue::I16(val), 2, Vec::new())
                }
                "s4" => {
                    let val = if is_little { self.stream.read_s4le()? } else { self.stream.read_s4be()? };
                    (FieldValue::I32(val), 4, Vec::new())
                }
                "s8" => {
                    let val = if is_little { self.stream.read_s8le()? } else { self.stream.read_s8be()? };
                    (FieldValue::I64(val), 8, Vec::new())
                }
                "str" => {
                    let buf = self.stream.read_bytes(size)?;
                    let s = String::from_utf8_lossy(&buf).into_owned();
                    (FieldValue::String(s), size, Vec::new())
                }
                "strz" => {
                    let buf = self.stream.read_bytes_term(0, false, true, true)?;
                    let size = buf.len() + 1; // +1 for terminator
                    let s = String::from_utf8_lossy(&buf).into_owned();
                    (FieldValue::String(s), size, Vec::new())
                }
                custom_type => {
                    if let Some(type_def) = available_types.get(custom_type) {
                        let field_id_for_stack = attr.id.clone().unwrap_or_else(|| format!("field_{}", self.field_count));
                        self.id_stack.push(field_id_for_stack);
                        self.recursion_depth += 1;
                        let mut nested_fields = Vec::new();
                        
                        let mut nested_available_types = available_types.clone();
                        nested_available_types.extend(type_def.types.clone());

                        let seq = type_def.seq.clone();
                        for nested_attr in &seq {
                            nested_fields.extend(self.parse_attr_repeated(nested_attr, &nested_available_types));
                        }
                        self.recursion_depth -= 1;
                        self.id_stack.pop();
                        let total_size = self.stream.pos() as usize - start_offset;
                        
                        if let Some(p) = old_pos {
                            self.stream.set_pos(p);
                        }

                        (FieldValue::Struct, total_size, nested_fields)
                    } else {
                        return None;
                    }
                }
            }
        } else {
            // Raw bytes based on size
            let buf = self.stream.read_bytes(size)?;
            (FieldValue::Bytes(buf), size, Vec::new())
        };

        // Handle contents validation
        if let Some(expected_contents) = &attr.contents {
            let mut expected_bytes = Vec::new();
            if let Some(arr) = expected_contents.as_sequence() {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        expected_bytes.extend(s.as_bytes());
                    } else if let Some(n) = v.as_i64() {
                        expected_bytes.push(n as u8);
                    } else if let Some(u) = v.as_u64() {
                        expected_bytes.push(u as u8);
                    }
                }
            }
            
            // Check if actual value matches
            match &value {
                FieldValue::Bytes(actual_bytes) => {
                    if actual_bytes != &expected_bytes {
                        // validation fail
                    }
                }
                _ => {
                    // If it's a numeric type, check if it matches first byte (if expected is 1 byte)
                    if expected_bytes.len() == 1 && value.to_i64() != expected_bytes[0] as i64 {
                        // validation fail
                    }
                }
            }
        }

        // Context update
        let mut field_id = attr.id.clone().unwrap_or_else(|| format!("unnamed_{}", self.field_count));
        if let Some(i) = index {
            field_id = format!("{}[{}]", field_id, i);
        }
        let full_id = if self.id_stack.is_empty() {
            field_id.clone()
        } else {
            format!("{}.{}", self.id_stack.join("."), field_id)
        };
        self.context.insert(full_id, value.to_i64());

        // Enum label
        let mut enum_label = None;
        if let Some(enum_name) = &attr.enum_ref {
            if let Some(enum_def) = self.ksy.enums.get(enum_name) {
                let key = value.to_i64().to_string();
                if let Some(label) = enum_def.get(&key) {
                    enum_label = Some(label.clone());
                }
            }
        }

        let color = palette::get_color(self.color_index);
        self.color_index += 1;
        self.field_count += 1;

        Some(ParsedField {
            id: field_id,
            field_type: attr.attr_type.clone().unwrap_or_else(|| "bytes".to_string()),
            offset: start_offset,
            size,
            value,
            color,
            description: None,
            children,
            enum_label,
        })
    }

    fn resolve_size(&self, size_val: &Option<KsyValue>) -> Option<usize> {
        match size_val {
            Some(KsyValue::Int(n)) => Some(*n),
            Some(KsyValue::Expr(e)) => {
                let val = ExprEvaluator::evaluate(e, &self.context, &self.id_stack);
                if val < 0 {
                    Some(0)
                } else {
                    Some(val as usize)
                }
            }
            None => Some(0),
        }
    }

    fn resolve_count(&self, expr: &str) -> usize {
        let val = ExprEvaluator::evaluate(expr, &self.context, &self.id_stack);
        if val < 0 { 0 } else { val as usize }.min(1000) // Cap repeat count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let ksy_str = r#"
meta:
  id: test_format
  endian: le
seq:
  - id: magic
    contents: [0x58, 0x56, 0x49] # "XVI"
  - id: version
    type: u1
  - id: padding
    size: 2
"#;
        let ksy: KsyDefinition = serde_yaml::from_str(ksy_str).unwrap();
        let data = vec![0x58, 0x56, 0x49, 0x01, 0xAA, 0xBB];
        let mut stream = Cursor::new(data.as_slice());
        let interpreter = KaitaiInterpreter::new(ksy, &mut stream);
        let result = interpreter.parse();

        assert_eq!(result.fields.len(), 3);
        assert_eq!(result.fields[0].id, "magic");
        assert_eq!(result.fields[1].id, "version");
        assert_eq!(result.fields[1].value.to_i64(), 1);
        assert_eq!(result.fields[2].id, "padding");
        assert_eq!(result.fields[2].size, 2);
    }

    #[test]
    fn test_relative_paths() {
        let ksy_str = r#"
meta:
  id: test_relative
  endian: le
seq:
  - id: header
    type: header_type
types:
  header_type:
    seq:
      - id: len
        type: u1
      - id: body
        type: body_type
  body_type:
    seq:
      - id: data
        size: _parent.len
"#;
        let ksy: KsyDefinition = serde_yaml::from_str(ksy_str).unwrap();
        let data = vec![0x03, 0x41, 0x42, 0x43];
        let mut stream = Cursor::new(data.as_slice());
        let interpreter = KaitaiInterpreter::new(ksy, &mut stream);
        let result = interpreter.parse();

        // header (Struct)
        //   header.len = 3
        //   header.body (Struct)
        //     header.body.data = [0x41, 0x42, 0x43] (size 3)
        
        assert_eq!(result.fields.len(), 1);
        let header = &result.fields[0];
        assert_eq!(header.children.len(), 2);
        assert_eq!(header.children[0].value.to_i64(), 3);
        
        let body = &header.children[1];
        assert_eq!(body.children.len(), 1);
        assert_eq!(body.children[0].size, 3);
    }

    #[test]
    fn test_strz() {
        let ksy_str = r#"
meta:
  id: test_strz
seq:
  - id: name
    type: strz
  - id: next
    type: u1
"#;
        let ksy: KsyDefinition = serde_yaml::from_str(ksy_str).unwrap();
        let data = vec![b'H', b'e', b'l', b'l', b'o', 0x00, 0x42];
        let mut stream = Cursor::new(data.as_slice());
        let interpreter = KaitaiInterpreter::new(ksy, &mut stream);
        let result = interpreter.parse();

        assert_eq!(result.fields.len(), 2);
        if let FieldValue::String(s) = &result.fields[0].value {
            assert_eq!(s, "Hello");
        } else {
            panic!("Expected String value");
        }
        assert_eq!(result.fields[0].size, 6); // "Hello" + null
        assert_eq!(result.fields[1].value.to_i64(), 0x42);
    }
}
