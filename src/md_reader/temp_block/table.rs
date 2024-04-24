use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, AtxHeading, BlockQuote, FencedCodeBlock, LineResult, TempBlock, ThematicBreak,
};
use crate::ast::{Alignment, Block};

pub struct Table {
    size: usize,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(header: String, alignments: Vec<Alignment>) -> Self {
        let mut result = Self { size: alignments.len(), alignments, rows: Vec::new() };
        result.split_rows(&header);
        result
    }

    pub fn next(&mut self, line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line, 4);
        if indent >= 4 {
            match iter.clone().all(char::is_whitespace) {
                false => self.push(line),
                true => LineResult::DoneSelf,
            }
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) => self.check_code_block(indent, c, line, &mut iter),
                Some(c @ ('*' | '_' | '-')) => self.check_break(c, line, &mut iter),
                Some('#') => self.check_atx_heading(line, &mut iter),
                Some('>') => LineResult::DoneSelfAndNew(TempBlock::BlockQuote(BlockQuote::new(
                    indent, line, &mut iter,
                ))),
                _ if !iter.all(char::is_whitespace) => self.push(line),
                _ => LineResult::DoneSelf,
            }
        }
    }

    pub fn finish(self) -> Block { Block::new_table(self.rows, self.alignments, self.size) }

    fn check_code_block(
        &mut self, indent: usize, first: char, line: &str, rest: &mut Peekable<Chars>,
    ) -> LineResult {
        FencedCodeBlock::check(indent, first, rest)
            .map(|f| LineResult::DoneSelfAndNew(TempBlock::FencedCodeBlock(f)))
            .unwrap_or_else(|| self.push(line))
    }

    fn check_break(&mut self, first: char, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        ThematicBreak::check(first, rest)
            .map(|t| LineResult::DoneSelfAndOther(TempBlock::ThematicBreak(t)))
            .unwrap_or_else(|| self.push(line))
    }

    fn check_atx_heading(&mut self, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        AtxHeading::check(rest)
            .map(|a| LineResult::DoneSelfAndOther(TempBlock::AtxHeading(a)))
            .unwrap_or_else(|| self.push(line))
    }

    fn push(&mut self, line: &str) -> LineResult {
        self.split_rows(line);
        LineResult::None
    }

    fn split_rows(&mut self, line: &str) {
        let mut iter = line.trim().chars().peekable();
        iter.next_if_eq(&'|');
        let mut result = Vec::new();
        let mut current = String::new();
        loop {
            match iter.next() {
                Some('\\') => current.push(iter.next_if_eq(&'|').unwrap_or('\\')),
                Some('|') | None => {
                    result.push(current);
                    if result.len() == self.size {
                        self.rows.push(result);
                        return;
                    }
                    current = String::new();
                },
                Some(c) => current.push(c),
            };
        }
    }
}
