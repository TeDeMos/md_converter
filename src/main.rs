#![feature(unwrap_infallible)]

use md_converter::inline_parser::InlineParser;
use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;
use md_converter::ast::Inline;

fn main() {
    let result = MdReader::read(
        "1.  A paragraph\n    with two lines.\n\n        indented code\n\n    > A block quote.\n",
    )
    .into_ok();
    dbg!(result);
}
