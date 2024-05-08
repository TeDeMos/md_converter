use std::iter::Peekable;
use std::str::Chars;

use crate::ast::Block;
use crate::md_reader::temp_block::{DoneResult, SkipIndent};

#[derive(Debug)]
pub struct ThematicBreak;

impl ThematicBreak {
    pub fn check2(line: SkipIndent) -> DoneResult {
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
    
    pub fn check(first: char, rest: &mut Peekable<Chars>) -> Option<Self> {
        let mut count = 1;
        for c in rest {
            match c {
                ' ' | '\t' => continue,
                c if c == first => count += 1,
                _ => return None,
            }
        }
        (count >= 3).then_some(Self)
    }

    pub fn finish(self) -> Block { Block::HorizontalRule }
}
