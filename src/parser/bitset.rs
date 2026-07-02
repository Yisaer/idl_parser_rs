//! Bitset definition parser: `bitset name { bitfield<N> name;* }`.

use nom::bytes::complete::tag;
use nom::character::complete::multispace1;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::delimited;
use nom::Parser;

use super::util::{identifier, in_ws, integer, ws0, ws1};
use super::IResult;
use crate::ast::{BitSet, BitSetField};

/// Parse a bitset definition.
pub(crate) fn parse_bitset(input: &str) -> IResult<'_, BitSet> {
    let (input, _) = delimited(ws0, tag("bitset"), ws1).parse(input)?;
    let (input, name) = map(identifier, |s: &str| s.to_string()).parse(input)?;
    let (input, _) = in_ws(tag("{")).parse(input)?;
    let (input, fields) = many0(parse_bitfield_def).parse(input)?;
    let (input, _) = in_ws(tag("}")).parse(input)?;

    Ok((input, BitSet { name, fields }))
}

/// Parse a single bitfield definition: `bitfield<N> name;`.
fn parse_bitfield_def(input: &str) -> IResult<'_, BitSetField> {
    let (input, _) = tag("bitfield")(input)?;
    let (input, width) = delimited(in_ws(tag("<")), integer, tag(">")).parse(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = map(identifier, |s: &str| s.to_string()).parse(input)?;
    let (input, _) = in_ws(tag(";")).parse(input)?;

    Ok((
        input,
        BitSetField {
            width: width as u8,
            name,
        },
    ))
}
