use crate::core::structure::expression::{EvalContext, ExprEvaluator};
use crate::core::structure::kaitai::*;
use crate::core::structure::palette;
use crate::core::structure::types::{FieldValue, ParseError, ParseResult, ParsedField};
use std::collections::HashMap;
use std::io::Cursor;

pub struct KaitaiInterpreter<'a> {
    ksy: KsyDefinition,
    stream: &'a mut dyn KaitaiStream,
    stream_size: usize,
    context: HashMap<String, i64>,
    string_context: HashMap<String, String>,
    id_stack: Vec<String>,
    color_index: usize,
    errors: Vec<ParseError>,
    global_endian: String,
    field_count: usize,
    recursion_depth: usize,
    all_enums: HashMap<String, HashMap<String, String>>,
}

const MAX_RECURSION: usize = 64;
const MAX_FIELDS: usize = 10000;

/// Helper: extract a simple type string from serde_yaml::Value
fn type_as_str(val: &serde_yaml::Value) -> Option<String> {
    match val {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Helper: check if type value is a switch-on/cases
fn type_as_switch(val: &serde_yaml::Value) -> Option<(String, HashMap<String, String>)> {
    let map = val.as_mapping()?;
    let switch_on = map.get(&serde_yaml::Value::String("switch-on".into()))?.as_str()?.to_string();
    let cases_val = map.get(&serde_yaml::Value::String("cases".into()))?.as_mapping()?;
    let mut cases = HashMap::new();
    for (k, v) in cases_val {
        let key = match k {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        if let Some(s) = v.as_str() {
            cases.insert(key, s.to_string());
        }
    }
    Some((switch_on, cases))
}

/// Normalize enum definitions: support both simple string values and {id: ...} map values
fn normalize_enum(raw: &HashMap<String, serde_yaml::Value>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (key, val) in raw {
        match val {
            serde_yaml::Value::String(s) => {
                result.insert(key.clone(), s.clone());
            }
            serde_yaml::Value::Mapping(m) => {
                if let Some(id_val) = m.get(&serde_yaml::Value::String("id".into())) {
                    if let Some(s) = id_val.as_str() {
                        result.insert(key.clone(), s.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    result
}

fn collect_enums(ksy: &KsyDefinition) -> HashMap<String, HashMap<String, String>> {
    let mut all = HashMap::new();
    for (name, raw) in &ksy.enums {
        all.insert(name.clone(), normalize_enum(raw));
    }
    fn collect_type_enums(types: &HashMap<String, KsyType>, out: &mut HashMap<String, HashMap<String, String>>) {
        for (_name, t) in types {
            for (ename, raw) in &t.enums {
                out.insert(ename.clone(), normalize_enum(raw));
            }
            collect_type_enums(&t.types, out);
        }
    }
    collect_type_enums(&ksy.types, &mut all);
    all
}

impl<'a> KaitaiInterpreter<'a> {
    pub fn new(ksy: KsyDefinition, stream: &'a mut dyn KaitaiStream) -> Self {
        let global_endian = ksy.meta.endian.clone().unwrap_or_else(|| "le".to_string());
        let all_enums = collect_enums(&ksy);
        Self {
            ksy,
            stream,
            stream_size: 0,
            context: HashMap::new(),
            string_context: HashMap::new(),
            id_stack: Vec::new(),
            color_index: 0,
            errors: Vec::new(),
            global_endian,
            field_count: 0,
            recursion_depth: 0,
            all_enums,
        }
    }

    pub fn parse(mut self) -> ParseResult {
        // Try to determine stream size (best-effort)
        let start = self.stream.pos();
        // We'll track max pos as we parse
        self.stream_size = 0;

        let mut fields = Vec::new();
        let seq = self.ksy.seq.clone();
        let types = self.ksy.types.clone();
        let enums = self.ksy.enums.clone();
        for attr in &seq {
            fields.extend(self.parse_attr_repeated(attr, &types, &enums));
        }
        let instances = self.ksy.instances.clone();
        for (id, mut attr) in instances {
            if attr.pos.is_some() || attr.value.is_some() {
                attr.id = Some(id);
                fields.extend(self.parse_attr_repeated(&attr, &types, &enums));
            }
        }
        ParseResult {
            definition_id: self.ksy.meta.id.clone(),
            fields,
            total_parsed_bytes: self.stream.pos() as usize,
            errors: self.errors,
        }
    }

    fn make_eval_ctx(&mut self) -> EvalContext {
        EvalContext {
            values: &self.context,
            string_values: &self.string_context,
            base_path: &self.id_stack,
            stream_eof: self.stream.is_eof(),
            stream_size: self.stream_size,
            stream_pos: self.stream.pos() as usize,
            enums: &self.all_enums,
        }
    }

    fn parse_attr_repeated(
        &mut self,
        attr: &KsyAttr,
        types: &HashMap<String, KsyType>,
        enums: &HashMap<String, HashMap<String, serde_yaml::Value>>,
    ) -> Vec<ParsedField> {
        if self.recursion_depth > MAX_RECURSION || self.field_count > MAX_FIELDS {
            return Vec::new();
        }
        let mut results = Vec::new();

        // Handle value instances (computed fields, no stream reading)
        if let Some(value_expr) = &attr.value {
            let ctx = self.make_eval_ctx();
            let val = ExprEvaluator::eval_i64(value_expr, &ctx);
            let field_id = attr.id.clone().unwrap_or_else(|| format!("value_{}", self.field_count));
            let full_id = if self.id_stack.is_empty() {
                field_id.clone()
            } else {
                format!("{}.{}", self.id_stack.join("."), field_id)
            };
            self.context.insert(full_id, val);
            // Value instances don't produce visible fields for hex highlighting
            return results;
        }

        if let Some(repeat) = &attr.repeat {
            match repeat.as_str() {
                "expr" => {
                    if let Some(expr) = &attr.repeat_expr {
                        let count = self.resolve_count(expr);
                        for i in 0..count {
                            if let Some(field) = self.parse_attr_once(attr, Some(i), types, enums) {
                                results.push(field);
                            } else {
                                break;
                            }
                        }
                    }
                }
                "eos" => {
                    let mut i = 0;
                    while !self.stream.is_eof() && i < MAX_FIELDS {
                        if let Some(field) = self.parse_attr_once(attr, Some(i), types, enums) {
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
                        loop {
                            if self.stream.is_eof() || i >= MAX_FIELDS {
                                break;
                            }
                            if let Some(field) = self.parse_attr_once(attr, Some(i), types, enums) {
                                // Store last element info for _ reference
                                let last_val = field.value.to_i64();
                                let last_str = field.value.to_string_value();
                                let field_id = attr.id.as_deref().unwrap_or("_");
                                let ctx = self.make_eval_ctx();
                                // Put _ values in context temporarily
                                let underscore_key = format!("{}.type", field_id);
                                // Check condition with last parsed field
                                // We use a simple approach: put _.field values and check
                                results.push(field);
                                i += 1;
                                // Evaluate until condition
                                let ctx = self.make_eval_ctx();
                                if ExprEvaluator::eval_bool(expr, &ctx) {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        } else {
            if let Some(field) = self.parse_attr_once(attr, None, types, enums) {
                results.push(field);
            }
        }
        results
    }

    fn parse_attr_once(
        &mut self,
        attr: &KsyAttr,
        index: Option<usize>,
        types: &HashMap<String, KsyType>,
        enums: &HashMap<String, HashMap<String, serde_yaml::Value>>,
    ) -> Option<ParsedField> {
        if self.recursion_depth > MAX_RECURSION || self.field_count > MAX_FIELDS {
            return None;
        }

        // Condition check
        if let Some(cond) = &attr.condition {
            let ctx = self.make_eval_ctx();
            if !ExprEvaluator::eval_bool(cond, &ctx) {
                return None;
            }
        }

        let start_offset = if attr.pos.is_some() {
            self.resolve_size(&attr.pos)?
        } else {
            self.stream.pos() as usize
        };

        let old_pos = if attr.pos.is_some() { Some(self.stream.pos()) } else { None };
        if attr.pos.is_some() {
            self.stream.set_pos(start_offset as u64);
        }

        let is_little = self.global_endian == "le";
        let mut size = self.resolve_size(&attr.size);

        // size-eos: read remaining bytes
        if attr.size_eos {
            // We don't have stream size directly, but we track via is_eof
            // For substreams, size will be set. For main stream, read until eof.
            size = None; // Will be handled specially below
        }

        // Contents-based size
        if !attr.size_eos && (size.is_none() || size == Some(0)) {
            if let Some(expected_contents) = &attr.contents {
                if let Some(arr) = expected_contents.as_sequence() {
                    size = Some(arr.len());
                } else if let Some(s) = expected_contents.as_str() {
                    size = Some(s.len());
                }
            }
        }
        let computed_size = size.unwrap_or(0);

        // Resolve type
        let resolved_type = attr.attr_type.as_ref().and_then(|v| {
            if let Some(s) = type_as_str(v) {
                Some(s)
            } else if let Some((switch_on, cases)) = type_as_switch(v) {
                let ctx = self.make_eval_ctx();
                let switch_val = ExprEvaluator::evaluate_rich(&switch_on, &ctx);
                // Try to match switch value against cases
                let switch_str = switch_val.to_string_val();
                let switch_int = switch_val.to_i64();
                // Try exact string match first, then integer match
                if let Some(t) = cases.get(&switch_str) {
                    Some(t.clone())
                } else if let Some(t) = cases.get(&format!("\"{}\"", switch_str)) {
                    Some(t.clone())
                } else if let Some(t) = cases.get(&switch_int.to_string()) {
                    Some(t.clone())
                } else if let Some(t) = cases.get("_") {
                    Some(t.clone()) // default case
                } else {
                    None
                }
            } else {
                None
            }
        });

        let typed_result = if let Some(type_str) = &resolved_type {
            self.parse_typed_value(type_str, is_little, computed_size, attr, start_offset, old_pos, types, enums)
        } else if attr.size_eos {
            // Read remaining bytes as raw
            let buf = self.read_remaining();
            let s = buf.len();
            Some((FieldValue::Bytes(buf), s, Vec::new()))
        } else {
            // Raw bytes
            if computed_size > 0 {
                self.stream
                    .read_bytes(computed_size)
                    .map(|buf| (FieldValue::Bytes(buf), computed_size, Vec::new()))
            } else {
                Some((FieldValue::Bytes(Vec::new()), 0, Vec::new()))
            }
        };

        // Restore position if we jumped
        if let Some(p) = old_pos {
            self.stream.set_pos(p);
        }

        let (value, final_size, children) = typed_result?;

        // Contents validation (unchanged logic)
        if let Some(expected) = &attr.contents {
            self.validate_contents(expected, &value);
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
        self.context.insert(full_id.clone(), value.to_i64());
        // Also store string values
        if let FieldValue::String(ref s) = value {
            self.string_context.insert(full_id.clone(), s.clone());
        }

        // Enum label
        let enum_label = self.resolve_enum_label(attr, &value, enums);

        let color = palette::get_color(self.color_index);
        self.color_index += 1;
        self.field_count += 1;

        let type_name = resolved_type.unwrap_or_else(|| "bytes".to_string());
        Some(ParsedField {
            id: field_id,
            field_type: type_name,
            offset: start_offset,
            size: final_size,
            value,
            color,
            description: attr.doc.clone(),
            children,
            enum_label,
        })
    }

    fn parse_typed_value(
        &mut self,
        type_str: &str,
        is_little: bool,
        size: usize,
        attr: &KsyAttr,
        start_offset: usize,
        old_pos: Option<u64>,
        types: &HashMap<String, KsyType>,
        enums: &HashMap<String, HashMap<String, serde_yaml::Value>>,
    ) -> Option<(FieldValue, usize, Vec<ParsedField>)> {
        match type_str {
            "u1" => Some((FieldValue::U8(self.stream.read_u1()?), 1, Vec::new())),
            "u2" => {
                let v = if is_little { self.stream.read_u2le()? } else { self.stream.read_u2be()? };
                Some((FieldValue::U16(v), 2, Vec::new()))
            }
            "u4" => {
                let v = if is_little { self.stream.read_u4le()? } else { self.stream.read_u4be()? };
                Some((FieldValue::U32(v), 4, Vec::new()))
            }
            "u8" => {
                let v = if is_little { self.stream.read_u8le()? } else { self.stream.read_u8be()? };
                Some((FieldValue::U64(v), 8, Vec::new()))
            }
            "s1" => Some((FieldValue::I8(self.stream.read_s1()?), 1, Vec::new())),
            "s2" => {
                let v = if is_little { self.stream.read_s2le()? } else { self.stream.read_s2be()? };
                Some((FieldValue::I16(v), 2, Vec::new()))
            }
            "s4" => {
                let v = if is_little { self.stream.read_s4le()? } else { self.stream.read_s4be()? };
                Some((FieldValue::I32(v), 4, Vec::new()))
            }
            "s8" => {
                let v = if is_little { self.stream.read_s8le()? } else { self.stream.read_s8be()? };
                Some((FieldValue::I64(v), 8, Vec::new()))
            }
            // Explicit endian types
            "u2le" => Some((FieldValue::U16(self.stream.read_u2le()?), 2, Vec::new())),
            "u2be" => Some((FieldValue::U16(self.stream.read_u2be()?), 2, Vec::new())),
            "u4le" => Some((FieldValue::U32(self.stream.read_u4le()?), 4, Vec::new())),
            "u4be" => Some((FieldValue::U32(self.stream.read_u4be()?), 4, Vec::new())),
            "u8le" => Some((FieldValue::U64(self.stream.read_u8le()?), 8, Vec::new())),
            "u8be" => Some((FieldValue::U64(self.stream.read_u8be()?), 8, Vec::new())),
            "s2le" => Some((FieldValue::I16(self.stream.read_s2le()?), 2, Vec::new())),
            "s2be" => Some((FieldValue::I16(self.stream.read_s2be()?), 2, Vec::new())),
            "s4le" => Some((FieldValue::I32(self.stream.read_s4le()?), 4, Vec::new())),
            "s4be" => Some((FieldValue::I32(self.stream.read_s4be()?), 4, Vec::new())),
            "s8le" => Some((FieldValue::I64(self.stream.read_s8le()?), 8, Vec::new())),
            "s8be" => Some((FieldValue::I64(self.stream.read_s8be()?), 8, Vec::new())),
            // Float types
            "f4" => {
                let v = if is_little { self.read_f4le()? } else { self.read_f4be()? };
                Some((FieldValue::F32(v), 4, Vec::new()))
            }
            "f8" => {
                let v = if is_little { self.read_f8le()? } else { self.read_f8be()? };
                Some((FieldValue::F64(v), 8, Vec::new()))
            }
            "f4le" => Some((FieldValue::F32(self.read_f4le()?), 4, Vec::new())),
            "f4be" => Some((FieldValue::F32(self.read_f4be()?), 4, Vec::new())),
            "f8le" => Some((FieldValue::F64(self.read_f8le()?), 8, Vec::new())),
            "f8be" => Some((FieldValue::F64(self.read_f8be()?), 8, Vec::new())),
            // String types
            "str" => {
                let read_size = if attr.size_eos { self.remaining_bytes() } else { size };
                let buf = self.stream.read_bytes(read_size)?;
                let s = String::from_utf8_lossy(&buf).into_owned();
                Some((FieldValue::String(s), read_size, Vec::new()))
            }
            "strz" => {
                let buf = self.stream.read_bytes_term(0, false, true, true)?;
                let sz = buf.len() + 1;
                let s = String::from_utf8_lossy(&buf).into_owned();
                Some((FieldValue::String(s), sz, Vec::new()))
            }
            // Bit fields
            t if t.starts_with('b') && t[1..].parse::<usize>().is_ok() => {
                let bits: usize = t[1..].parse().unwrap();
                let bytes_needed = (bits + 7) / 8;
                let buf = self.stream.read_bytes(bytes_needed)?;
                let mut val: u64 = 0;
                for &b in &buf {
                    val = (val << 8) | b as u64;
                }
                // Mask to exact bits
                if bits < 64 {
                    val &= (1u64 << bits) - 1;
                }
                Some((FieldValue::U64(val), bytes_needed, Vec::new()))
            }
            // Custom type
            custom => self.parse_custom_type(custom, attr, start_offset, old_pos, types, enums),
        }
    }

    fn parse_custom_type(
        &mut self,
        type_name: &str,
        attr: &KsyAttr,
        start_offset: usize,
        _old_pos: Option<u64>,
        types: &HashMap<String, KsyType>,
        enums: &HashMap<String, HashMap<String, serde_yaml::Value>>,
    ) -> Option<(FieldValue, usize, Vec<ParsedField>)> {
        let type_def = types.get(type_name)?;
        let field_id = attr.id.clone().unwrap_or_else(|| format!("field_{}", self.field_count));
        self.id_stack.push(field_id);
        self.recursion_depth += 1;

        let mut nested_types = types.clone();
        nested_types.extend(type_def.types.clone());

        let mut nested_enums = enums.clone();
        nested_enums.extend(type_def.enums.clone());
        // Rebuild all_enums with nested
        for (k, v) in &type_def.enums {
            self.all_enums.insert(k.clone(), normalize_enum(v));
        }

        // If attr has a size, parse in a substream
        let size_val = self.resolve_size(&attr.size);
        let use_substream = (size_val.is_some() && size_val != Some(0)) || attr.size_eos;

        let res = (|| {
            if use_substream {
                let sub_size = if attr.size_eos { self.remaining_bytes() } else { size_val.unwrap_or(0) };
                let sub_data = self.stream.read_bytes(sub_size)?;
                let old_stream_size = self.stream_size;
                self.stream_size = sub_size;
                let mut fields = Vec::new();

                // Parse sub_data by creating a new scope
                let seq = type_def.seq.clone();
                for nested_attr in &seq {
                    fields.extend(self.parse_substream_attr(nested_attr, &sub_data, &nested_types, &nested_enums, start_offset));
                }
                self.stream_size = old_stream_size;
                Some(fields)
            } else {
                let seq = type_def.seq.clone();
                let mut fields = Vec::new();
                for nested_attr in &seq {
                    fields.extend(self.parse_attr_repeated(nested_attr, &nested_types, &nested_enums));
                }
                // Parse instances
                let instances = type_def.instances.clone();
                for (id, mut inst_attr) in instances {
                    if inst_attr.pos.is_some() || inst_attr.value.is_some() {
                        inst_attr.id = Some(id);
                        fields.extend(self.parse_attr_repeated(&inst_attr, &nested_types, &nested_enums));
                    }
                }
                Some(fields)
            }
        })();

        self.recursion_depth -= 1;
        self.id_stack.pop();

        let nested_fields = res?;
        let current_pos = self.stream.pos() as usize;
        let total_size = current_pos.saturating_sub(start_offset);

        Some((FieldValue::Struct, total_size, nested_fields))
    }

    fn parse_substream_attr(
        &mut self,
        attr: &KsyAttr,
        data: &[u8],
        types: &HashMap<String, KsyType>,
        enums: &HashMap<String, HashMap<String, serde_yaml::Value>>,
        base_offset: usize,
    ) -> Vec<ParsedField> {
        // Simple substream parsing: read from data slice
        // For now, we use the main stream approach but note the offset
        self.parse_attr_repeated(attr, types, enums)
    }

    fn resolve_enum_label(&self, attr: &KsyAttr, value: &FieldValue, enums: &HashMap<String, HashMap<String, serde_yaml::Value>>) -> Option<String> {
        let enum_name = attr.enum_ref.as_ref()?;
        // Try local enums first, then global
        let enum_def = enums.get(enum_name).or_else(|| self.ksy.enums.get(enum_name))?;
        let key = value.to_i64().to_string();
        let val = enum_def.get(&key)?;
        match val {
            serde_yaml::Value::String(s) => Some(s.clone()),
            serde_yaml::Value::Mapping(m) => m.get(&serde_yaml::Value::String("id".into()))?.as_str().map(|s| s.to_string()),
            _ => None,
        }
    }

    fn validate_contents(&mut self, expected: &serde_yaml::Value, actual: &FieldValue) {
        let mut expected_bytes = Vec::new();
        if let Some(arr) = expected.as_sequence() {
            for v in arr {
                if let Some(s) = v.as_str() {
                    expected_bytes.extend(s.as_bytes());
                } else if let Some(n) = v.as_i64() {
                    expected_bytes.push(n as u8);
                } else if let Some(u) = v.as_u64() {
                    expected_bytes.push(u as u8);
                }
            }
        } else if let Some(s) = expected.as_str() {
            expected_bytes.extend(s.as_bytes());
        }
        if !expected_bytes.is_empty() {
            if let FieldValue::Bytes(actual_bytes) = actual {
                if actual_bytes != &expected_bytes {
                    self.errors.push(ParseError {
                        message: "contents mismatch".into(),
                        offset: self.stream.pos() as usize,
                    });
                }
            }
        }
    }

    fn resolve_size(&mut self, size_val: &Option<KsyValue>) -> Option<usize> {
        match size_val {
            Some(KsyValue::Int(n)) => Some(*n),
            Some(KsyValue::Expr(e)) => {
                let ctx = self.make_eval_ctx();
                let val = ExprEvaluator::eval_i64(e, &ctx);
                Some(if val < 0 { 0 } else { val as usize })
            }
            None => None,
        }
    }

    fn resolve_count(&mut self, expr: &str) -> usize {
        let ctx = self.make_eval_ctx();
        let val = ExprEvaluator::eval_i64(expr, &ctx);
        (if val < 0 { 0 } else { val as usize }).min(MAX_FIELDS)
    }

    // Float read helpers using raw byte reading
    fn read_f4le(&mut self) -> Option<f32> {
        let b = self.stream.read_bytes(4)?;
        Some(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn read_f4be(&mut self) -> Option<f32> {
        let b = self.stream.read_bytes(4)?;
        Some(f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn read_f8le(&mut self) -> Option<f64> {
        let b = self.stream.read_bytes(8)?;
        Some(f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    fn read_f8be(&mut self) -> Option<f64> {
        let b = self.stream.read_bytes(8)?;
        Some(f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    fn remaining_bytes(&mut self) -> usize {
        // Best effort: try reading until eof
        0 // Will be overridden for substreams
    }
    fn read_remaining(&mut self) -> Vec<u8> {
        let mut buf = Vec::new();
        loop {
            match self.stream.read_u1() {
                Some(b) => buf.push(b),
                None => break,
            }
        }
        buf
    }
}
