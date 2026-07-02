//! String decoder: handles fixed-length and dynamic strings with BOM detection.
//!
//! Supported encodings: UTF-8, UTF-16BE, UTF-16LE (auto-detected via BOM).

use super::{Decoder, DecoderError, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
enum StringEncoding {
    Utf8,
    Utf16Be,
    Utf16Le,
}

impl Decoder {
    /// Decode a string field.
    pub(super) fn decode_string<'a>(
        &self,
        data: &'a [u8],
        length: Option<u32>,
    ) -> Result<(Value, &'a [u8]), DecoderError> {
        match length {
            Some(fixed_len) => self.decode_fixed_string(data, fixed_len as usize),
            None => self.decode_dynamic_string(data),
        }
    }

    /// Dynamic string: length prefix + BOM + content + terminator.
    fn decode_dynamic_string<'a>(&self, data: &'a [u8]) -> Result<(Value, &'a [u8]), DecoderError> {
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
        let content_bytes = extract_string_content(&string_data[bom_len..], encoding)?;

        // 4. Convert to Rust String
        let s = decode_string_content(content_bytes, encoding)?;

        Ok((Value::Str(s), &remaining[byte_len..]))
    }

    /// Fixed-length string: fixed-width buffer with BOM + content + terminator + padding.
    fn decode_fixed_string<'a>(
        &self,
        data: &'a [u8],
        fixed_len: usize,
    ) -> Result<(Value, &'a [u8]), DecoderError> {
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
        let content_bytes = extract_string_content(&string_data[bom_len..], encoding)?;

        // 3. Convert; trim trailing null padding bytes
        let s = decode_string_content(content_bytes, encoding)?
            .trim_end_matches('\0')
            .to_string();

        Ok((Value::Str(s), &data[fixed_len..]))
    }
}

/// Detect encoding from Byte Order Mark (BOM).
///
/// Returns `(encoding, bom_byte_length)`.
/// Defaults to UTF-8 without BOM if no BOM detected.
fn detect_bom(data: &[u8]) -> Result<(StringEncoding, usize), DecoderError> {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        Ok((StringEncoding::Utf8, 3))
    } else if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        Ok((StringEncoding::Utf16Be, 2))
    } else if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
        Ok((StringEncoding::Utf16Le, 2))
    } else {
        // Require BOM — no default encoding fallback (matching Go converter behavior)
        Err(DecoderError::InvalidData(
            "unknown encoding: no valid BOM found".into(),
        ))
    }
}

/// Extract content bytes up to (but not including) the encoding-specific terminator.
fn extract_string_content(data: &[u8], encoding: StringEncoding) -> Result<&[u8], DecoderError> {
    match encoding {
        StringEncoding::Utf8 => {
            let pos = data
                .iter()
                .position(|&b| b == 0x00)
                .ok_or_else(|| DecoderError::InvalidData("string terminator not found".into()))?;
            Ok(&data[..pos])
        }
        StringEncoding::Utf16Be | StringEncoding::Utf16Le => {
            // Search on 2-byte boundaries to avoid false positives
            let pos = data
                .chunks_exact(2)
                .position(|chunk| chunk == [0x00, 0x00])
                .ok_or_else(|| DecoderError::InvalidData("string terminator not found".into()))?;
            Ok(&data[..pos * 2])
        }
    }
}

/// Convert raw bytes to String based on detected encoding.
fn decode_string_content(data: &[u8], encoding: StringEncoding) -> Result<String, DecoderError> {
    match encoding {
        StringEncoding::Utf8 => String::from_utf8(data.to_vec())
            .map_err(|e| DecoderError::InvalidData(format!("invalid UTF-8: {}", e))),
        StringEncoding::Utf16Be => decode_utf16(data, true),
        StringEncoding::Utf16Le => decode_utf16(data, false),
    }
}

/// Decode UTF-16 bytes (big or little endian) to a String.
fn decode_utf16(data: &[u8], big_endian: bool) -> Result<String, DecoderError> {
    if !data.len().is_multiple_of(2) {
        return Err(DecoderError::InvalidData(
            "UTF-16 data length must be even".into(),
        ));
    }

    let u16s: Vec<u16> = data
        .chunks_exact(2)
        .map(|chunk| {
            if big_endian {
                u16::from_be_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_le_bytes([chunk[0], chunk[1]])
            }
        })
        .collect();

    String::from_utf16(&u16s)
        .map_err(|e| DecoderError::InvalidData(format!("invalid UTF-16: {:?}", e)))
}
