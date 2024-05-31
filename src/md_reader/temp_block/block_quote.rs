use std::iter;

use crate::ast::Block;
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{LineResult, Links, TempBlock};

/// Struct representing an unfinished block quote
#[derive(Debug)]
pub struct BlockQuote {
    /// Current open block
    pub current: Box<TempBlock>,
    /// Finished blocks
    finished: Vec<TempBlock>,
}

impl BlockQuote {
    /// Creates a block quote from a given non-blank line
    pub fn new(line: &SkipIndent) -> Self {
        let mut content = line.skip_indent_rest();
        content.move_indent_capped(1);
        let (current, finished) = TempBlock::new_empty(content);
        Self { current: Box::new(current), finished }
    }

    /// Parses next non-blank line of a document
    pub fn next(&mut self, line: SkipIndent, links: &mut Links) -> LineResult {
        match line.indent {
            0..=3 =>
                if line.first == '>' {
                    let mut content = line.skip_indent_rest();
                    content.move_indent_capped(1);
                    self.current.next(content, &mut self.finished, links);
                    LineResult::None
                } else {
                    self.current.next_continuation(line)
                },
            4.. => self.current.next_indented_continuation(line),
        }
    }

    /// Finishes the block quote into a [`Block`]
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
