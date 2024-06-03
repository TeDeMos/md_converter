use crate::ast::Pandoc;
use crate::traits::AstWriter;

/// Serializes a [`Pandoc`] ast representation into JSON for easy communication with Pandoc app
pub struct NativeWriter;

impl AstWriter for NativeWriter {
    type WriteError = serde_json::Error;

    fn write(self, ast: Pandoc) -> Result<String, Self::WriteError> { serde_json::to_string(&ast) }
}
