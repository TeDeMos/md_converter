use crate::ast::{attr_empty, Block};
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{LineResult, TempBlock};

/// Struct representing an unfinished code block
#[derive(Debug)]
pub struct IndentedCodeBlock {
    /// Content of the block
    content: String,
    /// Whether the block has blank lines at the end (used by lists to check they are loose)
    pub ends_with_blank: bool,
    /// Index of the end of the last non-blank line
    last_non_blank_end: usize,
}

impl IndentedCodeBlock {
    /// Creates a new code block starting with a given line (assumes the line is indented at least 4
    /// spaces)
    pub fn new(mut line: SkipIndent) -> Self {
        line.move_indent(4);
        let content = line.get_full();
        let last_non_blank_end = content.len();
        Self {
            content,
            ends_with_blank: false,
            last_non_blank_end,
        }
    }

    /// Parses next non-blank line of a document
    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        match line.indent {
            0..=3 => TempBlock::check_block_known_indent(line).into_line_result_paragraph(true),
            4.. => {
                self.push(line);
                LineResult::None
            }
        }
    }

    /// Pushes a non-blank line
    fn push(&mut self, mut line: SkipIndent) {
        line.move_indent(4);
        self.content.push('\n');
        line.push_full(&mut self.content);
        self.ends_with_blank = false;
        self.last_non_blank_end = self.content.len();
    }

    /// Pushes a blank line
    pub fn push_blank(&mut self, indent: usize) {
        let len = indent.saturating_sub(4);
        self.content.reserve(len + 1);
        self.content.push('\n');
        for _ in 0..len {
            self.content.push(' ');
        }
        self.ends_with_blank = true;
    }

    /// Finishes the indented code block into a [`Block`] removing trailing blank lines first
    pub fn finish(mut self) -> Block {
        if self.ends_with_blank {
            self.content.truncate(self.last_non_blank_end);
        }
        Block::CodeBlock(attr_empty(), self.content)
    }
}
