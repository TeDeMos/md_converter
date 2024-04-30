use crate::ast::Pandoc;

pub trait AstReader {
    type ReadError;

    fn read(str: &str) -> Result<Pandoc, Self::ReadError>;
}

pub trait AstWriter {
    type WriteError;

    fn write(&mut self, ast: Pandoc) -> Result<String, Self::WriteError>;
}
