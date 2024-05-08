use std::iter::{Peekable, Rev};
use std::str::{CharIndices, Chars};

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
use crate::md_reader::temp_block::list::StarDashResult;

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
            Self::Empty => Self::next_empty(SkipIndent::new(line, 0)),
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

    fn next_empty(line: Option<SkipIndent>) -> LineResult {
        match line {
            Some(line) => match line.indent {
                0..=3 => Self::next_empty_known_indent(line),
                4.. => IndentedCodeBlock::new2(line).new(),
            },
            None => LineResult::None,
        }
    }

    fn next_empty_known_indent(line: SkipIndent) -> LineResult {
        match line.first {
            '~' | '`' => match FencedCodeBlock::check2(line) {
                NewResult::New(b) => LineResult::New(b),
                NewResult::Text(s) => Paragraph::new2(s).new(),
            },
            '*' | '-' => match List::check_star_dash2(line) {
                StarDashResult::List(l) => l.new(),
                StarDashResult::Break(b) => b.done(),
                StarDashResult::Text(s) => Paragraph::new2(s).new(),
            },
            '_' => match ThematicBreak::check2(line) {
                DoneResult::Done(b) => LineResult::Done(b),
                DoneResult::Text(s) => Paragraph::new2(s).new(),
            },
            '+' => match List::check_plus2(line) {
                NewResult::New(b) => LineResult::New(b),
                NewResult::Text(s) => Paragraph::new2(s).new(),
            },
            '#' => match AtxHeading::check2(line) {
                DoneResult::Done(b) => LineResult::Done(b),
                DoneResult::Text(s) => Paragraph::new2(s).new(),
            },
            '>' => BlockQuote::new2(line).new(),
            '0'..='9' => match List::check_number2(line) {
                NewResult::New(b) => LineResult::New(b),
                NewResult::Text(s) => Paragraph::new2(s).new(),
            },
            _ => Paragraph::new2(line).new(),
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

pub enum NewResult<'a> {
    New(TempBlock),
    Text(SkipIndent<'a>),
}

pub enum DoneResult<'a> {
    Done(TempBlock),
    Text(SkipIndent<'a>),
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

#[derive(Clone)]
pub struct SkipIndent<'a> {
    first: char,
    indent: usize,
    total: usize,
    line: &'a str,
}

impl<'a> SkipIndent<'a> {
    fn new(line: &'a str, indent: usize) -> Option<Self> {
        let mut total = indent;
        for (i, c) in line.char_indices() {
            match c {
                ' ' => total += 1,
                '\t' => total = total + (4 - (total % 4)),
                c => {
                    return Some(Self {
                        first: c,
                        indent: total - indent,
                        total,
                        line: unsafe { line.get_unchecked(i..) },
                    });
                },
            }
        }
        None
    }

    fn move_indent(&mut self, indent: usize) { self.indent -= indent; }

    fn get_rest(&self) -> &str { unsafe { self.line.get_unchecked(self.first.len_utf8()..) } }

    fn iter_rest(&self) -> Iter { Iter::new(self.get_rest()) }

    fn indent_iter_rest(&self) -> IndentIter { IndentIter::new(self.get_rest(), self.total + 1) }
    
    fn skip_indent_rest(&'a self) -> Option<Self> { Self::new(self.get_rest(), self.total + 1) }

    fn get_full(&self) -> String {
        match self.indent {
            0 => self.line.to_owned(),
            c => " ".repeat(c) + self.line,
        }
    }
}

struct Iter<'a> {
    source: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> Iter<'a> {
    fn new(source: &'a str) -> Self { Self { source, iter: source.char_indices().peekable() } }

    fn skip_while_eq(&mut self, c: char) -> usize {
        let mut result = 0;
        loop {
            match self.iter.peek() {
                Some(&(_, current)) if current == c => {
                    self.iter.next();
                    result += 1;
                },
                Some(_) | None => return result,
            }
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.iter.peek() {
                Some((_, ' ' | '\t')) => {
                    self.iter.next();
                },
                Some(_) | None => return,
            }
        }
    }
    
    fn skip_whitespace_min_one(&mut self) -> bool {
        let mut any = false;
        loop {
            match self.iter.peek() {
                Some((_, ' ' | '\t')) => {
                    any = true;
                    self.iter.next();
                },
                Some(_) | None => return any,
            }
        }
    }
    
    fn ended(&mut self) -> bool {
        self.iter.peek().is_none()
    }

    fn get_str(&mut self) -> &str {
        match self.iter.peek() {
            Some(&(i, _)) => unsafe { self.source.get_unchecked(i..) },
            None => "",
        }
    }

    fn iter_rest_rev(&mut self) -> RevIter { RevIter::new(self.get_str()) }

    fn any_eq(&self, c: char) -> bool { self.iter.clone().any(|(_, current)| current == c) }

    fn get_string(&mut self) -> String { self.get_str().to_owned() }

    fn get_string_trimmed(&mut self) -> String { self.get_str().trim_end().to_owned() }
}

struct IndentIter<'a> {
    indent: usize,
    source: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> IndentIter<'a> {
    fn new(source: &'a str, indent: usize) -> Self {
        Self { indent, source, iter: source.char_indices().peekable() }
    }

    fn get_number(&mut self, first: char) -> Option<(usize, usize)> {
        let mut result = first as usize - '0' as usize;
        let mut length = 1;
        loop {
            match self.iter.peek() {
                Some(&(_, c @ '0'..='9')) => {
                    length += 1;
                    if length > 9 {
                        return None;
                    }
                    result = 10 * result + (c as usize - '0' as usize);
                    self.indent += 1;
                    self.iter.next();
                },
                Some(_) | None => return Some((result, length)),
            }
        }
    }
    
    fn get_closing(&mut self) -> Option<char> {
        self.iter.next_if(|(_, c)| matches!(c, '.' | ')')).map(|x| x.1)
    }

    fn skip_indent(&mut self) -> Option<SkipIndent> {
        match self.iter.peek() {
            Some(&(i, _)) =>
                SkipIndent::new(unsafe { self.source.get_unchecked(i..) }, self.indent),
            None => None,
        }
    }
}

struct RevIter<'a> {
    source: &'a str,
    iter: Peekable<Rev<CharIndices<'a>>>,
}

impl<'a> RevIter<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, iter: source.char_indices().rev().peekable() }
    }

    fn skip_while_eq(&mut self, c: char) -> usize {
        let mut count = 0;
        loop {
            match self.iter.peek() {
                Some(&(_, current)) if current == c => {
                    count += 1;
                    self.iter.next();
                },
                Some(_) | None => return count,
            }
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.iter.peek() {
                Some(&(_, ' ' | '\t')) => {
                    self.iter.next();
                },
                Some(_) | None => return,
            }
        }
    }
    
    fn get_str(&mut self) -> &str {
        match self.iter.peek() {
            Some(&(i, c)) => unsafe { self.source.get_unchecked(..i + c.len_utf8()) },
            None => "",
        }
    }

    fn next_if_whitespace_or_none(&mut self) -> bool {
        match self.iter.peek() {
            Some((_, ' ' | '\t')) | None => {
                self.iter.next();
                true
            },
            Some(_) => false,
        }
    }

    fn get_string(&mut self) -> String { self.get_str().to_owned() }
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
