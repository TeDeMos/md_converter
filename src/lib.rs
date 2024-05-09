#![feature(let_chains, unwrap_infallible, never_type, if_let_guard)]
#![warn(clippy::pedantic, clippy::nursery)]
#![deny(dead_code)]

pub mod ast;
pub mod inline_parser;
pub mod latex_writer;
pub mod md_reader;
pub mod traits;
pub mod typst_writer;
