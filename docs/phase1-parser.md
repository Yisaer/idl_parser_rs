# Phase 1: IDL Parser (nom-based)

## Overview

Parse OMG IDL text into structured Rust AST types using `nom` parser combinators.
The parser covers the IDL subset used by veloFlux GBF streams: modules, structs, basic
types, arrays, sequences, strings, annotations, and bitsets.

## Grammar (Supported Subset)

```
idl_file       ::= module
module         ::= "module" identifier "{" module_content* "}"
module_content ::= module | struct_def | bitset_def
struct_def     ::= "struct" identifier "{" field* "}"
field          ::= annotation* type_ref identifier ";"
bitset_def     ::= "bitset" identifier "{" bitfield_def* "}"
bitfield_def   ::= "bitfield" "<" integer ">" identifier ";"

type_ref       ::= array_type | base_type
array_type     ::= base_type "[" integer "]"
base_type      ::= sequence_type
                 | "octet"
                 | "short"
                 | "unsigned" "short"
                 | "long"                    -- must come before "long" "long"
                 | "unsigned" "long"         -- must come before "unsigned" "long" "long"
                 | "long" "long"
                 | "unsigned" "long" "long"
                 | "float"
                 | "double"
                 | "boolean"
                 | string_type
                 | bitfield_type
                 | type_name

sequence_type  ::= "sequence" "<" type_ref ">"
string_type    ::= "string" | "string" "<" integer ">"
bitfield_type  ::= "bitfield" "<" integer ">"
type_name      ::= identifier

annotation     ::= "@" identifier [ "(" kv_pairs ")" ]
kv_pairs       ::= kv_pair ("," kv_pair)*
kv_pair        ::= identifier "=" value
value          ::= quoted_string | identifier

identifier     ::= [a-zA-Z_] [a-zA-Z0-9_]*
integer        ::= [0-9]+
quoted_string  ::= "\"" [a-zA-Z0-9_./]* "\""
comment        ::= "//" .* (CRLF | CR | LF)
```

Deferred: `enum`, `union`, `interface`, `exception`, `typedef`, `const`.

## AST Types

```rust
// src/ast/types.rs

use std::collections::HashMap;

/// Top-level parsed result.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub content: Vec<ModuleContent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleContent {
    Module(Box<Module>),
    Struct(Struct),
    BitSet(BitSet),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub annotations: Vec<Annotation>,
    pub field_type: TypeRef,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BitSet {
    pub name: String,
    pub fields: Vec<BitSetField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BitSetField {
    pub width: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub name: String,
    pub values: HashMap<String, String>,
}

/// All IDL type references as a single enum.
/// Recursive types (Array, Sequence) use Box to prevent infinite size.
#[derive(Debug, Clone, PartialEq)]
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
    String { length: Option<u32> },       // None = dynamic, Some(n) = fixed
    Array { inner: Box<TypeRef>, size: u32 },
    Sequence { inner: Box<TypeRef> },
    BitField { width: u8 },
    TypeName { name: String },             // Self-defined type reference
}
```

### Design Decision: enum vs trait

The Go version uses a `TypeRef` interface with 13 concrete structs. In Rust, we use a single
`TypeRef` enum instead of a trait with dynamic dispatch:

| Go (interface + type switch)         | Rust (enum + match)                     |
|--------------------------------------|-----------------------------------------|
| `t.TypeRefType()` dispatch in converter | `match type_ref { ... }` exhaustive    |
| Missing case = silent fallthrough     | Missing case = compile error            |
| Heap allocation per variant          | Inline storage (except Box recursion)   |
| 13 separate `Parse*` functions       | 13 `alt` branches in one parser         |

Verdict: enum is safer and more idiomatic for this use case. Trait would add unnecessary
complexity for a closed set of variants.

## Parser Module Structure

```
src/parser/
├── mod.rs              # pub fn parse_idl(input: &str) -> Result<Module, ParseError>
├── module.rs           # module parser: "module" name "{" contents "}"
├── struct_type.rs      # struct parser: "struct" name "{" fields "}"
├── field.rs            # field parser: [annotations] type name ";"
├── type_ref.rs         # type_ref dispatcher: array | base_type
├── base_type.rs        # base_type: alt(all 13 primitive type parsers)
├── sequence.rs         # "sequence" "<" type_ref ">"
├── array.rs            # base_type "[" integer "]"
├── string_type.rs      # "string" ["<" integer ">"]
├── bitset.rs           # "bitset" name "{" bitfield_def* "}"
├── annotation.rs       # "@" name ["(" kv_pairs ")"]
├── primitives.rs       # octet, short, long, float, double, boolean parsers
├── comment.rs          # comment skipping (// ... \n)
└── util.rs             # whitespace, identifier, integer, quoted_string helpers
```

### Public API

```rust
// src/parser/mod.rs

use crate::ast::Module;

/// Parse an OMG IDL string and return the root Module.
pub fn parse_idl(input: &str) -> Result<Module, ParseError> {
    // ...
}
```

## nom Mapping (gomme → nom)

| Go gomme                               | Rust nom                                              |
|----------------------------------------|-------------------------------------------------------|
| `gomme.Token("module")`                | `tag("module")`                                       |
| `gomme.Alpha1()`                       | `alpha1`                                              |
| `gomme.Alphanumeric0()`                | `alphanumeric0`                                       |
| `gomme.Whitespace0/1()`               | `multispace0` / `multispace1`                         |
| `gomme.Delimited(a, p, b)`            | `delimited(a, p, b)`                                  |
| `gomme.SeparatedPair(a, sep, b)`      | `separated_pair(a, sep, b)`                           |
| `gomme.Preceded(a, p)`                | `preceded(a, p)`                                      |
| `gomme.Terminated(p, a)`              | `terminated(p, a)`                                    |
| `gomme.Alternative(a, b, c)`          | `alt((a, b, c))`                                      |
| `gomme.Many0(p)`                      | `many0(p)`                                            |
| `gomme.Optional(p)`                   | `opt(p)`                                              |
| `gomme.Map(p, fn)`                    | `map(p, fn)`                                          |
| `gomme.Recognize(p)`                  | `recognize(p)`                                        |
| `utils.InEmpty(parser)`               | `delimited(multispace0, parser, multispace0)`         |
| `utils.InLeftEmpty(parser)`           | `preceded(multispace0, parser)`                       |
| `utils.ParseComment`                  | Custom `comment` parser                               |
| `utils.ParseEmpty0/1`                 | `comment_or_whitespace0/1` (comment-aware whitespace) |

### Whitespace Handling

The Go version distinguishes between `gomme.Whitespace0/1` (standard whitespace) and
`utils.ParseEmpty0/1` (whitespace OR comments). We replicate this with a custom
`comment_or_whitespace` combinator:

```rust
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{line_ending, multispace1};
use nom::combinator::recognize;
use nom::sequence::pair;

/// Match a single-line comment: "//" ... (CRLF | CR | LF)
fn comment(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        tag("//"),
        pair(take_until("\n"), line_ending),  // take_until handles CR/CRLF/LF
    ))(input)
}

/// Match zero or more whitespace-or-comment tokens.
fn ws0(input: &str) -> IResult<&str, &str> {
    recognize(many0(alt((multispace1, comment))))(input)
}

/// Between-ws helper: ws0 before and after the parser.
fn in_ws<'a, F, O>(parser: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(ws0, parser, ws0)
}
```

## Parser Implementation Details

### Key Challenge: Alt Ordering for Multi-Word Types

`unsigned long long` and `unsigned long` share prefixes with `long long` and `long`.
Order in `alt()` matters — longer matches must come first:

```rust
fn parse_base_type(input: &str) -> IResult<&str, TypeRef> {
    alt((
        parse_sequence,             // Must check "sequence" keyword first
        parse_unsigned_long_long,   // "unsigned long long" before "unsigned long"
        parse_unsigned_long,        // "unsigned long" before "unsigned short"
        parse_long_long,            // "long long" before "long"
        parse_unsigned_short,
        parse_long,
        parse_octet,
        parse_short,
        parse_double,               // "double" before... (no conflict actually)
        parse_float,
        parse_boolean,
        parse_string_type,
        parse_bitfield,             // "bitfield<N>"
        parse_type_name,           // Fallback: any identifier
    ))(input)
}
```

**Rationale**: If `parse_long` tried before `parse_long_long`, it would consume "long" and
leave " long" unconsumed, causing a parse error. nom's `alt` tries each branch in order
and backtracks on failure.

### Array Suffix Detection

The Go version pre-scans for `[N]` before deciding Array vs BaseType. With nom, we use a
more natural `opt` combinator:

```rust
fn parse_type_ref(input: &str) -> IResult<&str, TypeRef> {
    let (remaining, base) = parse_base_type(input)?;
    // Check for optional array suffix "[N]"
    match opt(delimited(tag("["), parse_integer, tag("]")))(remaining)? {
        (remaining2, Some(size)) => Ok((remaining2, TypeRef::Array {
            inner: Box::new(base),
            size,
        })),
        (_, None) => Ok((remaining, base)),
    }
}
```

This is cleaner than Go's pre-scan approach and handles nesting (e.g., `octet[10][5]` is
naturally parsed as `Array { inner: Array { inner: Octet, size: 10 }, size: 5 }`).

### Sequence Parser

```rust
fn parse_sequence(input: &str) -> IResult<&str, TypeRef> {
    let (remaining, _) = tag("sequence")(input)?;
    let (remaining, _) = ws0(remaining)?;
    let (remaining, inner) = delimited(
        tag("<"),
        preceded(ws0, parse_type_ref),
        preceded(ws0, tag(">")),
    )(remaining)?;
    Ok((remaining, TypeRef::Sequence { inner: Box::new(inner) }))
}
```

### Annotation Parser

```rust
fn parse_annotation(input: &str) -> IResult<&str, Annotation> {
    let (remaining, _) = tag("@")(input)?;
    let (remaining, name) = parse_identifier(remaining)?;
    // Optional key=value pairs in parens
    let (remaining, values) = opt(delimited(
        preceded(ws0, tag("(")),
        parse_kv_pairs,
        preceded(ws0, tag(")")),
    ))(remaining)?;
    Ok((remaining, Annotation {
        name: name.to_string(),
        values: values.unwrap_or_default(),
    }))
}
```

## Error Handling

Use `nom::error::VerboseError` for detailed position-aware errors:

```rust
use nom::error::{VerboseError, convert_error};
use nom::Err;

type IResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    // Optionally: line number, column, remaining input snippet
}

impl From<Err<VerboseError<&str>>> for ParseError {
    fn from(e: Err<VerboseError<&str>>) -> Self {
        match e {
            Err::Error(e) | Err::Failure(e) => ParseError {
                message: convert_error("", e),  // approximate — real impl tracks full input
            },
            Err::Incomplete(_) => ParseError {
                message: "incomplete input".to_string(),
            },
        }
    }
}

pub fn parse_idl(input: &str) -> Result<Module, ParseError> {
    let (remaining, module) = parse_module(input).map_err(ParseError::from)?;
    // Ensure everything was consumed
    let _ = ws0(remaining).map_err(ParseError::from)?;
    if !remaining.is_empty() {
        return Err(ParseError {
            message: format!("trailing input: '{}'", &remaining[..remaining.len().min(50)]),
        });
    }
    Ok(module)
}
```

## Testing Strategy

### Unit Tests (per submodule)

Each parser submodule gets targeted tests with small IDL fragments:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_octet() {
        let (rem, ty) = parse_octet("octet").unwrap();
        assert_eq!(ty, TypeRef::Octet);
        assert!(rem.is_empty());
    }

    #[test]
    fn test_parse_unsigned_long_long() {
        let (rem, ty) = parse_unsigned_long_long("unsigned long long").unwrap();
        assert_eq!(ty, TypeRef::UnsignedLongLong);
    }

    #[test]
    fn test_long_long_before_long() {
        // Verify "long long" is not consumed as "long" + leftover " long"
        let (rem, ty) = parse_base_type("long long name").unwrap();
        assert_eq!(ty, TypeRef::LongLong);
        assert!(rem.starts_with("name"));
    }
}
```

### Integration Tests (end-to-end)

Full IDL documents matching Go test cases:

```rust
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
    // ... verify structure
}

#[test]
fn test_parse_bitset() {
    let idl = r#"
        module spi {
            bitset idbits {
                bitfield<4> bid;
                bitfield<12> cid;
            };
            struct CANFrame {
                octet header;
                idbits id;
            };
        }
    "#;
    let module = parse_idl(idl).unwrap();
    // ... verify bitset fields
}
```

## Dependencies

```toml
[dependencies]
nom = "8"                # Parser combinators
thiserror = "2"          # Ergonomic error types (optional)
```

## Go → Rust Parser Comparison

| Feature | Go (gomme) | Rust (nom) |
|---------|-----------|------------|
| Parser type | `gomme.Parser[I, O]` function type | `FnMut(&str) -> IResult<&str, O>` |
| Result type | `struct { Output, Remaining, Err }` | `Result<(&str, O), Err>` |
| Map | `gomme.Map(p, fn)` | `map(p, fn)` |
| Error info | String error | `VerboseError` with position stack |
| Zero-copy | Manual string slicing | `&str` spans natively |
| Generic support | Go 1.18+ generics | First-class generics |
| Community | Small, niche | Large, de facto standard for Rust |
