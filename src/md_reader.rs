use std::iter::Peekable;
use std::str::Chars;

use crate::ast::{attr_empty, Block, Pandoc};
use crate::inline_parser::InlineParser;

pub struct MdReader;

impl MdReader {
    fn parse(source: &str) -> Pandoc {
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

enum CurrentBlock {
    None,
    Paragraph(Vec<String>),
    IndentedCodeBlock(Vec<String>),
}

enum NextLineResult {
    Consumed,
    Started(CurrentBlock),
    Finished(Block),
    FinishedAndStarted(Block, CurrentBlock),
    FinishedTwo(Block, Block),
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
            CurrentBlock::Paragraph(lines) => Self::paragraph_next(lines, line),
            CurrentBlock::IndentedCodeBlock(lines) => Self::indented_code_block_next(lines, line),
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
            CurrentBlock::Paragraph(lines) => Some(Self::paragraph_finish(lines)),
            CurrentBlock::IndentedCodeBlock(lines) => Some(Self::indented_code_block_finish(lines)),
        }
    }

    fn none_next(line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent >= 4 {
            if !iter.clone().all(char::is_whitespace) {
                NextLineResult::Started(Self::start_indented_code_block(&mut iter))
            } else {
                NextLineResult::Consumed
            }
        } else {
            match iter.next() {
                Some(c @ ('*' | '_' | '-')) =>
                    if Self::check_thematic_break(c, &mut iter) {
                        NextLineResult::Finished(Block::HorizontalRule)
                    } else {
                        NextLineResult::Started(Self::start_paragraph(line))
                    },
                Some('#') =>
                    if let Some(heading) = Self::check_atx_heading(&mut iter) {
                        NextLineResult::Finished(heading)
                    } else {
                        NextLineResult::Started(Self::start_paragraph(line))
                    },
                _ if !iter.all(char::is_whitespace) =>
                    NextLineResult::Started(Self::start_paragraph(line)),
                _ => NextLineResult::Consumed,
            }
        }
    }

    fn paragraph_next(lines: &mut Vec<String>, line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent >= 4 {
            lines.push(line.to_owned());
            NextLineResult::Consumed
        } else {
            match iter.next() {
                Some('=') =>
                    if Self::check_setext_heading(&mut iter) {
                        NextLineResult::Finished(Self::setext_finish(1, lines))
                    } else {
                        lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                Some('-') => match Self::check_setext_heading_or_thematic_break(&mut iter) {
                    SetextHeadingResult::Line => {
                        lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                    SetextHeadingResult::ThematicBreak => NextLineResult::FinishedTwo(
                        Self::paragraph_finish(lines),
                        Block::HorizontalRule,
                    ),
                    SetextHeadingResult::Setext =>
                        NextLineResult::Finished(Self::setext_finish(2, lines)),
                },
                Some(c @ ('*' | '_')) =>
                    if Self::check_thematic_break(c, &mut iter) {
                        NextLineResult::FinishedTwo(
                            Self::paragraph_finish(lines),
                            Block::HorizontalRule,
                        )
                    } else {
                        lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                Some('#') =>
                    if let Some(heading) = Self::check_atx_heading(&mut iter) {
                        NextLineResult::FinishedTwo(Self::paragraph_finish(lines), heading)
                    } else {
                        lines.push(line.to_owned());
                        NextLineResult::Consumed
                    },
                _ if !iter.all(char::is_whitespace) => {
                    lines.push(line.to_owned());
                    NextLineResult::Consumed
                },
                _ => NextLineResult::Finished(Self::paragraph_finish(lines)),
            }
        }
    }

    fn indented_code_block_next(lines: &mut Vec<String>, line: &str) -> NextLineResult {
        let (indent, mut iter) = Self::skip_indent(line, 4);
        if indent >= 4 {
            lines.push(iter.collect());
            NextLineResult::Consumed
        } else {
            match iter.next() {
                Some(c @ ('*' | '_' | '-')) =>
                    if Self::check_thematic_break(c, &mut iter) {
                        NextLineResult::FinishedTwo(
                            Self::indented_code_block_finish(lines),
                            Block::HorizontalRule,
                        )
                    } else {
                        NextLineResult::FinishedAndStarted(
                            Self::indented_code_block_finish(lines),
                            Self::start_paragraph(line),
                        )
                    },
                Some('#') =>
                    if let Some(heading) = Self::check_atx_heading(&mut iter) {
                        NextLineResult::FinishedTwo(
                            Self::indented_code_block_finish(lines),
                            heading,
                        )
                    } else {
                        NextLineResult::FinishedAndStarted(
                            Self::indented_code_block_finish(lines),
                            Self::start_paragraph(line),
                        )
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

    fn paragraph_finish(lines: &[String]) -> Block { Block::Para(InlineParser::parse_lines(lines)) }

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
                Some('#') if count <= 5 => count += 1,
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

    fn start_paragraph(line: &str) -> CurrentBlock { Self::Paragraph(vec![line.to_owned()]) }

    fn start_indented_code_block(rest: &mut Peekable<Chars>) -> CurrentBlock {
        Self::IndentedCodeBlock(vec![rest.collect()])
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
}
