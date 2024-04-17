use std::iter;
use std::iter::Peekable;
use std::str::Chars;

use crate::ast::{
    attr_empty, Alignment, Block, Caption, Cell, ColSpan, ColWidth, Pandoc, Row, RowHeadColumns,
    RowSpan, TableBody, TableFoot, TableHead,
};
use crate::inline_parser::InlineParser;

pub struct MdReader;

impl MdReader {
    pub fn parse(source: &str) -> Pandoc {
        let mut current = CurrentBlock::None;
        let mut result: Vec<Block> = Vec::new();
        for line in source.lines() {
            current.next_line(line, &mut result);
        }
        if let Some(new) = current.finish() {
            result.push(new);
        }
        Pandoc { blocks: result, ..Default::default() }
    }
}

pub enum CurrentBlock {
    None,
    Paragraph(Paragraph),
    IndentedCodeBlock(Vec<String>),
    FencedCodeBlock(FencedCodeBlock),
    Table(Table),
}

struct Paragraph {
    lines: Vec<String>,
    table_header_length: usize,
}

struct FencedCodeBlock {
    indent: usize,
    fence_size: usize,
    fence_char: char,
    info: String,
    content: String,
}

enum NextLineResult {
    Consumed,
    Started(CurrentBlock),
    Finished(Block),
    FinishedAndStarted(Block, CurrentBlock),
    FinishedTwo(Block, Block),
}

struct Table {
    size: usize,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}

enum SetextHeadingResult {
    Line,
    Setext,
    ThematicBreak,
}

impl CurrentBlock {
    fn next_line(&mut self, line: &str, blocks: &mut Vec<Block>) {
        let result = match self {
            CurrentBlock::None => Self::none_next(line),
            CurrentBlock::Paragraph(paragraph) => Self::paragraph_next(paragraph, line),
            CurrentBlock::IndentedCodeBlock(lines) => Self::indented_code_block_next(lines, line),
            CurrentBlock::FencedCodeBlock(code) => Self::fenced_code_block_next(code, line),
            CurrentBlock::Table(table) => Self::table_next(table, line),
        };
        match result {
            NextLineResult::Started(new) => *self = new,
            NextLineResult::Finished(block) => {
                blocks.push(block);
                *self = Self::None;
            },
            NextLineResult::FinishedAndStarted(block, new) => {
                blocks.push(block);
                *self = new;
            },
            NextLineResult::FinishedTwo(b1, b2) => {
                blocks.push(b1);
                blocks.push(b2);
                *self = Self::None;
            },
            NextLineResult::Consumed => {},
        }
    }

    fn finish(&mut self) -> Option<Block> {
        match self {
            CurrentBlock::None => None,
            CurrentBlock::Paragraph(paragraph) =>
                Some(Self::paragraph_finish(&mut paragraph.lines)),
            CurrentBlock::IndentedCodeBlock(lines) => Some(Self::indented_code_block_finish(lines)),
            CurrentBlock::FencedCodeBlock(code) => Some(Self::fenced_code_block_finish(code, true)),
            CurrentBlock::Table(table) => Some(Self::finish_table(table)),
        }
    }

    fn none_next(line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent >= 4 {
            match iter.clone().all(char::is_whitespace) {
                true => NextLineResult::Consumed,
                false => NextLineResult::Started(Self::start_indented_code_block(&mut iter)),
            }
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) =>
                    match Self::check_fenced_code_block(indent, c, &mut iter) {
                        Some(code) => NextLineResult::Started(code),
                        None => NextLineResult::Started(Self::start_paragraph(line)),
                    },
                Some(c @ ('*' | '_' | '-')) => match Self::check_thematic_break(c, &mut iter) {
                    true => NextLineResult::Finished(Block::HorizontalRule),
                    false => NextLineResult::Started(Self::start_paragraph(line)),
                },
                Some('#') => match Self::check_atx_heading(&mut iter) {
                    Some(heading) => NextLineResult::Finished(heading),
                    None => NextLineResult::Started(Self::start_paragraph(line)),
                },
                _ if !iter.all(char::is_whitespace) =>
                    NextLineResult::Started(Self::start_paragraph(line)),
                _ => NextLineResult::Consumed,
            }
        }
    }

    fn paragraph_next(paragraph: &mut Paragraph, line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line.trim_end(), 4);
        if indent >= 4 {
            paragraph.lines.push(line.to_owned());
            paragraph.table_header_length = 0;
            NextLineResult::Consumed
        } else {
            match iter.next() {
                Some('=') => match Self::check_setext_heading(&mut iter) {
                    true => NextLineResult::Finished(Self::setext_finish(1, &mut paragraph.lines)),
                    false => {
                        paragraph.lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                },
                Some('-') => match Self::check_setext_heading_or_thematic_break(&mut iter) {
                    SetextHeadingResult::Line => {
                        paragraph.lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                    SetextHeadingResult::ThematicBreak => NextLineResult::FinishedTwo(
                        Self::paragraph_finish(&mut paragraph.lines),
                        Block::HorizontalRule,
                    ),
                    SetextHeadingResult::Setext =>
                        NextLineResult::Finished(Self::setext_finish(2, &mut paragraph.lines)),
                },
                Some(c @ ('*' | '_')) => match Self::check_thematic_break(c, &mut iter) {
                    true => NextLineResult::FinishedTwo(
                        Self::paragraph_finish(&mut paragraph.lines),
                        Block::HorizontalRule,
                    ),
                    false => {
                        paragraph.lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                },
                Some(c @ ('~' | '`')) =>
                    match Self::check_fenced_code_block(indent, c, &mut iter) {
                        Some(code) => NextLineResult::FinishedAndStarted(
                            Self::paragraph_finish(&mut paragraph.lines),
                            code,
                        ),
                        None => {
                            paragraph.lines.push(line.to_owned());
                            NextLineResult::Consumed
                        },
                    },
                Some('#') => match Self::check_atx_heading(&mut iter) {
                    Some(heading) => NextLineResult::FinishedTwo(
                        Self::paragraph_finish(&mut paragraph.lines),
                        heading,
                    ),
                    None => {
                        paragraph.lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                },
                _ if !iter.all(char::is_whitespace) => {
                    if paragraph.table_header_length != 0
                        && let Some(alignments) =
                            Self::check_table_delimiter(line, paragraph.table_header_length)
                    {
                        let table = CurrentBlock::Table(Table {
                            size: paragraph.table_header_length,
                            alignments,
                            rows: vec![Self::split_rows(
                                &paragraph.lines.pop().unwrap(),
                                paragraph.table_header_length,
                            )],
                        });
                        match paragraph.lines.is_empty() {
                            true => NextLineResult::Started(table),
                            false => NextLineResult::FinishedAndStarted(
                                Self::paragraph_finish(&paragraph.lines),
                                table,
                            ),
                        }
                    } else {
                        paragraph.lines.push(line.to_owned());
                        paragraph.table_header_length = Self::check_table_header(line);
                        NextLineResult::Consumed
                    }
                },
                _ => NextLineResult::Finished(Self::paragraph_finish(&mut paragraph.lines)),
            }
        }
    }

    fn split_rows(line: &str, columns: usize) -> Vec<String> {
        let mut iter = line.trim().chars().peekable();
        iter.next_if_eq(&'|');
        let mut result = Vec::new();
        let mut current = String::new();
        loop {
            match iter.next() {
                Some('\\') => current.push(iter.next_if_eq(&'|').unwrap_or('\\')),
                Some('|') | None => {
                    result.push(current);
                    if result.len() == columns {
                        return result;
                    }
                    current = String::new();
                },
                Some(c) => current.push(c),
            };
        }
    }

    fn indented_code_block_next(lines: &mut Vec<String>, line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent >= 4 {
            lines.push(iter.collect());
            NextLineResult::Consumed
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) =>
                    match Self::check_fenced_code_block(indent, c, &mut iter) {
                        Some(code) => NextLineResult::FinishedAndStarted(
                            Self::indented_code_block_finish(lines),
                            code,
                        ),
                        None => NextLineResult::FinishedAndStarted(
                            Self::indented_code_block_finish(lines),
                            Self::start_paragraph(line),
                        ),
                    },
                Some(c @ ('*' | '_' | '-')) => match Self::check_thematic_break(c, &mut iter) {
                    true => NextLineResult::FinishedTwo(
                        Self::indented_code_block_finish(lines),
                        Block::HorizontalRule,
                    ),
                    false => NextLineResult::FinishedAndStarted(
                        Self::indented_code_block_finish(lines),
                        Self::start_paragraph(line),
                    ),
                },
                Some('#') => match Self::check_atx_heading(&mut iter) {
                    Some(heading) => NextLineResult::FinishedTwo(
                        Self::indented_code_block_finish(lines),
                        heading,
                    ),
                    None => NextLineResult::FinishedAndStarted(
                        Self::indented_code_block_finish(lines),
                        Self::start_paragraph(line),
                    ),
                },
                Some(_) => NextLineResult::FinishedAndStarted(
                    Self::indented_code_block_finish(lines),
                    Self::start_paragraph(line),
                ),
                _ => {
                    lines.push(iter.collect());
                    NextLineResult::Consumed
                },
            }
        }
    }

    fn fenced_code_block_next(code: &mut FencedCodeBlock, line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent <= 3 {
            let mut count = 0;
            while let Some(c) = iter.next()
                && c == code.fence_char
            {
                count += 1;
            }
            if count >= code.fence_size {
                loop {
                    match iter.next() {
                        Some(' ' | '\t') => continue,
                        Some(_) => break,
                        None =>
                            return NextLineResult::Finished(Self::fenced_code_block_finish(
                                code, false,
                            )),
                    }
                }
            }
        }
        if code.indent > 0 {
            let (_, iter) = Self::skip_indent(line, code.indent);
            for c in iter {
                code.content.push(c);
            }
        } else {
            code.content.push_str(line);
        }
        code.content.push('\n');
        NextLineResult::Consumed
    }

    fn table_next(table: &mut Table, line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent >= 4 {
            match iter.clone().all(char::is_whitespace) {
                false => {
                    table.rows.push(Self::split_rows(line, table.size));
                    NextLineResult::Consumed
                },
                true => NextLineResult::Finished(Self::finish_table(table)),
            }
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) =>
                    match Self::check_fenced_code_block(indent, c, &mut iter) {
                        Some(code) =>
                            NextLineResult::FinishedAndStarted(Self::finish_table(table), code),
                        None => {
                            table.rows.push(Self::split_rows(line, table.size));
                            NextLineResult::Consumed
                        },
                    },
                Some(c @ ('*' | '_' | '-')) => match Self::check_thematic_break(c, &mut iter) {
                    true => NextLineResult::FinishedTwo(
                        Self::finish_table(table),
                        Block::HorizontalRule,
                    ),
                    false => {
                        table.rows.push(Self::split_rows(line, table.size));
                        NextLineResult::Consumed
                    },
                },
                Some('#') => match Self::check_atx_heading(&mut iter) {
                    Some(heading) =>
                        NextLineResult::FinishedTwo(Self::finish_table(table), heading),
                    None => {
                        table.rows.push(Self::split_rows(line, table.size));
                        NextLineResult::Consumed
                    },
                },
                _ if !iter.all(char::is_whitespace) => {
                    table.rows.push(Self::split_rows(line, table.size));
                    NextLineResult::Consumed
                },
                _ => NextLineResult::Finished(Self::finish_table(table)),
            }
        }
    }

    fn paragraph_finish(lines: &[String]) -> Block { Block::Para(InlineParser::parse_lines(lines)) }

    fn make_cell(s: String) -> Cell {
        let inline = InlineParser::parse_line(&s);
        Cell(
            attr_empty(),
            Alignment::Default,
            RowSpan(1),
            ColSpan(1),
            if inline.is_empty() {
                Vec::new()
            } else {
                vec![Block::Plain(InlineParser::parse_line(&s))]
            },
        )
    }

    fn finish_table(table: &mut Table) -> Block {
        let rows = std::mem::replace(&mut table.rows, Vec::new());
        let mut iter = rows.into_iter();
        Block::Table(
            attr_empty(),
            Caption(None, Vec::new()),
            table.alignments.iter().map(|a| (a.clone(), ColWidth::ColWidthDefault)).collect(),
            TableHead(attr_empty(), vec![Row(
                attr_empty(),
                iter.next().unwrap().into_iter().map(Self::make_cell).collect(),
            )]),
            vec![TableBody(
                attr_empty(),
                RowHeadColumns(0),
                Vec::new(),
                iter.map(|r| {
                    let rest = table.size - r.len();
                    Row(
                        attr_empty(),
                        r.into_iter()
                            .map(Self::make_cell)
                            .chain(
                                iter::repeat_with(|| {
                                    Cell(
                                        attr_empty(),
                                        Alignment::Default,
                                        RowSpan(1),
                                        ColSpan(1),
                                        Vec::new(),
                                    )
                                })
                                .take(rest),
                            )
                            .collect(),
                    )
                })
                .collect(),
            )],
            TableFoot(attr_empty(), Vec::new()),
        )
    }

    fn setext_finish(level: i32, lines: &[String]) -> Block {
        Block::Header(level, attr_empty(), InlineParser::parse_lines(lines))
    }

    fn indented_code_block_finish(lines: &mut Vec<String>) -> Block {
        while let Some(last) = lines.last()
            && last.chars().all(char::is_whitespace)
        {
            lines.pop();
        }
        let mut result = String::new();
        for l in lines {
            result.push_str(&l);
            result.push('\n');
        }
        result.pop();
        Block::CodeBlock(attr_empty(), result)
    }

    fn fenced_code_block_finish(code: &mut FencedCodeBlock, force_new_line: bool) -> Block {
        // todo fix this temp solution (String::new() doesn't allocate but still)
        let mut info = std::mem::replace(&mut code.info, String::new());
        let mut content = std::mem::replace(&mut code.content, String::new());
        if !force_new_line {
            content.pop();
        }
        if let Some(n) = info.find(' ') {
            info.truncate(n)
        }
        let info = if info.is_empty() { Vec::new() } else { vec![info] };
        Block::CodeBlock(("".into(), info, Vec::new()), content)
    }

    fn check_thematic_break(first: char, rest: &mut Peekable<Chars>) -> bool {
        let mut count = 1;
        for c in rest {
            match c {
                ' ' | '\t' => continue,
                c if c == first => count += 1,
                _ => return false,
            }
        }
        count >= 3
    }

    fn check_atx_heading(rest: &mut Peekable<Chars>) -> Option<Block> {
        let mut count = 1;
        loop {
            match rest.next() {
                Some('#') if count < 6 => count += 1,
                Some(' ') => break,
                None => return Some(Block::Header(count, attr_empty(), Vec::new())),
                _ => return None,
            }
        }
        let mut result: String = rest.collect();
        let trimmed = result.trim_end().trim_end_matches('#');
        if matches!(trimmed.chars().next_back(), None | Some(' ')) {
            result.truncate(trimmed.len().saturating_sub(1))
        }
        Some(Block::Header(count, attr_empty(), InlineParser::parse_line(&result)))
    }

    fn check_setext_heading(rest: &mut Peekable<Chars>) -> bool {
        let mut whitespace = false;
        loop {
            match rest.next() {
                Some('=') if !whitespace => continue,
                Some(' ' | '\t') => whitespace = true,
                Some(_) => return false,
                None => return true,
            }
        }
    }

    fn check_setext_heading_or_thematic_break(rest: &mut Peekable<Chars>) -> SetextHeadingResult {
        let mut count = 1;
        let mut whitespace = false;
        let mut thematic = false;
        loop {
            match rest.next() {
                Some('-') => {
                    count += 1;
                    if whitespace {
                        thematic = true;
                    }
                },
                Some(' ' | '\t') => whitespace = true,
                Some(_) => return SetextHeadingResult::Line,
                None =>
                    return if thematic && count >= 3 {
                        SetextHeadingResult::ThematicBreak
                    } else {
                        SetextHeadingResult::Setext
                    },
            }
        }
    }

    fn check_fenced_code_block(
        indent: usize, first: char, rest: &mut Peekable<Chars>,
    ) -> Option<CurrentBlock> {
        let mut count = 1;
        while rest.next_if_eq(&first).is_some() {
            count += 1;
        }
        if count < 3 {
            return None;
        }
        while matches!(rest.peek(), Some(' ' | '\t')) {
            rest.next();
        }
        let mut info: String = rest.collect();
        info.truncate(info.trim_end().len());
        if first == '`' && info.contains('`') {
            return None;
        }
        Some(CurrentBlock::FencedCodeBlock(FencedCodeBlock {
            indent,
            fence_char: first,
            fence_size: count,
            info,
            content: String::new(),
        }))
    }

    fn start_paragraph(line: &str) -> CurrentBlock {
        CurrentBlock::Paragraph(Paragraph {
            lines: vec![line.to_owned()],
            table_header_length: Self::check_table_header(line),
        })
    }

    fn start_indented_code_block(rest: &mut Peekable<Chars>) -> CurrentBlock {
        Self::IndentedCodeBlock(vec![rest.collect()])
    }

    pub fn check_table_header(line: &str) -> usize {
        let mut iter = line.trim().chars();
        let (mut count, mut escape, mut first) = match iter.next() {
            Some('\\') => (0, true, false),
            Some('|') => (1, false, true),
            _ => (0, false, false),
        };
        let mut previous = false;
        for c in iter {
            if c == '\\' && !escape {
                escape = true;
                previous = false;
            } else if c == '|' && !escape {
                count += 1;
                previous = true;
                escape = false;
            } else {
                escape = false;
                previous = false;
            }
        }
        if count == 0 {
            0
        } else {
            count + 1 - previous as usize - first as usize
        }
    }

    pub fn check_table_delimiter(line: &str, size: usize) -> Option<Vec<Alignment>> {
        let mut iter = line.trim().chars().peekable();
        iter.next_if_eq(&'|');
        let mut result = Vec::new();
        for i in 0..size {
            loop {
                if iter.next_if(|c| matches!(c, ' ' | '\t')).is_none() {
                    break;
                }
            }
            let left = iter.next_if_eq(&':').is_some();
            iter.next_if_eq(&'-')?;
            loop {
                if iter.next_if_eq(&'-').is_none() {
                    break;
                }
            }
            let right = iter.next_if_eq(&':').is_some();
            loop {
                if iter.next_if(|c| matches!(c, ' ' | '\t')).is_none() {
                    break;
                }
            }
            if iter.next_if_eq(&'|').is_none() && i != size - 1 {
                return None;
            }
            result.push(match (left, right) {
                (false, false) => Alignment::Default,
                (false, true) => Alignment::Right,
                (true, false) => Alignment::Left,
                (true, true) => Alignment::Center,
            });
        }
        iter.next().is_none().then_some(result)
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
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::process::{Command, Stdio};

    use super::*;
    use crate::ast::*;

    fn test(examples: Vec<&str>, offset: usize) {
        let mut results = Vec::new();
        for (i, e) in examples.into_iter().enumerate() {
            let mut child = Command::new("pandoc")
                .args(["-f", "gfm", "-t", "json"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.as_mut().unwrap().write_all(e.as_bytes()).unwrap();
            let number = i + offset;
            let expected = if number == 68 {
                Pandoc {
                    blocks: vec![Block::HorizontalRule, Block::HorizontalRule],
                    ..Default::default()
                }
            } else {
                serde_json::from_str(
                    std::str::from_utf8(&child.wait_with_output().unwrap().stdout).unwrap(),
                )
                .unwrap()
            };
            let result = MdReader::parse(e);
            if result.blocks == expected.blocks {
                println!("\x1b[32mExample {} : success", number);
                println!("Input:\n{}", e);
                println!("Output:\n{:?}", result);
            } else {
                println!("\x1b[31mExample {} : failure", number);
                println!("Input:\n{}", e);
                println!("Output:\n{:?}", result);
                println!("Expected: \n{:?}", expected);
                results.push(number)
            }
        }
        if !results.is_empty() {
            panic!("Tests {:?} failed", results)
        }
    }

    #[test]
    fn test_thematic_break() {
        test(
            vec![
                "***\n---\n___", "+++", "===", "--\n**\n__", " ***\n  ***\n   ***", "    ***",
                "Foo\n    ***", "_____________________________________", " - - -",
                " **  * ** * ** * **", "-     -      -      -", "- - - -    ",
                "_ _ _ _ a\n\na------\n\n---a---", " *-*", "- foo\n***\n- bar", "Foo\n***\nbar",
                "Foo\n---\nbar", "* Foo\n* * *\n* Bar", "- Foo\n- * * *",
            ],
            13,
        );
    }

    #[test]
    fn test_atx_header() {
        test(
            vec![
                "# foo\n## foo\n### foo\n#### foo\n##### foo\n###### foo", "####### foo",
                "#5 bolt\n\n#hashtag", "\\## *bar* \\*baz\\*",
                "#                  foo                     ", " ### foo\n  ## foo\n   # foo",
                "    # foo", "foo\n    # bar", "## foo ##\n  ###   bar    ###",
                "# foo ##################################\n##### foo ##", "### foo ###     ",
                "### foo ### b", "# foo#", "### foo \\###\n## foo #\\##\n# foo \\#",
                "****\n## foo\n****", "Foo bar\n# baz\nBar foo", "## \n#\n### ###",
            ],
            32,
        );
    }

    #[test]
    fn test_setext_header() {
        test(
            vec![
                "Foo *bar*\n=========\n\nFoo *bar*\n---------", "Foo *bar\nbaz*\n====",
                "  Foo *bar\nbaz*\t\n====", "Foo\n-------------------------\n\nFoo\n=",
                "   Foo\n---\n\n  Foo\n-----\n\n  Foo\n  ===", "    Foo\n    ---\n\n    Foo\n---",
                "Foo\n   ----      ", "Foo\n    ---", "Foo\n= =\n\nFoo\n--- -", "Foo  \n-----",
                "Foo\\\n----", "`Foo\n----\n`\n\n<a title=\"a lot\n---\nof dashes\"/>",
                "> Foo\n---", "> foo\nbar\n===", "- Foo\n---", "Foo\nBar\n---",
                "---\nFoo\n---\nBar\n---\nBaz", "\n====", "---\n---", "- foo\n-----",
                "    foo\n---", "> foo\n-----", "\\> foo\n------", "Foo\n\nbar\n---\nbaz",
                "Foo\nbar\n\n---\n\nbaz", "Foo\nbar\n* * *\nbaz", "Foo\nbar\n\\---\nbaz",
            ],
            50,
        )
    }

    #[test]
    fn test_indented_code_block() {
        test(
            vec![
                "    a simple\n      indented code block", "  - foo\n\n    bar",
                "1.  foo\n\n    - bar", "    <a/>\n    *hi*\n\n    - one",
                "    chunk1\n\n    chunk2\n\n\n    chunk3", "    chunk1\n      \n      chunk2",
                "Foo\n    bar", "    foo\nbar",
                "# Heading\n    foo\nHeading\n------\n    foo\n----", "        foo\n    bar",
                "    \n    foo\n    ", "    foo  ",
            ],
            77,
        )
    }

    #[test]
    fn test_fenced_code_block() {
        test(
            vec![
                "```\n<\n >\n```", "~~~\n<\n >\n~~~", "``\nfoo\n``", "```\naaa\n~~~\n```",
                "~~~\n\naaa\n```\n~~~", "````\naaa\n```\n``````", "~~~~\naaa\n~~~\n~~~~", "```",
                "`````\n\n```\naaa", "> ```\n> aaa\n\nbbb", "```\n\n  \n```", "```\n```",
                " ```\n aaa\naaa\n```", "  ```\naaa\n  aaa\naaa\n  ```",
                "   ```\n   aaa\n    aaa\n  aaa\n   ```", "    ```\n    aaa\n    ```",
                "```\naaa\n  ```", "   ```\naaa\n  ```", "```\naaa\n    ```", "``` ```\naaa",
                "~~~~~~\naaa\n~~~ ~~", "foo\n```\nbar\n```\nbaz", "foo\n---\n~~~\nbar\n~~~\n# baz",
                "```ruby\ndef foo(x)\n  return 3\nend\n```",
                "~~~~    ruby startline=3 $%@#$\ndef foo(x)\n  return 3\nend\n~~~~~~~",
                "````;\n````", "``` aa ```\nfoo", "~~~ aa ``` ~~~\nfoo\n~~~", "```\n``` aaa\n```",
            ],
            89,
        )
    }

    #[test]
    fn test_table() {
        test(
            vec![
                "| foo | bar |\n| --- | --- |\n| baz | bim |",
                "| abc | defghi |\n:-: | -----------:\nbar | baz",
                "| f\\|oo  |\n| ------ |\n| b `\\|` az |\n| b **\\|** im |",
                "| abc | def |\n| --- | --- |\n| bar | baz |\n> bar",
                "| abc | def |\n| --- | --- |\n| bar | baz |\nbar\n\nbar",
                "| abc | def |\n| --- |\n| bar |",
                "| abc | def |\n| --- | --- |\n| bar |\n| bar | baz | boo |",
                "| abc | def |\n| --- | --- |",
            ],
            198,
        )
    }
}
