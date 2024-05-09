use super::{
    AtxHeading, BlockQuote, DoneResult, FencedCodeBlock, IndentedCodeBlock, LineResult, List,
    ListBreakResult, NewResult, Paragraph, SkipIndent, SkipIndentResult, ThematicBreak,
    ToLineResult,
};
use crate::ast::{Alignment, Block};

#[derive(Debug)]
pub(crate) struct Table {
    size: usize,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub(crate) fn check<'a>(line: SkipIndent<'a>, paragraph: &mut Paragraph) -> NewResult<'a> {
        if paragraph.table_header_length == 0 {
            return NewResult::Text(line);
        }
        let mut iter = line.iter_full();
        iter.next_if_eq('|');
        let mut alignments = Vec::new();
        for i in 0..paragraph.table_header_length {
            iter.skip_whitespace();
            let left = iter.next_if_eq(':');
            if !iter.skip_while_eq_min_one('-') {
                return NewResult::Text(line);
            }
            let right = iter.next_if_eq(':');
            iter.skip_whitespace();
            if !iter.next_if_eq('|') && i != paragraph.table_header_length - 1 {
                return NewResult::Text(line);
            }
            alignments.push(match (left, right) {
                (false, false) => Alignment::Default,
                (false, true) => Alignment::Right,
                (true, false) => Alignment::Left,
                (true, true) => Alignment::Center,
            });
        }
        iter.skip_whitespace();
        if iter.ended() {
            let mut result =
                Self { size: paragraph.table_header_length, alignments, rows: Vec::new() };
            result.split_rows(&paragraph.lines.pop().unwrap());
            NewResult::New(result.into())
        } else {
            NewResult::Text(line)
        }
    }

    pub(crate) fn next(&mut self, line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(line) => match line.indent {
                0..=3 => match line.first {
                    '~' | '`' => match FencedCodeBlock::check(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => self.push(s.line),
                    },
                    '*' | '-' => match List::check_star_dash(line) {
                        ListBreakResult::List(l) => l.done_self_and_new(),
                        ListBreakResult::Break(b) => b.done_self_and_other(),
                        ListBreakResult::Text(s) => self.push(s.line),
                    },
                    '_' => match ThematicBreak::check(line) {
                        DoneResult::Done(b) => LineResult::DoneSelfAndNew(b),
                        DoneResult::Text(s) => self.push(s.line),
                    },
                    '+' => match List::check_plus(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => self.push(s.line),
                    },
                    '#' => match AtxHeading::check(line) {
                        DoneResult::Done(b) => LineResult::DoneSelfAndOther(b),
                        DoneResult::Text(s) => self.push(s.line),
                    },
                    '>' => BlockQuote::new(line).done_self_and_new(),
                    '0'..='9' => match List::check_number(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => self.push(s.line),
                    },
                    _ => self.push(line.line),
                },
                4.. => IndentedCodeBlock::new(line).done_self_and_new(),
            },
            SkipIndentResult::Blank(_) => LineResult::DoneSelf,
        }
    }

    pub(crate) fn next_blank(&mut self) -> LineResult { LineResult::DoneSelf }

    pub(crate) fn finish(self) -> Block { Block::new_table(self.rows, self.alignments, self.size) }

    fn push(&mut self, line: &str) -> LineResult {
        self.split_rows(line);
        LineResult::None
    }

    pub(crate) fn check_header(line: &str) -> usize {
        let mut iter = line.chars();
        let mut count = 1;
        let mut escape = match iter.next() {
            Some('\\') => true,
            _ => false,
        };
        let mut detected = false;
        for c in iter {
            if detected && !matches!(c, ' ' | '\t') {
                detected = false;
                count += 1;
            }
            if c == '\\' {
                escape = !escape;
            } else {
                if c == '|' {
                    detected = !escape
                }
                escape = false;
            }
        }
        count
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
