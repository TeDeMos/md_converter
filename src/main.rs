#![feature(unwrap_infallible)]

use md_converter::ast::Inline;
use md_converter::inline_parser::InlineParser;
use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;

fn main() {
    let result = MdReader::read(
        "1.  A paragraph\n    with two lines.\n\n        indented code\n\n    > A block quote.\n",
    )
    .into_ok();
    dbg!(result);
    let mut s = String::from("dal[pjfela");
    unsafe {
        for b in s.as_bytes_mut() {
            if *b == b'\n' {
                *b = b' ';
            }
        }
    }
}