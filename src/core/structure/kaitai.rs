use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// KaitaiStream represents the binary stream reader for Kaitai Struct formats,
/// supporting bit-level reading, endian-specific integer/float reads, and substreams.
pub struct KaitaiStream<'a> {
    data: &'a [u8],
    pos: usize,
    bits: u64,
    bits_left: usize,
}

impl<'a> KaitaiStream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bits: 0,
            bits_left: 0,
        }
    }

    pub fn pos(&self) -> u64 {
        self.pos as u64
    }

    pub fn set_pos(&mut self, pos: u64) {
        self.align_to_byte();
        self.pos = (pos as usize).min(self.data.len());
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len() && self.bits_left == 0
    }

    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn align_to_byte(&mut self) {
        self.bits_left = 0;
        self.bits = 0;
    }

    pub fn read_bits_int_be(&mut self, n: usize) -> Option<u64> {
        if n > 64 {
            return None;
        }
        if n == 0 {
            return Some(0);
        }
        let mut res: u64 = 0;
        let bits_needed = n as isize - self.bits_left as isize;
        self.bits_left = ((-bits_needed) & 7) as usize;

        if bits_needed > 0 {
            let bytes_needed = ((bits_needed - 1) / 8 + 1) as usize;
            if self.pos + bytes_needed > self.data.len() {
                return None;
            }
            let buf = &self.data[self.pos..self.pos + bytes_needed];
            self.pos += bytes_needed;
            for i in 0..bytes_needed {
                res = (res << 8) | buf[i] as u64;
            }

            let new_bits = res;
            let mut shifted_res = res >> self.bits_left;
            if bits_needed < 64 && self.bits_left < 64 {
                let bits_to_shift = bits_needed as usize;
                if bits_to_shift < 64 {
                    shifted_res |= self.bits << bits_to_shift;
                }
            }
            res = shifted_res;
            self.bits = new_bits;
        } else {
            let shift_amount = (-bits_needed) as usize;
            res = self.bits >> shift_amount;
        }

        let mask = (1u64 << self.bits_left) - 1;
        self.bits &= mask;

        Some(res)
    }

    pub fn read_bits_int_le(&mut self, n: usize) -> Option<u64> {
        if n > 64 {
            return None;
        }
        if n == 0 {
            return Some(0);
        }
        let mut res: u64 = 0;
        let bits_needed = n as isize - self.bits_left as isize;

        if bits_needed > 0 {
            let bytes_needed = ((bits_needed - 1) / 8 + 1) as usize;
            if self.pos + bytes_needed > self.data.len() {
                return None;
            }
            let buf = &self.data[self.pos..self.pos + bytes_needed];
            self.pos += bytes_needed;
            for i in 0..bytes_needed {
                res |= (buf[i] as u64) << (i * 8);
            }

            let new_bits = if bits_needed < 64 {
                res >> bits_needed as usize
            } else {
                0
            };
            res = (res << self.bits_left) | self.bits;
            self.bits = new_bits;
        } else {
            res = self.bits;
            self.bits >>= n;
        }

        self.bits_left = ((-bits_needed) & 7) as usize;

        if n < 64 {
            let mask = (1u64 << n) - 1;
            res &= mask;
        }
        Some(res)
    }

    pub fn read_u1(&mut self) -> Option<u8> {
        self.align_to_byte();
        if self.pos < self.data.len() {
            let val = self.data[self.pos];
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }

    pub fn read_u2le(&mut self) -> Option<u16> {
        self.align_to_byte();
        if self.pos + 2 <= self.data.len() {
            let val = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(val)
        } else {
            None
        }
    }

    pub fn read_u2be(&mut self) -> Option<u16> {
        self.align_to_byte();
        if self.pos + 2 <= self.data.len() {
            let val = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(val)
        } else {
            None
        }
    }

    pub fn read_u4le(&mut self) -> Option<u32> {
        self.align_to_byte();
        if self.pos + 4 <= self.data.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
            self.pos += 4;
            Some(u32::from_le_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_u4be(&mut self) -> Option<u32> {
        self.align_to_byte();
        if self.pos + 4 <= self.data.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
            self.pos += 4;
            Some(u32::from_be_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_u8le(&mut self) -> Option<u64> {
        self.align_to_byte();
        if self.pos + 8 <= self.data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
            self.pos += 8;
            Some(u64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_u8be(&mut self) -> Option<u64> {
        self.align_to_byte();
        if self.pos + 8 <= self.data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
            self.pos += 8;
            Some(u64::from_be_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_s1(&mut self) -> Option<i8> {
        Some(self.read_u1()? as i8)
    }

    pub fn read_s2le(&mut self) -> Option<i16> {
        Some(self.read_u2le()? as i16)
    }

    pub fn read_s2be(&mut self) -> Option<i16> {
        Some(self.read_u2be()? as i16)
    }

    pub fn read_s4le(&mut self) -> Option<i32> {
        Some(self.read_u4le()? as i32)
    }

    pub fn read_s4be(&mut self) -> Option<i32> {
        Some(self.read_u4be()? as i32)
    }

    pub fn read_s8le(&mut self) -> Option<i64> {
        Some(self.read_u8le()? as i64)
    }

    pub fn read_s8be(&mut self) -> Option<i64> {
        Some(self.read_u8be()? as i64)
    }

    pub fn read_f4le(&mut self) -> Option<f32> {
        self.align_to_byte();
        if self.pos + 4 <= self.data.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
            self.pos += 4;
            Some(f32::from_le_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_f4be(&mut self) -> Option<f32> {
        self.align_to_byte();
        if self.pos + 4 <= self.data.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
            self.pos += 4;
            Some(f32::from_be_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_f8le(&mut self) -> Option<f64> {
        self.align_to_byte();
        if self.pos + 8 <= self.data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
            self.pos += 8;
            Some(f64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_f8be(&mut self) -> Option<f64> {
        self.align_to_byte();
        if self.pos + 8 <= self.data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
            self.pos += 8;
            Some(f64::from_be_bytes(bytes))
        } else {
            None
        }
    }

    pub fn read_bytes(&mut self, size: usize) -> Option<Vec<u8>> {
        self.align_to_byte();
        if self.pos + size <= self.data.len() {
            let val = self.data[self.pos..self.pos + size].to_vec();
            self.pos += size;
            Some(val)
        } else {
            None
        }
    }

    pub fn read_bytes_remaining(&mut self) -> Option<Vec<u8>> {
        self.align_to_byte();
        let size = self.data.len() - self.pos;
        self.read_bytes(size)
    }

    pub fn read_bytes_full(&mut self) -> Option<Vec<u8>> {
        self.read_bytes_remaining()
    }

    pub fn read_bytes_term(&mut self, term: u8, include: bool, consume: bool, _eos_error: bool) -> Option<Vec<u8>> {
        self.align_to_byte();
        let mut buf = Vec::new();
        let mut idx = self.pos;
        while idx < self.data.len() {
            let b = self.data[idx];
            if b == term {
                if include {
                    buf.push(b);
                }
                if consume {
                    idx += 1;
                }
                self.pos = idx;
                return Some(buf);
            }
            buf.push(b);
            idx += 1;
        }
        self.pos = idx;
        Some(buf)
    }

    pub fn read_bytes_term_multi(&mut self, terminator: &[u8], include: bool, consume: bool, _eos_error: bool) -> Option<Vec<u8>> {
        self.align_to_byte();
        let unit_size = terminator.len();
        if unit_size == 0 {
            return Some(Vec::new());
        }
        
        let data = &self.data[self.pos..];
        let len = data.len();
        let mut i_data = 0;
        let mut i_term = 0;
        let mut term_found = false;
        
        while i_data < len {
            if data[i_data] != terminator[i_term] {
                i_data += unit_size - i_term;
                i_term = 0;
                continue;
            }
            i_data += 1;
            i_term += 1;
            if i_term == unit_size {
                term_found = true;
                break;
            }
        }
        
        if term_found {
            let match_len = i_data;
            let result_len = if include { match_len } else { match_len - unit_size };
            let result = data[..result_len].to_vec();
            self.pos += if consume { match_len } else { match_len - unit_size };
            Some(result)
        } else {
            let result = data.to_vec();
            self.pos = self.data.len();
            Some(result)
        }
    }

    pub fn ensure_fixed_contents(&mut self, expected: &[u8]) -> Option<Vec<u8>> {
        let actual = self.read_bytes(expected.len())?;
        if actual == expected {
            Some(actual)
        } else {
            None
        }
    }
}

/// Kaitai Struct data processing utilities for custom data transformations (XOR, rotate, zlib).
pub mod process {
    pub fn xor_one(data: &[u8], key: u8) -> Vec<u8> {
        data.iter().map(|&b| b ^ key).collect()
    }

    pub fn xor_many(data: &[u8], key: &[u8]) -> Vec<u8> {
        if key.is_empty() {
            return data.to_vec();
        }
        data.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect()
    }

    pub fn rotate_left(data: &[u8], amount: u32, group_size: usize) -> Result<Vec<u8>, String> {
        if group_size != 1 {
            return Err(format!("unable to rotate group of {} bytes yet", group_size));
        }
        let mask = group_size * 8 - 1;
        let amount = amount & (mask as u32);
        let anti_amount = (8 - amount) & 7;
        
        let result = data.iter()
            .map(|&b| {
                ((b << amount) & 0xff) | (b >> anti_amount)
            })
            .collect();
        Ok(result)
    }

    pub fn zlib_decompress(data: &[u8]) -> Option<Vec<u8>> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(data);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf).ok().map(|_| buf)
    }
}

pub fn bytes_strip_right(data: &[u8], pad_byte: u8) -> Vec<u8> {
    let mut new_len = data.len();
    while new_len > 0 && data[new_len - 1] == pad_byte {
        new_len -= 1;
    }
    data[..new_len].to_vec()
}

pub fn bytes_terminate(data: &[u8], term: u8, include: bool) -> Vec<u8> {
    let mut new_len = 0;
    let max_len = data.len();
    while new_len < max_len && data[new_len] != term {
        new_len += 1;
    }
    if include && new_len < max_len {
        new_len += 1;
    }
    data[..new_len].to_vec()
}

pub fn byte_array_compare(a: &[u8], b: &[u8]) -> i32 {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        let cmp = a[i] as i32 - b[i] as i32;
        if cmp != 0 {
            return cmp;
        }
    }
    a.len() as i32 - b.len() as i32
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
