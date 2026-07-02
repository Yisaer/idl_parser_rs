//! AST types for OMG IDL parsed output.
//!
//! All types implement `Debug`, `Clone`, and `PartialEq` for testability.

use std::collections::HashMap;

/// Top-level parsed result of an IDL file.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub content: Vec<ModuleContent>,
}

/// Content items within a module: sub-modules, struct definitions, or bitsets.
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleContent {
    Module(Box<Module>),
    Struct(Struct),
    BitSet(BitSet),
}

/// A struct definition with named fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
}

/// A single field within a struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub annotations: Vec<Annotation>,
    pub field_type: TypeRef,
    pub name: String,
}

/// A bitset definition with bitfield fields.
#[derive(Debug, Clone, PartialEq)]
pub struct BitSet {
    pub name: String,
    pub fields: Vec<BitSetField>,
}

/// A single bitfield within a bitset.
#[derive(Debug, Clone, PartialEq)]
pub struct BitSetField {
    pub width: u8,
    pub name: String,
}

/// An annotation on a field, e.g., `@format(dbc="path")`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    pub name: String,
    pub values: HashMap<String, String>,
}

/// All supported IDL type references.
///
/// Recursive types (`Array`, `Sequence`) use `Box` to prevent infinite size.
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
    /// String type: `None` = dynamic, `Some(n)` = fixed length `n`.
    String {
        length: Option<u32>,
    },
    /// Fixed-size array: `inner_type[size]`.
    Array {
        inner: Box<TypeRef>,
        size: u32,
    },
    /// Dynamic sequence: `sequence<inner_type>`.
    Sequence {
        inner: Box<TypeRef>,
    },
    /// Bitfield with specified width in bits.
    BitField {
        width: u8,
    },
    /// Reference to a user-defined type by name.
    TypeName {
        name: String,
    },
}
