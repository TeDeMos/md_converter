use crate::ast::{Block, Inline};
use crate::inline_parser::InlineParser;
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
    /// Length of the table header defined by the last line, 0 if last line is not a table header
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

    /// Parses next non-blank line of a document, returning a [`LineResult`]
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

    /// Parses non-blank line of a document as a continuation line indented at most 3 spaces
    pub fn next_continuation(&mut self, line: SkipIndent) -> LineResult {
        TempBlock::check_block_known_indent(line).into_line_result(true, |s| {
            self.push_header_no_indent_check(&s);
            LineResult::None
        })
    }

    /// Parses non-blank line of a document as a continuation line indented at least 4 spaces
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
            let Some(label) = iter.get_str_until_unescaped(']') else { break };
            if label.len() > 999 || label.trim().is_empty() {
                break;
            }
            if !iter.next_if_eq(':') {
                break;
            }
            iter.skip_whitespace_new_line();
            let Some(destination) = iter.get_link_destination() else { break };
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
    pub fn finish(self) -> Option<Block> {
        if self.content.is_empty() {
            let char = match self.setext {
                0 => return None,
                1 => "=",
                2 => "-",
                _ => unreachable!(),
            };
            Some(Block::Para(vec![Inline::Str(char.repeat(self.setext_char_count))]))
        } else {
            let parsed = InlineParser::parse_lines(&self.content);
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
        let mut whitespace = false;
        let mut iter = line.line.chars();
        loop {
            match iter.next() {
                Some('=') if !whitespace => continue,
                Some(' ' | '\t') => whitespace = true,
                Some(_) => return self.push_full_check(line),
                None => {
                    self.setext = 1;
                    return LineResult::DoneSelf;
                },
            }
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
