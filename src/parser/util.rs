//! Comment and whitespace-aware parsing utilities.

use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{alpha1, digit1, line_ending, multispace1};
use nom::combinator::{map, recognize};
use nom::multi::many0_count;
use nom::sequence::pair;
use nom::Parser;

use super::IResult;

/// Match a single-line comment: `// ... (CRLF | CR | LF)`.
pub(crate) fn comment(input: &str) -> IResult<'_, &str> {
    recognize(pair(
        tag("//"),
        pair(take_until("\n"), line_ending), // handles \n, \r\n, \r
    ))
    .parse(input)
}

/// Match zero or more whitespace-or-comment tokens.
pub(crate) fn ws0(input: &str) -> IResult<'_, &str> {
    recognize(many0_count(alt((multispace1, comment)))).parse(input)
}

/// Match one or more whitespace-or-comment tokens.
pub(crate) fn ws1(input: &str) -> IResult<'_, &str> {
    let (rem, s) = recognize(many0_count(alt((multispace1, comment)))).parse(input)?;
    if s.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::MultiSpace,
        )));
    }
    Ok((rem, s))
}

/// Skip whitespace/comments before and after a parser.
/// Returns a new parser that handles surrounding whitespace.
pub(crate) fn in_ws<'a, F, O>(
    mut parser: F,
) -> impl Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>>
where
    F: Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>>,
{
    move |input: &'a str| {
        let (input, _) = ws0(input)?;
        let (input, out) = parser.parse(input)?;
        let (input, _) = ws0(input)?;
        Ok((input, out))
    }
}

/// Parse an identifier: `[a-zA-Z_][a-zA-Z0-9_]*`.
pub(crate) fn identifier(input: &str) -> IResult<'_, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        nom::multi::many0_count(nom::character::complete::satisfy(|c: char| {
            c.is_alphanumeric() || c == '_'
        })),
    ))
    .parse(input)
}

/// Parse a positive decimal integer.
pub(crate) fn integer(input: &str) -> IResult<'_, u32> {
    map(digit1, |s: &str| s.parse::<u32>().unwrap_or(0)).parse(input)
}
