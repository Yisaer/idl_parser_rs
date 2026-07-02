//! String type parser: `string` or `string<N>`.

use nom::bytes::complete::tag;
use nom::combinator::opt;
use nom::sequence::delimited;
use nom::Parser;

use super::util::{in_ws, integer};
use super::IResult;
use crate::ast::TypeRef;

/// Parse a string type (used inside base_type).
/// Note: does NOT consume trailing whitespace after `>` — the caller (field parser)
/// handles the required whitespace between type and field name.
pub(crate) fn parse_string_inner(input: &str) -> IResult<'_, TypeRef> {
    let (input, _) = tag("string")(input)?;

    // Optionally match "<N>"
    // Use tag(">") without ws0 wrapper to preserve trailing whitespace for field parser
    let (input, length) = opt(delimited(in_ws(tag("<")), integer, tag(">"))).parse(input)?;

    Ok((input, TypeRef::String { length }))
}
