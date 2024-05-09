use derive_more::From;

use atx_heading::AtxHeading;
use block_quote::BlockQuote;
use fenced_code_block::FencedCodeBlock;
use indented_code_block::IndentedCodeBlock;
use iters::{SkipIndent, SkipIndentResult};
use list::{List, ListBreakResult, ListBreakSetextResult};
use paragraph::Paragraph;
use table::Table;
use thematic_break::ThematicBreak;

use crate::ast::Block;

mod atx_heading;
mod block_quote;
mod fenced_code_block;
mod indented_code_block;
mod iters;
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
    pub(crate) fn next_line(&mut self, line: &str, finished: &mut Vec<Self>) {
        self.next(SkipIndent::new(line, 0), finished);
    }

    fn next(&mut self, line: SkipIndentResult, finished: &mut Vec<Self>) {
        let result = self.next_no_apply(line);
        self.apply_result(result, finished);
    }

    fn next_no_apply(&mut self, line: SkipIndentResult) -> LineResult {
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

    fn apply_result(&mut self, result: LineResult, finished: &mut Vec<Self>) {
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

    pub(crate) fn finish(self) -> Option<Block> {
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

    fn next_empty(line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(line) => match line.indent {
                0..=3 => Self::next_empty_known_indent(line),
                4.. => IndentedCodeBlock::new(line).new(),
            },
            SkipIndentResult::Blank(_) => LineResult::None,
        }
    }

    fn next_empty_known_indent(line: SkipIndent) -> LineResult {
        match line.first {
            '~' | '`' => match FencedCodeBlock::check(line) {
                NewResult::New(b) => LineResult::New(b),
                NewResult::Text(s) => Paragraph::new(s).new(),
            },
            '*' | '-' => match List::check_star_dash(line) {
                ListBreakResult::List(l) => l.new(),
                ListBreakResult::Break(b) => b.done(),
                ListBreakResult::Text(s) => Paragraph::new(s).new(),
            },
            '_' => match ThematicBreak::check(line) {
                DoneResult::Done(b) => LineResult::Done(b),
                DoneResult::Text(s) => Paragraph::new(s).new(),
            },
            '+' => match List::check_plus(line) {
                NewResult::New(b) => LineResult::New(b),
                NewResult::Text(s) => Paragraph::new(s).new(),
            },
            '#' => match AtxHeading::check(line) {
                DoneResult::Done(b) => LineResult::Done(b),
                DoneResult::Text(s) => Paragraph::new(s).new(),
            },
            '>' => BlockQuote::new(line).new(),
            '0'..='9' => match List::check_number(line) {
                NewResult::New(b) => LineResult::New(b),
                NewResult::Text(s) => Paragraph::new(s).new(),
            },
            _ => Paragraph::new(line).new(),
        }
    }

    fn take(&mut self) -> Self { std::mem::take(self) }

    fn replace(&mut self, new: Self) -> Self { std::mem::replace(self, new) }
}

impl Default for TempBlock {
    fn default() -> Self { Self::Empty }
}

enum LineResult {
    None,
    DoneSelf,
    New(TempBlock),
    Done(TempBlock),
    DoneSelfAndNew(TempBlock),
    DoneSelfAndOther(TempBlock),
}

pub enum NewResult<'a> {
    New(TempBlock),
    Text(SkipIndent<'a>),
}

pub enum DoneResult<'a> {
    Done(TempBlock),
    Text(SkipIndent<'a>),
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
