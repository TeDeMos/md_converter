use std::iter::Peekable;
use std::str::Chars;

use crate::ast::Block;

pub struct ThematicBreak;

impl ThematicBreak {
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
