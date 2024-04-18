use std::iter::Peekable;
use std::str::Chars;

use super::paragraph::Paragraph;
use super::{skip_indent, LineResult, TempBlock};
use crate::ast::{attr_empty, Block};
use super::thematic_break::ThematicBreak;
use super::atx_heading::AtxHeading;
use super::fenced_code_block::FencedCodeBlock;

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
                Some(c @ ('~' | '`')) => self.check_fenced_code_block(indent, c, line, &mut iter),
                Some(c @ ('*' | '_' | '-')) => self.check_thematic_break(c, line, &mut iter),
                Some('#') => self.check_atx_heading(line, &mut iter),
                Some(_) => LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line))),
                _ => {
                    self.0.push(iter.collect());
                    LineResult::None
                },
            }
        }
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

    fn check_fenced_code_block(
        &mut self, indent: usize, first: char, line: &str, rest: &mut Peekable<Chars>,
    ) -> LineResult {
        LineResult::DoneSelfAndNew(
            FencedCodeBlock::check(indent, first, rest)
                .map(TempBlock::FencedCodeBlock)
                .unwrap_or_else(|| TempBlock::Paragraph(Paragraph::new(line))),
        )
    }

    fn check_thematic_break(
        &mut self, first: char, line: &str, rest: &mut Peekable<Chars>,
    ) -> LineResult {
        ThematicBreak::check(first, rest)
            .map(|t| LineResult::DoneSelfAndOther(TempBlock::ThematicBreak(t)))
            .unwrap_or_else(|| {
                LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line)))
            })
    }

    fn check_atx_heading(&mut self, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        AtxHeading::check(rest)
            .map(|a| LineResult::DoneSelfAndOther(TempBlock::AtxHeading(a)))
            .unwrap_or_else(|| {
                LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line)))
            })
    }
}
