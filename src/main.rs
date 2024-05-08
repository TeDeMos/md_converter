#![feature(unwrap_infallible)]

use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;

fn main() {
    let result = MdReader::read("### foo ###").into_ok();
    dbg!(result);
}
