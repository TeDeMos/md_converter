use crate::ast::Block;
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::CheckResult;

/// Struct representing a finished thematic break
#[derive(Debug)]
pub struct ThematicBreak;

impl ThematicBreak {
    /// Checks if a line is a thematic break assuming the first char was a `'_'`
    pub fn check(line: SkipIndent) -> CheckResult {
        let mut count = 1;
        for c in line.get_rest().chars() {
            match c {
                ' ' | '\t' => continue,
                '_' => count += 1,
                _ => return CheckResult::Text(line),
            }
        }
        if count >= 3 {
            CheckResult::Done(Self.into())
        } else {
            CheckResult::Text(line)
        }
    }

    /// Finishes a thematic break into a [`Block`]
    pub const fn finish() -> Block {
        Block::HorizontalRule
    }
}
