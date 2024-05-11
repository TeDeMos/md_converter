use crate::ast::Block;
use crate::md_reader::temp_block::{CheckResult, SkipIndent};

#[derive(Debug)]
pub struct ThematicBreak;

impl ThematicBreak {
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

    pub const fn finish() -> Block { Block::HorizontalRule }
}
