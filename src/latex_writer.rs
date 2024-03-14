use crate::ast::Pandoc;
use crate::traits::AstWriter;

pub struct LatexWriter;

impl AstWriter for LatexWriter {
    type WriteError = ();

    fn write(&mut self, ast: Pandoc) -> Result<String, Self::WriteError> {
        todo!()
    }
}