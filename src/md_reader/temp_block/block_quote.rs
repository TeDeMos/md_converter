use std::iter;

use crate::ast::Block;
use crate::md_reader::temp_block::{LineResult, SkipIndent, TempBlock};

#[derive(Debug)]
pub struct BlockQuote {
    pub current: Box<TempBlock>,
    finished: Vec<TempBlock>,
}

impl BlockQuote {
    pub fn new(line: &SkipIndent) -> Self {
        let mut content = line.skip_indent_rest();
        content.inspect(|s| s.move_indent_capped(1));
        let (current, finished) = TempBlock::new_empty(content);
        Self { current: Box::new(current), finished }
    }

    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        match line.indent {
            0..=3 =>
                if line.first == '>' {
                    let mut content = line.skip_indent_rest();
                    content.inspect(|s| s.move_indent_capped(1));
                    self.current.next(content, &mut self.finished);
                    LineResult::None
                } else {
                    self.current.next_continuation(line)
                },
            4.. => self.current.next_indented_continuation(line),
        }
    }

    pub fn finish(self) -> Block {
        Block::BlockQuote(
            self.finished
                .into_iter()
                .chain(iter::once(*self.current))
                .filter_map(TempBlock::finish)
                .collect(),
        )
    }
}
