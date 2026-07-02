//! Sequence type parser: `sequence<T>`.

use nom::bytes::complete::tag;
use nom::Parser;

use super::r#type::parse_type_ref;
use super::util::in_ws;
use super::IResult;
use crate::ast::TypeRef;

/// Parse a sequence type (used inside base_type).
/// Note: does NOT consume trailing whitespace after `>` — the caller (field parser)
/// handles the required whitespace between type and field name.
pub(crate) fn parse_sequence_inner(input: &str) -> IResult<'_, TypeRef> {
    let (input, _) = tag("sequence")(input)?;
    let (input, _) = in_ws(tag("<")).parse(input)?;
    let (input, inner) = in_ws(parse_type_ref).parse(input)?;
    // Use tag(">") without ws0 wrapper to preserve trailing whitespace
    let (input, _) = tag(">")(input)?;

    Ok((
        input,
        TypeRef::Sequence {
            inner: Box::new(inner),
        },
    ))
}
