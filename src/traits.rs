use crate::ast::Pandoc;
use std::collections::HashMap;
use std::error::Error;

pub trait AstReader {
    type ReadError: Error;

    fn read(self, str: &str) -> Result<Pandoc, Self::ReadError>;
}

pub trait AstWriter {
    type WriteError: Error;

    fn write(self, ast: Pandoc) -> Result<String, Self::WriteError>;
}
