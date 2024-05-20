use atx_heading::AtxHeading;
use block_quote::BlockQuote;
use derive_more::From;
use fenced_code_block::FencedCodeBlock;
use indented_code_block::IndentedCodeBlock;
use iters::{SkipIndent, SkipIndentResult};
use list::{CheckOrSetextResult, List};
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

#[derive(From, Debug, Default)]
pub enum TempBlock {
    #[default]
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
    pub fn next_str(&mut self, line: &str, finished: &mut Vec<Self>) {
        self.next(SkipIndent::skip(line, 0), finished);
    }

    fn next(&mut self, line: SkipIndentResult, finished: &mut Vec<Self>) {
        let result = match line {
            SkipIndentResult::Line(line) => self.next_line(line),
            SkipIndentResult::Blank(i) => self.next_blank(i).0,
        };
        self.apply_result(result, finished);
    }

    fn next_line(&mut self, line: SkipIndent) -> LineResult {
        match self {
            Self::Empty => Self::empty_next_line(line),
            Self::Paragraph(p) => p.next(line),
            Self::IndentedCodeBlock(i) => i.next(line),
            Self::FencedCodeBlock(f) => f.next(line),
            Self::Table(t) => t.next(line),
            Self::BlockQuote(b) => b.next(line),
            Self::List(l) => l.next(line),
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
    }

    fn next_blank(&mut self, indent: usize) -> (LineResult, bool) {
        match self {
            Self::Empty => return (LineResult::None, true),
            Self::Paragraph(_) | Self::Table(_) | Self::BlockQuote(_) =>
                return (LineResult::DoneSelf, true),
            Self::IndentedCodeBlock(i) => i.push_blank(indent),
            Self::FencedCodeBlock(f) => f.push_blank(indent),
            Self::List(l) => l.next_blank(indent),
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
        (LineResult::None, false)
    }

    fn next_continuation(&mut self, line: SkipIndent) -> LineResult {
        match self {
            Self::Paragraph(p) => p.next_continuation(line),
            Self::BlockQuote(b) => b.current.next_continuation(line),
            Self::List(List { current: Some(c), .. }) => c.current.next_continuation(line),
            _ => Self::check_block_known_indent(line).into_line_result_paragraph(true),
        }
    }

    fn next_indented_continuation(&mut self, line: SkipIndent) -> LineResult {
        match self {
            Self::Paragraph(p) => {
                p.next_indented_continuation(&line);
                LineResult::None
            },
            Self::BlockQuote(b) => b.current.next_indented_continuation(line),
            Self::List(List { current: Some(c), .. }) => c.current.next_indented_continuation(line),
            _ => LineResult::DoneSelfAndNew(IndentedCodeBlock::new(line).into()),
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

    pub fn finish(self) -> Option<Block> {
        match self {
            Self::Empty => None,
            Self::Paragraph(p) => Some(p.finish()),
            Self::AtxHeading(a) => Some(a.finish()),
            Self::ThematicBreak(_) => Some(ThematicBreak::finish()),
            Self::IndentedCodeBlock(i) => Some(i.finish()),
            Self::FencedCodeBlock(c) => Some(c.finish()),
            Self::Table(t) => Some(t.finish()),
            Self::BlockQuote(b) => Some(b.finish()),
            Self::List(l) => Some(l.finish()),
        }
    }

    fn new_empty(line: SkipIndentResult) -> (Self, Vec<Self>) {
        match line {
            SkipIndentResult::Line(line) => {
                let mut new = Self::Empty;
                let mut finished = Vec::new();
                new.apply_result(Self::empty_next_line(line), &mut finished);
                (new, finished)
            },
            SkipIndentResult::Blank(_) => (Self::Empty, Vec::new()),
        }
    }

    fn new_empty_known_indent(line: SkipIndent) -> (Self, Vec<Self>) {
        let mut new = Self::Empty;
        let mut finished = Vec::new();
        new.apply_result(Self::empty_next_line_known_indent(line), &mut finished);
        (new, finished)
    }

    fn check_block(line: SkipIndent) -> CheckResult {
        match line.indent {
            0..=3 => Self::check_block_known_indent(line),
            4.. => CheckResult::New(IndentedCodeBlock::new(line).into()),
        }
    }

    fn check_block_known_indent(line: SkipIndent) -> CheckResult {
        match line.first {
            '#' => AtxHeading::check(line),
            '_' => ThematicBreak::check(line),
            '~' | '`' => FencedCodeBlock::check(line),
            '>' => CheckResult::New(BlockQuote::new(&line).into()),
            '*' | '-' => List::check_star_dash(line),
            '+' => List::check_plus(line),
            '0'..='9' => List::check_number(line),
            _ => CheckResult::Text(line),
        }
    }

    fn empty_next_line(line: SkipIndent) -> LineResult {
        Self::check_block(line).into_line_result_paragraph(false)
    }

    fn empty_next_line_known_indent(line: SkipIndent) -> LineResult {
        Self::check_block_known_indent(line).into_line_result_paragraph(false)
    }

    fn take(&mut self) -> Self { std::mem::take(self) }

    fn replace(&mut self, new: Self) -> Self { std::mem::replace(self, new) }

    const fn is_empty(&self) -> bool { matches!(self, Self::Empty) }

    const fn as_list(&self) -> Option<&List> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }
}

pub enum LineResult {
    None,
    DoneSelf,
    New(TempBlock),
    Done(TempBlock),
    DoneSelfAndNew(TempBlock),
    DoneSelfAndOther(TempBlock),
}

impl LineResult {
    pub const fn is_done_or_new(&self) -> bool { matches!(self, Self::New(_) | Self::Done(_)) }

    pub const fn is_done_self_and_new_or_other(&self) -> bool {
        matches!(self, Self::DoneSelfAndNew(_) | Self::DoneSelfAndOther(_))
    }
}

pub enum CheckResult<'a> {
    New(TempBlock),
    Done(TempBlock),
    Text(SkipIndent<'a>),
}

impl<'a> CheckResult<'a> {
    pub fn into_line_result_paragraph(self, done_self: bool) -> LineResult {
        match (self, done_self) {
            (CheckResult::New(b), false) => LineResult::New(b),
            (CheckResult::New(b), true) => LineResult::DoneSelfAndNew(b),
            (CheckResult::Done(b), false) => LineResult::Done(b),
            (CheckResult::Done(b), true) => LineResult::DoneSelfAndOther(b),
            (CheckResult::Text(s), false) => LineResult::New(Paragraph::new(&s).into()),
            (CheckResult::Text(s), true) => LineResult::DoneSelfAndNew(Paragraph::new(&s).into()),
        }
    }

    pub fn into_line_result<F>(self, done_self: bool, text_function: F) -> LineResult
    where F: FnOnce(SkipIndent<'a>) -> LineResult {
        match (self, done_self) {
            (CheckResult::New(b), false) => LineResult::New(b),
            (CheckResult::New(b), true) => LineResult::DoneSelfAndNew(b),
            (CheckResult::Done(b), false) => LineResult::Done(b),
            (CheckResult::Done(b), true) => LineResult::DoneSelfAndOther(b),
            (CheckResult::Text(s), _) => text_function(s),
        }
    }
}

pub enum NewResult<'a> {
    New(TempBlock),
    Text(SkipIndent<'a>),
}
