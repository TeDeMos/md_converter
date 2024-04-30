use crate::ast::Pandoc;
use crate::traits::AstWriter;

pub struct TypstWriter;

impl AstWriter for TypstWriter {
    type WriteError = ();

    fn write(&mut self, _ast: Pandoc) -> Result<String, Self::WriteError> { todo!() }
}
