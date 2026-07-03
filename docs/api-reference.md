# API Reference

This document describes all public types and functions exposed by `idl_parser_rs`.
It is intended for developers and AI coding agents integrating this crate.

## Table of Contents

- [Parsing](#parsing)
  - [`parse_idl`](#parse_idl)
  - [`ParseError`](#parseerror)
- [AST Types](#ast-types)
  - [`Module`](#module)
  - [`ModuleContent`](#modulecontent)
  - [`Struct`](#struct)
  - [`Field`](#field)
  - [`BitSet`](#bitset)
  - [`BitSetField`](#bitsetfield)
  - [`Annotation`](#annotation)
  - [`TypeRef`](#typeref)
- [Decoding](#decoding)
  - [`Decoder`](#decoder)
  - [`decode_packet`](#decode_packet)
  - [`DecodedPacket`](#decodedpacket)
  - [`DecodedFrame`](#decodedframe)
  - [`DecoderConfig`](#decoderconfig)
  - [`Value`](#value)
  - [`DecoderError`](#decodererror)

---

## Parsing

### `parse_idl`

```rust
pub fn parse_idl(input: &str) -> Result<ast::Module, parser::ParseError>
```

Parse an OMG IDL string into a `Module` AST.

**Parameters:**
- `input` — Full IDL source text. Leading/trailing whitespace and `//` comments are ignored.

**Returns:**
- `Ok(Module)` — The parsed module tree.
- `Err(ParseError)` — Syntax error with descriptive message (includes remaining input context).

**Example:**

```rust
let m = parse_idl("module example { struct Point { long x; long y; }; }")?;
```

### `ParseError`

```rust
pub struct ParseError {
    pub message: String,
}
```

Wraps nom parser errors. Implements `Display`, `Error`. The `message` field contains the
nom error description including the position where parsing failed.

---

## AST Types

All types in `idl_parser_rs::ast` implement `Debug`, `Clone`, and `PartialEq`.

### `Module`

```rust
pub struct Module {
    pub name: String,
    pub content: Vec<ModuleContent>,
}
```

Top-level parsed result. Represents a `module name { ... }` block.

### `ModuleContent`

```rust
pub enum ModuleContent {
    Module(Box<Module>),
    Struct(Struct),
    BitSet(BitSet),
}
```

Items that can appear inside a module: sub-modules, struct definitions, or bitsets.

### `Struct`

```rust
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
}
```

A `struct name { field* }` definition. Fields are decoded in declaration order.

### `Field`

```rust
pub struct Field {
    pub annotations: Vec<Annotation>,
    pub field_type: TypeRef,
    pub name: String,
}
```

A single field within a struct.

- `annotations` — Zero or more `@name(k=v, ...)` annotations. The `@format(dbc="...")`
  annotation on `sequence<frame>` fields signals to veloFlux that the payload should
  be decoded by `arxml_converter_rs`.
- `field_type` — The type of the field.
- `name` — The field identifier.

### `BitSet`

```rust
pub struct BitSet {
    pub name: String,
    pub fields: Vec<BitSetField>,
}
```

A `bitset name { ... }` definition. Bitsets are parsed but **not decoded** by the
decoder (bitfield decoding is not supported in struct fields).

### `BitSetField`

```rust
pub struct BitSetField {
    pub width: u8,
    pub name: String,
}
```

A single bitfield within a bitset: `bitfield<width> name`.

### `Annotation`

```rust
pub struct Annotation {
    pub name: String,
    pub values: HashMap<String, String>,
}
```

An annotation on a field. Examples:

| IDL syntax | `name` | `values` |
|---|---|---|
| `@format` | `"format"` | `{}` |
| `@format()` | `"format"` | `{}` |
| `@format(a=b)` | `"format"` | `{"a": "b"}` |
| `@format(a="b", c=123)` | `"format"` | `{"a": "b", "c": "123"}` |
| `@format(dbc="path/file")` | `"format"` | `{"dbc": "path/file"}` |

Note: all values are stored as `String` (the `"123"` in `c=123` is a string).

### `TypeRef`

```rust
pub enum TypeRef {
    Octet,
    Short,
    UnsignedShort,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
    Float,
    Double,
    Boolean,
    String { length: Option<u32> },
    Array { inner: Box<TypeRef>, size: u32 },
    Sequence { inner: Box<TypeRef> },
    BitField { width: u8 },
    TypeName { name: String },
}
```

All supported IDL type references. This is a closed enum — every variant maps to one
IDL type syntax.

| Variant | IDL Syntax | Decodes To |
|---|---|---|
| `Octet` | `octet` | `Value::U8` |
| `Short` | `short` | `Value::I16` |
| `UnsignedShort` | `unsigned short` | `Value::U16` |
| `Long` | `long` | `Value::I32` |
| `UnsignedLong` | `unsigned long` | `Value::U32` |
| `LongLong` | `long long` | `Value::I64` |
| `UnsignedLongLong` | `unsigned long long` | `Value::U64` |
| `Float` | `float` | `Value::F32` |
| `Double` | `double` | `Value::F64` |
| `Boolean` | `boolean` | `Value::Bool` |
| `String { length: None }` | `string` | `Value::Str` (dynamic, length-prefixed) |
| `String { length: Some(n) }` | `string<n>` | `Value::Str` (fixed `n` bytes) |
| `Array { inner, size }` | `T[n]` | `Value::List` (or `Value::Bytes` if `T=octet`) |
| `Sequence { inner }` | `sequence<T>` | `Value::List` (or `Value::Bytes` if `T=octet`) |
| `BitField { width }` | `bitfield<w>` | Not decodable in structs (decoder returns error) |
| `TypeName { name }` | `SomeType` | `Value::Struct` (looks up named struct definition) |

---

## Decoding

### `Decoder`

```rust
pub struct Decoder { /* private fields */ }

impl Decoder {
    /// Create a decoder from a parsed Module and configuration.
    pub fn new(config: DecoderConfig, module: Module) -> Result<Self, DecoderError>;

    /// Decode binary data using the struct identified by schema_id.
    pub fn decode(
        &mut self,
        schema_id: &str,
        data: &[u8],
    ) -> Result<HashMap<String, Value>, DecoderError>;
}
```

**`schema_id`** is a dot-separated path locating the target struct:
- `"Point"` — Search for a struct named `Point` in the root module.
- `"spi.packet"` — Navigate into `module spi`, then find struct `packet`.

**Decoding behavior:**
- Fields are decoded in struct declaration order.
- Multi-byte integers use configurable endianness (big-endian by default).
- `sequence<T>` and dynamic `string` are length-prefixed (field width: 1, 2, or 4 bytes).
- `string` fields are BOM-prefixed (UTF-8, UTF-16BE, or UTF-16LE) and null-terminated.
- After each variable-length field (`string`, `sequence`), padding bytes are consumed
  to align to `padding_length` boundary.
- `header_length` bytes are skipped before decoding begins.

### `decode_packet`

```rust
impl Decoder {
    pub fn decode_packet(
        &mut self,
        schema_id: &str,
        data: &[u8],
    ) -> Result<DecodedPacket, DecoderError>;
}
```

Typed GBF packet decode. This is the **preferred API for veloFlux GBF streams**.
It decodes the outer packet struct and extracts frames into typed structures,
carrying `@format(dbc="...")` annotations through to each frame.

**How it works:**
1. Locates the target struct via `schema_id` (e.g., `"spi.packet"`).
2. Decodes fields sequentially: extracts `ts` (u64), skips `len` (u16),
   decodes the `sequence<frame>` field.
3. For each frame in the sequence, extracts `id`, `len`, `payload`.
4. Looks up the `@format(dbc="...")` annotation on the sequence field
   in the AST and attaches it to every `DecodedFrame`.
5. Returns a typed `DecodedPacket`.

### `DecodedPacket`

```rust
pub struct DecodedPacket {
    /// Timestamp from the packet header (unsigned long long).
    pub ts: u64,
    /// Decoded CAN frames.
    pub frames: Vec<DecodedFrame>,
}
```

### `DecodedFrame`

```rust
pub struct DecodedFrame {
    /// CAN frame ID (from the frame's `id` field).
    pub can_id: u32,
    /// Raw payload bytes (from the frame's `payload` field).
    pub payload: Vec<u8>,
    /// Byte length of the payload (from the frame's `len` field).
    pub len: u32,
    /// Value of `@format(dbc="...")` annotation from the outer sequence field.
    /// `None` if no `@format` annotation is present.
    pub format_annotation: Option<String>,
}
```

### `DecoderConfig`

```rust
pub struct DecoderConfig {
    pub is_little_endian: bool,       // default: false (big-endian)
    pub length_field_length: u8,      // default: 4 (valid: 1, 2, 4)
    pub enable_array_length_header: bool, // default: false
    pub padding_length: usize,        // default: 1 (valid: 1,2,4,8,16,32,64)
    pub header_length: usize,         // default: 0
}
```

| Field | Purpose |
|---|---|
| `is_little_endian` | Byte order for multi-byte integers and floats. |
| `length_field_length` | Width in bytes of the length prefix for `sequence` and dynamic `string`. |
| `enable_array_length_header` | If `true`, arrays are prefixed with a length field that must match the declared size. |
| `padding_length` | Alignment boundary for padding after variable-length fields. |
| `header_length` | Bytes to skip before starting decode (e.g., protocol headers). |

**GBF stream typical config:**

```rust
DecoderConfig {
    is_little_endian: false,
    length_field_length: 4,
    padding_length: 1,          // or 4 for aligned protocols
    enable_array_length_header: false,
    header_length: 0,
}
```

### `Value`

```rust
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
    Bytes(Vec<u8>),
}
```

Type-safe decoded value enum. Consumers should `match` on the variant to extract the
typed value.

**Key behaviors:**
- `octet` fields decode to `Value::U8`.
- `short` → `I16`, `long` → `I32`, `long long` → `I64`, and their unsigned variants.
- `float` → `F32`, `double` → `F64`.
- `boolean` → `Bool` (any non-zero byte is `true`).
- `sequence<octet>` and `octet[N]` decode to `Value::Bytes` (raw bytes for
  `arxml_converter_rs` handoff). Other array/sequence types decode to `Value::List`.
- `TypeName` fields decode to `Value::Struct` (recursively resolved).
- `string` decodes to `Value::Str` (BOM-stripped, null-trimmed).

### `DecoderError`

```rust
pub enum DecoderError {
    InvalidConfig(String),
    SchemaNotFound(String),
    UnexpectedEndOfInput { expected: usize, got: usize },
    InvalidData(String),
    UnsupportedType(String),
}
```

| Variant | When |
|---|---|
| `InvalidConfig` | Invalid `length_field_length` or `padding_length` value. |
| `SchemaNotFound` | `schema_id` does not resolve to a struct. |
| `UnexpectedEndOfInput` | Not enough bytes to decode a field. |
| `InvalidData` | Data format error (e.g., invalid UTF-8, missing string terminator). |
| `UnsupportedType` | Attempted to decode `bitfield` in a struct field. |

All variants implement `Display` and `Error`.

---

## Complete Example: Parse + Decode

```rust
use idl_parser_rs::parse_idl;
use idl_parser_rs::decoder::{Decoder, DecoderConfig, Value};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse the IDL schema
    let idl = r#"
        module spi {
            struct frame {
                unsigned long id;
                unsigned long len;
                sequence<octet> payload;
            };
        }
    "#;
    let module = parse_idl(idl)?;

    // 2. Create a decoder with default config (big-endian, 4-byte length fields)
    let mut decoder = Decoder::new(DecoderConfig::default(), module)?;

    // 3. Decode binary data
    // id=0x4F0 (4 bytes), len=3 (4 bytes), sequence_len=3 (4 bytes), payload=[1,2,3]
    let data: Vec<u8> = [
        &0x000004F0u32.to_be_bytes()[..],
        &0x00000003u32.to_be_bytes()[..],
        &0x00000003u32.to_be_bytes()[..],
        &[0x01, 0x02, 0x03][..],
    ].concat();

    let fields = decoder.decode("frame", &data)?;

    // 4. Use the decoded values
    assert_eq!(fields.get("id"), Some(&Value::U32(1264)));
    assert_eq!(fields.get("len"), Some(&Value::U32(3)));
    assert_eq!(fields.get("payload"), Some(&Value::Bytes(vec![1, 2, 3])));

    Ok(())
}
```

## Integration with arxml_converter_rs

Use `decode_packet` for type-safe frame extraction with automatic annotation propagation:

```rust
// 1. Decode the GBF packet with typed API
let packet = idl_decoder.decode_packet("spi.packet", &binary_data)?;

// 2. Iterate frames — each has can_id, payload, and format_annotation
for frame in &packet.frames {
    if let Some(dbc_path) = &frame.format_annotation {
        // 3. Decode CAN signals with arxml_converter_rs
        let signals = arxml_converter.decode(frame.can_id, &frame.payload)?;
        // merge signals into output record
    }
}
```

Alternatively, use the generic `decode` API for manual extraction:

```rust
let fields = idl_decoder.decode("spi.packet", &binary_data)?;
if let Value::List(frames) = &fields["frames"] {
    for frame in frames {
        if let Value::Struct(f) = frame {
            let can_id = /* extract from f["id"] */;
            let payload = /* extract from f["payload"] */;
            let signals = arxml_converter.decode(can_id, payload)?;
        }
    }
}
```

The two crates are intentionally decoupled — `idl_parser_rs` handles the outer wire format,
`arxml_converter_rs` handles CAN signal extraction from payload bytes.
