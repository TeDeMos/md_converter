use std::iter;
use std::iter::Peekable;
use std::str::Chars;

use crate::ast::{attr_empty, Block, Meta, Pandoc};
use crate::inline_parser::InlineParser;
use crate::traits::AstReader;

pub struct MdReader {
    source: String,
}

impl AstReader for MdReader {
    type ReadError = ();

    fn read(source: String) -> Result<Pandoc, Self::ReadError> { Self { source }.parse() }
}

impl MdReader {
    fn parse(&mut self) -> Result<Pandoc, <Self as AstReader>::ReadError> {
        let mut temp: Option<Box<dyn TempBlock>> = None;
        let mut result: Vec<Block> = Vec::new();
        for line in self.source.lines().chain(iter::once("")) {
            match temp.as_mut() {
                Some(b) => match b.check_line(line) {
                    LineResult::Consumed => continue,
                    LineResult::BlockFinished(b) => result.push(b),
                    LineResult::BlockSplit(finished, current) => {
                        if let Some(b) = finished {
                            result.push(b);
                        }
                        temp = Some(current);
                    },
                },
                _ => {},
            }
            if let Some(b) = ThematicBreak::begin(line) {
                temp = Some(Box::new(b));
            } else if let Some(ah) = AtxHeading::begin(line) {
                temp = Some(Box::new(ah));
            } else if let Some(p) = Paragraph::begin(line) {
                temp = Some(Box::new(p));
            }
        }
        Ok(Pandoc { meta: Meta::default(), blocks: result })
    }
}

enum LineResult {
    Consumed,
    BlockFinished(Block),
    BlockSplit(Option<Block>, Box<dyn TempBlock>),
}

trait TempBlock {
    fn begin(line: &str) -> Option<Self>
    where Self: Sized;
    fn check_line(&mut self, line: &str) -> LineResult;
}

fn skip_indent(line: &str) -> (i32, Peekable<Chars>) {
    let mut iter = line.chars().peekable();
    let mut indent = 0;
    loop {
        match iter.peek() {
            Some('\t') => indent = (indent + 4) / 4,
            Some(' ') => indent += 1,
            _ => return (indent, iter),
        }
        iter.next();
    }
}

struct ThematicBreak;

impl TempBlock for ThematicBreak {
    fn begin(line: &str) -> Option<Self> {
        let (indent, mut iter) = skip_indent(line);
        if indent >= 4 {
            return None;
        }
        let thematic_char = match iter.next() {
            Some(c @ ('*' | '-' | '_')) => c,
            _ => return None,
        };
        let mut count = 1;
        for c in iter {
            if c == ' ' || c == '\t' {
                continue;
            } else if c == thematic_char {
                count += 1;
            } else {
                return None;
            }
        }
        if count >= 3 {
            Some(Self)
        } else {
            None
        }
    }

    fn check_line(&mut self, _: &str) -> LineResult {
        LineResult::BlockFinished(Block::HorizontalRule)
    }
}

struct AtxHeading(i32, String);

impl AtxHeading {
    fn take(&mut self) -> String { std::mem::replace(&mut self.1, String::new()) }
}

impl TempBlock for AtxHeading {
    fn begin(line: &str) -> Option<Self>
    where Self: Sized {
        let (indent, mut iter) = skip_indent(line);
        if indent >= 4 {
            return None;
        }
        let mut count = 0;
        while let Some(&'#') = iter.peek() {
            iter.next();
            count += 1;
        }
        if !((1..=6).contains(&count) && matches!(iter.next(), None | Some(' '))) {
            return None;
        }
        while matches!(iter.peek(), Some(' ' | '\t')) {
            iter.next();
        }
        let mut rest: String = iter.collect();
        rest.truncate(rest.trim_end().len());
        let mut trailing = 0;
        let mut rev = rest.chars().rev().peekable();
        while rev.next_if_eq(&'#').is_some() {
            trailing += 1;
        }
        if trailing >= 1 && matches!(rev.peek(), Some(&' ') | None) {
            rest.truncate((rest.len() - trailing).saturating_sub(1));
        }
        Some(Self(count, rest))
    }

    fn check_line(&mut self, _: &str) -> LineResult {
        LineResult::BlockFinished(Block::Header(
            self.0,
            attr_empty(),
            InlineParser::parse(vec![self.take()]),
        ))
    }
}

struct Paragraph(Vec<String>);

impl Paragraph {
    fn take(&mut self) -> Vec<String> { std::mem::replace(&mut self.0, vec![]) }
}

impl TempBlock for Paragraph {
    fn begin(line: &str) -> Option<Self> {
        if line.chars().any(|c| c != ' ' && c != '\t') {
            Some(Paragraph(vec![line.to_owned()]))
        } else {
            None
        }
    }

    fn check_line(&mut self, line: &str) -> LineResult {
        if let Some(t) = ThematicBreak::begin(line) {
            LineResult::BlockSplit(Some(Block::Para(InlineParser::parse(self.take()))), Box::new(t))
        } else if let Some(ah) = AtxHeading::begin(line) {
            LineResult::BlockSplit(Some(Block::Para(InlineParser::parse(self.take()))), Box::new(ah))
        } else if line.chars().any(|c| c != ' ' && c != '\t') {
            self.0.push(line.to_owned());
            LineResult::Consumed
        } else {
            LineResult::BlockFinished(Block::Para(InlineParser::parse(self.take())))
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
            let expected: Pandoc = serde_json::from_str(
                std::str::from_utf8(&child.wait_with_output().unwrap().stdout).unwrap(),
            )
            .unwrap();
            let result = MdReader::read(e.into()).unwrap();
            let number = i + offset;
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
}
