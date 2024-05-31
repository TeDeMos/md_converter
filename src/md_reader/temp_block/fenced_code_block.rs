use crate::ast::Block;
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{CheckResult, LineResult};

/// Struct representing an unfinished fenced code block
#[derive(Debug)]
pub struct FencedCodeBlock {
    /// Indent of the opening code fence
    indent: usize,
    /// Amount of chars used for the opening code fence
    fence_size: usize,
    /// Char used for the beginning code fence
    fence_char: char,
    /// Info string
    info: String,
    /// Content
    content: String,
}

impl FencedCodeBlock {
    /// Checks if the line is beginning a fenced code block assuming the first char was a ``'`'`` or
    /// a `'~'`
    pub fn check(line: SkipIndent) -> CheckResult {
        let mut iter = line.iter_rest();
        let fence_size = iter.skip_while_eq(line.first) + 1;
        if fence_size < 3 {
            return CheckResult::Text(line);
        }
        iter.skip_whitespace();
        if line.first == '`' && iter.any_eq('`') {
            return CheckResult::Text(line);
        }
        CheckResult::New(
            Self {
                indent: line.indent,
                fence_size,
                fence_char: line.first,
                info: iter.get_string_trimmed(),
                content: String::new(),
            }
            .into(),
        )
    }

    /// Parses next non-blank line of a document
    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        if line.indent < 4 && line.first == self.fence_char {
            let mut iter = line.iter_rest();
            if iter.skip_while_eq(self.fence_char) + 1 >= self.fence_size {
                iter.skip_whitespace();
                if iter.ended() {
                    return LineResult::DoneSelf;
                }
            }
        }
        self.push(line);
        LineResult::None
    }

    /// Pushes a non-blank line
    fn push(&mut self, mut line: SkipIndent) {
        line.move_indent_capped(self.indent);
        line.push_full(&mut self.content);
        self.content.push('\n');
    }

    /// Pushes a blank line
    pub fn push_blank(&mut self, indent: usize) {
        let indent = indent.saturating_sub(self.indent);
        self.content.reserve(indent + 1);
        for _ in 0..indent {
            self.content.push(' ');
        }
        self.content.push('\n');
    }

    /// Finishes the fenced code block into a [`Block`].
    pub fn finish(mut self) -> Block {
        self.content.pop();
        if let Some(n) = self.info.find(' ') {
            self.info.truncate(n);
        }
        let info = if self.info.is_empty() { Vec::new() } else { vec![self.info] };
        Block::CodeBlock((String::new(), info, Vec::new()), self.content)
    }
}
