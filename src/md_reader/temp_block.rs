use atx_heading::AtxHeading;
use block_quote::BlockQuote;
use derive_more::From;
use fenced_code_block::FencedCodeBlock;
use indented_code_block::IndentedCodeBlock;
use list::{CheckOrSetextResult, List};
use paragraph::Paragraph;
use table::Table;
use thematic_break::ThematicBreak;

use crate::ast::Block;
use crate::md_reader::iters::{SkipIndent, SkipIndentResult};
use crate::md_reader::Links;

mod atx_heading;
mod block_quote;
mod fenced_code_block;
mod indented_code_block;
mod list;
mod paragraph;
mod table;
mod thematic_break;

/// Enum representing an unfinished block element
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
    /// Parses next line of a document, pushing finished blocks into the `finished` argument and
    /// finished links into the `links` argument
    pub fn next_str(&mut self, line: &str, finished: &mut Vec<Self>, links: &mut Links) {
        self.next(SkipIndent::skip(line, 0), finished, links);
    }

    /// Parses next line of a document after skipping indent pushing finished blocks into the
    /// `finished` argument and finished links into the `links` argument
    fn next(&mut self, line: SkipIndentResult, finished: &mut Vec<Self>, links: &mut Links) {
        let result = match line {
            SkipIndentResult::Line(line) => self.next_line(line, links),
            SkipIndentResult::Blank(i) => self.next_blank(i, links).0,
        };
        self.apply_result(result, finished, links);
    }

    /// Parses non-blank line of a document, pushing finished links into the `links` argument.
    /// Returns a [`LineResult`] as a result
    /// # Panics
    /// If the block is [`Self::AtxHeading`] or [`Self::ThematicBreak`] which are always passed
    /// as finished
    fn next_line(&mut self, line: SkipIndent, links: &mut Links) -> LineResult {
        match self {
            Self::Empty => Self::empty_next_line(line),
            Self::Paragraph(p) => p.next(line),
            Self::IndentedCodeBlock(i) => i.next(line),
            Self::FencedCodeBlock(f) => f.next(line),
            Self::Table(t) => t.next(line),
            Self::BlockQuote(b) => b.next(line, links),
            Self::List(l) => l.next(line, links),
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
    }

    /// Parses a blank line of a document, pushing finished links into the `links` argument.
    /// Returns a [`LineResult`] as a result and a [`bool`] if the blank line is a gap between
    /// block elements or within a block element (used by [`List`] to decide if the items are loose
    /// or not)
    /// # Panics
    /// If the block is [`Self::AtxHeading`] or [`Self::ThematicBreak`] which are always passed
    /// as finished
    fn next_blank(&mut self, indent: usize, links: &mut Links) -> (LineResult, bool) {
        match self {
            Self::Empty => return (LineResult::None, true),
            Self::Paragraph(_) | Self::Table(_) | Self::BlockQuote(_) =>
                return (LineResult::DoneSelf, true),
            Self::IndentedCodeBlock(i) => i.push_blank(indent),
            Self::FencedCodeBlock(f) => f.push_blank(indent),
            Self::List(l) => l.next_blank(indent, links),
            Self::AtxHeading(_) | Self::ThematicBreak(_) => unreachable!(),
        }
        (LineResult::None, false)
    }

    /// Parses non-blank line of a document as a continuation line pushing finished links into the
    /// `links` argument. Returns a [`LineResult`] as a result. Used by [`BlockQuote`] when the line
    /// is missing the `'>'` char or by [`List`] when the line isn't indented enough but in both
    /// cases only if the line is indented by at most 3 spaces
    fn next_continuation(&mut self, line: SkipIndent) -> LineResult {
        match self {
            Self::Paragraph(p) => p.next_continuation(line),
            Self::BlockQuote(b) => b.current.next_continuation(line),
            Self::List(List { current: Some(c), .. }) => c.current.next_continuation(line),
            _ => Self::check_block_known_indent(line).into_line_result_paragraph(true),
        }
    }

    /// Parses non-blank line of a document as a continuation line pushing finished links into the
    /// `links` argument. Returns a [`LineResult`] as a result. Used by [`BlockQuote`] when the line
    /// is missing the `'>'` char or by [`List`] when the line isn't indented enough but in both
    /// cases only if the line is indented by at least 4 spaces
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

    /// Applies [`LineResult`] assuming no links can be created, pushing finished blocks into the
    /// `finished` argument. Used by [`BlockQuote`] and [`List`] when starting the first block
    fn apply_result_no_links(&mut self, result: LineResult, finished: &mut Vec<Self>) {
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

    /// Applies [`LineResult`] pushing finished blocks into the `finished` argument and finished
    /// links into the [`links`] argument
    fn apply_result(&mut self, result: LineResult, finished: &mut Vec<Self>, links: &mut Links) {
        match result {
            LineResult::None => {},
            LineResult::New(new) => *self = new,
            LineResult::DoneSelf => {
                self.finish_links(links);
                finished.push(self.take());
            },
            LineResult::Done(mut block) => {
                block.finish_links(links);
                finished.push(block);
            },
            LineResult::DoneSelfAndNew(block) => {
                self.finish_links(links);
                finished.push(self.replace(block));
            },
            LineResult::DoneSelfAndOther(mut block) => {
                self.finish_links(links);
                block.finish_links(links);
                finished.push(self.take());
                finished.push(block);
            },
        }
    }

    /// Extracts links from a finished block, pushing them into the `links` argument.
    pub fn finish_links(&mut self, links: &mut Links) {
        match self {
            Self::Paragraph(p) => p.add_links(links),
            Self::BlockQuote(b) => b.current.finish_links(links),
            Self::List(List { current: Some(c), .. }) => c.current.finish_links(links),
            _ => {},
        }
    }

    /// Finishes block into a [`Block`]
    pub fn finish(self) -> Option<Block> {
        match self {
            Self::Empty => None,
            Self::Paragraph(p) => p.finish(),
            Self::AtxHeading(a) => Some(a.finish()),
            Self::ThematicBreak(_) => Some(ThematicBreak::finish()),
            Self::IndentedCodeBlock(i) => Some(i.finish()),
            Self::FencedCodeBlock(c) => Some(c.finish()),
            Self::Table(t) => Some(t.finish()),
            Self::BlockQuote(b) => Some(b.finish()),
            Self::List(l) => Some(l.finish()),
        }
    }

    /// Creates a new block from a line after skipping indent. Used by [`BlockQuote`] when creating
    /// the first block. Returns current block and finished blocks
    fn new_empty(line: SkipIndentResult) -> (Self, Vec<Self>) {
        match line {
            SkipIndentResult::Line(line) => {
                let mut new = Self::Empty;
                let mut finished = Vec::new();
                new.apply_result_no_links(Self::empty_next_line(line), &mut finished);
                (new, finished)
            },
            SkipIndentResult::Blank(_) => (Self::Empty, Vec::new()),
        }
    }

    /// Creates a new block from a non-blank line after skipping indent. Used by [`List`] when
    /// creating the first block. Returns current block and finished blocks
    fn new_empty_known_indent(line: SkipIndent) -> (Self, Vec<Self>) {
        let mut new = Self::Empty;
        let mut finished = Vec::new();
        new.apply_result_no_links(Self::empty_next_line_known_indent(line), &mut finished);
        (new, finished)
    }

    /// Checks if a new block can be started from a non-blank line. Returns a [`CheckResult`]
    fn check_block(line: SkipIndent) -> CheckResult {
        match line.indent {
            0..=3 => Self::check_block_known_indent(line),
            4.. => CheckResult::New(IndentedCodeBlock::new(line).into()),
        }
    }

    /// Checks if a new block can be started from a non-blank line assuming the indent is at most 3
    /// spaces. Returns a [`CheckResult`]
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

    /// Parses next non-blank line of the document when the current block is [`Self::Empty`]
    fn empty_next_line(line: SkipIndent) -> LineResult {
        Self::check_block(line).into_line_result_paragraph(false)
    }

    /// Parses next non-blank line indented of the document when it's indented at most 3 spaces and
    /// the current block is [`Self::Empty`]
    fn empty_next_line_known_indent(line: SkipIndent) -> LineResult {
        Self::check_block_known_indent(line).into_line_result_paragraph(false)
    }

    /// Replaces self with the default value ([`Self::Empty`]), returning the previous value
    fn take(&mut self) -> Self { std::mem::take(self) }

    /// Replaces self with the new value, returning the previous value
    fn replace(&mut self, new: Self) -> Self { std::mem::replace(self, new) }

    /// Returns whether the current value is [`Self::Empty`]
    const fn is_empty(&self) -> bool { matches!(self, Self::Empty) }
    
    /// Returns whether the current item ends with a gap (used by [`List`] to check if it is loose,
    /// only checks blocks that are not ended by blank lines)
    fn ends_with_gap(&self) -> bool {
        match self {
            Self::IndentedCodeBlock(i) => i.ends_with_blank,
            Self::List(l) => l.ends_with_blank(),
            _ => false
        }
    }
}

/// Enum representing every possible result after parsing a line of a document
pub enum LineResult {
    /// Line was consumed and nothing changed
    None,
    /// Current block is finished, no new blocks started
    DoneSelf,
    /// New block is started, current is ignored
    New(TempBlock),
    /// New block is finished, current is ignored
    Done(TempBlock),
    /// Current block is finished and a new block is started
    DoneSelfAndNew(TempBlock),
    /// Current block and a new block are both finished
    DoneSelfAndOther(TempBlock),
}

impl LineResult {
    /// Returns whether current variant is [`Self::New`] or [`Self::Done`]
    const fn is_done_or_new(&self) -> bool { matches!(self, Self::New(_) | Self::Done(_)) }

    /// Returns whether current variant is [`Self::DoneSelfAndNew`] or [`Self::DoneSelfAndOther`]
    const fn is_done_self_and_new_or_other(&self) -> bool {
        matches!(self, Self::DoneSelfAndNew(_) | Self::DoneSelfAndOther(_))
    }
}

/// Enum representing result of checking if a new block is started or finished
pub enum CheckResult<'a> {
    /// New block is started
    New(TempBlock),
    /// New block is finished
    Done(TempBlock),
    /// No new block started, text has to be processed
    Text(SkipIndent<'a>),
}

impl<'a> CheckResult<'a> {
    /// Converts [`CheckResult`] into a [`LineResult`]. Text is converted into a new [`Paragraph`].
    /// New block is converted into [`LineResult::New`] or [`LineResult::DoneSelfAndNew`] depending
    /// on the `done_self` argument. Done block is converted into [`LineResult::Done`] or
    /// [`LineResult::DoneSelfAndOther`] depending on the `done_self` argument.
    fn into_line_result_paragraph(self, done_self: bool) -> LineResult {
        match (self, done_self) {
            (CheckResult::New(b), false) => LineResult::New(b),
            (CheckResult::New(b), true) => LineResult::DoneSelfAndNew(b),
            (CheckResult::Done(b), false) => LineResult::Done(b),
            (CheckResult::Done(b), true) => LineResult::DoneSelfAndOther(b),
            (CheckResult::Text(s), false) => LineResult::New(Paragraph::new(&s).into()),
            (CheckResult::Text(s), true) => LineResult::DoneSelfAndNew(Paragraph::new(&s).into()),
        }
    }

    /// Converts [`CheckResult`] into a [`LineResult`]. Text is converted into a [`LineResult`] with
    /// the given `text_function` argument regardless of the `done_self` argument. New block is
    /// converted into [`LineResult::New`] or [`LineResult::DoneSelfAndNew`] depending on the
    /// `done_self` argument. Done block is converted into [`LineResult::Done`] or
    /// [`LineResult::DoneSelfAndOther`] depending on the `done_self` argument.
    fn into_line_result<F>(self, done_self: bool, text_function: F) -> LineResult
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

/// Enum representing result of checking if a new block is started
pub enum NewResult<'a> {
    New(TempBlock),
    Text(SkipIndent<'a>),
}
