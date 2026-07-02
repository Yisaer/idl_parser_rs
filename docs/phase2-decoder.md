# Phase 2: IDL Decoder (Binary Decoding)

## Overview

Given a parsed IDL `Module` AST and a `&[u8]` binary buffer, decode the bytes into
structured `Value` types according to the struct definitions. This phase implements the
binary wire-format decoding — the "outer struct" layer that produces typed fields like
`ts`, `id`, `len`, and raw `payload` bytes for downstream signal-level decoding by
`arxml_converter_rs`.

## Value Type

The decoder's output must handle all supported IDL types. We use a closed enum for type
safety and exhaustiveness checking:

```rust
// src/decoder/value.rs

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Str(String),
    List(Vec<Value>),
    Struct(HashMap<String, Value>),
    Bytes(Vec<u8>),  // Raw bytes for sequence<octet> payloads
}
```

### Design Decision: Why enum over serde_json::Value

| Approach | Pros | Cons |
|----------|------|------|
| `serde_json::Value` | Universal, easy to serialize | Loses type precision: `u8` becomes `Number(42)` — downstream can't tell `u8` from `i64` |
| `enum Value` (ours) | Exact type preservation, match-type-safe consumers | Requires manual Display/Serialize impl |

For the GBF use case, downstream consumers (veloFlux, `arxml_converter_rs`) need to know
exact bit widths. An `i16` must stay `i16`, not get collapsed into a generic `Number`.
Verdict: custom `Value` enum.

Additional note: `Bytes(Vec<u8>)` is separate from `List(Vec<Value>)` to distinguish raw
octet sequences from structured lists. This matters for `sequence<octet>` payloads that
will be fed to `arxml_converter_rs` for signal extraction.

## DecoderConfig

All parameters that affect decoding behavior are collected into a single config struct.
This mirrors the Go `IDlConverterConfig`:

```rust
// src/decoder/config.rs

#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// Byte order for multi-byte numeric types.
    pub is_little_endian: bool,

    /// Length of the length-prefix field for dynamic-length types
    /// (sequence, dynamic string). Must be 1, 2, or 4.
    pub length_field_length: u8,

    /// If true, array decoding first reads a length header and validates
    /// it matches the declared array size.
    pub enable_array_length_header: bool,

    /// Alignment boundary for padding after variable-length fields.
    /// Must be 1, 2, 4, 8, 16, 32, or 64.
    pub padding_length: usize,

    /// Number of bytes to skip before starting decode (e.g., for protocol headers).
    pub header_length: usize,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            is_little_endian: false,          // Big-endian default (CAN convention)
            length_field_length: 4,            // 4-byte length prefix default
            enable_array_length_header: false,
            padding_length: 1,                 // No padding by default
            header_length: 0,
        }
    }
}

impl DecoderConfig {
    pub fn validate(&self) -> Result<(), DecoderError> {
        if ![1, 2, 4].contains(&self.length_field_length) {
            return Err(DecoderError::InvalidConfig(
                format!("length_field_length must be 1, 2, or 4, got {}", self.length_field_length)
            ));
        }
        if ![1, 2, 4, 8, 16, 32, 64].contains(&self.padding_length) {
            return Err(DecoderError::InvalidConfig(
                format!("padding_length must be 1,2,4,8,16,32,64, got {}", self.padding_length)
            ));
        }
        Ok(())
    }
}
```

## Decoder Struct & Public API

```rust
// src/decoder/mod.rs

use crate::ast::Module;
use std::collections::HashMap;

pub mod config;
pub mod value;

pub use config::DecoderConfig;
pub use value::Value;

#[derive(Debug)]
pub enum DecoderError {
    InvalidConfig(String),
    SchemaNotFound(String),
    NotAStruct(String),
    UnexpectedEndOfInput { expected: usize, got: usize },
    TypeMismatch { expected: String, actual: String },
    InvalidData(String),
}

pub struct Decoder {
    config: DecoderConfig,
    module: Module,
    total_bytes: usize,  // Track total consumed for padding calculation
}

impl Decoder {
    /// Create a new decoder from a parsed IDL Module.
    pub fn new(config: DecoderConfig, module: Module) -> Result<Self, DecoderError> {
        config.validate()?;
        Ok(Self {
            config,
            module,
            total_bytes: 0,
        })
    }

    /// Decode binary data using the struct identified by schema_id.
    ///
    /// `schema_id` is a dot-separated path, e.g., `"spi.packet"` to locate
    /// the target struct within nested modules.
    pub fn decode(&mut self, schema_id: &str, data: &[u8]) -> Result<HashMap<String, Value>, DecoderError> {
        // 1. Skip header_length bytes
        let data = if self.config.header_length > 0 {
            if data.len() < self.config.header_length {
                return Err(DecoderError::UnexpectedEndOfInput {
                    expected: self.config.header_length,
                    got: data.len(),
                });
            }
            &data[self.config.header_length..]
        } else {
            data
        };

        self.total_bytes = data.len() + self.config.header_length;

        // 2. Navigate to target struct
        let (target_struct, _target_module) = self.travel_module(schema_id)?;

        // 3. Decode
        let (result, _remaining) = self.decode_struct(data, target_struct)?;
        Ok(result)
    }

    /// Step-wise API: decode only to frame level.
    /// Returns each frame's id, payload bytes, and any @format annotation value.
    /// The caller (veloFlux) then passes (id, payload) to arxml_converter_rs
    /// for CAN signal extraction.
    pub fn decode_frames(
        &mut self,
        schema_id: &str,
        data: &[u8],
    ) -> Result<Vec<DecodedFrame>, DecoderError> {
        let fields = self.decode(schema_id, data)?;
        // Extract frames from the decoded result
        // Implementation: walk the struct, find sequence<frame> fields,
        // extract per-frame (id, payload, annotation)
        todo!("see veloFlux integration section")
    }
}

/// A single decoded frame, ready for signal-level decoding by arxml_converter_rs.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub can_id: u32,
    pub payload: Vec<u8>,
    pub format_annotation: Option<String>,  // e.g., "gbf/sim.json" from @format(dbc="...")
}
```

## Internal Decoding Architecture

### Dispatch by TypeRef

The core decode loop dispatches on `TypeRef` variants via pattern matching. Each variant
has a dedicated decode function. This is the equivalent of Go's `ParseDataByType` switch:

```rust
impl Decoder {
    fn decode_by_type(&self, data: &[u8], type_ref: &TypeRef) -> Result<(Value, &[u8]), DecoderError> {
        match type_ref {
            TypeRef::Octet        => self.decode_u8(data),
            TypeRef::Short        => self.decode_i16(data),
            TypeRef::UnsignedShort=> self.decode_u16(data),
            TypeRef::Long         => self.decode_i32(data),
            TypeRef::UnsignedLong => self.decode_u32(data),
            TypeRef::LongLong     => self.decode_i64(data),
            TypeRef::UnsignedLongLong => self.decode_u64(data),
            TypeRef::Float        => self.decode_f32(data),
            TypeRef::Double       => self.decode_f64(data),
            TypeRef::Boolean      => self.decode_bool(data),
            TypeRef::String { length } => self.decode_string(data, *length),
            TypeRef::Array { inner, size } => self.decode_array(data, inner, *size),
            TypeRef::Sequence { inner } => self.decode_sequence(data, inner),
            TypeRef::BitField { .. } => Err(DecoderError::TypeMismatch {
                expected: "bitfield in bitset only".into(),
                actual: "bitfield in struct".into(),
            }),
            TypeRef::TypeName { name } => self.decode_type_name(data, name),
        }
    }
}
```

### Primitive Type Decoders

Use `byteorder` crate for endian-aware multi-byte reads. Single-byte types and booleans
are endian-independent:

```rust
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

impl Decoder {
    fn endian(&self) -> byteorder::NativeEndian { /* placeholder */ }

    fn read_bytes(&self, data: &[u8], n: usize) -> Result<(&[u8], &[u8]), DecoderError> {
        if data.len() < n {
            Err(DecoderError::UnexpectedEndOfInput { expected: n, got: data.len() })
        } else {
            Ok(data.split_at(n))
        }
    }

    fn decode_u8(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 1)?;
        Ok((Value::U8(chunk[0]), rest))
    }

    fn decode_i16(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 2)?;
        let val = if self.config.is_little_endian {
            i16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            i16::from_be_bytes([chunk[0], chunk[1]])
        };
        Ok((Value::I16(val), rest))
    }

    fn decode_u16(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 2)?;
        let val = if self.config.is_little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        Ok((Value::U16(val), rest))
    }

    fn decode_i32(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 4)?;
        let val = if self.config.is_little_endian {
            i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        Ok((Value::I32(val), rest))
    }

    fn decode_u32(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 4)?;
        let val = if self.config.is_little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        Ok((Value::U32(val), rest))
    }

    fn decode_i64(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 8)?;
        let val = if self.config.is_little_endian {
            i64::from_le_bytes(chunk.try_into().unwrap())
        } else {
            i64::from_be_bytes(chunk.try_into().unwrap())
        };
        Ok((Value::I64(val), rest))
    }

    fn decode_u64(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 8)?;
        let val = if self.config.is_little_endian {
            u64::from_le_bytes(chunk.try_into().unwrap())
        } else {
            u64::from_be_bytes(chunk.try_into().unwrap())
        };
        Ok((Value::U64(val), rest))
    }

    fn decode_f32(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 4)?;
        let bits = if self.config.is_little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        Ok((Value::F32(f32::from_bits(bits)), rest))
    }

    fn decode_f64(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 8)?;
        let bits = if self.config.is_little_endian {
            u64::from_le_bytes(chunk.try_into().unwrap())
        } else {
            u64::from_be_bytes(chunk.try_into().unwrap())
        };
        Ok((Value::F64(f64::from_bits(bits)), rest))
    }

    fn decode_bool(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        let (chunk, rest) = self.read_bytes(data, 1)?;
        Ok((Value::Bool(chunk[0] != 0), rest))
    }
}
```

**Note**: Instead of `byteorder` crate, we use Rust's built-in `from_le_bytes`/`from_be_bytes`
methods. These were stabilized in Rust 1.32 and are zero-cost. The `byteorder` crate is
unnecessary for fixed-size types; it may still be useful for streaming reads if needed.

### Struct Decoder

```rust
impl Decoder {
    fn decode_struct(
        &self,
        data: &[u8],
        st: &Struct,
    ) -> Result<(HashMap<String, Value>, &[u8]), DecoderError> {
        let mut remaining = data;
        let mut fields = HashMap::with_capacity(st.fields.len());

        for field in &st.fields {
            let (value, rest) = self.decode_by_type(remaining, &field.field_type)?;
            fields.insert(field.name.clone(), value);
            remaining = rest;

            // Apply padding after variable-length fields
            if self.needs_padding(&field.field_type) && !remaining.is_empty() {
                remaining = self.consume_padding(remaining)?;
            }
        }

        Ok((fields, remaining))
    }

    /// Fields that require padding after decoding: dynamic strings and sequences.
    /// These have variable encoded length, so padding aligns the next field.
    fn needs_padding(&self, type_ref: &TypeRef) -> bool {
        matches!(type_ref,
            TypeRef::String { length: None } |
            TypeRef::Sequence { .. }
        )
    }
}
```

### Array Decoder

```rust
impl Decoder {
    fn decode_array(
        &self,
        data: &[u8],
        inner_type: &TypeRef,
        size: u32,
    ) -> Result<(Value, &[u8]), DecoderError> {
        let mut remaining = data;

        // Optional: read and validate length header
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
            let (value, rest) = self.decode_by_type(remaining, inner_type)
                .map_err(|e| DecoderError::InvalidData(
                    format!("array element {}: {}", i, e)
                ))?;
            elements.push(value);
            remaining = rest;
        }

        Ok((Value::List(elements), remaining))
    }
}
```

### Sequence Decoder (Length-Prefixed)

```rust
impl Decoder {
    /// Read a length field of configurable width (1, 2, or 4 bytes).
    fn decode_length_field(&self, data: &[u8]) -> Result<(usize, &[u8]), DecoderError> {
        match self.config.length_field_length {
            1 => {
                let (chunk, rest) = self.read_bytes(data, 1)?;
                Ok((chunk[0] as usize, rest))
            }
            2 => {
                let (chunk, rest) = self.read_bytes(data, 2)?;
                let val = if self.config.is_little_endian {
                    u16::from_le_bytes([chunk[0], chunk[1]])
                } else {
                    u16::from_be_bytes([chunk[0], chunk[1]])
                };
                Ok((val as usize, rest))
            }
            4 => {
                let (chunk, rest) = self.read_bytes(data, 4)?;
                let val = if self.config.is_little_endian {
                    u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                } else {
                    u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                };
                Ok((val as usize, rest))
            }
            n => Err(DecoderError::InvalidConfig(format!(
                "invalid length_field_length {}", n
            ))),
        }
    }

    /// Decode a length-prefixed sequence.
    ///
    /// The sequence data is `total_byte_length` bytes long. We parse elements
    /// from that slice until exhausted, then verify we consumed exactly all bytes.
    fn decode_sequence(
        &self,
        data: &[u8],
        inner_type: &TypeRef,
    ) -> Result<(Value, &[u8]), DecoderError> {
        // 1. Read the total byte length of this sequence
        let (total_byte_len, remaining) = self.decode_length_field(data)?;

        if remaining.len() < total_byte_len {
            return Err(DecoderError::UnexpectedEndOfInput {
                expected: total_byte_len,
                got: remaining.len(),
            });
        }

        // 2. Decode elements from the byte slice
        let mut seq_data = &remaining[..total_byte_len];
        let mut elements = Vec::new();
        let mut index = 0;

        while !seq_data.is_empty() {
            let (value, rest) = self.decode_by_type(seq_data, inner_type)
                .map_err(|e| DecoderError::InvalidData(
                    format!("sequence element {}: {}", index, e)
                ))?;
            elements.push(value);
            seq_data = rest;
            index += 1;
        }

        // 3. Verify exact consumption
        // seq_data should now be empty (all bytes consumed by elements)

        Ok((Value::List(elements), &remaining[total_byte_len..]))
    }
}
```

### String Decoder

Handle both fixed-length and dynamic (BOM-prefixed, terminator-suffixed) strings.
Supported encodings: UTF-8, UTF-16BE, UTF-16LE (auto-detected via BOM).

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum StringEncoding {
    Utf8,
    Utf16Be,
    Utf16Le,
}

impl Decoder {
    fn decode_string(
        &self,
        data: &[u8],
        length: Option<u32>,
    ) -> Result<(Value, &[u8]), DecoderError> {
        match length {
            Some(fixed_len) => self.decode_fixed_string(data, fixed_len as usize),
            None => self.decode_dynamic_string(data),
        }
    }

    /// Dynamic string: length prefix + BOM + content + terminator.
    fn decode_dynamic_string(&self, data: &[u8]) -> Result<(Value, &[u8]), DecoderError> {
        // 1. Read string byte length
        let (byte_len, remaining) = self.decode_length_field(data)?;

        if remaining.len() < byte_len {
            return Err(DecoderError::UnexpectedEndOfInput {
                expected: byte_len,
                got: remaining.len(),
            });
        }

        let string_data = &remaining[..byte_len];

        // 2. Detect encoding from BOM
        let (encoding, bom_len) = detect_bom(string_data)?;

        // 3. Find terminator and extract content
        let content_bytes = extract_string_content(
            &string_data[bom_len..],
            encoding,
        )?;

        // 4. Convert to Rust String
        let s = decode_string_content(content_bytes, encoding)?;

        Ok((Value::Str(s), &remaining[byte_len..]))
    }

    /// Fixed-length string: fixed-width buffer with BOM + content + terminator.
    fn decode_fixed_string(
        &self,
        data: &[u8],
        fixed_len: usize,
    ) -> Result<(Value, &[u8]), DecoderError> {
        if data.len() < fixed_len {
            return Err(DecoderError::UnexpectedEndOfInput {
                expected: fixed_len,
                got: data.len(),
            });
        }

        let string_data = &data[..fixed_len];

        // 1. Detect encoding from BOM
        let (encoding, bom_len) = detect_bom(string_data)?;

        // 2. Find terminator and extract content
        let content_bytes = extract_string_content(
            &string_data[bom_len..],
            encoding,
        )?;

        // 3. Convert + trim padding
        let s = decode_string_content(content_bytes, encoding)?
            .trim_end_matches('\0')
            .to_string();

        Ok((Value::Str(s), &data[fixed_len..]))
    }
}

fn detect_bom(data: &[u8]) -> Result<(StringEncoding, usize), DecoderError> {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        Ok((StringEncoding::Utf8, 3))
    } else if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        Ok((StringEncoding::Utf16Be, 2))
    } else if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
        Ok((StringEncoding::Utf16Le, 2))
    } else {
        // Default: assume UTF-8 without BOM
        Ok((StringEncoding::Utf8, 0))
    }
}

fn extract_string_content(data: &[u8], encoding: StringEncoding) -> Result<&[u8], DecoderError> {
    let terminator: &[u8] = match encoding {
        StringEncoding::Utf8 => &[0x00],
        StringEncoding::Utf16Be | StringEncoding::Utf16Le => &[0x00, 0x00],
    };

    // Find terminator position
    let pos = data
        .windows(terminator.len())
        .position(|window| window == terminator)
        .ok_or_else(|| DecoderError::InvalidData("string terminator not found".into()))?;

    Ok(&data[..pos])
}

fn decode_string_content(data: &[u8], encoding: StringEncoding) -> Result<String, DecoderError> {
    match encoding {
        StringEncoding::Utf8 => {
            String::from_utf8(data.to_vec())
                .map_err(|e| DecoderError::InvalidData(format!("invalid UTF-8: {}", e)))
        }
        StringEncoding::Utf16Be => decode_utf16(data, byteorder::BigEndian),
        StringEncoding::Utf16Le => decode_utf16(data, byteorder::LittleEndian),
    }
}
```

### Padding Logic

After each dynamic-length field (dynamic string, sequence), the remaining data must be
aligned to `padding_length` boundaries:

```rust
impl Decoder {
    fn consume_padding(&self, data: &[u8]) -> Result<&[u8], DecoderError> {
        let consumed = self.total_bytes - data.len();
        let remainder = consumed % self.config.padding_length;
        if remainder == 0 {
            return Ok(data);
        }
        let padding_needed = self.config.padding_length - remainder;
        if padding_needed > data.len() {
            return Err(DecoderError::UnexpectedEndOfInput {
                expected: padding_needed,
                got: data.len(),
            });
        }
        Ok(&data[padding_needed..])
    }
}
```

### Schema Navigation (travel_module)

Given a dot-separated `schema_id` like `"spi"` or `"spi.packet"`, navigate the nested
module tree to find the target struct:

```rust
impl Decoder {
    fn travel_module(&self, schema_id: &str) -> Result<(&Struct, &Module), DecoderError> {
        let parts: Vec<&str> = schema_id.split('.').collect();
        // Wrap root in a virtual "header" module (matches Go behavior)
        let root = Module {
            name: "header".into(),
            content: vec![ModuleContent::Module(Box::new(self.module.clone()))],
        };
        self.travel(&parts, 0, &root)
    }

    fn travel(
        &self,
        parts: &[&str],
        idx: usize,
        module: &Module,
    ) -> Result<(&Struct, &Module), DecoderError> {
        let target = parts[idx];

        if idx == parts.len() - 1 {
            // Last segment: must be a struct
            for content in &module.content {
                if let ModuleContent::Struct(st) = content {
                    if st.name == target {
                        return Ok((st, module));
                    }
                }
            }
            return Err(DecoderError::SchemaNotFound(format!(
                "struct '{}' not found in module '{}'", target, module.name
            )));
        }

        // Intermediate segment: must be a sub-module
        for content in &module.content {
            if let ModuleContent::Module(sub_module) = content {
                if sub_module.name == target {
                    return self.travel(parts, idx + 1, sub_module);
                }
            }
        }

        Err(DecoderError::SchemaNotFound(format!(
            "module '{}' not found in '{}'", target, module.name
        )))
    }
}
```

## Integration with arxml_converter_rs

The `@format(dbc="...")` annotation on a `sequence<frame>` field signals that each
frame's payload needs CAN signal decoding. The decoder does NOT do this itself — it
preserves the annotation metadata for the caller (veloFlux).

### Integration Flow

```
┌─ veloFlux GBF Stream ──────────────────────────────────────────────┐
│                                                                     │
│  1. Parse IDL text → Module AST                                     │
│  2. Parse ARXML/DBC → signal definitions                            │
│  3. Receive binary data                                             │
│                                                                     │
│  4. idl_parser_rs::Decoder::decode("spi.packet", data)              │
│     → { "ts": 1731316891295,                                        │
│         "len": 88,                                                  │
│         "frames": [                                                 │
│           { "id": 1264, "len": 64, "payload": <64 raw bytes> }      │
│         ]                                                           │
│       }                                                             │
│                                                                     │
│  5. For each frame with @format annotation:                         │
│     arxml_converter_rs::decode(frame.id, frame.payload)             │
│     → { "BswAppVersion": 18446744073709551615,                      │
│         "BswCalVersion": 0, ... }                                   │
│                                                                     │
│  6. Merge: output = idl_fields + arxml_signal_fields                │
│     → { "ts": ..., "Mess0$Sig1": ..., "BswAppVersion": ..., ... }   │
└─────────────────────────────────────────────────────────────────────┘
```

### Frame-Level API Design

For step 4 above, veloFlux needs to iterate frames and extract `(can_id, payload)` pairs.
The decoder provides this through the `decode_frames` method or by the caller manually
walking the decoded `HashMap` structure. The preferred approach is a typed return from
`decode_frames`:

```rust
pub struct DecodedFrame {
    pub can_id: u32,
    pub payload: Vec<u8>,
    pub format_annotation: Option<String>,
}
```

This way, veloFlux can:

```rust
let decoder = Decoder::new(config, module)?;
let frames = decoder.decode_frames("spi.packet", &binary_data)?;

for frame in &frames {
    if let Some(dbc_path) = &frame.format_annotation {
        let signals = arxml_converter.decode(frame.can_id, &frame.payload)?;
        // merge signals into output record
    }
}
```

## Testing Strategy

### Unit Tests

Each primitive decoder function:

```rust
#[test]
fn test_decode_u16_big_endian() {
    let config = DecoderConfig { is_little_endian: false, ..Default::default() };
    let decoder = Decoder::new(config, empty_module()).unwrap();
    let (val, rest) = decoder.decode_u16(&[0x01, 0x02, 0x03]).unwrap();
    assert_eq!(val, Value::U16(258));  // 0x0102 = 258
    assert_eq!(rest, &[0x03]);
}

#[test]
fn test_decode_u16_little_endian() {
    let config = DecoderConfig { is_little_endian: true, ..Default::default() };
    let decoder = Decoder::new(config, empty_module()).unwrap();
    let (val, rest) = decoder.decode_u16(&[0x01, 0x02, 0x03]).unwrap();
    assert_eq!(val, Value::U16(513));  // 0x0201 = 513
    assert_eq!(rest, &[0x03]);
}
```

### Integration Tests (End-to-End)

Each test: IDL text + binary data → expected structured output.

```rust
#[test]
fn test_decode_simple_struct() {
    let idl = r#"
        module spi {
            struct frame {
                unsigned long id;
                unsigned long len;
                sequence<octet> payload;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    let config = DecoderConfig::default();

    let mut decoder = Decoder::new(config, module).unwrap();

    // Binary: id=0x4F0 (4 bytes), len=3 (4 bytes), seq_len=3 (4 bytes), payload=[1,2,3]
    let data: Vec<u8> = [
        &[0x00, 0x00, 0x04, 0xF0u8][..],  // id = 1264
        &[0x00, 0x00, 0x00, 0x03u8][..],  // len = 3
        &[0x00, 0x00, 0x00, 0x03u8][..],  // sequence byte length = 3
        &[0x01, 0x02, 0x03][..],           // payload
    ].concat();

    let result = decoder.decode("spi.frame", &data).unwrap();

    let mut expected = HashMap::new();
    expected.insert("id".into(), Value::U32(1264));
    expected.insert("len".into(), Value::U32(3));
    expected.insert("payload".into(), Value::List(vec![
        Value::U8(1), Value::U8(2), Value::U8(3),
    ]));

    assert_eq!(result, expected);
}
```

### Test Coverage Target

| Category | Coverage |
|----------|----------|
| All primitive types (both endians) | 100% |
| Struct with all field types | 100% |
| Array (with/without length header) | 100% |
| Sequence (length-prefixed) | 100% |
| String (fixed + dynamic, UTF-8/UTF-16) | 100% |
| Padding after dynamic fields | 100% |
| Schema navigation (nested modules) | 100% |
| Error cases (short data, missing struct) | 100% |

## Go → Rust Decoder Comparison

| Feature | Go converter | Rust decoder |
|---------|-------------|--------------|
| Config | `IDlConverterConfig` struct (json tags) | `DecoderConfig` struct (no serde required) |
| Output | `map[string]interface{}` (type-erased) | `HashMap<String, Value>` (typed enum) |
| String decode | BOM detection + manual loops | BOM detection + `String::from_utf8`/`from_utf16` |
| Endian handling | `binary.LittleEndian.Uint16()` etc. | Built-in `from_le_bytes`/`from_be_bytes` |
| Error handling | `(T, []byte, error)` triple return | `Result<(Value, &[u8]), DecoderError>` |
| Padding state | `TotalBytes` field on converter | `total_bytes` field on decoder |
| Schema init | Reads file from `SchemaPath` | Accepts parsed `Module` (caller handles I/O) |
| Travel | `list.Element` linked list traversal | Slice index recursion |

## Dependencies

```toml
[dependencies]
thiserror = "2"    # Ergonomic error enum derive
```

No `byteorder` needed — Rust's `from_le_bytes`/`from_be_bytes` cover all fixed-size types.
If streaming reads become necessary later, consider `byteorder`.
