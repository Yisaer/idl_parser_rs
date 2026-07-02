//! Struct definition parser: `struct name { field* }`.

use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::delimited;
use nom::Parser;

use super::field::parse_field;
use super::util::{identifier, in_ws, ws0, ws1};
use super::IResult;
use crate::ast::Struct;

/// Parse a struct definition.
pub(crate) fn parse_struct(input: &str) -> IResult<'_, Struct> {
    let (input, _) = delimited(ws0, tag("struct"), ws1).parse(input)?;
    let (input, name) = map(identifier, |s: &str| s.to_string()).parse(input)?;
    let (input, _) = in_ws(tag("{")).parse(input)?;
    let (input, fields) = many0(parse_field).parse(input)?;
    let (input, _) = in_ws(tag("}")).parse(input)?;

    Ok((input, Struct { name, fields }))
}
