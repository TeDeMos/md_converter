use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, AtxHeading, BlockQuote, FencedCodeBlock, LineResult, Table, TempBlock,
    ThematicBreak,
};
use crate::ast::{Alignment, Block};
use crate::inline_parser::InlineParser;

pub struct Paragraph {
    lines: Vec<String>,
    table_header_length: usize,
    setext: usize,
}

impl Paragraph {
    pub fn new(line: &str) -> Self {
        let mut result =
            Paragraph { lines: vec![line.to_owned()], table_header_length: 0, setext: 0 };
        result.check_table_header(line);
        result
    }

    pub fn next(&mut self, line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line.trim_end(), 4);
        if indent >= 4 {
            self.push(line, false)
        } else {
            match iter.next() {
                Some('=') => self.check_equals_setext(line, &mut iter),
                Some('-') => self.check_dash_setext_or_break(line, &mut iter),
                Some(c @ ('*' | '_')) => self.check_break(c, line, &mut iter, false),
                Some(c @ ('~' | '`')) => self.check_code_block(indent, c, line, &mut iter, false),
                Some('#') => self.check_atx_heading(line, &mut iter, false),
                Some('>') => LineResult::DoneSelfAndNew(TempBlock::BlockQuote(BlockQuote::new(
                    indent, line, &mut iter,
                ))),
                _ if !iter.all(char::is_whitespace) => self.push(line, false),
                _ => LineResult::DoneSelf,
            }
        }
    }

    pub fn next_no_split(
        &mut self, indent: usize, first: Option<char>, line: &str, iter: &mut Peekable<Chars>,
    ) -> LineResult {
        match first {
            Some(c @ ('-' | '*' | '_')) => self.check_break(c, line, iter, true),
            Some(c @ ('~' | '`')) => self.check_code_block(indent, c, line, iter, true),
            Some('#') => self.check_atx_heading(line, iter, true),
            _ if !iter.all(char::is_whitespace) => self.push(line, true),
            _ => LineResult::DoneSelf,
        }
    }

    pub fn finish(self) -> Block {
        let parsed = InlineParser::parse_lines(&self.lines);
        if self.setext == 0 {
            Block::Para(parsed)
        } else {
            Block::new_header(self.setext, parsed)
        }
    }

    fn check_equals_setext(&mut self, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        let mut whitespace = false;
        loop {
            match rest.next() {
                Some('=') if !whitespace => continue,
                Some(' ' | '\t') => whitespace = true,
                Some(_) => return self.push(line, false),
                None => {
                    self.setext = 1;
                    return LineResult::DoneSelf;
                },
            }
        }
    }

    fn check_dash_setext_or_break(&mut self, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        let mut count = 1;
        let mut whitespace = false;
        let mut thematic = false;
        loop {
            match rest.next() {
                Some('-') => {
                    count += 1;
                    if whitespace {
                        thematic = true;
                    }
                },
                Some(' ' | '\t') => whitespace = true,
                Some(_) => return self.push(line, false),
                None =>
                    return if thematic && count >= 3 {
                        LineResult::DoneSelfAndOther(TempBlock::ThematicBreak(ThematicBreak))
                    } else {
                        self.setext = 2;
                        LineResult::DoneSelf
                    },
            }
        }
    }

    fn check_break(
        &mut self, first: char, line: &str, rest: &mut Peekable<Chars>, non_split: bool,
    ) -> LineResult {
        ThematicBreak::check(first, rest)
            .map(|t| LineResult::DoneSelfAndOther(TempBlock::ThematicBreak(t)))
            .unwrap_or_else(|| self.push(line, non_split))
    }

    fn check_code_block(
        &mut self, indent: usize, first: char, line: &str, rest: &mut Peekable<Chars>,
        non_split: bool,
    ) -> LineResult {
        FencedCodeBlock::check(indent, first, rest)
            .map(|f| LineResult::DoneSelfAndNew(TempBlock::FencedCodeBlock(f)))
            .unwrap_or_else(|| self.push(line, non_split))
    }

    fn check_atx_heading(
        &mut self, line: &str, rest: &mut Peekable<Chars>, non_split: bool,
    ) -> LineResult {
        AtxHeading::check(rest)
            .map(|a| LineResult::DoneSelfAndOther(TempBlock::AtxHeading(a)))
            .unwrap_or_else(|| self.push(line, non_split))
    }

    fn check_table_header(&mut self, line: &str) {
        let mut iter = line.trim().chars();
        let (mut count, mut escape, first) = match iter.next() {
            Some('\\') => (0, true, false),
            Some('|') => (1, false, true),
            _ => (0, false, false),
        };
        let mut previous = false;
        for c in iter {
            if c == '\\' && !escape {
                escape = true;
                previous = false;
            } else if c == '|' && !escape {
                count += 1;
                previous = true;
                escape = false;
            } else {
                escape = false;
                previous = false;
            }
        }
        self.table_header_length =
            if count == 0 { 0 } else { count + 1 - previous as usize - first as usize };
    }

    fn check_table_delimiter(&self, line: &str) -> Option<Vec<Alignment>> {
        let mut iter = line.trim().chars().peekable();
        iter.next_if_eq(&'|');
        let mut result = Vec::new();
        for i in 0..self.table_header_length {
            while matches!(iter.peek(), Some(' ' | '\t')) {
                iter.next();
            }
            let left = iter.next_if_eq(&':').is_some();
            iter.next_if_eq(&'-')?;
            while iter.peek() == Some(&'-') {
                iter.next();
            }
            let right = iter.next_if_eq(&':').is_some();
            while matches!(iter.peek(), Some(' ' | '\t')) {
                iter.next();
            }
            if iter.next_if_eq(&'|').is_none() && i != self.table_header_length - 1 {
                return None;
            }
            result.push(match (left, right) {
                (false, false) => Alignment::Default,
                (false, true) => Alignment::Right,
                (true, false) => Alignment::Left,
                (true, true) => Alignment::Center,
            });
        }
        iter.next().is_none().then_some(result)
    }

    pub fn push(&mut self, line: &str, non_split: bool) -> LineResult {
        if !non_split
            && self.table_header_length != 0
            && let Some(alignments) = self.check_table_delimiter(line)
        {
            // Safety: paragraph is initialized with one item in vector, pop can't return None
            let table = TempBlock::Table(Table::new(self.lines.pop().unwrap(), alignments));
            match self.lines.is_empty() {
                true => LineResult::New(table),
                false => LineResult::DoneSelfAndNew(table),
            }
        } else {
            self.lines.push(line.to_owned());
            self.check_table_header(line);
            LineResult::None
        }
    }
}
