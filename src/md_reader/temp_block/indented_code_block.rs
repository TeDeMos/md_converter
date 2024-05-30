use crate::ast::{attr_empty, Block};
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{LineResult, TempBlock};

#[derive(Debug)]
pub struct IndentedCodeBlock(Vec<String>);

impl IndentedCodeBlock {
    pub fn new(mut line: SkipIndent) -> Self {
        line.move_indent(4);
        Self(vec![line.get_full()])
    }

    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        match line.indent {
            0..=3 => TempBlock::check_block_known_indent(line).into_line_result_paragraph(true),
            4.. => {
                self.push(line);
                LineResult::None
            },
        }
    }

    fn push(&mut self, mut line: SkipIndent) {
        line.move_indent(4);
        self.0.push(line.get_full());
    }

    pub fn push_blank(&mut self, indent: usize) {
        self.0.push(" ".repeat(indent.saturating_sub(4)));
    }

    pub fn finish(mut self) -> Block {
        while let Some(last) = self.0.last()
            && last.chars().all(char::is_whitespace)
        {
            self.0.pop();
        }
        let mut result = String::new();
        for l in self.0 {
            result.push_str(&l);
            result.push('\n');
        }
        result.pop();
        Block::CodeBlock(attr_empty(), result)
    }
}
