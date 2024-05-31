use crate::ast::Pandoc;
use crate::traits::AstWriter;
use derive_more::Display;
use std::error::Error;

pub struct TypstWriter;

impl AstWriter for TypstWriter {
    type WriteError = WriteError;

    fn write(self, ast: Pandoc) -> Result<String, Self::WriteError> {
        todo!()
    }
}

#[derive(Debug, Display)]
pub enum WriteError {
    NotImplemented(&'static str),
}

impl Error for WriteError {}
