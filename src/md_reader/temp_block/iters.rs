use std::iter::{Peekable, Rev};
use std::str::CharIndices;

pub enum SkipIndentResult<'a> {
    Line(SkipIndent<'a>),
    Blank(usize),
}

impl<'a> SkipIndentResult<'a> {
    pub fn inspect<F>(&mut self, f: F)
    where F: FnOnce(&mut SkipIndent<'a>) {
        match self {
            SkipIndentResult::Line(s) => f(s),
            SkipIndentResult::Blank(_) => {},
        }
    }
}

pub struct SkipIndent<'a> {
    pub first: char,
    pub indent: usize,
    total: usize,
    pub line: &'a str,
}

impl<'a> SkipIndent<'a> {
    pub fn skip(line: &'a str, indent: usize) -> SkipIndentResult {
        let mut total = indent;
        for (i, c) in line.char_indices() {
            match c {
                ' ' => total += 1,
                '\t' => total = total + (4 - (total % 4)),
                c => {
                    return SkipIndentResult::Line(Self {
                        first: c,
                        indent: total - indent,
                        total,
                        line: unsafe { line.get_unchecked(i..) },
                    });
                },
            }
        }
        SkipIndentResult::Blank(total - indent)
    }

    pub fn move_indent(&mut self, indent: usize) { self.indent -= indent; }

    pub fn move_indent_capped(&mut self, indent: usize) {
        self.indent = self.indent.saturating_sub(indent);
    }

    pub fn get_rest(&self) -> &'a str { unsafe { self.line.get_unchecked(self.first.len_utf8()..) } }

    pub fn iter_rest(&self) -> Iter<'a> { Iter::new(self.get_rest()) }

    pub fn iter_full(&self) -> Iter<'a> { Iter::new(self.line) }

    pub fn indent_iter_rest(&self) -> IndentIter<'a> {
        IndentIter::new(self.get_rest(), self.total + 1)
    }

    pub fn skip_indent_rest(&self) -> SkipIndentResult<'a> {
        Self::skip(self.get_rest(), self.total + 1)
    }

    pub fn get_full(&self) -> String {
        match self.indent {
            0 => self.line.to_owned(),
            c => " ".repeat(c) + self.line,
        }
    }

    pub fn push_full(&self, result: &mut String) {
        match self.indent {
            0 => result.push_str(self.line),
            c => {
                result.reserve(c + self.line.len());
                for _ in 0..c {
                    result.push(' ');
                }
                result.push_str(self.line);
            },
        }
    }
}

pub struct Iter<'a> {
    source: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> Iter<'a> {
    fn new(source: &'a str) -> Self { Self { source, iter: source.char_indices().peekable() } }

    pub fn skip_while_eq(&mut self, c: char) -> usize {
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

    pub fn next_if_eq(&mut self, c: char) -> bool {
        match self.iter.peek() {
            Some(&(_, current)) if current == c => {
                self.iter.next();
                true
            },
            Some(_) | None => false,
        }
    }

    pub fn skip_whitespace(&mut self) {
        loop {
            match self.iter.peek() {
                Some((_, ' ' | '\t')) => {
                    self.iter.next();
                },
                Some(_) | None => return,
            }
        }
    }

    pub fn skip_whitespace_min_one(&mut self) -> bool {
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

    pub fn skip_while_eq_min_one(&mut self, c: char) -> bool {
        let mut any = false;
        loop {
            match self.iter.peek() {
                Some(&(_, current)) if current == c => {
                    self.iter.next();
                    any = true;
                },
                Some(_) | None => return any,
            }
        }
    }

    pub fn ended(&mut self) -> bool { self.iter.peek().is_none() }

    fn get_str(&mut self) -> &str {
        match self.iter.peek() {
            Some(&(i, _)) => unsafe { self.source.get_unchecked(i..) },
            None => "",
        }
    }

    pub fn iter_rest_rev(&mut self) -> RevIter { RevIter::new(self.get_str()) }

    pub fn any_eq(&self, c: char) -> bool {
        self.iter.clone().any(|(_, current)| current == c)
    }

    pub fn get_string(&mut self) -> String { self.get_str().to_owned() }

    pub fn get_string_trimmed(&mut self) -> String { self.get_str().trim_end().to_owned() }
}

pub struct IndentIter<'a> {
    indent: usize,
    source: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> IndentIter<'a> {
    fn new(source: &'a str, indent: usize) -> Self {
        Self { indent, source, iter: source.char_indices().peekable() }
    }

    pub fn get_number(&mut self, first: char) -> Option<(usize, usize)> {
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

    pub fn get_closing(&mut self) -> Option<char> {
        self.iter.next_if(|(_, c)| matches!(c, '.' | ')')).map(|x| x.1)
    }

    pub fn skip_indent(&mut self) -> SkipIndentResult<'a> {
        match self.iter.peek() {
            Some(&(i, _)) =>
                SkipIndent::skip(unsafe { self.source.get_unchecked(i..) }, self.indent),
            None => SkipIndentResult::Blank(0),
        }
    }
}

pub struct RevIter<'a> {
    source: &'a str,
    iter: Peekable<Rev<CharIndices<'a>>>,
}

impl<'a> RevIter<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, iter: source.char_indices().rev().peekable() }
    }

    pub fn skip_while_eq(&mut self, c: char) -> usize {
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

    pub fn skip_whitespace(&mut self) {
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

    pub fn next_if_whitespace_or_none(&mut self) -> bool {
        match self.iter.peek() {
            Some((_, ' ' | '\t')) | None => {
                self.iter.next();
                true
            },
            Some(_) => false,
        }
    }

    pub fn get_string(&mut self) -> String { self.get_str().to_owned() }
}
