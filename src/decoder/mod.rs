//! Binary decoder for IDL-defined structs.
//!
//! Given a parsed `Module` AST and a `&[u8]` buffer, decodes bytes into
//! structured `Value` types according to struct field definitions.
//!
//! This crate handles the "outer struct" layer (wire format decoding).
//! CAN signal-level decoding is handled by `arxml_converter_rs`.

use crate::ast::{Module, ModuleContent, Struct, TypeRef};
use std::collections::HashMap;
use std::fmt;

mod codec;
mod string;

// ============================================================================
// Value type
// ============================================================================

/// All possible decoded values from IDL binary decoding.
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
    /// Raw bytes, used for sequence<octet> payloads that will be
    /// passed to arxml_converter_rs for CAN signal extraction.
    Bytes(Vec<u8>),
}

// ============================================================================
// DecoderConfig
// ============================================================================

/// Configuration for binary decoding behavior.
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

    /// Number of bytes to skip before starting decode.
    pub header_length: usize,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            is_little_endian: false,
            length_field_length: 4,
            enable_array_length_header: false,
            padding_length: 1,
            header_length: 0,
        }
    }
}

// ============================================================================
// DecoderError
// ============================================================================

/// Errors that can occur during binary decoding.
#[derive(Debug)]
pub enum DecoderError {
    InvalidConfig(String),
    SchemaNotFound(String),
    UnexpectedEndOfInput { expected: usize, got: usize },
    InvalidData(String),
    UnsupportedType(String),
}

impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid config: {}", msg),
            Self::SchemaNotFound(msg) => write!(f, "schema not found: {}", msg),
            Self::UnexpectedEndOfInput { expected, got } => {
                write!(
                    f,
                    "unexpected end of input: expected {} bytes, got {}",
                    expected, got
                )
            }
            Self::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            Self::UnsupportedType(msg) => write!(f, "unsupported type: {}", msg),
        }
    }
}

impl std::error::Error for DecoderError {}

// ============================================================================
// Decoder
// ============================================================================

/// Binary decoder that reads `&[u8]` data according to IDL struct definitions.
pub struct Decoder {
    config: DecoderConfig,
    module: Module,
    /// Track total bytes consumed for padding calculation.
    total_bytes: usize,
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

    /// Decode binary data using the struct identified by `schema_id`.
    ///
    /// `schema_id` is a dot-separated path, e.g., `"spi.packet"` to locate
    /// the target struct within nested modules.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let fields = decoder.decode("spi.packet", &binary_data)?;
    /// ```
    pub fn decode(
        &mut self,
        schema_id: &str,
        data: &[u8],
    ) -> Result<HashMap<String, Value>, DecoderError> {
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
        let (result, _remaining) = self.decode_struct(data, &target_struct)?;
        Ok(result)
    }
}

impl DecoderConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), DecoderError> {
        if ![1, 2, 4].contains(&self.length_field_length) {
            return Err(DecoderError::InvalidConfig(format!(
                "length_field_length must be 1, 2, or 4, got {}",
                self.length_field_length
            )));
        }
        if ![1, 2, 4, 8, 16, 32, 64].contains(&self.padding_length) {
            return Err(DecoderError::InvalidConfig(format!(
                "padding_length must be 1,2,4,8,16,32,64, got {}",
                self.padding_length
            )));
        }
        Ok(())
    }
}

// ============================================================================
// Schema navigation
// ============================================================================

impl Decoder {
    /// Navigate the module tree to find the struct identified by `schema_id`.
    fn travel_module(&self, schema_id: &str) -> Result<(Struct, &Module), DecoderError> {
        let parts: Vec<&str> = schema_id.split('.').collect();

        if parts.len() == 1 {
            // Single segment: search for struct directly in root module
            let name = parts[0];
            for content in &self.module.content {
                if let ModuleContent::Struct(st) = content {
                    if st.name == name {
                        return Ok((st.clone(), &self.module));
                    }
                }
            }
            return Err(DecoderError::SchemaNotFound(format!(
                "struct '{}' not found in module '{}'",
                name, self.module.name
            )));
        }

        // First segment must match root module name, or be skipped if it does.
        // e.g., "a.b.complex" where root module is "a".
        let start_idx = if self.module.name == parts[0] { 1 } else { 0 };

        if start_idx >= parts.len() {
            return Err(DecoderError::SchemaNotFound(format!(
                "invalid schema_id '{}'",
                schema_id
            )));
        }

        Self::travel(&parts, start_idx, &self.module)
    }

    fn travel<'a>(
        parts: &[&str],
        idx: usize,
        module: &'a Module,
    ) -> Result<(Struct, &'a Module), DecoderError> {
        let target = parts[idx];

        if idx == parts.len() - 1 {
            for content in &module.content {
                if let ModuleContent::Struct(st) = content {
                    if st.name == target {
                        return Ok((st.clone(), module));
                    }
                }
            }
            return Err(DecoderError::SchemaNotFound(format!(
                "struct '{}' not found in module '{}'",
                target, module.name
            )));
        }

        for content in &module.content {
            if let ModuleContent::Module(sub_module) = content {
                if sub_module.name == target {
                    return Self::travel(parts, idx + 1, sub_module);
                }
            }
        }

        Err(DecoderError::SchemaNotFound(format!(
            "module '{}' not found in '{}'",
            target, module.name
        )))
    }
}

// ============================================================================
// Padding
// ============================================================================

impl Decoder {
    /// Whether a field type requires padding after decoding.
    /// Dynamic strings and sequences have variable encoded length.
    fn needs_padding(&self, type_ref: &TypeRef) -> bool {
        matches!(
            type_ref,
            TypeRef::String { length: None } | TypeRef::Sequence { .. }
        )
    }

    /// Consume padding bytes to align to `padding_length` boundary.
    fn consume_padding<'a>(&self, data: &'a [u8]) -> Result<&'a [u8], DecoderError> {
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
