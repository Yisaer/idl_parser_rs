//! Integration tests migrated from idlparser Go test suite.
//!
//! Covers: module, struct, bitset, annotations, nested modules,
//! all primitive types, arrays, sequences, strings, type names.

use idl_parser_rs::ast::*;
use idl_parser_rs::parse_idl;
use std::collections::HashMap;

// ============================================================================
// TestParsing (Go: TestParsing)
// ============================================================================

#[test]
fn test_parsing_basic() {
    let code = r#"
        module spi {
            bitset idbits {
                bitfield<4> bid; // 4 bits for bus_id
                bitfield<12> cid;  // 12 bits for can_id
            };

            struct CANFrame {
                octet header;
                idbits id;
            };
        }
    "#;
    let module = parse_idl(code).unwrap();
    assert_eq!(module.name, "spi");
    assert_eq!(module.content.len(), 2);

    // First: bitset
    match &module.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.name, "idbits");
            assert_eq!(bs.fields.len(), 2);
            assert_eq!(bs.fields[0].name, "bid");
            assert_eq!(bs.fields[0].width, 4);
            assert_eq!(bs.fields[1].name, "cid");
            assert_eq!(bs.fields[1].width, 12);
        }
        _ => panic!("expected BitSet"),
    }

    // Second: struct
    match &module.content[1] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "CANFrame");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[1].name, "id");
            assert_eq!(
                s.fields[1].field_type,
                TypeRef::TypeName {
                    name: "idbits".to_string()
                }
            );
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// TestParseModule (Go: TestParseModule)
// ============================================================================

#[test]
fn test_parse_module_with_annotations() {
    let input = r#"
        module spi {
            bitset idbits {
                bitfield<4> bid; // 4 bits for bus_id
            };

            struct CANFrame {
                @format octet header;
                @format(a=b) idbits id;
            };
        }
    "#;
    let module = parse_idl(input).unwrap();
    assert_eq!(module.name, "spi");
    assert_eq!(module.content.len(), 2);

    // Bitset
    match &module.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.name, "idbits");
            assert_eq!(bs.fields.len(), 1);
            assert_eq!(bs.fields[0].name, "bid");
            assert_eq!(bs.fields[0].width, 4);
        }
        _ => panic!("expected BitSet"),
    }

    // Struct with annotations
    match &module.content[1] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "CANFrame");
            assert_eq!(s.fields.len(), 2);

            // @format octet header;
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[0].annotations.len(), 1);
            assert_eq!(s.fields[0].annotations[0].name, "format");
            assert!(s.fields[0].annotations[0].values.is_empty());

            // @format(a=b) idbits id;
            assert_eq!(s.fields[1].name, "id");
            assert_eq!(
                s.fields[1].field_type,
                TypeRef::TypeName {
                    name: "idbits".to_string()
                }
            );
            assert_eq!(s.fields[1].annotations.len(), 1);
            assert_eq!(s.fields[1].annotations[0].name, "format");
            assert_eq!(s.fields[1].annotations[0].values.get("a").unwrap(), "b");
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// All primitive type parsing tests
// ============================================================================

#[test]
fn test_all_primitive_types() {
    let idl = r#"
        module test {
            struct AllTypes {
                octet a;
                short b;
                unsigned short c;
                long d;
                unsigned long e;
                long long f;
                unsigned long long g;
                float h;
                double i;
                boolean j;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields.len(), 10);
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[1].field_type, TypeRef::Short);
            assert_eq!(s.fields[2].field_type, TypeRef::UnsignedShort);
            assert_eq!(s.fields[3].field_type, TypeRef::Long);
            assert_eq!(s.fields[4].field_type, TypeRef::UnsignedLong);
            assert_eq!(s.fields[5].field_type, TypeRef::LongLong);
            assert_eq!(s.fields[6].field_type, TypeRef::UnsignedLongLong);
            assert_eq!(s.fields[7].field_type, TypeRef::Float);
            assert_eq!(s.fields[8].field_type, TypeRef::Double);
            assert_eq!(s.fields[9].field_type, TypeRef::Boolean);
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// Array tests
// ============================================================================

#[test]
fn test_array_types() {
    let idl = r#"
        module test {
            struct WithArrays {
                octet[10] idList;
                unsigned long long[4] tsList;
                idbits[8] refList;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields.len(), 3);

            // octet[10]
            assert_eq!(
                s.fields[0].field_type,
                TypeRef::Array {
                    inner: Box::new(TypeRef::Octet),
                    size: 10
                }
            );

            // unsigned long long[4]
            assert_eq!(
                s.fields[1].field_type,
                TypeRef::Array {
                    inner: Box::new(TypeRef::UnsignedLongLong),
                    size: 4
                }
            );

            // idbits[8]
            assert_eq!(
                s.fields[2].field_type,
                TypeRef::Array {
                    inner: Box::new(TypeRef::TypeName {
                        name: "idbits".to_string()
                    }),
                    size: 8
                }
            );
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// Sequence tests
// ============================================================================

#[test]
fn test_sequence_types() {
    let idl = r#"
        module test {
            struct WithSequences {
                sequence<octet> payload;
                sequence<sequence<octet>> nested;
                sequence<CANFrame> frames;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields.len(), 3);

            // sequence<octet>
            assert_eq!(
                s.fields[0].field_type,
                TypeRef::Sequence {
                    inner: Box::new(TypeRef::Octet)
                }
            );

            // sequence<sequence<octet>>
            assert_eq!(
                s.fields[1].field_type,
                TypeRef::Sequence {
                    inner: Box::new(TypeRef::Sequence {
                        inner: Box::new(TypeRef::Octet)
                    })
                }
            );

            // sequence<CANFrame>
            assert_eq!(
                s.fields[2].field_type,
                TypeRef::Sequence {
                    inner: Box::new(TypeRef::TypeName {
                        name: "CANFrame".to_string()
                    })
                }
            );
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// String tests
// ============================================================================

#[test]
fn test_string_types() {
    let idl = r#"
        module test {
            struct WithStrings {
                string dynamicName;
                string<10> fixedName;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].field_type, TypeRef::String { length: None });
            assert_eq!(s.fields[1].field_type, TypeRef::String { length: Some(10) });
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// Annotation tests (Go: TestModuleJson)
// ============================================================================

#[test]
fn test_annotation_with_quoted_value() {
    let idl = r#"
        module spi {
            struct CANFrame {
                @format(a="b",c=123) octet header;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields[0].annotations.len(), 1);
            let anno = &s.fields[0].annotations[0];
            assert_eq!(anno.name, "format");
            assert_eq!(anno.values.get("a").unwrap(), "b");
            assert_eq!(anno.values.get("c").unwrap(), "123");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_annotation_with_path_value() {
    let idl = r#"
        module spi {
            struct frame {
                unsigned long id;
                octet len;
                sequence<octet> payload;
            };

            struct packet {
                unsigned long long ts;
                unsigned short len;
                @format (dbc=spii/test.idl) sequence<frame> frames;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    // Check packet struct
    match &module.content[1] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "packet");
            let anno = &s.fields[2].annotations[0];
            assert_eq!(anno.name, "format");
            assert_eq!(anno.values.get("dbc").unwrap(), "spii/test.idl");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_multiple_annotations() {
    let idl = r#"
        module spi {
            struct SPI {
                @format (type=canpack,dbc=ab) @merge sequence<CANFrame> messages;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            let field = &s.fields[0];
            assert_eq!(field.annotations.len(), 2);
            assert_eq!(field.annotations[0].name, "format");
            assert_eq!(field.annotations[0].values.get("dbc").unwrap(), "ab");
            assert_eq!(field.annotations[0].values.get("type").unwrap(), "canpack");
            assert_eq!(field.annotations[1].name, "merge");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_annotation_with_space_before_parens() {
    let idl = r#"
        module spi {
            struct SPI {
                @format (type=binpack) @merge sequence<SPI> packs;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields[0].annotations.len(), 2);
            assert_eq!(s.fields[0].annotations[0].name, "format");
            assert_eq!(
                s.fields[0].annotations[0].values.get("type").unwrap(),
                "binpack"
            );
            assert_eq!(s.fields[0].annotations[1].name, "merge");
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// Nested module tests (Go: TestNestedModuleJson)
// ============================================================================

#[test]
fn test_nested_modules() {
    let idl = r#"
        module outer {
            module inner1 {
                struct a {
                    octet id1;
                    octet id2;
                };
            };
            module inner2 {
                struct b {
                    octet id3;
                    octet id4;
                };
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    assert_eq!(module.name, "outer");
    assert_eq!(module.content.len(), 2);

    // inner1 -> struct a
    match &module.content[0] {
        ModuleContent::Module(inner) => {
            assert_eq!(inner.name, "inner1");
            assert_eq!(inner.content.len(), 1);
            match &inner.content[0] {
                ModuleContent::Struct(s) => {
                    assert_eq!(s.name, "a");
                    assert_eq!(s.fields.len(), 2);
                }
                _ => panic!("expected Struct"),
            }
        }
        _ => panic!("expected Module"),
    }

    // inner2 -> struct b
    match &module.content[1] {
        ModuleContent::Module(inner) => {
            assert_eq!(inner.name, "inner2");
            assert_eq!(inner.content.len(), 1);
            match &inner.content[0] {
                ModuleContent::Struct(s) => {
                    assert_eq!(s.name, "b");
                    assert_eq!(s.fields.len(), 2);
                }
                _ => panic!("expected Struct"),
            }
        }
        _ => panic!("expected Module"),
    }
}

// ============================================================================
// Error case tests (Go: TestParseError)
// ============================================================================

#[test]
fn test_parse_error_missing_semicolon() {
    let input = r#"
        module a {
            module b {
                struct c {
                    octet id1;
                    octet id2
                }
            };
        }
    "#;
    let result = parse_idl(input);
    assert!(
        result.is_err(),
        "expected parse error for missing semicolon"
    );
}

// ============================================================================
// GBF IDL test (the actual gbf.idl used in production)
// ============================================================================

#[test]
fn test_parse_gbf_idl() {
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
                @format(dbc="gbf/sim.json") sequence<frame> frames;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    assert_eq!(module.name, "spi");
    assert_eq!(module.content.len(), 2);

    // frame struct
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "frame");
            assert_eq!(s.fields.len(), 3);
            assert_eq!(s.fields[0].name, "id");
            assert_eq!(s.fields[0].field_type, TypeRef::UnsignedLong);
            assert_eq!(s.fields[1].name, "len");
            assert_eq!(s.fields[1].field_type, TypeRef::UnsignedLong);
            assert_eq!(s.fields[2].name, "payload");
            assert_eq!(
                s.fields[2].field_type,
                TypeRef::Sequence {
                    inner: Box::new(TypeRef::Octet)
                }
            );
        }
        _ => panic!("expected Struct"),
    }

    // packet struct
    match &module.content[1] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "packet");
            assert_eq!(s.fields.len(), 3);
            assert_eq!(s.fields[0].name, "ts");
            assert_eq!(s.fields[0].field_type, TypeRef::UnsignedLongLong);
            assert_eq!(s.fields[1].name, "len");
            assert_eq!(s.fields[1].field_type, TypeRef::UnsignedShort);
            assert_eq!(s.fields[2].name, "frames");
            // Check it's a sequence with annotation
            assert_eq!(
                s.fields[2].field_type,
                TypeRef::Sequence {
                    inner: Box::new(TypeRef::TypeName {
                        name: "frame".to_string()
                    })
                }
            );
            assert_eq!(s.fields[2].annotations.len(), 1);
            assert_eq!(s.fields[2].annotations[0].name, "format");
            assert_eq!(
                s.fields[2].annotations[0].values.get("dbc").unwrap(),
                "gbf/sim.json"
            );
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// Comment handling tests
// ============================================================================

#[test]
fn test_comments_in_struct() {
    let idl = r#"
        module test {
            struct WithComments {
                unsigned long id; // 4 byte id
                // A comment line
                unsigned long len; // 4 byte length
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "id");
            assert_eq!(s.fields[1].name, "len");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_comments_in_bitset() {
    let idl = r#"
        module test {
            bitset flags {
                bitfield<1> isReady; // ready flag
                bitfield<7> reserved; // reserved bits
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.fields.len(), 2);
            assert_eq!(bs.fields[0].name, "isReady");
            assert_eq!(bs.fields[0].width, 1);
            assert_eq!(bs.fields[1].name, "reserved");
            assert_eq!(bs.fields[1].width, 7);
        }
        _ => panic!("expected BitSet"),
    }
}

// ============================================================================
// Semicolon/syntax variant tests
// ============================================================================

#[test]
fn test_struct_without_trailing_semicolon() {
    // Without trailing ; after struct — should still parse
    let idl = r#"
        module test {
            struct NoSemicolon {
                octet x;
                octet y;
            }
        }
    "#;
    let module = parse_idl(idl).unwrap();
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields.len(), 2);
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_module_with_underscore_in_name() {
    let idl = r#"
        module test_mod {
            struct message_123 {
                @format(a="b",c=123) octet header;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    assert_eq!(module.name, "test_mod");
    match &module.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "message_123");
            assert_eq!(s.fields.len(), 1);
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
        }
        _ => panic!("expected Struct"),
    }
}

// ============================================================================
// Migrated from annotation/annotation_test.go
// ============================================================================

#[test]
fn test_annotation_basic() {
    // @format
    let idl = "module m { struct S { @format octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert!(anno[0].values.is_empty());

    // @format()
    let idl = "module m { struct S { @format() octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert!(anno[0].values.is_empty());

    // @format(a=b)
    let idl = "module m { struct S { @format(a=b) octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b");

    // @format(a="b")
    let idl = "module m { struct S { @format(a=\"b\") octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b");

    // @format(a = b)
    let idl = "module m { struct S { @format(a = b) octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b");

    // @format(a = b, c = d)
    let idl = "module m { struct S { @format(a = b, c = d) octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b");
    assert_eq!(anno[0].values.get("c").unwrap(), "d");

    // @format(a = "b", c = "d")
    let idl = "module m { struct S { @format(a = \"b\", c = \"d\") octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b");
    assert_eq!(anno[0].values.get("c").unwrap(), "d");

    // @format(a = "b", c = 123)
    let idl = "module m { struct S { @format(a = \"b\", c = 123) octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b");
    assert_eq!(anno[0].values.get("c").unwrap(), "123");

    // @format(a = "b.c", c = 123) — dot in quoted value
    let idl = "module m { struct S { @format(a = \"b.c\", c = 123) octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let anno = struct_first_field_annotations(&m, 0);
    assert_eq!(anno[0].name, "format");
    assert_eq!(anno[0].values.get("a").unwrap(), "b.c");
    assert_eq!(anno[0].values.get("c").unwrap(), "123");
}

#[test]
fn test_multiple_annotations_unit() {
    // @format @check
    let idl = "module m { struct S { @format @check octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let field = struct_first_field(&m, 0);
    assert_eq!(field.annotations.len(), 2);
    assert_eq!(field.annotations[0].name, "format");
    assert_eq!(field.annotations[1].name, "check");

    // @format(a=b) @check(c=d)
    let idl = "module m { struct S { @format(a=b) @check(c=d) octet a; }; }";
    let m = parse_idl(idl).unwrap();
    let field = struct_first_field(&m, 0);
    assert_eq!(field.annotations.len(), 2);
    assert_eq!(field.annotations[0].name, "format");
    assert_eq!(field.annotations[0].values.get("a").unwrap(), "b");
    assert_eq!(field.annotations[1].name, "check");
    assert_eq!(field.annotations[1].values.get("c").unwrap(), "d");
}

// ============================================================================
// Migrated from bitset/mod_test.go
// ============================================================================

#[test]
fn test_parse_bitset_field() {
    let idl = "module m { bitset S { bitfield<1> a; }; }";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.fields.len(), 1);
            assert_eq!(bs.fields[0].name, "a");
            assert_eq!(bs.fields[0].width, 1);
        }
        _ => panic!("expected BitSet"),
    }
}

#[test]
fn test_parse_bitset_full() {
    let idl = "
        module m {
            bitset S {
                bitfield<1> a; // 1bit
                bitfield<4> b; // 4bit
            };
        }
    ";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.name, "S");
            assert_eq!(bs.fields.len(), 2);
            assert_eq!(bs.fields[0].name, "a");
            assert_eq!(bs.fields[0].width, 1);
            assert_eq!(bs.fields[1].name, "b");
            assert_eq!(bs.fields[1].width, 4);
        }
        _ => panic!("expected BitSet"),
    }
}

// ============================================================================
// Migrated from struct_type/mod_test.go
// ============================================================================

#[test]
fn test_parse_struct_basic() {
    let idl = "
        module m {
            struct AB {
                octet header;
                long h2;
                unsigned long h3;
                unsigned long long h4;
            };
        }
    ";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "AB");
            assert_eq!(s.fields.len(), 4);
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[1].name, "h2");
            assert_eq!(s.fields[1].field_type, TypeRef::Long);
            assert_eq!(s.fields[2].name, "h3");
            assert_eq!(s.fields[2].field_type, TypeRef::UnsignedLong);
            assert_eq!(s.fields[3].name, "h4");
            assert_eq!(s.fields[3].field_type, TypeRef::UnsignedLongLong);
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_parse_struct_annotation() {
    let idl = "
        module m {
            struct AB {
                @format octet header;
            };
        }
    ";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "AB");
            assert_eq!(s.fields.len(), 1);
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[0].annotations.len(), 1);
            assert_eq!(s.fields[0].annotations[0].name, "format");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_struct_field_annotations_table() {
    // @format(a=b) + plain field
    let idl = "
        module m {
            struct AB {
                @format(a=b) octet header;
                long h2;
            };
        }
    ";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "AB");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[0].annotations.len(), 1);
            assert_eq!(s.fields[0].annotations[0].name, "format");
            assert_eq!(s.fields[0].annotations[0].values.get("a").unwrap(), "b");
            assert_eq!(s.fields[1].name, "h2");
            assert_eq!(s.fields[1].field_type, TypeRef::Long);
            assert!(s.fields[1].annotations.is_empty());
        }
        _ => panic!("expected Struct"),
    }

    // @format (no params)
    let idl = "
        module m {
            struct AB {
                @format octet header;
            };
        }
    ";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.fields[0].name, "header");
            assert_eq!(s.fields[0].field_type, TypeRef::Octet);
            assert_eq!(s.fields[0].annotations.len(), 1);
            assert_eq!(s.fields[0].annotations[0].name, "format");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_parse_struct_sequence() {
    let idl = "
        module m {
            struct AB {
                sequence<octet> payload;
            };
        }
    ";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::Struct(s) => {
            assert_eq!(s.name, "AB");
            assert_eq!(s.fields.len(), 1);
            assert_eq!(s.fields[0].name, "payload");
            assert_eq!(
                s.fields[0].field_type,
                TypeRef::Sequence {
                    inner: Box::new(TypeRef::Octet)
                }
            );
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_parse_struct_field_without_semicolon() {
    // Missing semicolon after second field
    let idl = "
        module m {
            struct AB {
                octet id1;
                octet id2
            };
        }
    ";
    let result = parse_idl(idl);
    assert!(
        result.is_err(),
        "expected parse error for missing semicolon"
    );
}

// ============================================================================
// Migrated from typeref/array_test.go
// ============================================================================

#[test]
fn test_parse_array_variants() {
    // short[8]
    let idl = "module m { struct S { short[8] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Short),
            size: 8
        }
    );

    // long[16]
    let idl = "module m { struct S { long[16] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Long),
            size: 16
        }
    );

    // unsigned short[4]
    let idl = "module m { struct S { unsigned short[4] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::UnsignedShort),
            size: 4
        }
    );

    // boolean[1]
    let idl = "module m { struct S { boolean[1] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Boolean),
            size: 1
        }
    );

    // float[32]
    let idl = "module m { struct S { float[32] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Float),
            size: 32
        }
    );
}

#[test]
fn test_parse_array_error() {
    // plain short without brackets — should parse as Short, not Array
    let idl = "module m { struct S { short a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Short);

    // short[abc] — invalid array, should fail
    let idl = "module m { struct S { short[abc] a; }; }";
    assert!(parse_idl(idl).is_err());

    // short[8 — missing closing bracket, should fail
    let idl = "module m { struct S { short[8 a; }; }";
    assert!(parse_idl(idl).is_err());
}

#[test]
fn test_parse_type_ref_with_array_dispatch() {
    // short[8] → Array
    let idl = "module m { struct S { short[8] a; }; }";
    let m = parse_idl(idl).unwrap();
    let ty = &struct_first_field(&m, 0).field_type;
    assert!(matches!(ty, TypeRef::Array { .. }));

    // long[16] → Array
    let idl = "module m { struct S { long[16] a; }; }";
    let m = parse_idl(idl).unwrap();
    let ty = &struct_first_field(&m, 0).field_type;
    assert!(matches!(ty, TypeRef::Array { .. }));

    // unsigned short[4] → Array
    let idl = "module m { struct S { unsigned short[4] a; }; }";
    let m = parse_idl(idl).unwrap();
    let ty = &struct_first_field(&m, 0).field_type;
    assert!(matches!(ty, TypeRef::Array { .. }));

    // plain short → Short (not Array)
    let idl = "module m { struct S { short a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Short);

    // plain long → Long (not Array)
    let idl = "module m { struct S { long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Long);
}

// ============================================================================
// Migrated from typeref/octet_test.go
// ============================================================================

#[test]
fn test_parse_octet() {
    let idl = "module m { struct S { octet a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Octet);
}

#[test]
fn test_parse_octet_array_via_type_ref() {
    // octet[8]
    let idl = "module m { struct S { octet[8] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Octet),
            size: 8
        }
    );

    // octet[1]
    let idl = "module m { struct S { octet[1] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Octet),
            size: 1
        }
    );

    // octet[16]
    let idl = "module m { struct S { octet[16] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Octet),
            size: 16
        }
    );

    // octet[255]
    let idl = "module m { struct S { octet[255] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::Octet),
            size: 255
        }
    );
}

#[test]
fn test_parse_octet_error() {
    // Empty input → error
    let result = parse_idl("");
    assert!(result.is_err());

    // "invalid" token is NOT an error at the TypeRef level —
    // it falls back to TypeName. But passing something that is
    // not a valid type at all should fail.
    // Test: missing field name after type (semicolon instead of name)
    let idl = "module m { struct S { octet; }; }";
    assert!(parse_idl(idl).is_err());
}

// ============================================================================
// Migrated from typeref/sequence_test.go
// ============================================================================

#[test]
fn test_sequence_variants() {
    // sequence<octet>
    let idl = "module m { struct S { sequence<octet> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Sequence {
            inner: Box::new(TypeRef::Octet)
        }
    );

    // sequence<long long>
    let idl = "module m { struct S { sequence<long long> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Sequence {
            inner: Box::new(TypeRef::LongLong)
        }
    );

    // sequence<long>
    let idl = "module m { struct S { sequence<long> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Sequence {
            inner: Box::new(TypeRef::Long)
        }
    );

    // sequence<unsigned long>
    let idl = "module m { struct S { sequence<unsigned long> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Sequence {
            inner: Box::new(TypeRef::UnsignedLong)
        }
    );

    // sequence<unsigned long long>
    let idl = "module m { struct S { sequence<unsigned long long> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Sequence {
            inner: Box::new(TypeRef::UnsignedLongLong)
        }
    );

    // sequence<idbits> (type name)
    let idl = "module m { struct S { sequence<idbits> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Sequence {
            inner: Box::new(TypeRef::TypeName {
                name: "idbits".to_string()
            })
        }
    );
}

// ============================================================================
// Migrated from typeref/type_test.go
// ============================================================================

#[test]
fn test_parse_bitfield_type() {
    let idl = "module m { bitset B { bitfield<8> a; }; }";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.fields[0].width, 8);
        }
        _ => panic!("expected BitSet"),
    }

    let idl = "module m { bitset B { bitfield<16> a; }; }";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.fields[0].width, 16);
        }
        _ => panic!("expected BitSet"),
    }

    let idl = "module m { bitset B { bitfield<32> a; }; }";
    let m = parse_idl(idl).unwrap();
    match &m.content[0] {
        ModuleContent::BitSet(bs) => {
            assert_eq!(bs.fields[0].width, 32);
        }
        _ => panic!("expected BitSet"),
    }
}

#[test]
fn test_parse_short() {
    let idl = "module m { struct S { short a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Short);
}

#[test]
fn test_parse_unsigned_short_variants() {
    // unsigned short
    let idl = "module m { struct S { unsigned short a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::UnsignedShort);

    // unsigned  short (double space)
    let idl = "module m { struct S { unsigned  short a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::UnsignedShort);
}

#[test]
fn test_parse_long() {
    let idl = "module m { struct S { long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Long);
}

#[test]
fn test_parse_unsigned_long_variants() {
    // unsigned long
    let idl = "module m { struct S { unsigned long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::UnsignedLong);

    // unsigned  long (double space)
    let idl = "module m { struct S { unsigned  long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::UnsignedLong);
}

#[test]
fn test_parse_long_long_variants() {
    // long long
    let idl = "module m { struct S { long long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::LongLong);

    // long  long (double space)
    let idl = "module m { struct S { long  long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::LongLong);
}

#[test]
fn test_parse_unsigned_long_long_variants() {
    // unsigned long long
    let idl = "module m { struct S { unsigned long long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::UnsignedLongLong
    );

    // unsigned  long  long (multi-space)
    let idl = "module m { struct S { unsigned  long  long a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::UnsignedLongLong
    );
}

#[test]
fn test_parse_boolean() {
    let idl = "module m { struct S { boolean a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Boolean);
}

#[test]
fn test_parse_float() {
    let idl = "module m { struct S { float a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(struct_first_field(&m, 0).field_type, TypeRef::Float);
}

#[test]
fn test_parse_string_variants() {
    // string (dynamic)
    let idl = "module m { struct S { string a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::String { length: None }
    );

    // string<10>
    let idl = "module m { struct S { string<10> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::String { length: Some(10) }
    );

    // string<255>
    let idl = "module m { struct S { string<255> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::String { length: Some(255) }
    );

    // string<1>
    let idl = "module m { struct S { string<1> a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::String { length: Some(1) }
    );
}

// ============================================================================
// Migrated from typeref/consistency_test.go
// ============================================================================

#[test]
fn test_parse_type_ref_consistency() {
    // Test that ParseTypeRef correctly identifies all types
    let cases: Vec<(&str, TypeRef)> = vec![
        ("short a", TypeRef::Short),
        ("long a", TypeRef::Long),
        ("boolean a", TypeRef::Boolean),
        ("float a", TypeRef::Float),
        ("string a", TypeRef::String { length: None }),
        ("octet a", TypeRef::Octet),
        ("unsigned short a", TypeRef::UnsignedShort),
        ("unsigned long a", TypeRef::UnsignedLong),
        ("long long a", TypeRef::LongLong),
        ("unsigned long long a", TypeRef::UnsignedLongLong),
        (
            "short[8] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::Short),
                size: 8,
            },
        ),
        (
            "long[16] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::Long),
                size: 16,
            },
        ),
        (
            "boolean[3] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::Boolean),
                size: 3,
            },
        ),
        (
            "float[32] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::Float),
                size: 32,
            },
        ),
        (
            "octet[8] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::Octet),
                size: 8,
            },
        ),
        (
            "unsigned short[4] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::UnsignedShort),
                size: 4,
            },
        ),
        (
            "MyType a",
            TypeRef::TypeName {
                name: "MyType".to_string(),
            },
        ),
        (
            "MyType[8] a",
            TypeRef::Array {
                inner: Box::new(TypeRef::TypeName {
                    name: "MyType".to_string(),
                }),
                size: 8,
            },
        ),
        (
            "sequence<long> a",
            TypeRef::Sequence {
                inner: Box::new(TypeRef::Long),
            },
        ),
    ];

    for (type_str, expected) in &cases {
        let idl = format!("module m {{ struct S {{ {}; }} }}", type_str);
        let m =
            parse_idl(&idl).unwrap_or_else(|e| panic!("failed to parse '{}': {:?}", type_str, e));
        assert_eq!(
            struct_first_field(&m, 0).field_type,
            *expected,
            "mismatch for type: {}",
            type_str
        );
    }
}

// ============================================================================
// Migrated from typeref/typename_test.go
// ============================================================================

#[test]
fn test_parse_type_name() {
    let idl = "module m { struct S { idbits a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::TypeName {
            name: "idbits".to_string()
        }
    );
}

#[test]
fn test_parse_type_name_array_via_type_ref() {
    // idbits[8]
    let idl = "module m { struct S { idbits[8] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::TypeName {
                name: "idbits".to_string()
            }),
            size: 8
        }
    );

    // MyType[255]
    let idl = "module m { struct S { MyType[255] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::TypeName {
                name: "MyType".to_string()
            }),
            size: 255
        }
    );

    // Test[1]
    let idl = "module m { struct S { Test[1] a; }; }";
    let m = parse_idl(idl).unwrap();
    assert_eq!(
        struct_first_field(&m, 0).field_type,
        TypeRef::Array {
            inner: Box::new(TypeRef::TypeName {
                name: "Test".to_string()
            }),
            size: 1
        }
    );
}

// ============================================================================
// Helper functions for test convenience
// ============================================================================

/// Get the first struct content from a module.
fn first_struct(m: &Module) -> &Struct {
    match &m.content[0] {
        ModuleContent::Struct(s) => s,
        _ => panic!("expected Struct"),
    }
}

/// Get the first field of the first struct.
fn struct_first_field(m: &Module, _idx: usize) -> &Field {
    let s = first_struct(m);
    &s.fields[0]
}

/// Get annotations of the first field of the first struct.
fn struct_first_field_annotations(m: &Module, _idx: usize) -> &Vec<Annotation> {
    let s = first_struct(m);
    &s.fields[0].annotations
}
