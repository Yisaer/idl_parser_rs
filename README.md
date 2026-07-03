# idl_parser_rs

OMG IDL (Interface Definition Language) parser and binary decoder written in Rust.
A rewrite of the Go [idlparser](https://github.com/Yisaer/idlparser).

## What It Does

1. **Parse** OMG IDL text into a structured AST (modules, structs, basic types, arrays, sequences, strings, bitsets, annotations).
2. **Decode** binary data according to the parsed IDL schema, producing typed values.

This crate handles the _outer wire format decoding_ — extracting fields like `ts`, `can_id`, `payload` bytes from raw binary. CAN signal-level decoding (bit-level extraction from payloads) is handled by [arxml_converter_rs](https://github.com/yisaer/arxml_converter_rs) via the `@format(dbc="...")` annotation bridge.

## Role in veloFlux

```
OMG IDL file → idl_parser_rs → type schema + decoder → structured fields
                                                              ↓
Binary CAN data → arxml_converter_rs → signal-level decoding → merged output
```

Used by [veloFlux](https://github.com/yisaer/veloFlux) GBF (Generic Binary Format) streams to decode binary wire data from MQTT topics into structured records consumable by streaming SQL pipelines.

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
idl_parser_rs = "0.1"
```

### Parse IDL text

```rust
use idl_parser_rs::parse_idl;

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
```

### Decode binary data

```rust
use idl_parser_rs::parse_idl;
use idl_parser_rs::decoder::{Decoder, DecoderConfig};

let module = parse_idl("module m { struct Point { octet x; octet y; }; }").unwrap();
let mut decoder = Decoder::new(DecoderConfig::default(), module).unwrap();

// Binary: x=10, y=20
let fields = decoder.decode("Point", &[0x0A, 0x14]).unwrap();

assert_eq!(fields.get("x").unwrap(), &idl_parser_rs::decoder::Value::U8(10));
assert_eq!(fields.get("y").unwrap(), &idl_parser_rs::decoder::Value::U8(20));
```

### Decode a GBF packet (typed API)

```rust
use idl_parser_rs::parse_idl;
use idl_parser_rs::decoder::{Decoder, DecoderConfig};

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
            @format(dbc="spi/sim.json") sequence<frame> frames;
        };
    }
"#;
let module = parse_idl(idl).unwrap();
let mut decoder = Decoder::new(DecoderConfig::default(), module).unwrap();

// Binary GBF packet: ts=100, 1 frame (id=1264, payload=[1,2,3])
let mut data = Vec::new();
data.extend_from_slice(&100u64.to_be_bytes());
data.extend_from_slice(&[0x00, 0x00]);  // len
let frame_bytes = [
    &1264u32.to_be_bytes()[..],
    &3u32.to_be_bytes()[..],
    &3u32.to_be_bytes()[..],
    &[0x01, 0x02, 0x03][..],
].concat();
data.extend_from_slice(&(frame_bytes.len() as u32).to_be_bytes());
data.extend_from_slice(&frame_bytes);

let packet = decoder.decode_packet("packet", &data).unwrap();
assert_eq!(packet.ts, 100);
assert_eq!(packet.frames.len(), 1);
assert_eq!(packet.frames[0].can_id, 1264);
assert_eq!(packet.frames[0].payload, vec![1, 2, 3]);
assert_eq!(packet.frames[0].format_annotation, Some("spi/sim.json".into()));
```

## Public API

### Parsing

```rust
pub fn parse_idl(input: &str) -> Result<ast::Module, parser::ParseError>
```

Parse an OMG IDL string into a `Module` AST. Returns `ParseError` on syntax errors or trailing garbage.

### Decoding

```rust
pub struct Decoder;
pub struct DecoderConfig { ... }
pub enum Value { U8, I16, U16, I32, U32, I64, U64, F32, F64, Bool, Str, List, Struct, Bytes }
pub enum DecoderError { ... }
pub struct DecodedFrame { pub can_id: u32, pub payload: Vec<u8>, pub len: u32, pub format_annotation: Option<String> }
pub struct DecodedPacket { pub ts: u64, pub frames: Vec<DecodedFrame> }

impl Decoder {
    pub fn new(config: DecoderConfig, module: Module) -> Result<Self, DecoderError>;
    /// Generic decode: returns HashMap<String, Value>
    pub fn decode(&mut self, schema_id: &str, data: &[u8]) -> Result<HashMap<String, Value>, DecoderError>;
    /// Typed GBF packet decode: returns DecodedPacket with extracted frames
    pub fn decode_packet(&mut self, schema_id: &str, data: &[u8]) -> Result<DecodedPacket, DecoderError>;
}
```

- **`Decoder::new`** — Create a decoder from a parsed `Module` and configuration.
- **`Decoder::decode`** — Decode `&[u8]` into `HashMap<String, Value>` (generic API).
- **`Decoder::decode_packet`** — Decode a GBF packet into `DecodedPacket`, with frames extracted as typed `DecodedFrame` structs. Carries `@format(dbc="...")` annotation through to each frame for downstream `arxml_converter_rs` handoff. **Preferred API for veloFlux GBF streams.**

See [API Reference](docs/api-reference.md) for full details on all types and methods.

## Supported IDL Grammar

```
module       ::= "module" identifier "{" module_content* "}"
module_content ::= module | struct_def | bitset_def
struct_def   ::= "struct" identifier "{" field* "}"
field        ::= annotation* type_ref identifier ";"
bitset_def   ::= "bitset" identifier "{" bitfield_def* "}"
bitfield_def ::= "bitfield" "<" integer ">" identifier ";"

type_ref     ::= array_type | base_type
array_type   ::= base_type "[" integer "]"
base_type    ::= "octet" | "short" | "unsigned short"
              | "long" | "unsigned long"
              | "long long" | "unsigned long long"
              | "float" | "double" | "boolean"
              | string_type | sequence_type | bitfield_type | type_name

string_type  ::= "string" | "string" "<" integer ">"
sequence_type ::= "sequence" "<" type_ref ">"
bitfield_type ::= "bitfield" "<" integer ">"
type_name    ::= identifier

annotation   ::= "@" identifier [ "(" kv_pairs ")" ]
```

Deferred: `enum`, `union`, `interface`, `exception`, `typedef`, `const`.

## Type Mapping

| IDL Type              | Rust Decode Type |
|-----------------------|-----------------|
| `octet`               | `Value::U8(u8)` |
| `short`               | `Value::I16(i16)` |
| `unsigned short`      | `Value::U16(u16)` |
| `long`                | `Value::I32(i32)` |
| `unsigned long`       | `Value::U32(u32)` |
| `long long`           | `Value::I64(i64)` |
| `unsigned long long`  | `Value::U64(u64)` |
| `float`               | `Value::F32(f32)` |
| `double`              | `Value::F64(f64)` |
| `boolean`             | `Value::Bool(bool)` |
| `string`              | `Value::Str(String)` |
| `string<N>`           | `Value::Str(String)` (fixed-length) |
| `T[N]`                | `Value::List(Vec<Value>)` / `Value::Bytes(Vec<u8>)` |
| `sequence<T>`         | `Value::List(Vec<Value>)` / `Value::Bytes(Vec<u8>)` |

`sequence<octet>` and `octet[N]` produce `Value::Bytes` for direct handoff to `arxml_converter_rs`.

## Project Structure

```
idl_parser_rs/
├── src/
│   ├── lib.rs              # Public API: parse_idl, re-exports
│   ├── ast/
│   │   └── types.rs        # Module, Struct, Field, TypeRef, Annotation, etc.
│   ├── parser/             # nom-based IDL parser (12 modules)
│   │   ├── module.rs       # "module name { ... }"
│   │   ├── struct_type.rs  # "struct name { ... }"
│   │   ├── bitset.rs       # "bitset name { ... }"
│   │   ├── base_type.rs    # All 13 type alternatives
│   │   ├── annotation.rs   # "@name(k=v, ...)"
│   │   └── ...
│   └── decoder/            # Binary decoder (3 modules)
│       ├── mod.rs          # Decoder, DecoderConfig, Value, DecoderError
│       ├── codec.rs        # Primitive + composite type decoding
│       └── string.rs       # BOM-aware string decoder (UTF-8/UTF-16)
├── tests/
│   ├── parser_tests.rs     # 46 parser integration tests
│   └── decoder_tests.rs    # 83 decoder integration tests (incl. 7 decode_packet tests)
├── docs/
│   ├── phase1-parser.md    # Parser design document
│   ├── phase2-decoder.md   # Decoder design document
│   └── api-reference.md    # Public API reference
├── Cargo.toml
├── Makefile
└── AGENTS.md               # Development guidelines for AI coding agents
```

## Development

```bash
make build      # Debug build
make release    # Optimized build
make test       # Run all 130 tests
make fmt        # Format code
make clippy     # Lint with -D warnings
make check      # fmt + clippy + test
```

## License

MIT
