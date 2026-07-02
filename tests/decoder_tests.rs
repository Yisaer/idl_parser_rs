//! Binary decoder integration tests.
//!
//! Migrated from idlparser Go `converter_test.go`, `parse_methods_test.go`,
//! and related test files.

use idl_parser_rs::decoder::{Decoder, DecoderConfig, Value};
use idl_parser_rs::parse_idl;
use std::collections::HashMap;

/// Helper: create a decoder from an IDL string and config.
fn make_decoder(idl: &str, config: DecoderConfig) -> Decoder {
    let module = parse_idl(idl).unwrap();
    Decoder::new(config, module).unwrap()
}

/// Helper: create a big-endian, 4-byte length, no-padding decoder.
fn default_decoder(idl: &str) -> Decoder {
    make_decoder(idl, DecoderConfig::default())
}

// ============================================================================
// Config validation
// ============================================================================

#[test]
fn test_config_validation() {
    let cfg = DecoderConfig {
        length_field_length: 3,
        ..Default::default()
    };
    assert!(cfg.validate().is_err());

    let cfg = DecoderConfig {
        padding_length: 3,
        ..Default::default()
    };
    assert!(cfg.validate().is_err());

    // Valid configs
    assert!(DecoderConfig::default().validate().is_ok());
    assert!(DecoderConfig {
        length_field_length: 2,
        padding_length: 8,
        ..Default::default()
    }
    .validate()
    .is_ok());
}

// ============================================================================
// Primitive type decode tests (via struct)
// ============================================================================

#[test]
fn test_decode_u8() {
    let idl = "module m { struct S { octet a; }; }";
    let mut dec = default_decoder(idl);
    let result = dec.decode("S", &[0x2A, 0x01]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::U8(42));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_i16() {
    let idl = "module m { struct S { short a; }; }";
    // Big-endian: 0x0102 = 258
    let mut dec = default_decoder(idl);
    let result = dec.decode("S", &[0x01, 0x02, 0x03]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::I16(258));
    assert_eq!(result, expected);

    // Little-endian: 0x0102 = 513 (0x0201)
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let result = dec.decode("S", &[0x01, 0x02]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::I16(513));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_u16() {
    let idl = "module m { struct S { unsigned short a; }; }";
    let mut dec = default_decoder(idl);
    // 0x0102 = 258
    let result = dec.decode("S", &[0x01, 0x02]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::U16(258));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_i32() {
    let idl = "module m { struct S { long a; }; }";
    let mut dec = default_decoder(idl);
    // 0x00000001 = 1
    let result = dec.decode("S", &[0x00, 0x00, 0x00, 0x01]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::I32(1));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_u32() {
    let idl = "module m { struct S { unsigned long a; }; }";
    let mut dec = default_decoder(idl);
    // 0x000004F0 = 1264
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::U32(1264));
    let result = dec.decode("S", &[0x00, 0x00, 0x04, 0xF0]).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_decode_i64() {
    let idl = "module m { struct S { long long a; }; }";
    let mut dec = default_decoder(idl);
    let result = dec
        .decode("S", &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
        .unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::I64(1));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_u64() {
    let idl = "module m { struct S { unsigned long long a; }; }";
    let mut dec = default_decoder(idl);
    // u64::MAX
    let data = [0xFFu8; 8];
    let result = dec.decode("S", &data).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::U64(u64::MAX));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_f32() {
    let idl = "module m { struct S { float a; }; }";
    let mut dec = default_decoder(idl);
    // 0x3F800000 = 1.0f32
    let result = dec.decode("S", &[0x3F, 0x80, 0x00, 0x00]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::F32(1.0));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_f64() {
    let idl = "module m { struct S { double a; }; }";
    let mut dec = default_decoder(idl);
    // 0x3FF0000000000000 = 1.0f64
    let data = [0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let result = dec.decode("S", &data).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::F64(1.0));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_bool_true() {
    let idl = "module m { struct S { boolean a; }; }";
    let mut dec = default_decoder(idl);
    let result = dec.decode("S", &[0x01]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::Bool(true));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_bool_false() {
    let idl = "module m { struct S { boolean a; }; }";
    let mut dec = default_decoder(idl);
    let result = dec.decode("S", &[0x00]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::Bool(false));
    assert_eq!(result, expected);
}

// ============================================================================
// Struct decode
// ============================================================================

#[test]
fn test_decode_simple_struct() {
    let idl = r#"
        module spi {
            struct Point {
                octet id1;
                octet id2;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    let result = dec.decode("Point", &[41, 42]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("id1".into(), Value::U8(41));
    expected.insert("id2".into(), Value::U8(42));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_multi_field_struct() {
    let idl = r#"
        module test {
            struct Mixed {
                octet a;
                unsigned long id;
                short val;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    let data = [
        0x2A, // octet = 42
        0x00, 0x00, 0x04, 0xF0, // unsigned long = 1264
        0x00, 0x01, // short = 1
    ];
    let result = dec.decode("Mixed", &data).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::U8(42));
    expected.insert("id".into(), Value::U32(1264));
    expected.insert("val".into(), Value::I16(1));
    assert_eq!(result, expected);
}

// ============================================================================
// Nested struct / TypeName decode
// ============================================================================

#[test]
fn test_decode_type_name() {
    let idl = r#"
        module test {
            struct Inner {
                octet x;
                octet y;
            };
            struct Outer {
                unsigned long id;
                Inner inner;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    // id=1, inner={x=10, y=20}
    let data = [
        0x00, 0x00, 0x00, 0x01, // id = 1
        0x0A, // inner.x = 10
        0x14, // inner.y = 20
    ];
    let result = dec.decode("Outer", &data).unwrap();
    let mut expected = HashMap::new();
    expected.insert("id".into(), Value::U32(1));
    let mut inner = HashMap::new();
    inner.insert("x".into(), Value::U8(10));
    inner.insert("y".into(), Value::U8(20));
    expected.insert("inner".into(), Value::Struct(inner));
    assert_eq!(result, expected);
}

// ============================================================================
// Array decode
// ============================================================================

#[test]
fn test_decode_array() {
    let idl = r#"
        module test {
            struct WithArray {
                octet[3] items;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    let result = dec.decode("WithArray", &[0x01, 0x02, 0x03]).unwrap();
    let mut expected = HashMap::new();
    // octet arrays produce Bytes
    expected.insert("items".into(), Value::Bytes(vec![1, 2, 3]));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_array_with_length_header() {
    let idl = r#"
        module test {
            struct WithArray {
                short[2] items;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            enable_array_length_header: true,
            ..Default::default()
        },
    );
    // length header: 0x00000002 = 2, then two shorts: 0x0001, 0x0002
    let result = dec
        .decode(
            "WithArray",
            &[0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x02],
        )
        .unwrap();
    let mut expected = HashMap::new();
    expected.insert(
        "items".into(),
        Value::List(vec![Value::I16(1), Value::I16(2)]),
    );
    assert_eq!(result, expected);
}

#[test]
fn test_decode_array_length_mismatch() {
    let idl = r#"
        module test {
            struct WithArray {
                octet[3] items;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            enable_array_length_header: true,
            ..Default::default()
        },
    );
    // Header says 5 but declared size is 3
    let result = dec.decode("WithArray", &[0x00, 0x00, 0x00, 0x05, 0x01, 0x02, 0x03]);
    assert!(result.is_err());
}

// ============================================================================
// Sequence decode
// ============================================================================

#[test]
fn test_decode_sequence_of_octet() {
    let idl = r#"
        module test {
            struct WithSeq {
                sequence<octet> payload;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    // length prefix = 3, then 3 bytes
    let result = dec
        .decode("WithSeq", &[0x00, 0x00, 0x00, 0x03, 0x10, 0x20, 0x30])
        .unwrap();
    let mut expected = HashMap::new();
    expected.insert("payload".into(), Value::Bytes(vec![0x10, 0x20, 0x30]));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_sequence_of_shorts() {
    let idl = r#"
        module test {
            struct WithSeq {
                sequence<short> items;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    // length prefix = 4 bytes (2 shorts), then 0x0001, 0x0002
    let result = dec
        .decode("WithSeq", &[0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02])
        .unwrap();
    let mut expected = HashMap::new();
    expected.insert(
        "items".into(),
        Value::List(vec![Value::I16(1), Value::I16(2)]),
    );
    assert_eq!(result, expected);
}

#[test]
fn test_decode_sequence_empty() {
    let idl = r#"
        module test {
            struct WithSeq {
                sequence<octet> payload;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    // length prefix = 0
    let result = dec.decode("WithSeq", &[0x00, 0x00, 0x00, 0x00]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("payload".into(), Value::Bytes(vec![]));
    assert_eq!(result, expected);
}

// ============================================================================
// Length field formats
// ============================================================================

#[test]
fn test_length_field_1_byte() {
    let idl = r#"
        module test {
            struct WithSeq {
                sequence<octet> payload;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            length_field_length: 1,
            ..Default::default()
        },
    );
    // 1-byte length = 2, then 2 bytes
    let result = dec.decode("WithSeq", &[0x02, 0xAA, 0xBB]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("payload".into(), Value::Bytes(vec![0xAA, 0xBB]));
    assert_eq!(result, expected);
}

#[test]
fn test_length_field_2_byte() {
    let idl = r#"
        module test {
            struct WithSeq {
                sequence<octet> payload;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            length_field_length: 2,
            ..Default::default()
        },
    );
    // 2-byte length = 3, then 3 bytes
    let result = dec
        .decode("WithSeq", &[0x00, 0x03, 0x11, 0x22, 0x33])
        .unwrap();
    let mut expected = HashMap::new();
    expected.insert("payload".into(), Value::Bytes(vec![0x11, 0x22, 0x33]));
    assert_eq!(result, expected);
}

// ============================================================================
// String decode
// ============================================================================

#[test]
fn test_decode_fixed_string_utf8() {
    let idl = r#"
        module test {
            struct WithStr {
                string<10> name;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    // 10-byte fixed buffer: BOM(EF BB BF) + "hello" + 0x00 terminator + padding
    let mut buf = vec![0xEF, 0xBB, 0xBF];
    buf.extend_from_slice(b"hello");
    buf.push(0x00);
    buf.resize(10, 0x00);
    let result = dec.decode("WithStr", &buf).unwrap();
    let mut expected = HashMap::new();
    expected.insert("name".into(), Value::Str("hello".into()));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_dynamic_string_utf8() {
    let idl = r#"
        module test {
            struct WithStr {
                string name;
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    // 4-byte length = 9, then: BOM(3) + "hello"(5) + 0x00(1) = 9
    let mut str_data = vec![0xEF, 0xBB, 0xBF];
    str_data.extend_from_slice(b"hello");
    str_data.push(0x00);
    let str_len = str_data.len() as u8;
    let mut data = vec![0x00, 0x00, 0x00, str_len];
    data.extend_from_slice(&str_data);
    let result = dec.decode("WithStr", &data).unwrap();
    let mut expected = HashMap::new();
    expected.insert("name".into(), Value::Str("hello".into()));
    assert_eq!(result, expected);
}

// ============================================================================
// GBF packet decode (end-to-end)
// ============================================================================

#[test]
fn test_decode_gbf_packet_like() {
    let idl = r#"
        module spi {
            struct frame {
                unsigned long id;
                unsigned long len;
                sequence<octet> payload;
            };
            struct packet {
                unsigned long long ts;
                unsigned short len;
                sequence<frame> frames;
            };
        }
    "#;
    let mut dec = default_decoder(idl);

    // Build binary: ts=100, len=2, then sequence<frame> with 1 frame
    // frame: id=0x4F0, len=3, payload=[1,2,3]
    let mut data = Vec::new();
    // ts: 8 bytes = 100
    data.extend_from_slice(&100u64.to_be_bytes());
    // len: 2 bytes = 0 (just placeholder)
    data.extend_from_slice(&[0x00, 0x00u8]);

    // frames sequence: 4-byte total length
    let frame_data: Vec<u8> = {
        let mut fd = Vec::new();
        fd.extend_from_slice(&0x000004F0u32.to_be_bytes()); // id
        fd.extend_from_slice(&0x00000003u32.to_be_bytes()); // len
        fd.extend_from_slice(&0x00000003u32.to_be_bytes()); // payload seq len
        fd.extend_from_slice(&[0x01, 0x02, 0x03]); // payload
        fd
    };
    let seq_len = frame_data.len() as u32;
    data.extend_from_slice(&seq_len.to_be_bytes());
    data.extend_from_slice(&frame_data);

    let result = dec.decode("packet", &data).unwrap();

    assert_eq!(result.get("ts").unwrap(), &Value::U64(100));
    // frames is a List of Struct values
    if let Some(Value::List(frames)) = result.get("frames") {
        assert_eq!(frames.len(), 1);
        if let Value::Struct(ref frame_fields) = frames[0] {
            assert_eq!(frame_fields.get("id").unwrap(), &Value::U32(0x4F0));
            assert_eq!(frame_fields.get("len").unwrap(), &Value::U32(3));
            assert_eq!(
                frame_fields.get("payload").unwrap(),
                &Value::Bytes(vec![1, 2, 3])
            );
        } else {
            panic!("expected frame to be Struct");
        }
    } else {
        panic!("expected frames to be List");
    }
}

// ============================================================================
// Error cases
// ============================================================================

#[test]
fn test_decode_not_enough_data() {
    let idl = "module m { struct S { unsigned long a; }; }";
    let mut dec = default_decoder(idl);
    // Need 4 bytes, only have 2
    let result = dec.decode("S", &[0x00, 0x01]);
    assert!(result.is_err());
}

#[test]
fn test_decode_schema_not_found() {
    let idl = "module m { struct S { octet a; }; }";
    let mut dec = default_decoder(idl);
    let result = dec.decode("NonExistent", &[0x01]);
    assert!(result.is_err());
}

#[test]
fn test_decode_bitfield_not_supported() {
    // bitfield in struct field is parseable (TypeRef::BitField) but decoder rejects it
    let idl = "module m { struct S { bitfield<4> a; }; }";
    let module = parse_idl(idl).unwrap();
    let mut dec = Decoder::new(DecoderConfig::default(), module).unwrap();
    let result = dec.decode("S", &[0x0F]);
    assert!(result.is_err());
}

#[test]
fn test_decode_header_length() {
    let idl = "module m { struct S { octet a; }; }";
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            header_length: 2,
            ..Default::default()
        },
    );
    // Skip 2 bytes, then octet = 42
    let result = dec.decode("S", &[0xFF, 0xFF, 0x2A]).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".into(), Value::U8(42));
    assert_eq!(result, expected);
}

#[test]
fn test_decode_header_too_short() {
    let idl = "module m { struct S { octet a; }; }";
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            header_length: 5,
            ..Default::default()
        },
    );
    let result = dec.decode("S", &[0x01]);
    assert!(result.is_err());
}

// ============================================================================
// Detailed primitive type tests (migrated from converter_test.go subtests)
// Each test validates specific hex values, signed/unsigned, both endians,
// and error cases for insufficient data.
// ============================================================================

#[test]
fn test_decode_u8_detailed() {
    let idl = "module m { struct S { octet a; }; }";
    let mut dec = default_decoder(idl);

    // value 42 with remainder
    let r = dec.decode("S", &[42, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U8(42));

    // value 255 (max)
    let r = dec.decode("S", &[255, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U8(255));

    // value 0
    let r = dec.decode("S", &[0, 100, 200]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U8(0));

    // insufficient data
    let mut dec2 = default_decoder(idl);
    assert!(dec2.decode("S", &[]).is_err());
}

#[test]
fn test_decode_i16_detailed() {
    let idl = "module m { struct S { short a; }; }";

    // big-endian: 0x3039 = 12345
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0x30, 0x39, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I16(12345));

    // little-endian: 0x3930 bytes → 0x3039 = 12345
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec.decode("S", &[0x39, 0x30, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I16(12345));

    // big-endian negative: 0xCFC7 = -12345
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0xCF, 0xC7, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I16(-12345));

    // little-endian negative: 0xC7CF → 0xCFC7 = -12345
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec.decode("S", &[0xC7, 0xCF, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I16(-12345));

    // insufficient data
    let mut dec = default_decoder(idl);
    assert!(dec.decode("S", &[0x30]).is_err());
}

#[test]
fn test_decode_u16_detailed() {
    let idl = "module m { struct S { unsigned short a; }; }";

    // 0xFFFF = 65535 (big endian)
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0xFF, 0xFF, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U16(65535));

    // 0xFFFF = 65535 (little endian — same bytes, same result)
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec.decode("S", &[0xFF, 0xFF, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U16(65535));

    // 0 (big endian)
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0, 0, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U16(0));

    // insufficient data
    let mut dec = default_decoder(idl);
    assert!(dec.decode("S", &[0xFF]).is_err());
}

#[test]
fn test_decode_i32_detailed() {
    let idl = "module m { struct S { long a; }; }";

    // 0x499602D2 = 1234567890 (big endian)
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0x49, 0x96, 0x02, 0xD2, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I32(1234567890));

    // same value little endian (bytes reversed)
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec.decode("S", &[0xD2, 0x02, 0x96, 0x49, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I32(1234567890));

    // negative: 0xB669FD2E = -1234567890 (big endian)
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0xB6, 0x69, 0xFD, 0x2E, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I32(-1234567890));

    // insufficient data
    let mut dec = default_decoder(idl);
    assert!(dec.decode("S", &[0x49, 0x96, 0x02]).is_err());
}

#[test]
fn test_decode_u32_detailed() {
    let idl = "module m { struct S { unsigned long a; }; }";

    // 0xFFFFFFFF = 4294967295
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0xFF, 0xFF, 0xFF, 0xFF, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U32(4294967295));

    // 0
    let r = dec.decode("S", &[0, 0, 0, 0, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U32(0));

    // insufficient data
    assert!(dec.decode("S", &[0xFF, 0xFF, 0xFF]).is_err());
}

#[test]
fn test_decode_i64_detailed() {
    let idl = "module m { struct S { long long a; }; }";

    // 0x7FFFFFFFFFFFFFFF = i64::MAX = 9223372036854775807
    let mut dec = default_decoder(idl);
    let data = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 1, 2, 3];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I64(9223372036854775807));

    // 0
    let data = [0u8; 10];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::I64(0));

    // insufficient data
    assert!(dec
        .decode("S", &[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
        .is_err());
}

#[test]
fn test_decode_u64_detailed() {
    let idl = "module m { struct S { unsigned long long a; }; }";

    // 0x7FFFFFFFFFFFFFFF = 9223372036854775807
    let mut dec = default_decoder(idl);
    let data = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 1, 2, 3];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U64(9223372036854775807));

    // 0
    let data = [0u8; 10];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::U64(0));

    // insufficient data
    assert!(dec
        .decode("S", &[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
        .is_err());
}

#[test]
fn test_decode_bool_detailed() {
    let idl = "module m { struct S { boolean a; }; }";
    let mut dec = default_decoder(idl);

    // true
    let r = dec.decode("S", &[0x01, 1, 2, 3]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Bool(true));

    // false
    let r = dec.decode("S", &[0x00, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Bool(false));

    // non-zero → true (0xFF)
    let r = dec.decode("S", &[0xFF, 100, 200]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Bool(true));

    // insufficient data
    assert!(dec.decode("S", &[]).is_err());
}

#[test]
fn test_decode_f32_detailed() {
    let idl = "module m { struct S { float a; }; }";

    // 3.14 big endian
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0x40, 0x48, 0xF5, 0xC3, 1, 2, 3]).unwrap();
    let v = match r.get("a").unwrap() {
        Value::F32(f) => *f,
        _ => panic!("expected F32"),
    };
    assert!((v - 3.14).abs() < 0.001);

    // 3.14 little endian (bytes reversed)
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec.decode("S", &[0xC3, 0xF5, 0x48, 0x40, 1, 2, 3]).unwrap();
    let v = match r.get("a").unwrap() {
        Value::F32(f) => *f,
        _ => panic!("expected F32"),
    };
    assert!((v - 3.14).abs() < 0.001);

    // 0.0
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0, 0, 0, 0, 10, 20]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::F32(0.0));

    // -2.5 (0xC0200000)
    let r = dec
        .decode("S", &[0xC0, 0x20, 0x00, 0x00, 100, 200])
        .unwrap();
    let v = match r.get("a").unwrap() {
        Value::F32(f) => *f,
        _ => panic!("expected F32"),
    };
    assert!((v - (-2.5)).abs() < 0.001);

    // insufficient data
    assert!(dec.decode("S", &[0x40, 0x48, 0xF5]).is_err());
}

// ============================================================================
// Array detailed tests (migrated from array_converter_test.go)
// ============================================================================

#[test]
fn test_decode_array_short8() {
    let idl = "module m { struct S { short[8] a; }; }";
    let mut dec = default_decoder(idl);
    // short[8] → 16 bytes
    let data = [
        0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00,
        0x08,
    ];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![
            Value::I16(1),
            Value::I16(2),
            Value::I16(3),
            Value::I16(4),
            Value::I16(5),
            Value::I16(6),
            Value::I16(7),
            Value::I16(8),
        ])
    );
}

#[test]
fn test_decode_array_long4() {
    let idl = "module m { struct S { long[4] a; }; }";
    let mut dec = default_decoder(idl);
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes());
    data.extend_from_slice(&2i32.to_be_bytes());
    data.extend_from_slice(&3i32.to_be_bytes());
    data.extend_from_slice(&4i32.to_be_bytes());
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![
            Value::I32(1),
            Value::I32(2),
            Value::I32(3),
            Value::I32(4),
        ])
    );
}

#[test]
fn test_decode_array_boolean3() {
    let idl = "module m { struct S { boolean[3] a; }; }";
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0x01, 0x00, 0x01]).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true)
        ])
    );
}

#[test]
fn test_decode_array_ushort2() {
    let idl = "module m { struct S { unsigned short[2] a; }; }";
    let mut dec = default_decoder(idl);
    let r = dec.decode("S", &[0x00, 0x01, 0x00, 0x02]).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![Value::U16(1), Value::U16(2)])
    );
}

#[test]
fn test_decode_array_short4_le() {
    let idl = "module m { struct S { short[4] a; }; }";
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec
        .decode("S", &[0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00])
        .unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![
            Value::I16(1),
            Value::I16(2),
            Value::I16(3),
            Value::I16(4),
        ])
    );
}

#[test]
fn test_decode_array_long2_le() {
    let idl = "module m { struct S { long[2] a; }; }";
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let r = dec
        .decode("S", &[0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00])
        .unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![Value::I32(1), Value::I32(2)])
    );
}

#[test]
fn test_decode_array_insufficient_data() {
    let idl = "module m { struct S { short[4] a; }; }";
    let mut dec = default_decoder(idl);
    // Only 4 bytes = 2 shorts, need 8 bytes = 4 shorts
    assert!(dec.decode("S", &[0x00, 0x01, 0x00, 0x02]).is_err());

    let idl2 = "module m { struct S { long[2] a; }; }";
    let mut dec2 = default_decoder(idl2);
    // Only 4 bytes = 1 long, need 8 bytes = 2 longs
    assert!(dec2.decode("S", &[0x00, 0x00, 0x00, 0x01]).is_err());
}

// ============================================================================
// Sequence detailed tests (migrated from converter_test.go subtests)
// ============================================================================

#[test]
fn test_decode_sequence_of_boolean() {
    let idl = "module m { struct S { sequence<boolean> a; }; }";
    let mut dec = default_decoder(idl);

    // 3 booleans: true, false, true
    let r = dec
        .decode("S", &[0, 0, 0, 3, 0x01, 0x00, 0xFF, 100, 200])
        .unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true)
        ])
    );

    // 1 boolean: false
    let r = dec.decode("S", &[0, 0, 0, 1, 0x00, 100, 200]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::List(vec![Value::Bool(false)]));

    // insufficient data
    assert!(dec.decode("S", &[0, 0, 0, 2, 0x01]).is_err());
}

// ============================================================================
// String encoding tests (migrated from string_converter_test.go)
// ============================================================================

#[test]
fn test_decode_fixed_string_utf8_with_padding() {
    let idl = "module m { struct S { string<10> a; }; }";
    let mut dec = default_decoder(idl);
    // BOM(3) + "Hi"(2) + terminator(1) + padding(4) = 10
    let mut buf = vec![0xEF, 0xBB, 0xBF];
    buf.extend_from_slice(b"Hi");
    buf.push(0x00);
    buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // padding
    let r = dec.decode("S", &buf).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Hi".into()));
}

#[test]
fn test_decode_fixed_string_utf16be() {
    let idl = "module m { struct S { string<8> a; }; }";
    let mut dec = default_decoder(idl);
    // UTF-16BE BOM(FE FF) + "Go"(00 47 00 6F) + terminator(00 00) = 8
    let data = [0xFE, 0xFF, 0x00, 0x47, 0x00, 0x6F, 0x00, 0x00];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Go".into()));
}

#[test]
fn test_decode_fixed_string_utf16be_padded() {
    let idl = "module m { struct S { string<10> a; }; }";
    let mut dec = default_decoder(idl);
    // BOM(2) + "Hi"(4) + terminator(2) + padding(2) = 10
    let data = [
        0xFE, 0xFF, // BOM
        0x00, 0x48, // 'H'
        0x00, 0x69, // 'i'
        0x00, 0x00, // terminator
        0x00, 0x00, // padding
    ];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Hi".into()));
}

#[test]
fn test_decode_fixed_string_utf16le() {
    let idl = "module m { struct S { string<8> a; }; }";
    let mut dec = default_decoder(idl);
    // UTF-16LE BOM(FF FE) + "Go"(47 00 6F 00) + terminator(00 00) = 8
    let data = [0xFF, 0xFE, 0x47, 0x00, 0x6F, 0x00, 0x00, 0x00];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Go".into()));
}

#[test]
fn test_decode_fixed_string_utf16le_padded() {
    let idl = "module m { struct S { string<10> a; }; }";
    let mut dec = default_decoder(idl);
    // BOM(2) + "Hi"(4) + terminator(2) + padding(2) = 10
    let data = [
        0xFF, 0xFE, // BOM
        0x48, 0x00, // 'H'
        0x69, 0x00, // 'i'
        0x00, 0x00, // terminator
        0x00, 0x00, // padding
    ];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Hi".into()));
}

#[test]
fn test_decode_fixed_string_with_remainder() {
    let idl = "module m { struct S { string<9> a; }; }";
    let mut dec = default_decoder(idl);
    // BOM(3) + "Hello"(5) + terminator(1) = 9, then extra bytes
    let mut buf = vec![0xEF, 0xBB, 0xBF];
    buf.extend_from_slice(b"Hello");
    buf.push(0x00);
    buf.extend_from_slice(&[0xFF, 0xFF, 0xFF]); // remainder
    let r = dec.decode("S", &buf).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Hello".into()));
}

#[test]
fn test_decode_fixed_string_chinese() {
    let idl = "module m { struct S { string<10> a; }; }";
    let mut dec = default_decoder(idl);
    // BOM(3) + "你好"(6 bytes UTF-8) + terminator(1) = 10
    let mut buf = vec![0xEF, 0xBB, 0xBF];
    buf.extend_from_slice("你好".as_bytes());
    buf.push(0x00);
    let r = dec.decode("S", &buf).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("你好".into()));
}

#[test]
fn test_decode_fixed_string_no_bom_error() {
    let idl = "module m { struct S { string<6> a; }; }";
    let mut dec = default_decoder(idl);
    // "Hello" + terminator — no BOM
    let r = dec.decode("S", &[0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00]);
    assert!(r.is_err());
}

#[test]
fn test_decode_fixed_string_insufficient_data() {
    let idl = "module m { struct S { string<10> a; }; }";
    let mut dec = default_decoder(idl);
    // Only 5 bytes, need 10
    let r = dec.decode("S", &[0xEF, 0xBB, 0xBF, 0x48, 0x65]);
    assert!(r.is_err());
}

#[test]
fn test_decode_fixed_string_no_terminator_error() {
    let idl = "module m { struct S { string<8> a; }; }";
    let mut dec = default_decoder(idl);
    // BOM(3) + "Hello"(5) = 8, but no terminator
    let mut buf = vec![0xEF, 0xBB, 0xBF];
    buf.extend_from_slice(b"Hello");
    let r = dec.decode("S", &buf);
    assert!(r.is_err());
}

// ============================================================================
// Dynamic string encoding tests (migrated from string_converter_test.go)
// ============================================================================

#[test]
fn test_decode_dynamic_string_utf16be() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // length=8, BOM(FE FF), "Go"(00 47 00 6F), terminator(00 00)
    let data = [0, 0, 0, 8, 0xFE, 0xFF, 0x00, 0x47, 0x00, 0x6F, 0x00, 0x00];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Go".into()));
}

#[test]
fn test_decode_dynamic_string_utf16le() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // length=8, BOM(FF FE), "Go"(47 00 6F 00), terminator(00 00)
    let data = [0, 0, 0, 8, 0xFF, 0xFE, 0x47, 0x00, 0x6F, 0x00, 0x00, 0x00];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Go".into()));
}

#[test]
fn test_decode_dynamic_string_chinese() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // length=10, BOM(3), "你好"(6 bytes UTF-8), terminator(1)
    let mut data = vec![0, 0, 0, 10, 0xEF, 0xBB, 0xBF];
    data.extend_from_slice("你好".as_bytes());
    data.push(0x00);
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("你好".into()));
}

#[test]
fn test_decode_dynamic_string_padded() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // length=10, BOM(3), "Hi"(2), terminator(1), padding(4)
    let data = [
        0, 0, 0, 10, // length
        0xEF, 0xBB, 0xBF, // BOM
        0x48, 0x69, // "Hi"
        0x00, // terminator
        0x00, 0x00, 0x00, 0x00, // padding
    ];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("Hi".into()));
}

#[test]
fn test_decode_dynamic_string_empty_with_bom() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // length=4, BOM(3), terminator(1)
    let data = [0, 0, 0, 4, 0xEF, 0xBB, 0xBF, 0x00];
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::Str("".into()));
}

#[test]
fn test_decode_dynamic_string_no_bom_error() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // "Hello" + terminator — no BOM
    let r = dec.decode("S", &[0, 0, 0, 6, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00]);
    assert!(r.is_err());
}

#[test]
fn test_decode_dynamic_string_invalid_bom_error() {
    let idl = "module m { struct S { string a; }; }";
    let mut dec = default_decoder(idl);
    // Invalid BOM bytes
    let r = dec.decode(
        "S",
        &[
            0, 0, 0, 8, 0xAA, 0xBB, 0xCC, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00,
        ],
    );
    assert!(r.is_err());
}

// ============================================================================
// Sequence of strings (migrated from converter_test.go)
// ============================================================================

#[test]
fn test_decode_sequence_of_strings() {
    let idl = "module m { struct S { sequence<string> a; }; }";
    let mut dec = default_decoder(idl);

    // 2 strings: "abc" + "xy"
    // seq len=21, str1 len=7(BOM+abc+term), str2 len=6(BOM+xy+term)
    let mut data = vec![0, 0, 0, 21]; // sequence length
                                      // "abc": len=7
    data.extend_from_slice(&[0, 0, 0, 7, 0xEF, 0xBB, 0xBF, b'a', b'b', b'c', 0x00]);
    // "xy": len=6
    data.extend_from_slice(&[0, 0, 0, 6, 0xEF, 0xBB, 0xBF, b'x', b'y', 0x00]);
    data.extend_from_slice(&[100, 200]);

    let r = dec.decode("S", &data).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![Value::Str("abc".into()), Value::Str("xy".into()),])
    );
}

#[test]
fn test_decode_sequence_of_strings_empty() {
    let idl = "module m { struct S { sequence<string> a; }; }";
    let mut dec = default_decoder(idl);
    // Empty sequence
    let r = dec.decode("S", &[0, 0, 0, 0, 100, 200]).unwrap();
    assert_eq!(r.get("a").unwrap(), &Value::List(vec![]));
}

#[test]
fn test_decode_sequence_of_strings_single() {
    let idl = "module m { struct S { sequence<string> a; }; }";
    let mut dec = default_decoder(idl);
    // "test": len=8
    let mut data = vec![0, 0, 0, 12]; // seq total
    data.extend_from_slice(&[0, 0, 0, 8, 0xEF, 0xBB, 0xBF, b't', b'e', b's', b't', 0x00]);
    data.extend_from_slice(&[100, 200]);
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![Value::Str("test".into())])
    );
}

#[test]
fn test_decode_sequence_of_strings_both_empty() {
    let idl = "module m { struct S { sequence<string> a; }; }";
    let mut dec = default_decoder(idl);
    // Two empty strings with BOM
    let mut data = vec![0, 0, 0, 16]; // seq total
    data.extend_from_slice(&[0, 0, 0, 4, 0xEF, 0xBB, 0xBF, 0x00]);
    data.extend_from_slice(&[0, 0, 0, 4, 0xEF, 0xBB, 0xBF, 0x00]);
    data.extend_from_slice(&[100, 200]);
    let r = dec.decode("S", &data).unwrap();
    assert_eq!(
        r.get("a").unwrap(),
        &Value::List(vec![Value::Str("".into()), Value::Str("".into())])
    );
}

// ============================================================================
// Complex struct decode tests (migrated from converter_test.go)
// ============================================================================

#[test]
fn test_decode_complex_struct_be() {
    let idl = r#"
        module a {
            module b {
                struct complex {
                    short id;
                    long timestamp;
                    float value;
                };
            };
        }
    "#;
    let mut dec = default_decoder(idl);
    let data = [
        0x30, 0x39, // short: 12345
        0x49, 0x96, 0x02, 0xD2, // long: 1234567890
        0x40, 0x48, 0xF5, 0xC3, // float: 3.14
    ];
    let r = dec.decode("a.b.complex", &data).unwrap();
    assert_eq!(r.get("id").unwrap(), &Value::I16(12345));
    assert_eq!(r.get("timestamp").unwrap(), &Value::I32(1234567890));
    let v = match r.get("value").unwrap() {
        Value::F32(f) => *f,
        _ => panic!("expected F32"),
    };
    assert!((v - 3.14).abs() < 0.001);
}

#[test]
fn test_decode_complex_struct_le() {
    let idl = r#"
        module a {
            module b {
                struct complex {
                    short id;
                    long timestamp;
                    float value;
                };
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: true,
            ..Default::default()
        },
    );
    let data = [
        0x39, 0x30, // short: 12345 LE
        0xD2, 0x02, 0x96, 0x49, // long: 1234567890 LE
        0xC3, 0xF5, 0x48, 0x40, // float: 3.14 LE
    ];
    let r = dec.decode("a.b.complex", &data).unwrap();
    assert_eq!(r.get("id").unwrap(), &Value::I16(12345));
    assert_eq!(r.get("timestamp").unwrap(), &Value::I32(1234567890));
    let v = match r.get("value").unwrap() {
        Value::F32(f) => *f,
        _ => panic!("expected F32"),
    };
    assert!((v - 3.14).abs() < 0.001);
}

// ============================================================================
// Struct padding integration tests (migrated from struct_padding_integration_test.go)
// ============================================================================

#[test]
fn test_struct_padding_two_dynamic_strings() {
    let idl = r#"
        module m {
            struct s {
                string A;
                string B;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            length_field_length: 4,
            padding_length: 4,
            ..Default::default()
        },
    );
    // string A: len=10 (BOM+Hello+term+pad=3+5+1+1), data=10 bytes, needs 2 padding
    // string B: len=10 (BOM+World+term+pad=3+5+1+1), data=10 bytes, last field no padding
    let mut data = Vec::new();
    // A: "Hello"
    data.extend_from_slice(&[0, 0, 0, 10]);
    data.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00, 0x00]);
    data.extend_from_slice(&[0x00, 0x00]); // padding to 4-byte alignment
                                           // B: "World"
    data.extend_from_slice(&[0, 0, 0, 10]);
    data.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x00, 0x00]);

    let r = dec.decode("s", &data).unwrap();
    assert_eq!(r.get("A").unwrap(), &Value::Str("Hello".into()));
    assert_eq!(r.get("B").unwrap(), &Value::Str("World".into()));
}

#[test]
fn test_struct_padding_nested() {
    let idl = r#"
        module m {
            struct inner {
                string field1;
                string field2;
            };
            struct outer {
                string fieldA;
                inner innerStruct;
                string fieldB;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            length_field_length: 4,
            padding_length: 4,
            ..Default::default()
        },
    );
    let mut data = Vec::new();
    // fieldA: "OuterA"
    data.extend_from_slice(&[0, 0, 0, 10]);
    data.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0x4F, 0x75, 0x74, 0x65, 0x72, 0x41, 0x00]);
    data.extend_from_slice(&[0x00, 0x00]); // padding
                                           // inner.field1: "Inner1"
    data.extend_from_slice(&[0, 0, 0, 10]);
    data.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0x49, 0x6E, 0x6E, 0x65, 0x72, 0x31, 0x00]);
    data.extend_from_slice(&[0x00, 0x00]); // padding
                                           // inner.field2: "Inner2"
    data.extend_from_slice(&[0, 0, 0, 10]);
    data.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0x49, 0x6E, 0x6E, 0x65, 0x72, 0x32, 0x00]);
    data.extend_from_slice(&[0x00, 0x00]); // padding (end of inner)
                                           // fieldB: "OuterB"
    data.extend_from_slice(&[0, 0, 0, 10]);
    data.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0x4F, 0x75, 0x74, 0x65, 0x72, 0x42, 0x00]);

    let r = dec.decode("outer", &data).unwrap();
    assert_eq!(r.get("fieldA").unwrap(), &Value::Str("OuterA".into()));
    assert_eq!(r.get("fieldB").unwrap(), &Value::Str("OuterB".into()));
    match r.get("innerStruct").unwrap() {
        Value::Struct(inner) => {
            assert_eq!(inner.get("field1").unwrap(), &Value::Str("Inner1".into()));
            assert_eq!(inner.get("field2").unwrap(), &Value::Str("Inner2".into()));
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_struct_padding_two_sequences() {
    let idl = r#"
        module m {
            struct arrayStruct {
                sequence<short> arrayA;
                sequence<short> arrayB;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            length_field_length: 4,
            padding_length: 4,
            ..Default::default()
        },
    );
    let mut data = Vec::new();
    // arrayA: 3 shorts = 6 bytes, needs 2 padding to align to 4
    data.extend_from_slice(&[0, 0, 0, 6]);
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x02, 0x00, 0x03]);
    data.extend_from_slice(&[0x00, 0x00]); // padding
                                           // arrayB: 2 shorts = 4 bytes (last field, no padding needed)
    data.extend_from_slice(&[0, 0, 0, 4]);
    data.extend_from_slice(&[0x00, 0x04, 0x00, 0x05]);

    let r = dec.decode("arrayStruct", &data).unwrap();
    assert_eq!(
        r.get("arrayA").unwrap(),
        &Value::List(vec![Value::I16(1), Value::I16(2), Value::I16(3)])
    );
    assert_eq!(
        r.get("arrayB").unwrap(),
        &Value::List(vec![Value::I16(4), Value::I16(5)])
    );
}

#[test]
fn test_struct_padding_nested_sequences() {
    let idl = r#"
        module m {
            struct innerArray {
                sequence<short> field1;
                sequence<short> field2;
            };
            struct outerArray {
                sequence<short> fieldA;
                innerArray innerStruct;
                sequence<short> fieldB;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            length_field_length: 4,
            padding_length: 4,
            ..Default::default()
        },
    );
    let mut data = Vec::new();
    // fieldA: [10, 20, 30]
    data.extend_from_slice(&[0, 0, 0, 6]);
    data.extend_from_slice(&[0x00, 0x0A, 0x00, 0x14, 0x00, 0x1E]);
    data.extend_from_slice(&[0x00, 0x00]); // padding
                                           // inner.field1: [40, 50, 60]
    data.extend_from_slice(&[0, 0, 0, 6]);
    data.extend_from_slice(&[0x00, 0x28, 0x00, 0x32, 0x00, 0x3C]);
    data.extend_from_slice(&[0x00, 0x00]); // padding
                                           // inner.field2: [70, 80]
    data.extend_from_slice(&[0, 0, 0, 4]);
    data.extend_from_slice(&[0x00, 0x46, 0x00, 0x50]);
    // fieldB: [90, 100]
    data.extend_from_slice(&[0, 0, 0, 4]);
    data.extend_from_slice(&[0x00, 0x5A, 0x00, 0x64]);

    let r = dec.decode("outerArray", &data).unwrap();
    assert_eq!(
        r.get("fieldA").unwrap(),
        &Value::List(vec![Value::I16(10), Value::I16(20), Value::I16(30)])
    );
    assert_eq!(
        r.get("fieldB").unwrap(),
        &Value::List(vec![Value::I16(90), Value::I16(100)])
    );
    match r.get("innerStruct").unwrap() {
        Value::Struct(inner) => {
            assert_eq!(
                inner.get("field1").unwrap(),
                &Value::List(vec![Value::I16(40), Value::I16(50), Value::I16(60)])
            );
            assert_eq!(
                inner.get("field2").unwrap(),
                &Value::List(vec![Value::I16(70), Value::I16(80)])
            );
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// S1 test — Chinese WIFI complex E2E (migrated from s1_test.go)
// ============================================================================

#[test]
fn test_s1_chinese_wifi_end_to_end() {
    let idl = r#"
        module m {
            struct WiFiAp {
                string wiFiApName;
                long wiFiApStrength;
                long wiFiApEncryption;
            };
            struct WifiApList {
                long wiFiApNum;
                sequence<WiFiAp> wiFiApArray;
            };
        }
    "#;
    let mut dec = make_decoder(
        idl,
        DecoderConfig {
            is_little_endian: false,
            length_field_length: 4,
            padding_length: 4,
            ..Default::default()
        },
    );

    // Binary data from s1_test.go hex string, manually constructed.
    // wiFiApNum = 2
    // wiFiApArray: sequence of 2 WiFiAp structs
    //
    // First WiFiAp:
    //   wiFiApName: "中文 WIFI" (dynamic string)
    //   wiFiApStrength: 12
    //   wiFiApEncryption: 34
    // Second WiFiAp:
    //   wiFiApName: "English WIFI" (dynamic string)
    //   wiFiApStrength: 56
    //   wiFiApEncryption: 78

    let mut data = Vec::new();

    // wiFiApNum = 2
    data.extend_from_slice(&2i32.to_be_bytes());

    // wiFiApArray sequence: total byte length
    // Let's build both WiFiAp structs first
    let mut ap1 = Vec::new();
    // wiFiApName: "中文 WIFI" in UTF-8 with BOM
    let name1_bytes: Vec<u8> = {
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice("中文 WIFI".as_bytes());
        b.push(0x00);
        let pad = (4 - (b.len() % 4)) % 4;
        b.resize(b.len() + pad, 0x00);
        b
    };
    ap1.extend_from_slice(&(name1_bytes.len() as u32).to_be_bytes());
    ap1.extend_from_slice(&name1_bytes);
    // padding after dynamic string (4-byte aligned)
    let consumed = 4 + name1_bytes.len();
    let pad = (4 - (consumed % 4)) % 4;
    ap1.resize(ap1.len() + pad, 0x00);
    // strength = 12
    ap1.extend_from_slice(&12i32.to_be_bytes());
    // encryption = 34
    ap1.extend_from_slice(&34i32.to_be_bytes());

    let mut ap2 = Vec::new();
    let name2_bytes: Vec<u8> = {
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice("English WIFI".as_bytes());
        b.push(0x00);
        let pad = (4 - (b.len() % 4)) % 4;
        b.resize(b.len() + pad, 0x00);
        b
    };
    ap2.extend_from_slice(&(name2_bytes.len() as u32).to_be_bytes());
    ap2.extend_from_slice(&name2_bytes);
    let consumed2 = 4 + name2_bytes.len();
    let pad2 = (4 - (consumed2 % 4)) % 4;
    ap2.resize(ap2.len() + pad2, 0x00);
    ap2.extend_from_slice(&56i32.to_be_bytes());
    ap2.extend_from_slice(&78i32.to_be_bytes());

    let array_len = (ap1.len() + ap2.len()) as u32;
    data.extend_from_slice(&array_len.to_be_bytes());
    data.extend_from_slice(&ap1);
    data.extend_from_slice(&ap2);

    let r = dec.decode("WifiApList", &data).unwrap();
    assert_eq!(r.get("wiFiApNum").unwrap(), &Value::I32(2));
    match r.get("wiFiApArray").unwrap() {
        Value::List(aps) => {
            assert_eq!(aps.len(), 2);
            // First AP
            match &aps[0] {
                Value::Struct(fields) => {
                    assert_eq!(
                        fields.get("wiFiApName").unwrap(),
                        &Value::Str("中文 WIFI".into())
                    );
                    assert_eq!(fields.get("wiFiApStrength").unwrap(), &Value::I32(12));
                    assert_eq!(fields.get("wiFiApEncryption").unwrap(), &Value::I32(34));
                }
                _ => panic!("expected Struct"),
            }
            // Second AP
            match &aps[1] {
                Value::Struct(fields) => {
                    assert_eq!(
                        fields.get("wiFiApName").unwrap(),
                        &Value::Str("English WIFI".into())
                    );
                    assert_eq!(fields.get("wiFiApStrength").unwrap(), &Value::I32(56));
                    assert_eq!(fields.get("wiFiApEncryption").unwrap(), &Value::I32(78));
                }
                _ => panic!("expected Struct"),
            }
        }
        _ => panic!("expected List"),
    }
}
