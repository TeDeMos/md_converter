#![feature(unwrap_infallible)]

use md_converter::inline_parser::InlineParser;
use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;
use md_converter::ast::Inline;

fn main() {
    let result = MdReader::read("* foo\n  * bar\n\n  baz\n").into_ok();
    dbg!(result);
}
