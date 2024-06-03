use std::iter::{Peekable, Rev};
use std::str::CharIndices;

/// Represents the result after skipping indent
#[derive(Debug)]
pub enum SkipIndentResult<'a> {
    /// A non blank line result
    Line(SkipIndent<'a>),
    /// A blank line with indent count
    Blank(usize),
}

impl<'a> SkipIndentResult<'a> {
    /// Checks if is in [`Self::Line`] variant and applies a function to modify it
    pub fn inspect_line<F>(&mut self, f: F)
    where F: FnOnce(&mut SkipIndent<'a>) {
        match self {
            SkipIndentResult::Line(s) => f(s),
            SkipIndentResult::Blank(_) => {},
        }
    }

    /// Moves into the [`Self::Line`] variant (for testing)
    /// # Panics
    /// If is not in [`Self::Line`] variant
    #[cfg(test)]
    pub const fn into_line(self) -> SkipIndent<'a> {
        match self {
            SkipIndentResult::Line(s) => s,
            SkipIndentResult::Blank(_) => panic!(),
        }
    }
}

/// Represents a result of skipping indent in a non--blank line
#[derive(Debug)]
pub struct SkipIndent<'a> {
    /// First found char of the line
    pub first: char,
    /// Intent skipped
    pub indent: usize,
    /// Total indent (for keeping track of tab-stops)
    total: usize,
    /// Line with trimmed indent from the start
    pub line: &'a str,
}

impl<'a> SkipIndent<'a> {
    /// Skips indent of a line with a given total indent for tracking tab-stops
    pub fn skip(line: &'a str, total_indent: usize) -> SkipIndentResult {
        let mut total = total_indent;
        for (i, c) in line.char_indices() {
            match c {
                ' ' => total += 1,
                '\t' => total = total + (4 - (total % 4)),
                c => {
                    return SkipIndentResult::Line(Self {
                        first: c,
                        indent: total - total_indent,
                        total,
                        // Safety: using index from CharIndices
                        line: unsafe { line.get_unchecked(i..) },
                    });
                },
            }
        }
        SkipIndentResult::Blank(total - total_indent)
    }

    /// Moves indent unchecked
    pub fn move_indent(&mut self, indent: usize) { self.indent -= indent; }

    /// Moves indent checking for overflow
    pub fn move_indent_capped(&mut self, indent: usize) {
        self.indent = self.indent.saturating_sub(indent);
    }

    /// Gets line without the first char
    pub fn get_rest(&self) -> &'a str {
        // Safety: using utf8 length of first char as index
        unsafe { self.line.get_unchecked(self.first.len_utf8()..) }
    }

    /// Iterates with [`Iter`] over the line without the first char
    pub fn iter_rest(&self) -> Iter<'a> { Iter::new(self.get_rest()) }

    /// Iterates with [`Iter`] over the full line
    pub fn iter_full(&self) -> Iter<'a> { Iter::new(self.line) }

    /// Iterates with [`IndentIter`] over the line without the first char
    pub fn indent_iter_rest(&self) -> IndentIter<'a> {
        IndentIter::new(self.get_rest(), self.total + 1)
    }

    /// Skips indent again from the line without the first char
    pub fn skip_indent_rest(&self) -> SkipIndentResult<'a> {
        Self::skip(self.get_rest(), self.total + 1)
    }

    /// Gets full line as owned string
    pub fn get_full(&self) -> String {
        match self.indent {
            0 => self.line.to_owned(),
            c => " ".repeat(c) + self.line,
        }
    }

    /// Pushes full line to an existing string
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

/// Custom iterator over a string with useful functions
pub struct Iter<'a> {
    source: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> Iter<'a> {
    /// Creates a new iterator over a given string slice
    pub fn new(source: &'a str) -> Self { Self { source, iter: source.char_indices().peekable() } }

    /// Peeks next char
    pub fn peek(&mut self) -> Option<char> { self.iter.peek().map(|x| x.1) }

    /// Gets next char
    pub fn next(&mut self) -> Option<char> { self.iter.next().map(|x| x.1) }

    /// Skips over all the occurrences of a char and returns the amount skipped
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

    /// Skips if next char is equal to a given char and returns if it skipped
    pub fn next_if_eq(&mut self, c: char) -> bool {
        match self.iter.peek() {
            Some(&(_, current)) if current == c => {
                self.iter.next();
                true
            },
            Some(_) | None => false,
        }
    }

    /// Skips over whitespace (spaces and tabs)
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

    /// Skips over whitespace (spaces, tabs and new lines)
    pub fn skip_whitespace_new_line(&mut self) {
        loop {
            match self.iter.peek() {
                Some((_, ' ' | '\t' | '\n')) => {
                    self.iter.next();
                },
                Some(_) | None => return,
            }
        }
    }

    /// Skips whitespace returning if at least one was skipped
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

    /// Skips over all the occurrences of a char and returns if at least one was skipped
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

    /// Returns if the iterator reached the end of the string
    pub fn ended(&mut self) -> bool { self.iter.peek().is_none() }

    /// Skips until reaches a given char without backslash before it. Returns none if it did not
    /// find such a char
    pub fn get_str_until_unescaped(&mut self, c: char) -> Option<&'a str> {
        let start = self.iter.peek()?.0;
        let mut escape = false;
        loop {
            match self.iter.next()? {
                (end, current) if !escape && current == c =>
                // Safety: start and end both from CharIndices
                    return Some(unsafe { self.source.get_unchecked(start..end) }),
                (_, current) => escape = current == '\\' && !escape,
            }
        }
    }

    /// Skips until reaches a given char without backslash before it while checking for an illegal
    /// char without backslash before it. Returns none if it did not find such a char, or it found
    /// an illegal one
    pub fn get_str_until_unescaped_without(
        &mut self, expected: char, illegal: char,
    ) -> Option<&'a str> {
        let start = self.iter.peek()?.0;
        let mut escape = false;
        loop {
            match self.iter.next()? {
                (end, current) if !escape && current == expected =>
                // Safety: start and end both from CharIndices
                    return Some(unsafe { self.source.get_unchecked(start..end) }),
                (_, current) if !escape && current == illegal => return None,
                (_, current) => escape = current == '\\' && !escape,
            }
        }
    }

    /// Skips until the end of the link destination and returns it. Returns none if rules for a link
    /// destination are not met
    pub fn get_link_destination(&mut self) -> Option<&'a str> {
        match self.iter.next()? {
            (s, '<') => {
                let mut escape = false;
                loop {
                    match self.iter.next()? {
                        (e, '>') if !escape =>
                        // Safety: s and e both from CharIndices, char at s is '<' with width 1
                            return Some(unsafe { self.source.get_unchecked((s + 1)..e) }),
                        (_, '\n') => return None,
                        (_, c) => escape = c == '\\' && !escape,
                    }
                }
            },
            (s, _) => loop {
                match self.iter.peek() {
                    Some(&(e, ' ' | '\t' | '\n')) =>
                    // Safety: s and e both from CharIndices
                        return Some(unsafe { self.source.get_unchecked(s..e) }),
                    // Safety: s from CharIndices
                    None => return Some(unsafe { self.source.get_unchecked(s..) }),
                    Some((_, c)) if c.is_ascii_control() => return None,
                    Some(_) => _ = self.iter.next(),
                }
            },
        }
    }

    /// Gets the rest of the slice
    pub fn get_str(&mut self) -> &'a str {
        match self.iter.peek() {
            // Safety: index from CharIndices
            Some(&(i, _)) => unsafe { self.source.get_unchecked(i..) },
            None => "",
        }
    }

    /// Iterates over the remaining slice with [`RevIter`]
    pub fn iter_rest_rev(&mut self) -> RevIter<'a> { RevIter::new(self.get_str()) }

    /// Without advancing the iterator checks if the remaining string contains a given char
    pub fn any_eq(&self, c: char) -> bool { self.iter.clone().any(|(_, current)| current == c) }

    /// Gets the rest of the slice as owned string
    pub fn get_string(&mut self) -> String { self.get_str().to_owned() }

    /// Gets the rest of the slice as owned string trimming the end
    pub fn get_string_trimmed(&mut self) -> String { self.get_str().trim_end().to_owned() }
}

/// Iterator that keeps track of indent for proper tab-stop treatment
pub struct IndentIter<'a> {
    indent: usize,
    source: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> IndentIter<'a> {
    /// Creates the iterator over a given slice with a given total indent
    fn new(source: &'a str, indent: usize) -> Self {
        Self { indent, source, iter: source.char_indices().peekable() }
    }

    /// Gets the number given the first char, returns the number and its digit count
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

    /// Gets next char if it's an ordered list item marker closing char
    pub fn get_closing(&mut self) -> Option<char> {
        self.iter.next_if(|(_, c)| matches!(c, '.' | ')')).map(|x| x.1)
    }

    /// Skips indent from the rest of the iterator
    pub fn skip_indent(&mut self) -> SkipIndentResult<'a> {
        match self.iter.peek() {
            Some(&(i, _)) =>
            // Safety: index from CharIndices
                SkipIndent::skip(unsafe { self.source.get_unchecked(i..) }, self.indent),
            None => SkipIndentResult::Blank(0),
        }
    }
}

/// Iters over a slice of a string in reverse
pub struct RevIter<'a> {
    source: &'a str,
    iter: Peekable<Rev<CharIndices<'a>>>,
}

impl<'a> RevIter<'a> {
    /// Creates a new iterator over a given string slice
    fn new(source: &'a str) -> Self {
        Self { source, iter: source.char_indices().rev().peekable() }
    }

    /// Skips over all the occurrences of a char and returns the amount skipped
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

    /// Skips over whitespace (spaces and tabs)
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

    /// Gets the rest of the slice
    fn get_str(&mut self) -> &str {
        match self.iter.peek() {
            //Safety: index from CharIndices and width of the peeked char
            Some(&(i, c)) => unsafe { self.source.get_unchecked(..i + c.len_utf8()) },
            None => "",
        }
    }

    /// Advances if next char is a space or a tab and returns if it advanced of iterator ended
    pub fn next_if_whitespace_or_none(&mut self) -> bool {
        match self.iter.peek() {
            Some((_, ' ' | '\t')) => {
                self.iter.next();
                true
            },
            None => true,
            Some(_) => false,
        }
    }

    /// Gets the rest of the slice as an owned string
    pub fn get_string(&mut self) -> String { self.get_str().to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_indent(line: &str, total: usize, expected_indent: usize, expected_total: usize) {
        if let SkipIndentResult::Line(SkipIndent { total, indent, .. }) =
            SkipIndent::skip(line, total)
        {
            if !(total == expected_total && indent == expected_indent) {
                println!("{total}, {indent}");
                panic!();
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn test_skip() {
        check_indent("  line", 0, 2, 2);
        check_indent("\tline", 0, 4, 4);
        check_indent(" \tline", 0, 4, 4);
        check_indent("  \tline", 0, 4, 4);
        check_indent("   \tline", 0, 4, 4);
        check_indent("    \tline", 0, 8, 8);
        check_indent("  \t line", 0, 5, 5);
        check_indent("  \t line", 1, 4, 5);
        check_indent("  \t line", 2, 7, 9);
    }
}
