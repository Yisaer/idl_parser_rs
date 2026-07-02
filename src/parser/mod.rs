//! IDL parser implementation using nom.
//!
//! Grammar (supported subset):
//! ```text
//! module       ::= "module" identifier "{" module_content* "}"
//! module_content ::= module | struct_def | bitset_def
//! struct_def   ::= "struct" identifier "{" field* "}"
//! field        ::= annotation* type_ref identifier ";"
//! bitset_def   ::= "bitset" identifier "{" bitfield_def* "}"
//! bitfield_def ::= "bitfield" "<" integer ">" identifier ";"
//!
//! type_ref     ::= array_type | base_type
//! array_type   ::= base_type "[" integer "]"
//! base_type    ::= sequence_type | primitive_types | string_type | bitfield_type | type_name
//! sequence_type ::= "sequence" "<" type_ref ">"
//! string_type  ::= "string" | "string" "<" integer ">"
//! bitfield_type ::= "bitfield" "<" integer ">"
//! type_name    ::= identifier
//! ```

mod annotation;
mod base_type;
mod bitset;
mod comment;
mod field;
mod module;
mod sequence;
mod string_type;
mod struct_type;
mod r#type;
mod util;

use crate::ast::Module;
use nom::error::Error;

/// Parse error type.
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// nom IResult alias using the default Error type.
type IResult<'a, O> = nom::IResult<&'a str, O>;

impl<'a> From<nom::Err<Error<&'a str>>> for ParseError {
    fn from(e: nom::Err<Error<&'a str>>) -> Self {
        ParseError {
            message: format!("{}", e),
        }
    }
}

/// Parse an IDL string into a Module AST.
///
/// Returns `ParseError` if the input cannot be parsed, including trailing
/// garbage after the module definition.
pub fn parse_idl(input: &str) -> Result<Module, ParseError> {
    let (remaining, module) = module::parse_module(input).map_err(ParseError::from)?;
    // Ensure everything after the module is only whitespace/comments
    let (remaining, _) = util::ws0(remaining).map_err(|_| ParseError {
        message: "failed to skip trailing whitespace".to_string(),
    })?;
    if !remaining.is_empty() {
        return Err(ParseError {
            message: format!(
                "trailing input: '{}'",
                &remaining[..remaining.len().min(50)]
            ),
        });
    }
    Ok(module)
}
