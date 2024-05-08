use std::iter::Peekable;
use std::str::Chars;

use super::{skip_indent, AtxHeading, BlockQuote, FencedCodeBlock, LineResult, List, ListResult, ParagraphListResult, Table, ThematicBreak, ToLineResult, SkipIndent};
use crate::ast::Block;
use crate::inline_parser::InlineParser;

#[derive(Debug)]
pub struct Paragraph {
    lines: Vec<String>,
    table_header_length: usize,
    setext: usize,
}

impl Paragraph {
    pub fn new2(line: SkipIndent) -> Self {
        Self {
            lines: vec![line.get_full()],
            table_header_length: Table::check_header2(line),
            setext: 0,
        }
    }
    
    pub fn new(line: &str) -> Self {
        Self {
            lines: vec![line.to_owned()],
            table_header_length: Table::check_header(line),
            setext: 0,
        }
    }
    

    pub fn next(&mut self, line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line.trim_end(), 4);
        if indent >= 4 {
            self.push(line, false)
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => self.push(line, false),
                },
                Some('*') => match List::check_star_non_empty(indent, line, &mut iter) {
                    ListResult::List(b) => b.done_self_and_new(),
                    ListResult::Break(b) => b.done_self_and_other(),
                    ListResult::None => self.push(line, false),
                },
                Some('-') => match List::check_dash_paragraph(indent, line, &mut iter) {
                    ParagraphListResult::List(b) => b.done_self_and_new(),
                    ParagraphListResult::Break(b) => b.done_self_and_other(),
                    ParagraphListResult::Setext => {
                        self.setext = 2;
                        LineResult::DoneSelf
                    },
                    ParagraphListResult::None => self.push(line, false),
                },
                Some('_') => match ThematicBreak::check('_', &mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => self.push(line, false),
                },
                Some('+') => match List::check_plus_non_empty(indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => self.push(line, false),
                },
                Some('#') => match AtxHeading::check(&mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => self.push(line, false),
                },
                Some('>') => BlockQuote::new(indent, line, &mut iter).done_self_and_new(),
                Some('1') => match List::check_number_paragraph(indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => self.push(line, false),
                },
                Some('=') => self.check_equals_setext(line, &mut iter),
                _ if !iter.all(char::is_whitespace) => self.push(line, false),
                _ => LineResult::DoneSelf,
            }
        }
    }

    pub fn continuation(
        &mut self, indent: usize, first: Option<char>, line: &str, iter: &mut Peekable<Chars>,
    ) -> LineResult {
        match first {
            Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, iter) {
                Some(b) => b.done_self_and_new(),
                None => self.push(line, true),
            },
            Some(c @ ('-' | '*')) => match List::check_other(c, indent, line, iter) {
                ListResult::List(b) => b.done_self_and_new(),
                ListResult::Break(b) => b.done_self_and_other(),
                ListResult::None => self.push(line, true),
            },
            Some('_') => match ThematicBreak::check('_', iter) {
                Some(b) => b.done_self_and_other(),
                None => self.push(line, true),
            },
            Some('+') => match List::check_plus(indent, line, iter) {
                Some(b) => b.done_self_and_new(),
                None => self.push(line, true),
            },
            Some('#') => match AtxHeading::check(iter) {
                Some(b) => b.done_self_and_other(),
                None => self.push(line, true),
            },
            Some('>') => BlockQuote::new(indent, line, iter).done_self_and_new(),
            Some('1') => match List::check_number_paragraph(indent, line, iter) {
                Some(b) => b.done_self_and_new(),
                None => self.push(line, true),
            },
            _ if !iter.all(char::is_whitespace) => self.push(line, true),
            _ => LineResult::DoneSelf,
        }
    }

    pub fn next_blank(&self) -> LineResult { LineResult::DoneSelf }

    pub fn finish(self) -> Block {
        let parsed = InlineParser::parse_lines(&self.lines);
        match self.setext {
            0 => Block::Para(parsed),
            _ => Block::new_header(self.setext, parsed),
        }
    }

    pub fn over_indented_continuation(&mut self, line: &str) { self.push(line, true); }

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

    pub fn push(&mut self, line: &str, non_split: bool) -> LineResult {
        // Safety: paragraph is initialized with one item in vector, pop can't return None
        if !non_split
            && let Some(t) =
                Table::check(line, self.lines.last().unwrap(), self.table_header_length)
        {
            self.lines.pop();
            match self.lines.is_empty() {
                true => t.new(),
                false => t.done_self_and_new(),
            }
        } else {
            self.lines.push(line.to_owned());
            self.table_header_length = Table::check_header(line);
            LineResult::None
        }
    }
}
