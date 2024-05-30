use crate::ast::{Alignment, Block};
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{LineResult, NewResult, Paragraph, TempBlock};

#[derive(Debug)]
pub struct Table {
    size: usize,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn check<'a>(line: SkipIndent<'a>, paragraph: &mut Paragraph) -> NewResult<'a> {
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
            result.push(paragraph.get_last_line());
            paragraph.trim_last_line();
            NewResult::New(result.into())
        } else {
            NewResult::Text(line)
        }
    }

    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        TempBlock::check_block(line).into_line_result(true, |s| {
            self.push(s.line);
            LineResult::None
        })
    }

    pub fn finish(self) -> Block { Block::new_table(self.rows, self.alignments) }

    pub fn check_header(line: &str) -> usize {
        let mut iter = line.chars();
        let mut count = 1;
        let mut escape = iter.next() == Some('\\');
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
                    detected = !escape;
                }
                escape = false;
            }
        }
        count
    }

    fn push(&mut self, line: &str) {
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
