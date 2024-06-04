use crate::ast::{Alignment, Block};
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::Links;
use crate::md_reader::temp_block::{LineResult, NewResult, Paragraph, TempBlock};

/// Struct representing an unfinished table
#[derive(Debug)]
pub struct Table {
    /// Alignments of each column
    alignments: Vec<Alignment>,
    /// Table rows
    rows: Vec<Vec<String>>,
}

impl Table {
    /// Checks if a line is a table delimiter row with the amount of columns matching the table
    /// header count of the previous line in a paragraph. If the check passes it removes the last
    /// line of the `paragraph` to use as the header
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
            let mut result = Self { alignments, rows: Vec::new() };
            result.push(paragraph.get_last_line());
            paragraph.trim_last_line();
            NewResult::New(result.into())
        } else {
            NewResult::Text(line)
        }
    }

    /// Parses next non-blank line of a document
    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        TempBlock::check_block(line).into_line_result(true, |s| {
            self.push(s.line);
            LineResult::None
        })
    }

    /// Finishes the table into a [`Block`]
    pub fn finish(self, links: &Links) -> Block {
        Block::new_table(self.rows, self.alignments, links)
    }

    /// Checks how many columns a table header defined by this line has
    pub fn check_header(line: &str) -> usize {
        let trimmed = line.trim();
        if trimmed == "|" {
            return 0;
        }
        let mut iter = line.trim().chars();
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

    /// Pushes a line splitting it into cells
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
                    if result.len() == self.alignments.len() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_row() {
        assert_eq!(Table::check_header("|"), 0);
        assert_eq!(Table::check_header("||"), 1);
        assert_eq!(Table::check_header("|||"), 2);
        assert_eq!(Table::check_header("||||"), 3);
        assert_eq!(Table::check_header("\\|"), 1);
        assert_eq!(Table::check_header("|\\|"), 1);
        assert_eq!(Table::check_header("no"), 1);
        assert_eq!(Table::check_header("|leading"), 1);
        assert_eq!(Table::check_header("trailing|"), 1);
        assert_eq!(Table::check_header("|both|"), 1);
        assert_eq!(Table::check_header("|many|many"), 2);
        assert_eq!(Table::check_header("many|many\\|many"), 2);
    }

    fn check_delimeter(line: &str, size: usize) -> bool {
        let mut paragraph = Paragraph::new(&SkipIndent::skip(&"|".repeat(size + 1), 0).into_line());
        matches!(
            Table::check(SkipIndent::skip(line, 0).into_line(), &mut paragraph),
            NewResult::New(_)
        )
    }

    #[test]
    fn delimeter_row() {
        assert!(!check_delimeter(":", 1));
        assert!(!check_delimeter("::", 1));
        assert!(check_delimeter(":-", 1));
        assert!(check_delimeter("-:", 1));
        assert!(check_delimeter("-:", 1));
        assert!(check_delimeter("|-:", 1));
        assert!(check_delimeter(":-|", 1));
        assert!(check_delimeter("|-|", 1));
        assert!(check_delimeter("-|-", 2));
        assert!(check_delimeter("|:-----:|----|", 2));
        assert!(!check_delimeter("|:-----:|::--|", 2));
    }

    fn push(line: &str, size: usize, expected: &[&str]) {
        let mut table = Table { alignments: vec![Alignment::Center; size], rows: Vec::new() };
        table.push(line);
        let result: Vec<_> = table.rows.last().unwrap().iter().map(String::as_str).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn check_push() {
        push("|", 1, &[""]);
        push("|", 2, &["", ""]);
        push("aaa", 2, &["aaa", ""]);
        push("aaa|", 2, &["aaa", ""]);
        push("|aaa", 2, &["aaa", ""]);
        push("|aaa|a", 2, &["aaa", "a"]);
        push("|aaa\\|aaa|", 2, &["aaa|aaa", ""]);
    }
}
