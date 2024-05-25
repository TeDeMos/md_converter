use crate::ast::Block;
use crate::inline_parser::InlineParser;
use crate::md_reader::temp_block::{
    AtxHeading, BlockQuote, CheckOrSetextResult, CheckResult, FencedCodeBlock, LineResult, List,
    NewResult, SkipIndent, Table, TempBlock, ThematicBreak,
};

#[derive(Debug)]
pub struct Paragraph {
    pub lines: Vec<String>,
    pub table_header_length: usize,
    setext: usize,
}

impl Paragraph {
    pub fn new(line: &SkipIndent) -> Self {
        Self {
            lines: vec![line.line.to_owned()],
            table_header_length: Table::check_header(line.line),
            setext: 0,
        }
    }

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
                    CheckOrSetextResult::Setext => {
                        self.setext = 2;
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

    pub fn next_continuation(&mut self, line: SkipIndent) -> LineResult {
        TempBlock::check_block_known_indent(line).into_line_result(true, |s| {
            self.push_header_no_indent_check(&s);
            LineResult::None
        })
    }

    pub fn next_indented_continuation(&mut self, line: &SkipIndent) { self.push_no_checks(line); }

    pub fn finish(self) -> Block {
        let parsed = InlineParser::parse_lines(&self.lines);
        match self.setext {
            0 => Block::Para(parsed),
            _ => Block::new_header(self.setext, parsed),
        }
    }

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

    fn push_full_check(&mut self, line: SkipIndent) -> LineResult {
        match Table::check(line, self) {
            NewResult::New(b) =>
                if self.lines.is_empty() {
                    LineResult::New(b)
                } else {
                    LineResult::DoneSelfAndNew(b)
                },
            NewResult::Text(t) => {
                self.lines.push(t.line.to_owned());
                self.table_header_length = Table::check_header(t.line);
                LineResult::None
            },
        }
    }

    fn push_header_check(&mut self, line: &SkipIndent) {
        self.lines.push(line.line.to_owned());
        self.table_header_length = Table::check_header(line.line);
    }

    fn push_header_no_indent_check(&mut self, line: &SkipIndent) {
        self.lines.push(line.line.to_owned());
        self.table_header_length = if line.indent > 0 { 0 } else { Table::check_header(line.line) };
    }

    fn push_no_checks(&mut self, line: &SkipIndent) { self.lines.push(line.line.to_owned()); }
}
