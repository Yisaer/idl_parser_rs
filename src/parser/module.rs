//! Module parser: the top-level `module name { content* }` rule.

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::delimited;
use nom::Parser;

use super::bitset::parse_bitset;
use super::struct_type::parse_struct;
use super::util::{identifier, in_ws, ws0, ws1};
use super::IResult;
use crate::ast::{Module, ModuleContent};

/// Parse a module definition (including nested modules).
pub(crate) fn parse_module(input: &str) -> IResult<'_, Module> {
    let (input, _) = delimited(ws0, tag("module"), ws1).parse(input)?;
    let (input, name) = map(identifier, |s: &str| s.to_string()).parse(input)?;
    let (input, _) = in_ws(tag("{")).parse(input)?;
    let (input, content) = many0(parse_module_content).parse(input)?;
    let (input, _) = in_ws(tag("}")).parse(input)?;

    Ok((input, Module { name, content }))
}

/// Parse module content: sub-module, struct, or bitset, optionally terminated by `;`.
fn parse_module_content(input: &str) -> IResult<'_, ModuleContent> {
    let (input, content) = alt((
        map(parse_struct, ModuleContent::Struct),
        map(parse_bitset, ModuleContent::BitSet),
        map(parse_module, |m| ModuleContent::Module(Box::new(m))),
    ))
    .parse(input)?;
    // Optional trailing semicolon (matching Go version behavior)
    let (input, _) = nom::combinator::opt(in_ws(tag(";"))).parse(input)?;
    Ok((input, content))
}
