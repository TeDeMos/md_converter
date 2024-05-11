use crate::ast::Block;
use crate::md_reader::temp_block::{CheckResult, LineResult, SkipIndent};

#[derive(Debug)]
pub struct FencedCodeBlock {
    indent: usize,
    fence_size: usize,
    fence_char: char,
    info: String,
    content: String,
}

impl FencedCodeBlock {
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

    fn push(&mut self, mut line: SkipIndent) {
        line.move_indent_capped(self.indent);
        line.push_full(&mut self.content);
        self.content.push('\n');
    }

    pub fn push_blank(&mut self, indent: usize) {
        let indent = indent.saturating_sub(self.indent);
        if indent > 0 {
            self.content.reserve(indent + 1);
            for _ in 0..indent {
                self.content.push(' ');
            }
        }
        self.content.push('\n');
    }

    pub fn finish(mut self) -> Block {
        self.content.pop();
        if let Some(n) = self.info.find(' ') {
            self.info.truncate(n);
        }
        let info = if self.info.is_empty() { Vec::new() } else { vec![self.info] };
        Block::CodeBlock((String::new(), info, Vec::new()), self.content)
    }
}
