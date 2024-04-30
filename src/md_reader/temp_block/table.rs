use super::{
    skip_indent, AtxHeading, BlockQuote, FencedCodeBlock, LineResult, List, ListResult,
    ThematicBreak, ToLineResult,
};
use crate::ast::{Alignment, Block};

#[derive(Debug)]
pub struct Table {
    size: usize,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn check(line: &str, header_line: &str, header_length: usize) -> Option<Self> {
        if header_length == 0 {
            return None;
        }
        let mut iter = line.trim().chars().peekable();
        iter.next_if_eq(&'|');
        let mut alignments = Vec::new();
        for i in 0..header_length {
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
            if iter.next_if_eq(&'|').is_none() && i != header_length - 1 {
                return None;
            }
            alignments.push(match (left, right) {
                (false, false) => Alignment::Default,
                (false, true) => Alignment::Right,
                (true, false) => Alignment::Left,
                (true, true) => Alignment::Center,
            });
        }
        iter.next().is_none().then(|| {
            let mut result = Self { size: header_length, alignments, rows: vec![] };
            result.split_rows(header_line);
            result
        })
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
                Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => self.push(line),
                },
                Some(c @ ('*' | '-')) => match List::check_other(c, indent, line, &mut iter) {
                    ListResult::List(b) => b.done_self_and_new(),
                    ListResult::Break(b) => b.done_self_and_other(),
                    ListResult::None => self.push(line),
                },
                Some('_') => match ThematicBreak::check('_', &mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => self.push(line),
                },
                Some('+') => match List::check_plus(indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => self.push(line),
                },
                Some('#') => match AtxHeading::check(&mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => self.push(line),
                },
                Some('>') => BlockQuote::new(indent, line, &mut iter).done_self_and_new(),
                Some(c @ '0'..='9') => match List::check_number(c, indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => self.push(line),
                },
                Some(_) => self.push(line),
                // _ if !iter.all(char::is_whitespace) => self.push(line),
                _ => LineResult::DoneSelf,
            }
        }
    }

    pub fn next_blank(&mut self) -> LineResult { LineResult::DoneSelf }

    pub fn finish(self) -> Block { Block::new_table(self.rows, self.alignments, self.size) }

    fn push(&mut self, line: &str) -> LineResult {
        self.split_rows(line);
        LineResult::None
    }

    pub fn check_header(line: &str) -> usize {
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
        if count == 0 {
            0
        } else {
            count + 1 - usize::from(previous) - usize::from(first)
        }
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
