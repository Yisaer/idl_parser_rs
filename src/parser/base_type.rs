//! Base type parser: all 13 primitive type alternatives plus sequence and string.
//!
//! **Critical**: `alt` branch ordering matters. Multi-word types must be tried
//! before their prefix words:
//! - `unsigned long long` before `unsigned long`
//! - `long long` before `long`
//! - `unsigned short` after `unsigned long` (no prefix conflict with `unsigned long`)

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::sequence::{delimited, preceded, separated_pair};
use nom::Parser;

use super::util::{in_ws, integer, ws1};
use super::IResult;
use crate::ast::TypeRef;

/// Parse a base type (without array suffix).
/// All 13 variants + sequence + string + bitfield + type_name.
pub(crate) fn parse_base_type(input: &str) -> IResult<'_, TypeRef> {
    alt((
        // sequence<T> — must be first (starts with keyword "sequence")
        parse_sequence_base,
        // Multi-word types: longest prefixes first
        parse_unsigned_long_long,
        parse_unsigned_long,
        parse_long_long,
        // Single-word and two-word types
        parse_unsigned_short,
        parse_long,
        parse_octet,
        parse_short,
        parse_double,
        parse_float,
        parse_boolean,
        // string or string<N>
        parse_string_base,
        // bitfield<N>
        parse_bitfield_base,
        // Fallback: any identifier (user-defined type)
        parse_type_name_base,
    ))
    .parse(input)
}

// --- individual primitive parsers ---

fn parse_octet(input: &str) -> IResult<'_, TypeRef> {
    map(tag("octet"), |_| TypeRef::Octet).parse(input)
}

fn parse_short(input: &str) -> IResult<'_, TypeRef> {
    map(tag("short"), |_| TypeRef::Short).parse(input)
}

fn parse_unsigned_short(input: &str) -> IResult<'_, TypeRef> {
    map(separated_pair(tag("unsigned"), ws1, tag("short")), |_| {
        TypeRef::UnsignedShort
    })
    .parse(input)
}

fn parse_long(input: &str) -> IResult<'_, TypeRef> {
    map(tag("long"), |_| TypeRef::Long).parse(input)
}

fn parse_unsigned_long(input: &str) -> IResult<'_, TypeRef> {
    map(separated_pair(tag("unsigned"), ws1, tag("long")), |_| {
        TypeRef::UnsignedLong
    })
    .parse(input)
}

fn parse_long_long(input: &str) -> IResult<'_, TypeRef> {
    map(separated_pair(tag("long"), ws1, tag("long")), |_| {
        TypeRef::LongLong
    })
    .parse(input)
}

fn parse_unsigned_long_long(input: &str) -> IResult<'_, TypeRef> {
    map(
        separated_pair(
            tag("unsigned"),
            ws1,
            separated_pair(tag("long"), ws1, tag("long")),
        ),
        |_| TypeRef::UnsignedLongLong,
    )
    .parse(input)
}

fn parse_float(input: &str) -> IResult<'_, TypeRef> {
    map(tag("float"), |_| TypeRef::Float).parse(input)
}

fn parse_double(input: &str) -> IResult<'_, TypeRef> {
    map(tag("double"), |_| TypeRef::Double).parse(input)
}

fn parse_boolean(input: &str) -> IResult<'_, TypeRef> {
    map(tag("boolean"), |_| TypeRef::Boolean).parse(input)
}

fn parse_string_base(input: &str) -> IResult<'_, TypeRef> {
    super::string_type::parse_string_inner(input)
}

fn parse_sequence_base(input: &str) -> IResult<'_, TypeRef> {
    super::sequence::parse_sequence_inner(input)
}

fn parse_bitfield_base(input: &str) -> IResult<'_, TypeRef> {
    map(
        preceded(
            tag("bitfield"),
            delimited(in_ws(tag("<")), integer, tag(">")),
        ),
        |width: u32| TypeRef::BitField { width: width as u8 },
    )
    .parse(input)
}

fn parse_type_name_base(input: &str) -> IResult<'_, TypeRef> {
    map(super::util::identifier, |name: &str| TypeRef::TypeName {
        name: name.to_string(),
    })
    .parse(input)
}
