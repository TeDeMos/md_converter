use crate::ast::{Block, Inline};
use crate::md_reader::inline_parser::InlineParser;
use crate::md_reader::iters::{Iter, SkipIndent};
use crate::md_reader::temp_block::{
    AtxHeading, BlockQuote, CheckOrSetextResult, CheckResult, FencedCodeBlock, LineResult, Links,
    List, NewResult, Table, TempBlock, ThematicBreak,
};

/// Struct representing an unfinished paragraph
#[derive(Debug)]
pub struct Paragraph {
    /// Content of the paragraph
    pub content: String,
    /// Amount of columns of the table header defined by the last line
    pub table_header_length: usize,
    /// Index of the start of the last line in `content`
    line_start: usize,
    /// 0 if paragraph is not a header, 1 if ends with `'='`, 2 if ends with `'-'`
    setext: usize,
    /// Number of `'='` or `'-'` characters used to make this paragraph a setext header
    setext_char_count: usize,
}

impl Paragraph {
    /// Creates a new paragraph starting with a given non-blank line
    pub fn new(line: &SkipIndent) -> Self {
        Self {
            content: line.line.to_owned(),
            line_start: 0,
            table_header_length: Table::check_header(line.line),
            setext: 0,
            setext_char_count: 0,
        }
    }

    /// Parses a non-blank line of a document
    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        let checked = match line.indent {
            0..=3 => match line.first {
                '=' => return self.push_check_setext(line),
                '#' => AtxHeading::check(line),
                '_' => ThematicBreak::check(line),
                '~' | '`' => FencedCodeBlock::check(line),
                '>' => CheckResult::New(BlockQuote::new(&line).into()),
                '*' => List::check_star_paragraph(line),
                '-' => match List::check_dash_paragraph(line) {
                    CheckOrSetextResult::Check(c) => c,
                    CheckOrSetextResult::Setext(n) => {
                        self.setext = 2;
                        self.setext_char_count = n;
                        return LineResult::DoneSelf;
                    },
                },
                '+' => List::check_plus_paragraph(line),
                '1' => List::check_number_paragraph(line),
                _ => CheckResult::Text(line),
            },
            4.. => {
                self.push_header_check(&line);
                return LineResult::None;
            },
        };
        checked.into_line_result(true, |s| self.push_full_check(s))
    }

    /// Parses a non-blank line of a document as a continuation line indented at most 3 spaces
    pub fn next_continuation(&mut self, line: SkipIndent) -> LineResult {
        TempBlock::check_block_known_indent(line).into_line_result(true, |s| {
            self.push_header_no_indent_check(&s);
            LineResult::None
        })
    }

    /// Parses a non-blank line of a document as a continuation line indented at least 4 spaces
    pub fn next_indented_continuation(&mut self, line: &SkipIndent) { self.push(line.line); }

    /// Extracts links from this paragraph adding them into the `links` argument
    pub fn add_links(&mut self, links: &mut Links) {
        let mut iter = Iter::new(&self.content);
        let mut changed = false;
        let mut current = self.content.as_str();
        loop {
            if !iter.next_if_eq('[') {
                break;
            }
            let Some(label) = iter.get_str_until_unescaped(']') else {
                break;
            };
            if label.len() > 999 || label.trim().is_empty() {
                break;
            }
            if !iter.next_if_eq(':') {
                break;
            }
            iter.skip_whitespace_new_line();
            let Some(destination) = iter.get_link_destination() else {
                break;
            };
            let whitespace = iter.skip_whitespace_min_one();
            let new_line = iter.next_if_eq('\n');
            let (title, check) = match iter.peek() {
                Some(c @ ('"' | '\'')) if whitespace || new_line => {
                    iter.next();
                    (iter.get_str_until_unescaped(c), true)
                },
                Some('(') if whitespace || new_line => {
                    iter.next();
                    (iter.get_str_until_unescaped_without(')', '('), true)
                },
                Some(_) =>
                    if new_line {
                        (None, false)
                    } else {
                        break;
                    },
                None => (None, false),
            };
            if check && title.is_none() {
                break;
            }
            if title.is_some() {
                iter.skip_whitespace();
                if !matches!(iter.next(), Some('\n') | None) {
                    break;
                }
            }
            links.add_new(label, destination, title);
            current = iter.get_str();
            changed = true;
        }
        #[allow(clippy::assigning_clones)]
        // clone_into not possible because current is the reference to content
        if changed {
            self.content = current.to_owned();
        }
    }

    /// Gets last line of this paragraph
    pub fn get_last_line(&self) -> &str {
        // Safety: self.line_start is 0 at first (beginning of the string) and before each push it's
        // set to the end of the previous string, therefore it's always on a UTF-8 char boundary
        unsafe { self.content.get_unchecked(self.line_start..) }
    }

    /// Trims last line and the preceding new line character
    pub fn trim_last_line(&mut self) { self.content.truncate(self.line_start.saturating_sub(1)); }

    /// Finishes the paragraph into a [`Block`]. If the content is empty and the block would be a
    /// setext heading it becomes a paragraph with just the setext heading underline. An empty
    /// paragraph returns [`None`].
    pub fn finish(self, links: &Links) -> Option<Block> {
        if self.content.is_empty() {
            let char = match self.setext {
                0 => return None,
                1 => "=",
                2 => "-",
                _ => unreachable!(),
            };
            Some(Block::Para(vec![Inline::Str(char.repeat(self.setext_char_count))]))
        } else {
            let parsed = InlineParser::parse_lines(&self.content, links);
            Some(match self.setext {
                0 => Block::Para(parsed),
                _ => Block::new_header(self.setext, parsed),
            })
        }
    }

    /// Pushes a line checking if it's a setext heading underline first, if it's not it performs a
    /// full [`Table`] check - check if a table is created and if not check whether the new
    /// line can be a table header.
    fn push_check_setext(&mut self, line: SkipIndent) -> LineResult {
        let mut iter = line.iter_rest();
        iter.skip_while_eq('=');
        iter.skip_whitespace();
        if iter.ended() {
            self.setext = 1;
            LineResult::DoneSelf
        } else {
            self.push_full_check(line)
        }
    }

    /// Pushes a line performing a full [`Table`] check first - check if a table is created and if
    /// not check whether the new line can be a table header.
    fn push_full_check(&mut self, line: SkipIndent) -> LineResult {
        match Table::check(line, self) {
            NewResult::New(b) =>
                if self.line_start == 0 {
                    LineResult::New(b)
                } else {
                    LineResult::DoneSelfAndNew(b)
                },
            NewResult::Text(t) => {
                self.push(t.line);
                self.table_header_length = Table::check_header(t.line);
                LineResult::None
            },
        }
    }

    /// Pushes a line performing a partial [`Table`] check first - only check whether the new line
    /// can be a table header.
    fn push_header_check(&mut self, line: &SkipIndent) {
        self.push(line.line);
        self.table_header_length = Table::check_header(line.line);
    }

    /// Pushes a line performing a partial [`Table`] check first - only check whether the new line
    /// can be a table header. Only performs this check if the line isn't indented at all because
    /// of bizarre continuation line edge cases
    fn push_header_no_indent_check(&mut self, line: &SkipIndent) {
        self.push(line.line);
        self.table_header_length = if line.indent > 0 { 0 } else { Table::check_header(line.line) };
    }

    /// Pushes a line without any checks
    fn push(&mut self, line: &str) {
        self.content.push('\n');
        self.line_start = self.content.len();
        self.content.push_str(line.trim_end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_links<'a, I>(i: I) -> (Paragraph, Links)
    where I: IntoIterator<Item = &'a str> {
        let mut iter = i.into_iter();
        let mut paragraph = Paragraph::new(&SkipIndent::skip(iter.next().unwrap(), 0).into_line());
        for s in iter {
            assert!(matches!(
                paragraph.next(SkipIndent::skip(s, 0).into_line()),
                LineResult::None | LineResult::DoneSelf
            ));
        }
        let mut links = Links::new();
        paragraph.add_links(&mut links);
        (paragraph, links)
    }

    fn assert_links<'a, I>(i: I, paragraph: bool, links: usize)
    where I: IntoIterator<Item = &'a str> {
        let (p, l) = get_links(i);
        assert_eq!(p.finish(&l).is_some(), paragraph);
        assert_eq!(l.len(), links);
    }

    #[test]
    fn test_links() {
        assert_links(["[foo]: url 'test'"], false, 1);
        assert_links(["line", "[foo]: url 'test'"], true, 0);
        assert_links(["[foo[]]: url 'test'"], true, 0);
        assert_links([(String::from("[") + &"a".repeat(1000) + "]: url 'test'").as_str()], true, 0);
        assert_links(["[      ]: url 'test'"], true, 0);
        assert_links(["[foo]:url 'test'"], false, 1);
        assert_links(["[foo]:                           url 'test'"], false, 1);
        assert_links(["[foo] url 'test'"], true, 0);
        assert_links(["[foo]:"], true, 0);
        assert_links(["[foo]:", "url"], false, 1);
        assert_links(["[foo]:", "url with space"], true, 0);
        assert_links(["[foo]:", "url_with", "linebreak"], true, 1);
        assert_links(["[foo]: <>"], false, 1);
        assert_links(["[foo]: <url>"], false, 1);
        assert_links(["[foo]: <url with spaces>"], false, 1);
        assert_links(["[foo]: <url with <>>"], true, 0);
        assert_links(["[foo]: <url with \\<\\>>"], false, 1);
        assert_links(["[foo]: <url with", "linebreak>"], true, 0);
        assert_links(["[foo]: url \"quotes"], true, 0);
        assert_links(["[foo]: url \"quotes\""], false, 1);
        assert_links(["[foo]: url \"quo\"tes\""], true, 0);
        assert_links(["[foo]: url \"quo\\\"tes\""], false, 1);
        assert_links(["[foo]: url 'quotes"], true, 0);
        assert_links(["[foo]: url 'quotes'"], false, 1);
        assert_links(["[foo]: url 'quo'tes'"], true, 0);
        assert_links(["[foo]: url 'quo\\'tes'"], false, 1);
        assert_links(["[foo]: url (brackets"], true, 0);
        assert_links(["[foo]: url (brackets)"], false, 1);
        assert_links(["[foo]: url (brack()ets)"], true, 0);
        assert_links(["[foo]: url (brack\\(\\)ets)"], false, 1);
        assert_links(["[foo]:", "url", "'title'"], false, 1);
        assert_links(["[foo]:", "url", "'title", "title", "title'"], false, 1);
        assert_links(["[foo]: url 'title'", "[other]: url 'other'"], false, 2);
        assert_links(["[foo]: url 'title'", "[foo]: url 'other'"], false, 1);
        assert_links(["[foo]: url 'title'", "======"], true, 1);
        assert_links(["[foo]: url 'title'", "------"], true, 1);
    }
}
