use super::{
    AtxHeading, BlockQuote, DoneResult, FencedCodeBlock, LineResult, List, ListBreakResult,
    ListBreakSetextResult, NewResult, SkipIndent, SkipIndentResult, Table, ThematicBreak,
    ToLineResult,
};
use crate::ast::Block;
use crate::inline_parser::InlineParser;

#[derive(Debug)]
pub(crate) struct Paragraph {
    pub(crate) lines: Vec<String>,
    pub(crate) table_header_length: usize,
    setext: usize,
}

impl Paragraph {
    pub(crate) fn new(line: SkipIndent) -> Self {
        Self {
            lines: vec![line.line.to_owned()],
            table_header_length: Table::check_header(line.line),
            setext: 0,
        }
    }

    pub(crate) fn next(&mut self, line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(line) => match line.indent {
                0..=3 => match line.first {
                    '~' | '`' => match FencedCodeBlock::check(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => self.push_full_check(s),
                    },
                    '*' => match List::check_star_paragraph(line) {
                        ListBreakResult::List(l) => l.done_self_and_new(),
                        ListBreakResult::Break(b) => b.done_self_and_other(),
                        ListBreakResult::Text(s) => self.push_full_check(s),
                    },
                    '-' => match List::check_dash_paragraph(line) {
                        ListBreakSetextResult::List(b) => b.done_self_and_new(),
                        ListBreakSetextResult::Break(b) => b.done_self_and_other(),
                        ListBreakSetextResult::Setext => {
                            self.setext = 2;
                            LineResult::DoneSelf
                        },
                        ListBreakSetextResult::Text(s) => self.push_full_check(s),
                    },
                    '_' => match ThematicBreak::check(line) {
                        DoneResult::Done(b) => LineResult::DoneSelfAndNew(b),
                        DoneResult::Text(s) => self.push_full_check(s),
                    },
                    '+' => match List::check_plus_paragraph(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => self.push_full_check(s),
                    },
                    '#' => match AtxHeading::check(line) {
                        DoneResult::Done(b) => LineResult::DoneSelfAndNew(b),
                        DoneResult::Text(s) => self.push_full_check(s),
                    },
                    '>' => BlockQuote::new(line).done_self_and_new(),
                    '1' => match List::check_number_paragraph(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => self.push_full_check(s),
                    },
                    '=' => self.push_check_setext(line),
                    _ => self.push_full_check(line),
                },
                4.. => {
                    self.push_header_check(line);
                    LineResult::None
                },
            },
            SkipIndentResult::Blank(_) => LineResult::DoneSelf,
        }
    }

    pub(crate) fn next_continuation(&mut self, line: SkipIndent) -> LineResult {
        match line.first {
            '~' | '`' => match FencedCodeBlock::check(line) {
                NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                NewResult::Text(s) => {
                    self.push_header_no_indent_check(s);
                    LineResult::None
                },
            },
            '*' | '-' => match List::check_star_dash(line) {
                ListBreakResult::List(l) => l.done_self_and_new(),
                ListBreakResult::Break(b) => b.done_self_and_other(),
                ListBreakResult::Text(s) => {
                    self.push_header_no_indent_check(s);
                    LineResult::None
                },
            },
            '_' => match ThematicBreak::check(line) {
                DoneResult::Done(b) => LineResult::DoneSelfAndNew(b),
                DoneResult::Text(s) => {
                    self.push_header_no_indent_check(s);
                    LineResult::None
                },
            },
            '+' => match List::check_plus(line) {
                NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                NewResult::Text(s) => {
                    self.push_header_no_indent_check(s);
                    LineResult::None
                },
            },
            '#' => match AtxHeading::check(line) {
                DoneResult::Done(b) => LineResult::DoneSelfAndNew(b),
                DoneResult::Text(s) => {
                    self.push_header_no_indent_check(s);
                    LineResult::None
                },
            },
            '>' => BlockQuote::new(line).done_self_and_new(),
            '0'..='9' => match List::check_number(line) {
                NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                NewResult::Text(s) => {
                    self.push_header_no_indent_check(s);
                    LineResult::None
                },
            },
            _ => {
                self.push_header_no_indent_check(line);
                LineResult::None
            },
        }
    }

    pub(crate) fn next_indented_continuation(&mut self, line: SkipIndent) { self.push_no_checks(line); }

    pub(crate) fn next_blank(&self) -> LineResult { LineResult::DoneSelf }

    pub(crate) fn finish(self) -> Block {
        let parsed = InlineParser::parse_lines(&self.lines);
        match self.setext {
            0 => Block::Para(parsed),
            _ => Block::new_header(self.setext, parsed),
        }
    }

    fn push_check_setext(&mut self, line: SkipIndent) -> LineResult {
        let mut whitespace = false;
        let mut iter = line.line.chars();
        loop {
            match iter.next() {
                Some('=') if !whitespace => continue,
                Some(' ' | '\t') => whitespace = true,
                Some(_) => return self.push_full_check(line),
                None => {
                    self.setext = 1;
                    return LineResult::DoneSelf;
                },
            }
        }
    }

    fn push_full_check(&mut self, line: SkipIndent) -> LineResult {
        match Table::check(line, self) {
            NewResult::New(b) => match self.lines.is_empty() {
                true => LineResult::New(b),
                false => LineResult::DoneSelfAndNew(b),
            },
            NewResult::Text(t) => {
                self.lines.push(t.line.to_owned());
                self.table_header_length = Table::check_header(t.line);
                LineResult::None
            },
        }
    }

    fn push_header_check(&mut self, line: SkipIndent) {
        self.lines.push(line.line.to_owned());
        self.table_header_length = Table::check_header(line.line);
    }

    fn push_header_no_indent_check(&mut self, line: SkipIndent) {
        self.lines.push(line.line.to_owned());
        self.table_header_length = if line.indent > 0 { 0 } else { Table::check_header(line.line) };
    }

    fn push_no_checks(&mut self, line: SkipIndent) { self.lines.push(line.line.to_owned()); }
}
