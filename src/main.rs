#![feature(unwrap_infallible)]

use md_converter::inline_parser::InlineParser;
use md_converter::md_reader::MdReader;
use md_converter::traits::AstReader;
use md_converter::ast::Inline;

fn main() {
    let _result = MdReader::read("### foo ###").into_ok();
    //let result = MdReader::read("> ```\n> aaa\n\nbbb").into_ok();
    let test = vec!["hello        rust \\' \\ab".to_string()];
    let result = InlineParser::parse_lines(&test);
    let Inline::Str(s) = &result[4] else {return};
    for c in s.chars(){
        println!("{}",c);
    }
    dbg!(result);
}
