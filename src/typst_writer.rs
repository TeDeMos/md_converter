use std::error::Error;

use derive_more::Display;

use crate::ast::{Block, ColSpec, Inline, Pandoc, Row, TableBody, TableHead};
use crate::traits::AstWriter;

#[derive(Default)]
pub struct TypstWriter {
    result: String,
    enum_level: usize,
}

impl TypstWriter {
    pub fn new() -> Self { Self { result: String::new(), enum_level: 0 } }
}

impl AstWriter for TypstWriter {
    type WriteError = WriteError;

    fn write(mut self, ast: Pandoc) -> Result<String, Self::WriteError> {
        self.push_str("#document()\n");
        self.write_blocks(ast.blocks)?;
        Ok(self.result)
    }
}

#[derive(Debug, Display)]
pub enum WriteError {
    NotImplemented(&'static str),
}

impl Error for WriteError {}

impl TypstWriter {
    fn push_str(&mut self, str: &str) { self.result.push_str(str) }

    fn push(&mut self, c: char) { self.result.push(c) }

    fn write_blocks(&mut self, blocks: Vec<Block>) -> Result<(), WriteError> {
        for b in blocks {
            self.write_block(b)?;
        }
        Ok(())
    }

    fn write_block(&mut self, block: Block) -> Result<(), WriteError> {
        match block {
            Block::Plain(p) | Block::Para(p) => {
                self.push('\n');
                self.write_inlines(p)?;
                self.push('\n');
            },
            Block::CodeBlock((l, ..), t) => self.write_code_block(&l, &t),
            Block::BlockQuote(b) => {
                self.push_str("\n> ");
                self.write_blocks(b)?;
                self.push('\n');
            },
            Block::OrderedList((s, ..), items) => {
                self.enum_level += 1;
                self.write_ordered_list(s, items)?;
                self.enum_level -= 1;
            },
            Block::BulletList(items) => self.write_bullet_list(items)?,
            Block::Header(l, _, i) => self.write_header(l, i)?,
            Block::HorizontalRule => self.push_str("\n---\n"),
            Block::Table(_, _, s, TableHead(_, h), b, _) => self.write_table(s, h, b)?,
            Block::LineBlock(_) =>
                return Err(WriteError::NotImplemented("Line block is not yet implemented")),
            Block::RawBlock(..) =>
                return Err(WriteError::NotImplemented("Raw block is not yet implemented")),
            Block::DefinitionList(_) =>
                return Err(WriteError::NotImplemented("Definition list is not yet implemented")),
            Block::Figure(..) =>
                return Err(WriteError::NotImplemented("Figure is not yet implemented")),
            Block::Div(..) => return Err(WriteError::NotImplemented("Div is not yet implemented")),
        };
        Ok(())
    }

    fn write_code_block(&mut self, language: &str, content: &str) {
        self.push_str("\n```");
        if !language.is_empty() {
            self.push_str(language);
        }
        self.push('\n');
        self.push_str(content);
        self.push_str("\n```\n");
    }

    fn write_ordered_list(&mut self, start: i32, items: Vec<Vec<Block>>) -> Result<(), WriteError> {
        self.push_str("\n1. ");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.push_str("\n1. ");
            }
            self.write_blocks(item.clone())?;
        }
        Ok(())
    }

    fn write_bullet_list(&mut self, items: Vec<Vec<Block>>) -> Result<(), WriteError> {
        self.push_str("\n- ");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.push_str("\n- ");
            }
            self.write_blocks(item.clone())?;
        }
        Ok(())
    }

    fn write_header(&mut self, level: i32, content: Vec<Inline>) -> Result<(), WriteError> {
        self.push('\n');
        for _ in 0..level {
            self.push('#');
        }
        self.push(' ');
        self.write_inlines(content)?;
        self.push('\n');
        Ok(())
    }

    fn write_table(
        &mut self, spec: Vec<ColSpec>, head: Vec<Row>, body: Vec<TableBody>,
    ) -> Result<(), WriteError> {
        self.push_str("\n|");
        let width = spec.len();
        for _ in &spec {
            self.push_str(" --- |");
        }
        self.push('\n');
        for r in head.into_iter().chain(body.into_iter().next().into_iter().flat_map(|b| b.3)) {
            self.push('|');
            for c in r.1.into_iter().take(width) {
                let mut c_iter = c.4.into_iter();
                let (Some(Block::Plain(i)), None) = (c_iter.next(), c_iter.next()) else {
                    return Err(WriteError::NotImplemented(
                        "Tables with nested blocks aren't yet implemented",
                    ));
                };
                self.write_inlines(i)?;
                self.push('|');
            }
            self.push('\n');
        }
        Ok(())
    }

    fn is_list_loose(list: &[Vec<Block>]) -> bool {
        list.iter()
            .flat_map(|v| v.iter())
            .find_map(|b| match b {
                Block::Para(_) => Some(false),
                Block::Plain(_) => Some(true),
                _ => None,
            })
            .unwrap_or(false)
    }

    fn write_inlines(&mut self, inlines: Vec<Inline>) -> Result<(), WriteError> {
        for i in inlines {
            self.write_inline(i)?;
        }
        Ok(())
    }

    fn write_inline(&mut self, inline: Inline) -> Result<(), WriteError> {
        match inline {
            Inline::Str(s) => self.write_str(&s),
            Inline::Emph(i) => {
                self.push('*');
                self.write_inlines(i)?;
                self.push('*');
            },
            Inline::Strong(i) => {
                self.push_str("**");
                self.write_inlines(i)?;
                self.push_str("**");
            },
            Inline::Strikeout(i) => {
                self.push_str("~~");
                self.write_inlines(i)?;
                self.push_str("~~");
            },
            Inline::Code(_, s) => {
                self.push('`');
                self.write_str(&s);
                self.push('`');
            },
            Inline::Space | Inline::SoftBreak => self.push(' '),
            Inline::LineBreak => self.push_str("\\\n"),
            Inline::Link(_, _, (u, t)) => {
                self.push_str("[");
                self.push_str(&t);
                self.push_str("](");
                self.push_str(&u);
                self.push_str(")");
            },
            Inline::Image(_, _, (u, _)) => {
                self.push_str("![Image](");
                self.push_str(&u);
                self.push_str(")");
            },
            Inline::Underline(_) =>
                return Err(WriteError::NotImplemented("Underline is not yet implemented")),
            Inline::Superscript(_) =>
                return Err(WriteError::NotImplemented("Superscript is not yet implemented")),
            Inline::Subscript(_) =>
                return Err(WriteError::NotImplemented("Subscript is not yet implemented")),
            Inline::SmallCaps(_) =>
                return Err(WriteError::NotImplemented("Small caps is not yet implemented")),
            Inline::Quoted(..) =>
                return Err(WriteError::NotImplemented("Quoted is not yet implemented")),
            Inline::Cite(..) =>
                return Err(WriteError::NotImplemented("Cite is not yet implemented")),
            Inline::Math(..) =>
                return Err(WriteError::NotImplemented("Math is not yet implemented")), //???
            Inline::RawInline(..) =>
                return Err(WriteError::NotImplemented("Raw inline is not yet implemented")),
            Inline::Note(_) =>
                return Err(WriteError::NotImplemented("Note is not yet implemented")),
            Inline::Span(..) =>
                return Err(WriteError::NotImplemented("Span is not yet implemented")),
            Inline::Temp(_) => todo!(),
            Inline::None => todo!(),
        }
        Ok(())
    }

    fn write_str(&mut self, str: &str) {
        for c in str.chars() {
            self.write_char(c);
        }
    }

    fn write_char(&mut self, c: char) {
        match c {
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                self.push('\\');
                self.push(c);
            },
            '~' => self.push('~'),
            '^' => self.push('^'),
            '\\' => self.push('\\'),
            '`' => self.push('`'),
            _ => self.push(c),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::*;

    use super::*;

    fn get_content(document: &str) -> &str {
        let start_pattern = "#document()\n";
        let start = document.find(start_pattern).unwrap() + start_pattern.len();
        document[start..].trim()
    }

    #[test]
    fn special_chars() {
        let p = Pandoc {
            meta: Meta::default(),
            blocks: vec![Block::Plain(vec![Inline::Str(String::from("&%$#_{}~^\\`"))])],
        };
        let result = TypstWriter::new().write(p).unwrap();
        let content = get_content(&result);
        let expected = "\\&\\%\\$\\#\\_\\{\\}~^\\`";
        assert_eq!(content, expected);
    }

    #[test]
    fn str() {
        let p = Pandoc {
            meta: Meta::default(),
            blocks: vec![Block::Plain(vec![Inline::Str(String::from("str"))])],
        };
        let result = TypstWriter::new().write(p).unwrap();
        let content = get_content(&result);
        let expected = "str";
        assert_eq!(content, expected);
    }
}
