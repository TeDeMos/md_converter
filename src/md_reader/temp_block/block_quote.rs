use std::iter;
use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, AtxHeading, FencedCodeBlock, IndentedCodeBlock, LineResult, List, ListResult,
    Paragraph, TempBlock, ThematicBreak, ToLineResult,
};
use crate::ast::Block;

#[derive(Debug)]
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
            } else {
                match self.over_indented_continuation(line) {
                    true => LineResult::None,
                    false => IndentedCodeBlock::new(&mut iter).done_self_and_new(),
                }
            }
        } else {
            match iter.next() {
                Some('>') => {
                    let line = if iter.peek() == Some(&' ') {
                        &line[indent + 2..]
                    } else {
                        &line[indent + 1..]
                    };
                    self.current.next(line, &mut self.finished);
                    LineResult::None
                },
                f if let Some(r) = self.continuation(indent, f, line, &mut iter.clone()) => r,
                Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(c @ ('*' | '-')) => match List::check_other(c, indent, line, &mut iter) {
                    ListResult::List(b) => b.done_self_and_new(),
                    ListResult::Break(b) => b.done_self_and_other(),
                    ListResult::None => Paragraph::new(line).done_self_and_new(),
                },
                Some('_') => match ThematicBreak::check('_', &mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('+') => match List::check_plus(indent, line, &mut iter) {
                    Some(l) => l.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('#') => match AtxHeading::check(&mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(c @ '0'..='9') => match List::check_number(c, indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(_) => LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line))),
                // _ if !iter.all(char::is_whitespace) =>
                //     LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line))),
                _ => LineResult::DoneSelf,
            }
        }
    }

    pub fn next_blank(&mut self) -> LineResult { LineResult::DoneSelf }

    pub fn finish(self) -> Block {
        Block::BlockQuote(
            self.finished
                .into_iter()
                .chain(iter::once(*self.current))
                .filter_map(TempBlock::finish)
                .collect(),
        )
    }

    pub fn continuation(
        &mut self, indent: usize, first: Option<char>, line: &str, rest: &mut Peekable<Chars>,
    ) -> Option<LineResult> {
        self.current.continuation(indent, first, line, rest)
    }

    pub fn over_indented_continuation(&mut self, line: &str) -> bool {
        self.current.over_indented_continuation(line)
    }
}
