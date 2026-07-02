//! Field parser: `[annotations] type_ref identifier ";"`.

use nom::bytes::complete::tag;
use nom::character::complete::multispace1;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::terminated;
use nom::Parser;

use super::annotation::parse_annotation;
use super::r#type::parse_type_ref;
use super::util::{identifier, in_ws, ws0};
use super::IResult;
use crate::ast::Field;

/// Parse a single field definition.
pub(crate) fn parse_field(input: &str) -> IResult<'_, Field> {
    // Parse optional annotations, each terminated by optional whitespace.
    // Using terminated(annotation, ws0) ensures that annotation failures
    // (first char is not '@') don't consume any whitespace.
    let (input, annotations) = many0(terminated(parse_annotation, ws0)).parse(input)?;
    let (input, field_type) = parse_type_ref(input)?;
    // Require at least one whitespace between type and name
    let (input, _) = multispace1(input)?;
    let (input, name) = map(identifier, |s: &str| s.to_string()).parse(input)?;
    let (input, _) = in_ws(tag(";")).parse(input)?;

    Ok((
        input,
        Field {
            annotations,
            field_type,
            name,
        },
    ))
}
