pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;

use std::fs;
use std::io;
use std::path::Path;

use crate::asm::ast::Program;
use crate::asm::diagnostics::Diagnostic;

pub fn parse_file(path: &Path) -> io::Result<(Program, Vec<Diagnostic>)> {
    let source = fs::read_to_string(path)?;
    Ok(parser::parse(&source))
}
