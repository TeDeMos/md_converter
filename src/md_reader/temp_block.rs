use std::iter::Peekable;
use std::str::Chars;

use atx_heading::AtxHeading;
use block_quote::BlockQuote;
use derive_more::From;
use fenced_code_block::FencedCodeBlock;
use indented_code_block::IndentedCodeBlock;
use list::{List, ListResult, ParagraphListResult};
use paragraph::Paragraph;
use table::Table;
use thematic_break::ThematicBreak;

use crate::ast::Block;

mod atx_heading;
mod block_quote;
mod fenced_code_block;
mod indented_code_block;
mod list;
mod paragraph;
mod table;
mod thematic_break;

#[derive(From, Debug)]
pub enum TempBlock {
    Empty,
    Paragraph(Paragraph),
    AtxHeading(AtxHeading),
    ThematicBreak(ThematicBreak),
    IndentedCodeBlock(IndentedCodeBlock),
    FencedCodeBlock(FencedCodeBlock),
    Table(Table),
    BlockQuote(BlockQuote),
    List(List),
}

impl TempBlock {
    pub fn next(&mut self, line: &str, finished: &mut Vec<Self>) {
        let result = self.next_no_apply(line);
        self.apply_result(result, finished);
    }

    pub fn next_no_apply(&mut self, line: &str) -> LineResult {
        match self {
            Self::Empty => Self::next_empty(line),
            Self::Paragraph(p) => p.next(line),
            Self::IndentedCodeBlock(i) => i.next(line),
            Self::FencedCodeBlock(f) => f.next(line),
            Self::Table(t) => t.next(line),
            Self::BlockQuote(b) => b.next(line),
            Self::List(l) => l.next(line),
            // Safety: atx headings and thematic breaks are always passed as finished
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
    }

    pub fn over_indented_continuation(&mut self, line: &str) -> bool {
        match self {
            Self::Paragraph(p) => {
                p.over_indented_continuation(line);
                true
            },
            Self::BlockQuote(b) => b.over_indented_continuation(line),
            Self::List(l) => l.over_indented_continuation(line),
            Self::Empty
            | Self::IndentedCodeBlock(_)
            | Self::FencedCodeBlock(_)
            | Self::Table(_) => false,
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
    }

    pub fn continuation(
        &mut self, indent: usize, first: Option<char>, line: &str, rest: &mut Peekable<Chars>,
    ) -> Option<LineResult> {
        match self {
            Self::Paragraph(p) => Some(p.continuation(indent, first, line, rest)),
            Self::BlockQuote(b) => b.continuation(indent, first, line, rest),
            Self::List(l) => l.continuation(indent, first, line, rest),
            Self::Empty
            | Self::IndentedCodeBlock(_)
            | Self::FencedCodeBlock(_)
            | Self::Table(_) => None,
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
    }

    pub fn apply_result(&mut self, result: LineResult, finished: &mut Vec<Self>) {
        match result {
            LineResult::None => {},
            LineResult::New(new) => *self = new,
            LineResult::DoneSelf => finished.push(self.take()),
            LineResult::Done(block) => finished.push(block),
            LineResult::DoneSelfAndNew(block) => finished.push(self.replace(block)),
            LineResult::DoneSelfAndOther(block) => {
                finished.push(self.take());
                finished.push(block);
            },
        }
    }

    pub fn finish(self) -> Option<Block> {
        match self {
            Self::Empty => None,
            Self::Paragraph(p) => Some(p.finish()),
            Self::AtxHeading(a) => Some(a.finish()),
            Self::ThematicBreak(t) => Some(t.finish()),
            Self::IndentedCodeBlock(i) => Some(i.finish()),
            Self::FencedCodeBlock(c) => Some(c.finish()),
            Self::Table(t) => Some(t.finish()),
            Self::BlockQuote(b) => Some(b.finish()),
            Self::List(l) => Some(l.finish()),
        }
    }

    fn next_blank(&mut self) -> LineResult {
        match self {
            Self::Empty => LineResult::None,
            Self::Paragraph(p) => p.next_blank(),
            Self::IndentedCodeBlock(i) => i.next_blank(),
            Self::FencedCodeBlock(f) => f.next_blank(),
            Self::Table(t) => t.next_blank(),
            Self::BlockQuote(b) => b.next_blank(),
            Self::List(l) => l.next_blank(),
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
    }

    fn next_empty(line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line, 4);
        if indent >= 4 {
            match iter.clone().all(char::is_whitespace) {
                true => LineResult::None,
                false => IndentedCodeBlock::new(&mut iter).new(),
            }
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, &mut iter) {
                    Some(b) => b.new(),
                    None => Paragraph::new(line).new(),
                },
                Some(c @ ('*' | '-')) => match List::check_other(c, indent, line, &mut iter) {
                    ListResult::List(l) => l.new(),
                    ListResult::Break(b) => b.done(),
                    ListResult::None => Paragraph::new(line).new(),
                },
                Some('_') => match ThematicBreak::check('_', &mut iter) {
                    Some(b) => b.done(),
                    None => Paragraph::new(line).new(),
                },
                Some('+') => match List::check_plus(indent, line, &mut iter) {
                    Some(l) => l.new(),
                    None => Paragraph::new(line).new(),
                },
                Some('#') => match AtxHeading::check(&mut iter) {
                    Some(b) => b.done(),
                    None => Paragraph::new(line).new(),
                },
                Some('>') => BlockQuote::new(indent, line, &mut iter).new(),
                Some(c @ '0'..='9') => match List::check_number(c, indent, line, &mut iter) {
                    Some(b) => b.new(),
                    None => Paragraph::new(line).new(),
                },
                Some(_) => Paragraph::new(line).new(),
                // _ if !iter.all(char::is_whitespace) => Paragraph::new(line).new(),
                _ => LineResult::None,
            }
        }
    }

    fn take(&mut self) -> Self { std::mem::take(self) }

    fn replace(&mut self, new: Self) -> Self { std::mem::replace(self, new) }
}

impl Default for TempBlock {
    fn default() -> Self { Self::Empty }
}

pub enum LineResult {
    None,
    DoneSelf,
    New(TempBlock),
    Done(TempBlock),
    DoneSelfAndNew(TempBlock),
    DoneSelfAndOther(TempBlock),
}

fn skip_indent(line: &str, limit: usize) -> (usize, Peekable<Chars>) {
    let mut iter = line.chars().peekable();
    let indent = skip_indent_iter(&mut iter, limit);
    (indent, iter)
}

fn skip_indent_iter(iter: &mut Peekable<Chars>, limit: usize) -> usize {
    let mut indent = 0;
    loop {
        match iter.peek() {
            Some('\t') => indent += 4 - indent % 4,
            Some(' ') => indent += 1,
            _ => return indent,
        }
        iter.next();
        if indent >= limit {
            return indent;
        }
    }
}

trait ToLineResult {
    fn new(self) -> LineResult;
    fn done(self) -> LineResult;
    fn done_self_and_new(self) -> LineResult;
    fn done_self_and_other(self) -> LineResult;
}

impl<T> ToLineResult for T
where T: Into<TempBlock>
{
    fn new(self) -> LineResult { LineResult::New(self.into()) }

    fn done(self) -> LineResult { LineResult::Done(self.into()) }

    fn done_self_and_new(self) -> LineResult { LineResult::DoneSelfAndNew(self.into()) }

    fn done_self_and_other(self) -> LineResult { LineResult::DoneSelfAndOther(self.into()) }
}
