
use gpui::Hsla;
use crate::core::structure::definition::{StructDefinition, FieldDef};
use crate::core::structure::palette;

#[derive(Debug, Clone)]
pub enum FieldValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    String(String),
    Bytes(Vec<u8>),
    Struct, // Complex type container
}

impl FieldValue {
    pub fn to_i64(&self) -> i64 {
        match self {
            FieldValue::U8(v) => *v as i64,
            FieldValue::U16(v) => *v as i64,
            FieldValue::U32(v) => *v as i64,
            FieldValue::U64(v) => *v as i64,
            FieldValue::I8(v) => *v as i64,
            FieldValue::I16(v) => *v as i64,
            FieldValue::I32(v) => *v as i64,
            FieldValue::I64(v) => *v,
            _ => 0,
        }
    }
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::U8(v) => write!(f, "{}", v),
            FieldValue::U16(v) => write!(f, "{}", v),
            FieldValue::U32(v) => write!(f, "{}", v),
            FieldValue::U64(v) => write!(f, "{}", v),
            FieldValue::I8(v) => write!(f, "{}", v),
            FieldValue::I16(v) => write!(f, "{}", v),
            FieldValue::I32(v) => write!(f, "{}", v),
            FieldValue::I64(v) => write!(f, "{}", v),
            FieldValue::String(v) => write!(f, "\"{}\"", v),
            FieldValue::Bytes(v) => write!(f, "[{} bytes]", v.len()),
            FieldValue::Struct => write!(f, "{{...}}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedField {
    pub id: String,
    pub field_type: String,
    pub offset: usize,
    pub size: usize,
    pub value: FieldValue,
    pub color: Hsla,
    pub description: Option<String>,
    pub children: Vec<ParsedField>,
    pub enum_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub definition_id: String,
    pub fields: Vec<ParsedField>,
    pub total_parsed_bytes: usize,
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn to_highlights(&self) -> Vec<(std::ops::Range<usize>, gpui::Hsla)> {
        let mut highlights = Vec::new();
        Self::collect_highlights(&self.fields, &mut highlights);
        highlights
    }

    fn collect_highlights(fields: &[ParsedField], highlights: &mut Vec<(std::ops::Range<usize>, gpui::Hsla)>) {
        for field in fields {
            // Add parent then children so children are drawn on top
            if field.size > 0 {
                highlights.push((field.offset..field.offset + field.size, field.color));
            }
            if !field.children.is_empty() {
                Self::collect_highlights(&field.children, highlights);
            }
        }
    }
}

pub struct StructParser<'a> {
    def: &'a StructDefinition,
    buffer: &'a [u8],
    offset: usize,
    errors: Vec<ParseError>,
    color_index: usize,
    global_endian: String,
    context: std::collections::HashMap<String, i64>,
    id_stack: Vec<String>,
}

impl<'a> StructParser<'a> {
    pub fn new(def: &'a StructDefinition, buffer: &'a [u8]) -> Self {
        let global_endian = def.meta.endian.clone().unwrap_or_else(|| "little".to_string());
        Self {
            def,
            buffer,
            offset: 0,
            errors: Vec::new(),
            color_index: 0,
            global_endian,
            context: std::collections::HashMap::new(),
            id_stack: Vec::new(),
        }
    }

    pub fn parse(mut self) -> ParseResult {
        let mut fields = Vec::new();

        // Check magic number if present
        if let Some(magic) = &self.def.meta.magic {
            if self.buffer.len() >= magic.len() {
                if &self.buffer[0..magic.len()] != magic.as_slice() {
                    self.errors.push(ParseError {
                        message: "Magic number mismatch".to_string(),
                        offset: 0,
                    });
                }
            } else {
                 self.errors.push(ParseError {
                        message: "Buffer too small for magic number".to_string(),
                        offset: 0,
                 });
            }
        }

        let def_fields = self.def.fields.clone();
        for field_def in def_fields {
            let parsed_fields = self.parse_field_repeated(&field_def);
            fields.extend(parsed_fields);
            if self.offset >= self.buffer.len() && !self.errors.is_empty() {
                break;
            }
        }

        ParseResult {
            definition_id: self.def.meta.id.clone(),
            fields,
            total_parsed_bytes: self.offset,
            errors: self.errors,
        }
    }

    fn parse_field_repeated(&mut self, field_def: &FieldDef) -> Vec<ParsedField> {
        let mut results = Vec::new();

        if let Some(repeat_type) = &field_def.repeat {
            match repeat_type.as_str() {
                "expr" => {
                    if let Some(expr) = &field_def.repeat_expr {
                        let count = crate::core::structure::expression::ExprEvaluator::evaluate(expr, &self.context) as usize;
                        for i in 0..count {
                            // Update context with loop index? Not in spec, but useful.
                            // For now just repeat.
                            if let Some(field) = self.parse_field_once(field_def, Some(i)) {
                                results.push(field);
                            } else {
                                break;
                            }
                        }
                    }
                }
                "until" => {
                    if let Some(expr) = &field_def.repeat_until {
                        let mut i = 0;
                        while self.offset < self.buffer.len() {
                            if crate::core::structure::expression::ExprEvaluator::evaluate_bool(expr, &self.context) {
                                break;
                            }
                            if let Some(field) = self.parse_field_once(field_def, Some(i)) {
                                results.push(field);
                                i += 1;
                            } else {
                                break;
                            }
                        }
                    }
                }
                "eof" => {
                    let mut i = 0;
                    while self.offset < self.buffer.len() {
                        if let Some(field) = self.parse_field_once(field_def, Some(i)) {
                            results.push(field);
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            if let Some(field) = self.parse_field_once(field_def, None) {
                results.push(field);
            }
        }

        results
    }

    fn parse_field_once(&mut self, field_def: &FieldDef, index: Option<usize>) -> Option<ParsedField> {
        // Evaluate condition
        if let Some(cond) = &field_def.condition {
             if !crate::core::structure::expression::ExprEvaluator::evaluate_bool(cond, &self.context) {
                 return None;
             }
        }

        let start_offset = self.offset;

        // Determine color
        let color = if let Some(hex) = &field_def.color {
            palette::hex_to_hsla(hex).unwrap_or_else(|| palette::get_color(self.color_index))
        } else {
            palette::get_color(self.color_index)
        };
        self.color_index += 1;

        let endian = field_def.endian.as_ref().unwrap_or(&self.global_endian);
        let is_little = endian == "little";

        let (value, size, children) = match field_def.field_type.as_str() {
            "u8" => {
                if self.offset + 1 > self.buffer.len() { return None; }
                let val = self.buffer[self.offset];
                self.offset += 1;
                (FieldValue::U8(val), 1, Vec::new())
            }
            "u16" => {
                if self.offset + 2 > self.buffer.len() { return None; }
                let bytes = [self.buffer[self.offset], self.buffer[self.offset + 1]];
                let val = if is_little { u16::from_le_bytes(bytes) } else { u16::from_be_bytes(bytes) };
                self.offset += 2;
                (FieldValue::U16(val), 2, Vec::new())
            }
            "u32" => {
                if self.offset + 4 > self.buffer.len() { return None; }
                let bytes = [self.buffer[self.offset], self.buffer[self.offset + 1], self.buffer[self.offset + 2], self.buffer[self.offset + 3]];
                let val = if is_little { u32::from_le_bytes(bytes) } else { u32::from_be_bytes(bytes) };
                self.offset += 4;
                (FieldValue::U32(val), 4, Vec::new())
            }
            "u64" => {
                if self.offset + 8 > self.buffer.len() { return None; }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&self.buffer[self.offset..self.offset + 8]);
                let val = if is_little { u64::from_le_bytes(bytes) } else { u64::from_be_bytes(bytes) };
                self.offset += 8;
                (FieldValue::U64(val), 8, Vec::new())
            }
            "i8" => {
                if self.offset + 1 > self.buffer.len() { return None; }
                let val = self.buffer[self.offset] as i8;
                self.offset += 1;
                (FieldValue::I8(val), 1, Vec::new())
            }
            "i16" => {
                if self.offset + 2 > self.buffer.len() { return None; }
                let bytes = [self.buffer[self.offset], self.buffer[self.offset + 1]];
                let val = if is_little { i16::from_le_bytes(bytes) } else { i16::from_be_bytes(bytes) };
                self.offset += 2;
                (FieldValue::I16(val), 2, Vec::new())
            }
            "i32" => {
                if self.offset + 4 > self.buffer.len() { return None; }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&self.buffer[self.offset..self.offset + 4]);
                let val = if is_little { i32::from_le_bytes(bytes) } else { i32::from_be_bytes(bytes) };
                self.offset += 4;
                (FieldValue::I32(val), 4, Vec::new())
            }
             "i64" => {
                if self.offset + 8 > self.buffer.len() { return None; }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&self.buffer[self.offset..self.offset + 8]);
                let val = if is_little { i64::from_le_bytes(bytes) } else { i64::from_be_bytes(bytes) };
                self.offset += 8;
                (FieldValue::I64(val), 8, Vec::new())
            }
            "str" => {
                let size = field_def.size.unwrap_or(0);
                if self.offset + size > self.buffer.len() { return None; }
                let bytes = &self.buffer[self.offset..self.offset + size];
                let s = String::from_utf8_lossy(bytes).into_owned();
                self.offset += size;
                (FieldValue::String(s), size, Vec::new())
            }
            "bytes" | "padding" => {
                let size = if let Some(size_expr) = &field_def.size_ref {
                    crate::core::structure::expression::ExprEvaluator::evaluate(size_expr, &self.context) as usize
                } else {
                    field_def.size.unwrap_or(0)
                };

                if self.offset + size > self.buffer.len() { return None; }
                let bytes = self.buffer[self.offset..self.offset + size].to_vec();
                self.offset += size;
                (FieldValue::Bytes(bytes), size, Vec::new())
            }
            custom_type => {
                if let Some(type_def) = self.def.types.get(custom_type) {
                    self.id_stack.push(field_def.id.clone());
                    let mut nested_fields = Vec::new();
                    let type_fields = type_def.fields.clone();
                    for nested_def in type_fields {
                         let nested_results = self.parse_field_repeated(&nested_def);
                         nested_fields.extend(nested_results);
                    }
                    self.id_stack.pop();
                    let total_size = self.offset - start_offset;
                    (FieldValue::Struct, total_size, nested_fields)
                } else {
                    // Unknown type, skip based on size if provided, else error
                    let size = field_def.size.unwrap_or(0);
                    if size > 0 {
                         self.offset += size;
                         (FieldValue::Bytes(vec![]), size, Vec::new())
                    } else {
                         self.errors.push(ParseError {
                             message: format!("Unknown type: {}", custom_type),
                             offset: start_offset
                         });
                         return None;
                    }
                }
            }
        };

        // Handle Enum
        let mut enum_label = None;
        if let Some(enum_name) = &field_def.enum_ref {
            if let Some(enum_def) = self.def.enums.get(enum_name) {
                let key = match &value {
                    FieldValue::U8(v) => v.to_string(),
                    FieldValue::U16(v) => v.to_string(),
                    FieldValue::U32(v) => v.to_string(),
                    FieldValue::I8(v) => v.to_string(),
                    FieldValue::I16(v) => v.to_string(),
                    FieldValue::I32(v) => v.to_string(),
                    _ => String::new()
                };
                if let Some(label) = enum_def.0.get(&key) {
                    enum_label = Some(label.clone());
                }
            }
        }

        // Add to context
        let mut field_id = field_def.id.clone();
        if let Some(i) = index {
            field_id = format!("{}[{}]", field_id, i);
        }

        let full_id = if self.id_stack.is_empty() {
            field_id.clone()
        } else {
            format!("{}.{}", self.id_stack.join("."), field_id)
        };
        self.context.insert(full_id, value.to_i64());

        Some(ParsedField {
            id: field_id,
            field_type: field_def.field_type.clone(),
            offset: start_offset,
            size,
            value,
            color,
            description: field_def.description.clone(),
            children,
            enum_label,
        })
    }
}
#[cfg(test)]
mod tests {
    use crate::core::structure::definition::*;
    use crate::core::structure::parser::*;

    #[test]
    fn test_parse_basic() {
        // Just a basic compilation check.
        assert!(true);
    }
}
