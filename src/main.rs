#![feature(unwrap_infallible)]

use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;

fn main() {
    let result = MdReader::read("1. ```\n   foo\n   ```\n\n   bar\n").into_ok();
    dbg!(result);
}
