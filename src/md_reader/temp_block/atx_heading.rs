use crate::ast::Block;
use crate::inline_parser::InlineParser;
use crate::md_reader::temp_block::{CheckResult, SkipIndent};

#[derive(Debug)]
pub struct AtxHeading {
    level: usize,
    content: String,
}

impl AtxHeading {
    pub fn check(line: SkipIndent) -> CheckResult {
        let mut iter = line.iter_rest();
        let count = 1 + iter.skip_while_eq('#');
        if count > 6 {
            return CheckResult::Text(line);
        }
        if iter.ended() {
            return CheckResult::Done(Self { level: count, content: String::new() }.into());
        }
        if !iter.skip_whitespace_min_one() {
            return CheckResult::Text(line);
        }
        let mut rev = iter.iter_rest_rev();
        rev.skip_whitespace();
        let any = rev.skip_while_eq('#') > 0;
        let content = if any && rev.next_if_whitespace_or_none() {
            rev.get_string()
        } else {
            iter.get_string()
        };
        CheckResult::Done(Self { level: count, content }.into())
    }

    pub fn finish(self) -> Block {
        Block::new_header(self.level, InlineParser::parse_line(self.content))
    }
}
