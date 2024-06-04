//! Module containing the [`NativeWriter`] type for writing [`Pandoc`] ast to JSON

use crate::ast::Pandoc;
use crate::traits::AstWriter;

/// Serializes a [`Pandoc`] ast representation into JSON for easy communication with Pandoc app
pub struct NativeWriter;

impl AstWriter for NativeWriter {
    type WriteError = serde_json::Error;

    fn write(self, mut ast: Pandoc) -> Result<String, Self::WriteError> {
        ast.pandoc_api_version = vec![1, 23, 1];
        serde_json::to_string(&ast)
    }
}
