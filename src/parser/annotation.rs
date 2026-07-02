//! Annotation parser: `@name` or `@name(k=v, ...)`.

use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt, recognize};
use nom::multi::{many0, many1, separated_list0};
use nom::sequence::{delimited, preceded, separated_pair};
use nom::Parser;

use super::util::{identifier, in_ws, ws0};
use super::IResult;
use crate::ast::Annotation;

/// Parse zero or more annotations, each preceded by optional whitespace.
#[allow(dead_code)]
pub(crate) fn parse_annotations(input: &str) -> IResult<'_, Vec<Annotation>> {
    many0(preceded(ws0, parse_annotation)).parse(input)
}

/// Parse a single annotation: `@name` or `@name(k=v, ...)`.
pub(crate) fn parse_annotation(input: &str) -> IResult<'_, Annotation> {
    let (input, _) = tag("@")(input)?;
    let (input, name) = map(identifier, |s: &str| s.to_string()).parse(input)?;

    // Optional key=value pairs in parentheses
    let (input, values) =
        opt(delimited(in_ws(tag("(")), parse_kv_pairs, in_ws(tag(")")))).parse(input)?;

    Ok((
        input,
        Annotation {
            name,
            values: values.unwrap_or_default(),
        },
    ))
}

/// Parse comma-separated key=value pairs.
fn parse_kv_pairs(input: &str) -> IResult<'_, HashMap<String, String>> {
    map(
        separated_list0(preceded(ws0, tag(",")), parse_kv_pair),
        |pairs| pairs.into_iter().collect(),
    )
    .parse(input)
}

/// Parse a single key=value pair.
fn parse_kv_pair(input: &str) -> IResult<'_, (String, String)> {
    separated_pair(
        map(in_ws(identifier), |s: &str| s.to_string()),
        in_ws(tag("=")),
        parse_value,
    )
    .parse(input)
}

/// Parse a value: either a quoted string or an unquoted token
/// (letters, digits, `.`, `/`, `_` — same as valid_chars).
fn parse_value(input: &str) -> IResult<'_, String> {
    alt((
        map(
            delimited(tag("\""), parse_valid_chars, tag("\"")),
            |s: &str| s.to_string(),
        ),
        map(parse_valid_chars, |s: &str| s.to_string()),
    ))
    .parse(input)
}

/// Parse valid characters inside a quoted string: letters, digits, `.`, `/`, `_`.
fn parse_valid_chars(input: &str) -> IResult<'_, &str> {
    recognize(many1(nom::character::complete::satisfy(|c: char| {
        c.is_alphanumeric() || c == '.' || c == '/' || c == '_'
    })))
    .parse(input)
}
