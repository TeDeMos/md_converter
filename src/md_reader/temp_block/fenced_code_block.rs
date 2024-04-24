use std::iter::Peekable;
use std::str::Chars;

use super::{skip_indent, LineResult};
use crate::ast::Block;

pub struct FencedCodeBlock {
    indent: usize,
    fence_size: usize,
    fence_char: char,
    info: String,
    content: String,
    // closed: bool,
}

impl FencedCodeBlock {
    pub fn check(indent: usize, first: char, rest: &mut Peekable<Chars>) -> Option<Self> {
        let mut count = 1;
        while rest.next_if_eq(&first).is_some() {
            count += 1;
        }
        if count < 3 {
            return None;
        }
        while matches!(rest.peek(), Some(' ' | '\t')) {
            rest.next();
        }
        let mut info: String = rest.collect();
        info.truncate(info.trim_end().len());
        if first == '`' && info.contains('`') {
            return None;
        }
        Some(FencedCodeBlock {
            indent,
            fence_char: first,
            fence_size: count,
            info,
            content: String::new(),
            // closed: false,
        })
    }

    pub fn next(&mut self, line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line, 4);
        if indent <= 3 {
            let mut count = 0;
            while let Some(c) = iter.next()
                && c == self.fence_char
            {
                count += 1;
            }
            if count >= self.fence_size {
                loop {
                    match iter.next() {
                        Some(' ' | '\t') => continue,
                        Some(_) => break,
                        None => {
                            // self.closed = true;
                            return LineResult::DoneSelf;
                        },
                    }
                }
            }
        }
        if self.indent > 0 {
            let (_, iter) = skip_indent(line, self.indent);
            for c in iter {
                self.content.push(c);
            }
        } else {
            self.content.push_str(line);
        }
        self.content.push('\n');
        LineResult::None
    }

    pub fn finish(mut self) -> Block {
        // if self.closed {
            self.content.pop();
        // }
        if let Some(n) = self.info.find(' ') {
            self.info.truncate(n)
        }
        let info = if self.info.is_empty() { Vec::new() } else { vec![self.info] };
        Block::CodeBlock((String::new(), info, Vec::new()), self.content)
    }
}
