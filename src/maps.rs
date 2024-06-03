//! Module containing containers for holding readers and writers

use std::collections::HashMap;
use std::error::Error;

use crate::ast::Pandoc;
use crate::traits::{AstReader, AstWriter};

/// Wrapper over an [`AstReader`] type that takes a function creating the reader and calls it,
/// calls the read function and wraps an error into a boxed trait object
pub type Reader = Box<dyn Fn(&str) -> Result<Pandoc, Box<dyn Error>>>;

/// Container for holding readers
#[derive(Default)]
pub struct ReaderMap(HashMap<&'static str, Reader>);

impl ReaderMap {
    /// Creates a new empty reader map
    #[must_use]
    pub fn new() -> Self { Self(HashMap::new()) }

    /// Adds a new reader to the map from a function creating an instance of the reader
    pub fn add<T, F>(&mut self, name: &'static str, reader_creator: F)
    where
        T: AstReader + 'static,
        T::ReadError: Error + 'static,
        F: Fn() -> T + 'static,
    {
        self.0.insert(
            name,
            Box::new(move |s| match reader_creator().read(s) {
                Ok(p) => Ok(p),
                Err(e) => Err(Box::new(e)),
            }),
        );
    }

    /// Gets an iterator over the keys of the map
    pub fn keys(&self) -> impl Iterator<Item = &&'static str> { self.0.keys() }

    /// Reads a string to a [`Pandoc`] ast with a given reader
    /// # Errors
    /// Returns an error received from a reader as a boxed trait object
    /// # Panics
    /// If key is not in map
    pub fn read(&self, name: &str, source: &str) -> Result<Pandoc, Box<dyn Error>> {
        self.0.get(name).unwrap()(source)
    }
}

/// Wrapper over an [`AstWriter`] type that takes a function creating the writer and calls it,
/// calls the write function and wraps an error into a boxed trait object
pub type Writer = Box<dyn Fn(Pandoc) -> Result<String, Box<dyn Error>>>;

/// Container for holding writers
#[derive(Default)]
pub struct WriterMap(HashMap<&'static str, Writer>);

impl WriterMap {
    /// Creates a new empty writer map
    #[must_use]
    pub fn new() -> Self { Self(HashMap::new()) }

    /// Adds a new writer to the map from a function creating an instance of the writer
    pub fn add<T, F>(&mut self, name: &'static str, writer_creator: F)
    where
        T: AstWriter + 'static,
        T::WriteError: Error + 'static,
        F: Fn() -> T + 'static,
    {
        self.0.insert(
            name,
            Box::new(move |p| match writer_creator().write(p) {
                Ok(s) => Ok(s),
                Err(e) => Err(Box::new(e)),
            }),
        );
    }

    /// Gets an iterator over the keys of the map
    pub fn keys(&self) -> impl Iterator<Item = &&'static str> { self.0.keys() }

    /// Writes a [`Pandoc`] ast to a string with a given writer
    /// # Errors
    /// Returns an error received from a writer as a boxed trait object
    /// # Panics
    /// If key is not in map
    pub fn write(&self, name: &str, pandoc: Pandoc) -> Result<String, Box<dyn Error>> {
        self.0.get(name).unwrap()(pandoc)
    }
}
