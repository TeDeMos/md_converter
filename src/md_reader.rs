//! Module containing the [`MdReader`] type used for parsing GitHub Flavoured Markdown

use std::convert::Infallible;
use std::iter;

pub use links::{Link, Links};
use temp_block::TempBlock;

use crate::ast::Pandoc;
use crate::traits::AstReader;

pub mod inline_parser;
mod iters;
mod links;
mod temp_block;

/// Struct used for parsing GitHub Flavoured Markdown into the [`Pandoc`] type
pub struct MdReader;

impl AstReader for MdReader {
    type ReadError = Infallible;

    fn read(self, source: &str) -> Result<Pandoc, Self::ReadError> {
        let mut current = TempBlock::default();
        let mut finished = Vec::new();
        let mut links = Links::new();
        for line in source.lines() {
            current.next_str(line, &mut finished, &mut links);
        }
        current.finish_links(&mut links);
        let result = finished
            .into_iter()
            .chain(iter::once(current))
            .filter_map(|t| t.finish(&links))
            .collect();
        Ok(Pandoc { blocks: result, ..Default::default() })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::process::{Command, Stdio};

    use lazy_static::lazy_static;

    use crate::ast::*;

    use super::*;

    lazy_static! {
        static ref TESTS: Vec<String> =
            serde_json::from_str(&fs::read_to_string("test/github.json").unwrap()).unwrap();
    }

    fn test(first: usize, last: usize) {
        let mut results = Vec::new();
        for (i, e) in TESTS[(first - 1)..last].iter().enumerate() {
            let mut child = Command::new("pandoc")
                .args(["-f", "gfm", "-t", "json"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.as_mut().unwrap().write_all(e.as_bytes()).unwrap();
            let number = i + first;
            let expected: Pandoc = serde_json::from_str(
                std::str::from_utf8(&child.wait_with_output().unwrap().stdout).unwrap(),
            )
            .unwrap();
            let result = MdReader.read(e).unwrap();
            if result.blocks == expected.blocks {
                println!("\n\x1b[32mExample {number} : success");
                println!("Input:\n{e}");
                println!("Output:\n{result:?}");
            } else {
                println!("\n\x1b[31mExample {number} : failure");
                println!("Input:\n{e}");
                println!("Output:\n{result:?}");
                println!("Expected: \n{expected:?}");
                results.push(number);
            }
        }
        assert!(results.is_empty(), "Tests {results:?} failed");
    }

    #[test]
    fn tabs_and_precedence() { test(1, 12) }

    #[test]
    fn thematic_breaks() { test(13, 31) }

    #[test]
    fn atx_headings() { test(32, 49) }

    #[test]
    fn setext_headings() { test(50, 76) }

    #[test]
    fn indented_code_blocks() { test(77, 88) }

    #[test]
    fn fenced_code_blocks() { test(89, 117) }

    #[test]
    fn html_blocks() { test(118, 160) }

    #[test]
    fn link_reference_definitions() { test(161, 188) }

    #[test]
    fn paragraphs_and_blank_lines() { test(189, 197) }

    #[test]
    fn tables() { test(198, 205) }

    #[test]
    fn block_quotes() { test(206, 230) }

    #[test]
    fn list_items() { test(231, 280) }

    #[test]
    fn lists() { test(281, 306) }

    #[test]
    fn escaping() { test(309, 320) }

    #[test]
    fn code_span() { test(338, 359) }

    #[test]
    fn emph() { test(360, 488) }

    #[test]
    fn emph_singular() { test(345, 346) }
}
