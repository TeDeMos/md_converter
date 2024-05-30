use crate::ast::Block;
use crate::inline_parser::InlineParser;
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::CheckResult;

/// Struct representing a finished atx heading
#[derive(Debug)]
pub struct AtxHeading {
    /// Level of the heading
    level: usize,
    /// Heading content
    content: String,
}

impl AtxHeading {
    /// Checks if the line is an atx heading assuming the first char was a `'#`'
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

    /// Finishes a heading into a [`Block`] by parsing the content
    pub fn finish(self) -> Block {
        Block::new_header(self.level, InlineParser::parse_lines(&self.content))
    }
}
