use crate::ast::Pandoc;
use crate::traits::AstReader;

pub struct MdReader;

impl AstReader for MdReader {
    type ReadError = ();

    fn read(&mut self, str: &str) -> Result<Pandoc, Self::ReadError> {
        todo!()
    }
}