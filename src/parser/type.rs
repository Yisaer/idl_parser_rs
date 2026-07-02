//! Top-level type_ref parser: dispatches to array or base type.

use nom::bytes::complete::tag;
use nom::combinator::opt;
use nom::sequence::delimited;
use nom::Parser;

use super::base_type::parse_base_type;
use super::util::integer;
use super::IResult;
use crate::ast::TypeRef;

/// Parse a type_ref: tries `Array { inner, size }` first, falls back to base_type.
///
/// Uses `opt` to check for `[N]` suffix after the base type parse.
/// If found, wraps the base in `TypeRef::Array`.
pub(crate) fn parse_type_ref(input: &str) -> IResult<'_, TypeRef> {
    let (remaining, base) = parse_base_type(input)?;

    // Check for optional array suffix "[N]"
    let array_suffix = opt(delimited(tag("["), integer, tag("]"))).parse(remaining);

    match array_suffix {
        Ok((remaining2, Some(size))) => Ok((
            remaining2,
            TypeRef::Array {
                inner: Box::new(base),
                size,
            },
        )),
        _ => Ok((remaining, base)),
    }
}
