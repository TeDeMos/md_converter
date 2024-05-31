#![feature(unwrap_infallible)]

use md_converter::ast::Inline;
use md_converter::inline_parser::InlineParser;
use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;

fn main() {
    let result = MdReader::read(
        "a\n:-"
    )
    .into_ok();
    dbg!(result);
}