pub mod ast;
pub mod decoder;
pub mod parser;

/// Parse IDL source code into a Module AST.
///
/// This is the main entry point for parsing OMG IDL text.
///
/// # Example
///
/// ```
/// use idl_parser_rs::parse_idl;
///
/// let idl = r#"
///     module example {
///         struct Point {
///             long x;
///             long y;
///         };
///     }
/// "#;
/// let module = parse_idl(idl).unwrap();
/// assert_eq!(module.name, "example");
/// ```
pub fn parse_idl(input: &str) -> Result<ast::Module, parser::ParseError> {
    parser::parse_idl(input)
}
