use super::{LineResult, NewResult, SkipIndent, SkipIndentResult};
use crate::ast::Block;

#[derive(Debug)]
pub(crate) struct FencedCodeBlock {
    indent: usize,
    fence_size: usize,
    fence_char: char,
    info: String,
    content: String,
}

impl FencedCodeBlock {
    pub(crate) fn check(line: SkipIndent) -> NewResult {
        let mut iter = line.iter_rest();
        let fence_size = iter.skip_while_eq(line.first) + 1;
        if fence_size < 3 {
            return NewResult::Text(line);
        }
        iter.skip_whitespace();
        if line.first == '`' && iter.any_eq('`') {
            return NewResult::Text(line);
        }
        NewResult::New(
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

    pub(crate) fn next(&mut self, line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(line) => {
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
            },
            SkipIndentResult::Blank(indent) => self.push_empty(indent),
        }
        LineResult::None
    }

    fn push(&mut self, mut line: SkipIndent) {
        line.move_indent_capped(self.indent);
        line.push_full(&mut self.content);
        self.content.push('\n');
    }

    pub(crate) fn next_blank(&mut self, indent: usize) -> LineResult {
        self.push_empty(indent);
        LineResult::None
    }

    fn push_empty(&mut self, indent: usize) {
        let indent = indent.saturating_sub(self.indent);
        if indent > 0 {
            self.content.reserve(indent + 1);
            for _ in 0..indent {
                self.content.push(' ');
            }
        }
        self.content.push('\n');
    }

    pub(crate) fn finish(mut self) -> Block {
        self.content.pop();
        if let Some(n) = self.info.find(' ') {
            self.info.truncate(n);
        }
        let info = if self.info.is_empty() { Vec::new() } else { vec![self.info] };
        Block::CodeBlock((String::new(), info, Vec::new()), self.content)
    }
}
