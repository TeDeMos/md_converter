//! Module containing the [`NativeReader`] type for reading [`Pandoc`] ast from JSON

use crate::ast::Pandoc;
use crate::traits::AstReader;

/// Deserialized a [`Pandoc`] ast representation from JSON for easy communication with Pandoc app
pub struct NativeReader;

impl AstReader for NativeReader {
    type ReadError = serde_json::Error;

    fn read(self, str: &str) -> Result<Pandoc, Self::ReadError> { serde_json::from_str(str) }
}
