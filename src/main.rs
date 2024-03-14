use std::fs;

use md_converter::ast::Pandoc;

fn main() {
    let content = fs::read_to_string("test.json").unwrap();
    let pandoc: Pandoc = serde_json::from_str(&content).unwrap();
}
