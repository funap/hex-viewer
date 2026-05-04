// This file will be responsible for converting byte sequences into strings corresponding to a specified encoding
// (e.g., UTF-8, Shift JIS). It will also include logic for detecting the encoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Ascii,
    Utf8,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    pub fn decode_char_at(&self, buffer: &[u8], offset: usize) -> Option<(char, usize)> {
        match self {
            Encoding::Ascii => {
                if offset < buffer.len() {
                    let b = buffer[offset];
                    if b >= 32 && b <= 126 {
                        Some((b as char, 1))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Encoding::Utf8 => {
                if offset >= buffer.len() { return None; }
                let b = buffer[offset];
                let len = if b & 0x80 == 0 { 1 }
                          else if b & 0xE0 == 0xC0 { 2 }
                          else if b & 0xF0 == 0xE0 { 3 }
                          else if b & 0xF8 == 0xF0 { 4 }
                          else { return None; }; // Invalid start byte or continuation byte
                
                if offset + len <= buffer.len() {
                    if let Ok(s) = std::str::from_utf8(&buffer[offset..offset+len]) {
                        let c = s.chars().next().unwrap();
                        let is_printable = !c.is_control() && c != '\u{FFFD}';
                        if is_printable {
                            return Some((c, len));
                        }
                    }
                }
                None
            }
            Encoding::Utf16Le | Encoding::Utf16Be => {
                let is_le = *self == Encoding::Utf16Le;
                if offset % 2 != 0 { return None; }
                if offset + 2 <= buffer.len() {
                    let u1 = if is_le {
                        u16::from_le_bytes([buffer[offset], buffer[offset+1]])
                    } else {
                        u16::from_be_bytes([buffer[offset], buffer[offset+1]])
                    };
                    
                    if (0xD800..=0xDBFF).contains(&u1) { // High surrogate
                        if offset + 4 <= buffer.len() {
                            let u2 = if is_le {
                                u16::from_le_bytes([buffer[offset+2], buffer[offset+3]])
                            } else {
                                u16::from_be_bytes([buffer[offset+2], buffer[offset+3]])
                            };
                            if (0xDC00..=0xDFFF).contains(&u2) { // Low surrogate
                                if let Some(c) = std::char::decode_utf16([u1, u2]).next().and_then(|r| r.ok()) {
                                    let is_printable = !c.is_control() && c != '\u{FFFD}';
                                    if is_printable {
                                        return Some((c, 4));
                                    }
                                }
                            }
                        }
                    } else if !(0xDC00..=0xDFFF).contains(&u1) { // Not a low surrogate
                        if let Some(c) = std::char::decode_utf16([u1]).next().and_then(|r| r.ok()) {
                            let is_printable = !c.is_control() && c != '\u{FFFD}';
                            if is_printable {
                                return Some((c, 2));
                            }
                        }
                    }
                }
                None
            }
        }
    }

    pub fn is_continuation_byte(&self, buffer: &[u8], offset: usize) -> bool {
        if offset >= buffer.len() { return false; }
        match self {
            Encoding::Ascii => false,
            Encoding::Utf8 => {
                if buffer[offset] & 0xC0 != 0x80 {
                    return false;
                }
                for i in 1..=3 {
                    if offset >= i {
                        let start_idx = offset - i;
                        if buffer[start_idx] & 0xC0 != 0x80 {
                            if let Some((_, len)) = self.decode_char_at(buffer, start_idx) {
                                return start_idx + len > offset;
                            } else {
                                return false;
                            }
                        }
                    }
                }
                false
            }
            Encoding::Utf16Le | Encoding::Utf16Be => {
                if offset % 2 != 0 {
                    let start_idx = offset - 1;
                    if let Some((_, len)) = self.decode_char_at(buffer, start_idx) {
                        return start_idx + len > offset;
                    }
                    if start_idx >= 2 {
                        let prev_start = start_idx - 2;
                        if let Some((_, len)) = self.decode_char_at(buffer, prev_start) {
                            return prev_start + len > offset;
                        }
                    }
                    false
                } else {
                    if offset >= 2 {
                        let prev_start = offset - 2;
                        if let Some((_, len)) = self.decode_char_at(buffer, prev_start) {
                            return prev_start + len > offset;
                        }
                    }
                    false
                }
            }
        }
    }
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::Ascii
    }
}
