//! # Library for parsing GitHub Flavoured Markdown into LaTeX and Typst
//!
//! This library provides a Pandoc compatible type for representing a
//! parsed document, traits for parsing documents into and from this
//! type as well as implementations for a gfm reader and LaTeX and
//! Typst writers.

#![feature(let_chains, unwrap_infallible, never_type, if_let_guard)]
#![warn(clippy::pedantic, clippy::nursery)]
//#![deny(dead_code)]
#![warn(missing_docs)]

pub mod ast;
pub mod latex_writer;
pub mod md_reader;
pub mod native_reader;
pub mod native_writer;
pub mod traits;
pub mod typst_writer;
