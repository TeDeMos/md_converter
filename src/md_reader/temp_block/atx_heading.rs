use std::iter::Peekable;
use std::str::Chars;

use crate::ast::Block;
use crate::inline_parser::InlineParser;

#[derive(Debug)]
pub struct AtxHeading {
    level: usize,
    content: String,
}

impl AtxHeading {
    pub fn check(rest: &mut Peekable<Chars>) -> Option<Self> {
        let mut count = 1;
        loop {
            match rest.next() {
                Some('#') if count < 6 => count += 1,
                Some(' ' | '\t') => break,
                None => return Some(Self { level: count, content: String::new() }),
                _ => return None,
            }
        }
        let mut result: String = rest.collect();
        let trimmed = result.trim_end().trim_end_matches('#');
        if matches!(trimmed.chars().next_back(), None | Some(' ' | '\t')) {
            result.truncate(trimmed.len().saturating_sub(1));
        }
        Some(Self { level: count, content: result })
    }

    pub fn finish(self) -> Block {
        Block::new_header(self.level, InlineParser::parse_line(self.content))
    }
}
