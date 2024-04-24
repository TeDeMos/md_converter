use std::iter;
use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, AtxHeading, FencedCodeBlock, IndentedCodeBlock, LineResult, Paragraph, TempBlock,
    ThematicBreak,
};
use crate::ast::Block;

pub struct BlockQuote {
    current: Box<TempBlock>,
    finished: Vec<TempBlock>,
}

impl BlockQuote {
    pub fn new(indent: usize, line: &str, rest: &mut Peekable<Chars>) -> Self {
        // Safety: skip_indent only accepts tabs and spaces - 1 byte chars
        let line =
            if rest.peek() == Some(&' ') { &line[indent + 2..] } else { &line[indent + 1..] };
        let mut current = Box::new(TempBlock::default());
        let mut finished = Vec::new();
        current.next(line, &mut finished);
        Self { current, finished }
    }

    pub fn next(&mut self, line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line, 4);
        if indent >= 4 {
            if iter.clone().all(char::is_whitespace) {
                LineResult::DoneSelf
            } else if let TempBlock::Paragraph(p) = self.current.as_mut() {
                let result = p.push(line, true);
                self.current.apply_result(result, &mut self.finished);
                LineResult::None
            } else {
                LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(IndentedCodeBlock::new(
                    &mut iter,
                )))
            }
        } else {
            match (iter.next(), self.current.as_mut()) {
                (Some('>'), c) => {
                    let line = if iter.peek() == Some(&' ') {
                        &line[indent + 2..]
                    } else {
                        &line[indent + 1..]
                    };
                    c.next(line, &mut self.finished);
                    LineResult::None
                },
                (f, TempBlock::Paragraph(p)) => p.next_no_split(indent, f, line, &mut iter),
                (_, TempBlock::BlockQuote(b)) => b.next(line),
                (Some(c @ ('~' | '`')), _) => self.check_code_block(indent, c, line, &mut iter),
                (Some(c @ ('*' | '_' | '-')), _) => self.check_thematic_break(c, line, &mut iter),
                (Some('#'), _) => self.check_atx_heading(line, &mut iter),
                _ if !iter.all(char::is_whitespace) =>
                    LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line))),
                _ => LineResult::DoneSelf,
            }
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

    fn check_code_block(
        &mut self, indent: usize, first: char, line: &str, rest: &mut Peekable<Chars>,
    ) -> LineResult {
        LineResult::DoneSelfAndNew(
            FencedCodeBlock::check(indent, first, rest)
                .map(TempBlock::FencedCodeBlock)
                .unwrap_or_else(|| TempBlock::Paragraph(Paragraph::new(line))),
        )
    }

    fn check_thematic_break(
        &mut self, first: char, line: &str, rest: &mut Peekable<Chars>,
    ) -> LineResult {
        ThematicBreak::check(first, rest)
            .map(|t| LineResult::DoneSelfAndOther(TempBlock::ThematicBreak(t)))
            .unwrap_or_else(|| {
                LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line)))
            })
    }

    fn check_atx_heading(&mut self, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        AtxHeading::check(rest)
            .map(|a| LineResult::DoneSelfAndOther(TempBlock::AtxHeading(a)))
            .unwrap_or_else(|| {
                LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line)))
            })
    }
}
