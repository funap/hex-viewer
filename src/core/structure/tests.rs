#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::core::structure::types::FieldValue;
    use crate::core::structure::{KsyDefinition, KaitaiInterpreter};

    fn parse_ksy_yaml(yaml: &str) -> KsyDefinition {
        serde_yaml::from_str(yaml).expect("Failed to parse YAML")
    }

    #[test]
    fn test_endian_explicit() {
        let yaml = r#"
meta:
  id: test_endian
seq:
  - id: val1
    type: u2le
  - id: val2
    type: u4be
"#;
        let ksy = parse_ksy_yaml(yaml);
        let data = vec![0x01, 0x02, 0x00, 0x00, 0x00, 0x05];
        let mut stream = Cursor::new(data.as_slice());
        let interpreter = KaitaiInterpreter::new(ksy, &mut stream);
        let result = interpreter.parse();

        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[0].id, "val1");
        if let FieldValue::U16(v) = result.fields[0].value {
            assert_eq!(v, 0x0201); // little endian
        } else { panic!("Wrong type"); }

        assert_eq!(result.fields[1].id, "val2");
        if let FieldValue::U32(v) = result.fields[1].value {
            assert_eq!(v, 5); // big endian
        } else { panic!("Wrong type"); }
    }

    #[test]
    fn test_switch_on() {
        let yaml = r#"
meta:
  id: test_switch
seq:
  - id: tag
    type: u1
  - id: body
    type:
      switch-on: tag
      cases:
        1: u2le
        2: u4le
"#;
        let ksy = parse_ksy_yaml(yaml);
        let data = vec![0x01, 0xFF, 0x00];
        let mut stream = Cursor::new(data.as_slice());
        let interpreter = KaitaiInterpreter::new(ksy.clone(), &mut stream);
        let result1 = interpreter.parse();

        assert_eq!(result1.fields.len(), 2);
        if let FieldValue::U16(v) = result1.fields[1].value {
            assert_eq!(v, 0xFF);
        } else { panic!("Expected U16"); }

        let data2 = vec![0x02, 0x11, 0x22, 0x33, 0x44];
        let mut stream2 = Cursor::new(data2.as_slice());
        let interpreter2 = KaitaiInterpreter::new(ksy, &mut stream2);
        let result2 = interpreter2.parse();
        
        assert_eq!(result2.fields.len(), 2);
        if let FieldValue::U32(v) = result2.fields[1].value {
            assert_eq!(v, 0x44332211);
        } else { panic!("Expected U32"); }
    }

    #[test]
    fn test_size_eos() {
        let yaml = r#"
meta:
  id: test_eos
seq:
  - id: first
    type: u1
  - id: rest
    size-eos: true
"#;
        let ksy = parse_ksy_yaml(yaml);
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut stream = Cursor::new(data.as_slice());
        let interpreter = KaitaiInterpreter::new(ksy, &mut stream);
        let result = interpreter.parse();

        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[1].size, 3);
        if let FieldValue::Bytes(b) = &result.fields[1].value {
            assert_eq!(b, &[0xBB, 0xCC, 0xDD]);
        } else { panic!("Expected Bytes"); }
    }

    #[test]
    fn test_expression_bitwise() {
        use crate::core::structure::expression::{ExprEvaluator, EvalContext};
        use std::collections::HashMap;
        let mut values = HashMap::new();
        values.insert("flags".to_string(), 0b1010_1100);
        
        let string_values = HashMap::new();
        let base_path = vec![];
        let enums = HashMap::new();

        let ctx = EvalContext {
            values: &values,
            string_values: &string_values,
            base_path: &base_path,
            stream_eof: false,
            stream_size: 0,
            stream_pos: 0,
            enums: &enums,
        };

        assert_eq!(ExprEvaluator::eval_i64("flags & 0b1000_0000", &ctx), 128);
        assert_eq!(ExprEvaluator::eval_i64("flags | 0x03", &ctx), 0b1010_1111);
        assert_eq!(ExprEvaluator::eval_i64("1 << 3", &ctx), 8);
    }
}
