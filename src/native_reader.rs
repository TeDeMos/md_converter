use crate::ast::Pandoc;
use crate::traits::AstReader;

pub struct NativeReader;

impl AstReader for NativeReader {
    type ReadError = serde_json::Error;

    fn read(self, str: &str) -> Result<Pandoc, Self::ReadError> { serde_json::from_str(str) }
}
