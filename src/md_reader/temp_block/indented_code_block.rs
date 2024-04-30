use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, AtxHeading, BlockQuote, FencedCodeBlock, LineResult, List, ListResult, Paragraph,
    ThematicBreak, ToLineResult,
};
use crate::ast::{attr_empty, Block};

#[derive(Debug)]
pub struct IndentedCodeBlock(Vec<String>);

impl IndentedCodeBlock {
    pub fn new(rest: &mut Peekable<Chars>) -> Self { Self(vec![rest.collect()]) }

    pub fn next(&mut self, line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line, 4);
        if indent >= 4 {
            self.0.push(iter.collect());
            LineResult::None
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(c @ ('*' | '-')) => match List::check_other(c, indent, line, &mut iter) {
                    ListResult::List(b) => b.done_self_and_new(),
                    ListResult::Break(b) => b.done_self_and_other(),
                    ListResult::None => Paragraph::new(line).new(),
                },
                Some('_') => match ThematicBreak::check('_', &mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('+') => match List::check_plus(indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('#') => match AtxHeading::check(&mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('>') => BlockQuote::new(indent, line, &mut iter).done_self_and_new(),
                Some(c @ '0'..='9') => match List::check_number(c, indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(_) => Paragraph::new(line).done_self_and_new(),
                _ => {
                    self.0.push(iter.collect());
                    LineResult::None
                },
            }
        }
    }

    pub fn next_blank(&mut self) -> LineResult {
        self.0.push(String::new());
        LineResult::None
    }

    pub fn finish(mut self) -> Block {
        while let Some(last) = self.0.last()
            && last.chars().all(char::is_whitespace)
        {
            self.0.pop();
        }
        let mut result = String::new();
        for l in self.0 {
            result.push_str(&l);
            result.push('\n');
        }
        result.pop();
        Block::CodeBlock(attr_empty(), result)
    }
}
