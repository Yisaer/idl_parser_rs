//! Core decode dispatch and primitive type decoders.
//!
//! The main dispatch function `decode_by_type` matches on `TypeRef` variants
//! and delegates to the appropriate primitive or composite decoder.

use crate::ast::{Struct, TypeRef};
use std::collections::HashMap;

use super::{Decoder, DecoderError, Value};

impl Decoder {
    // ========================================================================
    // Dispatch
    // ========================================================================

    /// Decode data according to the given type reference.
    /// Returns the decoded value and the remaining unread bytes.
    pub(super) fn decode_by_type<'a>(
        &self,
        data: &'a [u8],
        type_ref: &TypeRef,
    ) -> Result<(Value, &'a [u8]), DecoderError> {
        match type_ref {
            TypeRef::Octet => self.decode_u8(data),
            TypeRef::Short => self.decode_i16(data),
            TypeRef::UnsignedShort => self.decode_u16(data),
            TypeRef::Long => self.decode_i32(data),
            TypeRef::UnsignedLong => self.decode_u32(data),
            TypeRef::LongLong => self.decode_i64(data),
            TypeRef::UnsignedLongLong => self.decode_u64(data),
            TypeRef::Float => self.decode_f32(data),
            TypeRef::Double => self.decode_f64(data),
            TypeRef::Boolean => self.decode_bool(data),
            TypeRef::String { length } => self.decode_string(data, *length),
            TypeRef::Array { inner, size } => self.decode_array(data, inner, *size),
            TypeRef::Sequence { inner } => self.decode_sequence(data, inner),
            TypeRef::BitField { .. } => Err(DecoderError::UnsupportedType(
                "bitfield decoding not supported in struct fields".into(),
            )),
            TypeRef::TypeName { name } => self.decode_type_name(data, name),
        }
    }

    // ========================================================================
    // Composite decoders
    // ========================================================================

    /// Decode a struct's fields sequentially from the data buffer.
    pub(super) fn decode_struct<'a>(
        &self,
        data: &'a [u8],
        st: &Struct,
    ) -> Result<(HashMap<String, Value>, &'a [u8]), DecoderError> {
        let mut remaining = data;
        let mut fields = HashMap::with_capacity(st.fields.len());

        for field in &st.fields {
            let (value, rest) = self.decode_by_type(remaining, &field.field_type)?;
            fields.insert(field.name.clone(), value);
            remaining = rest;

            if self.needs_padding(&field.field_type) && !remaining.is_empty() {
                remaining = self.consume_padding(remaining)?;
            }
        }

        Ok((fields, remaining))
    }

    /// Decode a fixed-size array.
    fn decode_array<'a>(
        &self,
        data: &'a [u8],
        inner_type: &TypeRef,
        size: u32,
    ) -> Result<(Value, &'a [u8]), DecoderError> {
        let mut remaining = data;

        if self.config.enable_array_length_header {
            let (length, rest) = self.decode_length_field(remaining)?;
            if length != size as usize {
                return Err(DecoderError::InvalidData(format!(
                    "array length header {} does not match declared size {}",
                    length, size
                )));
            }
            remaining = rest;
        }

        let mut elements = Vec::with_capacity(size as usize);
        for i in 0..size {
            let (value, rest) = self
                .decode_by_type(remaining, inner_type)
                .map_err(|e| DecoderError::InvalidData(format!("array element {}: {}", i, e)))?;
            elements.push(value);
            remaining = rest;
        }

        if matches!(inner_type, TypeRef::Octet) {
            let bytes: Vec<u8> = elements
                .into_iter()
                .filter_map(|v| match v {
                    Value::U8(b) => Some(b),
                    _ => None,
                })
                .collect();
            return Ok((Value::Bytes(bytes), remaining));
        }

        Ok((Value::List(elements), remaining))
    }

    /// Decode a length-prefixed sequence.
    fn decode_sequence<'a>(
        &self,
        data: &'a [u8],
        inner_type: &TypeRef,
    ) -> Result<(Value, &'a [u8]), DecoderError> {
        let (total_byte_len, remaining) = self.decode_length_field(data)?;

        if remaining.len() < total_byte_len {
            return Err(DecoderError::UnexpectedEndOfInput {
                expected: total_byte_len,
                got: remaining.len(),
            });
        }

        let mut seq_data = &remaining[..total_byte_len];
        let mut elements = Vec::new();
        let mut index = 0;

        while !seq_data.is_empty() {
            let (value, rest) = self.decode_by_type(seq_data, inner_type).map_err(|e| {
                DecoderError::InvalidData(format!("sequence element {}: {}", index, e))
            })?;
            elements.push(value);
            seq_data = rest;
            index += 1;
        }

        if matches!(inner_type, TypeRef::Octet) {
            let bytes: Vec<u8> = elements
                .into_iter()
                .filter_map(|v| match v {
                    Value::U8(b) => Some(b),
                    _ => None,
                })
                .collect();
            return Ok((Value::Bytes(bytes), &remaining[total_byte_len..]));
        }

        Ok((Value::List(elements), &remaining[total_byte_len..]))
    }

    /// Decode a type name (user-defined type reference).
    fn decode_type_name<'a>(
        &self,
        data: &'a [u8],
        name: &str,
    ) -> Result<(Value, &'a [u8]), DecoderError> {
        let target_struct = self.find_struct_by_name(name)?;
        let (struct_value, remaining) = self.decode_struct(data, &target_struct)?;
        Ok((Value::Struct(struct_value), remaining))
    }

    fn find_struct_by_name(&self, name: &str) -> Result<Struct, DecoderError> {
        Self::find_struct_in_module(&self.module, name)
    }

    fn find_struct_in_module(
        module: &crate::ast::Module,
        name: &str,
    ) -> Result<Struct, DecoderError> {
        for content in &module.content {
            match content {
                crate::ast::ModuleContent::Struct(s) if s.name == name => {
                    return Ok(s.clone());
                }
                crate::ast::ModuleContent::Module(m) => {
                    if let Ok(s) = Self::find_struct_in_module(m, name) {
                        return Ok(s);
                    }
                }
                _ => {}
            }
        }
        Err(DecoderError::SchemaNotFound(format!(
            "struct '{}' not found",
            name
        )))
    }
}

// ========================================================================
// Length field reader
// ========================================================================

impl Decoder {
    pub(super) fn decode_length_field<'a>(
        &self,
        data: &'a [u8],
    ) -> Result<(usize, &'a [u8]), DecoderError> {
        match self.config.length_field_length {
            1 => {
                let (chunk, rest) = read_bytes(data, 1)?;
                Ok((chunk[0] as usize, rest))
            }
            2 => {
                let (chunk, rest) = read_bytes(data, 2)?;
                let val = if self.config.is_little_endian {
                    u16::from_le_bytes([chunk[0], chunk[1]])
                } else {
                    u16::from_be_bytes([chunk[0], chunk[1]])
                };
                Ok((val as usize, rest))
            }
            4 => {
                let (chunk, rest) = read_bytes(data, 4)?;
                let val = if self.config.is_little_endian {
                    u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                } else {
                    u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                };
                Ok((val as usize, rest))
            }
            n => Err(DecoderError::InvalidConfig(format!(
                "invalid length_field_length {}",
                n
            ))),
        }
    }
}

// ========================================================================
// Primitive type decoders
// ========================================================================

impl Decoder {
    pub(super) fn decode_u8<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 1)?;
        Ok((Value::U8(chunk[0]), rest))
    }

    pub(super) fn decode_i16<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 2)?;
        let val = if self.config.is_little_endian {
            i16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            i16::from_be_bytes([chunk[0], chunk[1]])
        };
        Ok((Value::I16(val), rest))
    }

    pub(super) fn decode_u16<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 2)?;
        let val = if self.config.is_little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        Ok((Value::U16(val), rest))
    }

    pub(super) fn decode_i32<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 4)?;
        let val = if self.config.is_little_endian {
            i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        Ok((Value::I32(val), rest))
    }

    pub(super) fn decode_u32<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 4)?;
        let val = if self.config.is_little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        Ok((Value::U32(val), rest))
    }

    pub(super) fn decode_i64<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 8)?;
        let val = if self.config.is_little_endian {
            i64::from_le_bytes(chunk.try_into().unwrap())
        } else {
            i64::from_be_bytes(chunk.try_into().unwrap())
        };
        Ok((Value::I64(val), rest))
    }

    pub(super) fn decode_u64<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 8)?;
        let val = if self.config.is_little_endian {
            u64::from_le_bytes(chunk.try_into().unwrap())
        } else {
            u64::from_be_bytes(chunk.try_into().unwrap())
        };
        Ok((Value::U64(val), rest))
    }

    pub(super) fn decode_f32<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 4)?;
        let bits = if self.config.is_little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        Ok((Value::F32(f32::from_bits(bits)), rest))
    }

    pub(super) fn decode_f64<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 8)?;
        let bits = if self.config.is_little_endian {
            u64::from_le_bytes(chunk.try_into().unwrap())
        } else {
            u64::from_be_bytes(chunk.try_into().unwrap())
        };
        Ok((Value::F64(f64::from_bits(bits)), rest))
    }

    pub(super) fn decode_bool<'a>(
        &self,
        data: &'a [u8],
    ) -> Result<(Value, &'a [u8]), DecoderError> {
        let (chunk, rest) = read_bytes(data, 1)?;
        Ok((Value::Bool(chunk[0] != 0), rest))
    }
}

// ========================================================================
// Helper: read N bytes or fail
// ========================================================================

fn read_bytes(data: &[u8], n: usize) -> Result<(&[u8], &[u8]), DecoderError> {
    if data.len() < n {
        Err(DecoderError::UnexpectedEndOfInput {
            expected: n,
            got: data.len(),
        })
    } else {
        Ok(data.split_at(n))
    }
}
