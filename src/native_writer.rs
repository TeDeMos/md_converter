use crate::ast::Pandoc;
use crate::traits::AstWriter;

pub struct NativeWriter;

impl AstWriter for NativeWriter {
    type WriteError = serde_json::Error;

    fn write(self, ast: Pandoc) -> Result<String, Self::WriteError> {
        serde_json::to_string(&ast)
    }
}