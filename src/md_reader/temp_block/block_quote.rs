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
        content.inspect_line(|l| l.indent = l.indent.saturating_sub(1));
        let (current, finished) = TempBlock::new_empty(content);
        Self { current: Box::new(current), finished }
    }

    /// Parses next non-blank line of a document
    pub fn next(&mut self, line: SkipIndent, links: &mut Links) -> LineResult {
        match line.indent {
            0..=3 =>
                if line.first == '>' {
                    let mut content = line.skip_indent_rest();
                    content.inspect_line(|l| l.indent = l.indent.saturating_sub(1));
                    self.current.next(content, &mut self.finished, links);
                    LineResult::None
                } else {
                    self.current.next_continuation(line)
                },
            4.. => self.current.next_indented_continuation(line),
        }
    }

    /// Finishes the block quote into a [`Block`]
    pub fn finish(self, links: &Links) -> Block {
        Block::BlockQuote(
            self.finished
                .into_iter()
                .chain(iter::once(*self.current))
                .filter_map(|t| t.finish(links))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(line: &str) -> BlockQuote { BlockQuote::new(&SkipIndent::skip(line, 0).into_line()) }

    fn next(block_quote: &mut BlockQuote, line: &str) -> LineResult {
        block_quote.next(SkipIndent::skip(line, 0).into_line(), &mut Links::new())
    }

    fn assert_consumed(block_quote: &mut BlockQuote, line: &str) {
        assert!(matches!(next(block_quote, line), LineResult::None));
    }

    fn assert_new(block_quote: &mut BlockQuote, line: &str) {
        assert!(matches!(next(block_quote, line), LineResult::DoneSelfAndNew(_)));
    }

    #[test]
    fn gt_continues() {
        let mut para = new("> line");
        assert_consumed(&mut para, "> next");
        let mut other = new("> ***");
        assert_consumed(&mut other, "> next");
    }

    #[test]
    fn continuation() {
        let mut para = new("> line");
        assert_consumed(&mut para, "next");
        let mut other = new("> ***");
        assert_new(&mut other, "next");
    }

    #[test]
    fn indented_continuation() {
        let mut para = new("> line");
        assert_consumed(&mut para, "    next");
        let mut other = new("> ***");
        assert_new(&mut other, "    next");
    }

    #[test]
    fn nested_continuation() {
        let mut block = new(">>> nested");
        assert_consumed(&mut block, " next");
        assert_consumed(&mut block, ">> next");
        assert_consumed(&mut block, "> next");
    }
}
