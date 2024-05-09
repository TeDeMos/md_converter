use crate::ast::Block;
use crate::md_reader::temp_block::{DoneResult, SkipIndent};

#[derive(Debug)]
pub(crate) struct ThematicBreak;

impl ThematicBreak {
    pub(crate) fn check(line: SkipIndent) -> DoneResult {
        let mut count = 1;
        for c in line.get_rest().chars() {
            match c {
                ' ' | '\t' => continue,
                '_' => count += 1,
                _ => return DoneResult::Text(line),
            }
        }
        if count >= 3 {
            DoneResult::Done(ThematicBreak.into())
        } else {
            DoneResult::Text(line)
        }
    }

    pub(crate) fn finish(self) -> Block { Block::HorizontalRule }
}
