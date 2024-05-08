use std::iter::Peekable;
use std::str::Chars;

use crate::ast::Block;
use crate::inline_parser::InlineParser;
use crate::md_reader::temp_block::{DoneResult, SkipIndent};

#[derive(Debug)]
pub struct AtxHeading {
    level: usize,
    content: String,
}

impl AtxHeading {
    pub fn check2(line: SkipIndent) -> DoneResult {
        let mut iter = line.iter_rest();
        let count = 1 + iter.skip_while_eq('#');
        if count > 6 {
            return DoneResult::Text(line);
        }
        if iter.ended() {
            return DoneResult::Done(Self { level: count, content: String::new() }.into())
        }
        if !iter.skip_whitespace_min_one() {
            return DoneResult::Text(line);
        }
        let mut rev = iter.iter_rest_rev();
        rev.skip_whitespace();
        let any = rev.skip_while_eq('#') > 0;
        let content = if any && rev.next_if_whitespace_or_none() {
            rev.get_string()
        } else {
            iter.get_string()
        };
        DoneResult::Done(Self { level: count, content }.into())
    }
    
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
