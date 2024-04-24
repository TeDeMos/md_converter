use std::iter::Peekable;
use std::str::Chars;

use atx_heading::AtxHeading;
use empty::Empty;
use fenced_code_block::FencedCodeBlock;
use indented_code_block::IndentedCodeBlock;
use paragraph::Paragraph;
use table::Table;
use thematic_break::ThematicBreak;
use block_quote::BlockQuote;

use crate::ast::Block;

mod atx_heading;
mod empty;
mod fenced_code_block;
mod indented_code_block;
mod paragraph;
mod table;
mod thematic_break;
mod block_quote;

pub enum TempBlock {
    Empty(Empty),
    Paragraph(Paragraph),
    AtxHeading(AtxHeading),
    ThematicBreak(ThematicBreak),
    IndentedCodeBlock(IndentedCodeBlock),
    FencedCodeBlock(FencedCodeBlock),
    Table(Table),
    BlockQuote(BlockQuote)
}

impl TempBlock {
    pub fn next(&mut self, line: &str, finished: &mut Vec<Self>) {
        let result = match self {
            TempBlock::Empty(_) => Empty::next(line),
            TempBlock::Paragraph(p) => p.next(line),
            TempBlock::IndentedCodeBlock(i) => i.next(line),
            TempBlock::FencedCodeBlock(f) => f.next(line),
            TempBlock::Table(t) => t.next(line),
            TempBlock::BlockQuote(b) => b.next(line),
            //Safety: atx headings and thematic breaks are always passed as finished
            TempBlock::AtxHeading(_) | TempBlock::ThematicBreak(_) => unreachable!(),
        };
        self.apply_result(result, finished);
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
            TempBlock::Empty(_) => None,
            TempBlock::Paragraph(p) => Some(p.finish()),
            TempBlock::AtxHeading(a) => Some(a.finish()),
            TempBlock::ThematicBreak(t) => Some(t.finish()),
            TempBlock::IndentedCodeBlock(i) => Some(i.finish()),
            TempBlock::FencedCodeBlock(c) => Some(c.finish()),
            TempBlock::Table(table) => Some(table.finish()),
            TempBlock::BlockQuote(b) => Some(b.finish()),
        }
    }

    fn take(&mut self) -> Self { std::mem::replace(self, Self::default()) }

    fn replace(&mut self, new: Self) -> Self { std::mem::replace(self, new) }
}

impl Default for TempBlock {
    fn default() -> Self { Self::Empty(Empty) }
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
    let mut indent = 0;
    loop {
        match iter.peek() {
            Some('\t') => indent += 4 - indent % 4,
            Some(' ') => indent += 1,
            _ => return (indent, iter),
        }
        iter.next();
        if indent >= limit {
            return (indent, iter);
        }
    }
}
