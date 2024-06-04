//! # Library for parsing GitHub Flavoured Markdown into LaTeX and Typst
//!
//! This library provides a Pandoc compatible type for representing a
//! parsed document, traits for parsing documents into and from this
//! type as well as implementations for a gfm reader and LaTeX and
//! Typst writers.

#![warn(clippy::pedantic, clippy::nursery)]

pub mod ast;
pub mod latex_writer;
pub mod maps;
pub mod md_reader;
pub mod native_reader;
pub mod native_writer;
pub mod traits;
pub mod typst_writer;
