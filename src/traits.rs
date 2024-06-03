//! Module containing traits from reading to and writing from [`Pandoc`] ast

use std::error::Error;

use crate::ast::Pandoc;

/// Trait for reading a file format and parsing it into a [`Pandoc`] ast representation.
pub trait AstReader {
    /// Conversion error
    type ReadError: Error;

    /// Reads a given string slice and parses it into a [`Pandoc`] ast representation.
    /// # Errors
    /// Returns an error when parsing was not successful
    fn read(self, str: &str) -> Result<Pandoc, Self::ReadError>;
}

/// Trait for writing a [`Pandoc`] ast representation into a file format
pub trait AstWriter {
    /// Writing error
    type WriteError: Error;

    /// Writes a given [`Pandoc`] ast representation into a file format
    /// # Errors
    /// Returns an error when writing was not successful
    fn write(self, ast: Pandoc) -> Result<String, Self::WriteError>;
}
