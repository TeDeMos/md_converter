//! Module containing the [`TypstWriter`] type used for writing Typst

use std::error::Error;

use derive_more::Display;

use crate::ast::{Alignment, Block, ColSpec, Inline, Pandoc, Row, TableBody, TableHead};
use crate::traits::AstWriter;

/// Writes a [`Pandoc`] ast representation to Typst. For now only [`Block`] and `[Inline`] elements
/// available in GitHub Flavoured Markdown are supported
#[derive(Default)]
pub struct TypstWriter {
    result: String,
    in_emph: bool,
    in_strong: bool,
    beginning: String,
}

impl TypstWriter {
    /// Creates a new [`TypstWriter`]
    #[must_use]
    pub fn new() -> Self {
        Self { result: String::new(), in_emph: false, in_strong: false, beginning: String::new() }
    }
}

impl AstWriter for TypstWriter {
    type WriteError = WriteError;

    fn write(mut self, ast: Pandoc) -> Result<String, Self::WriteError> {
        self.write_blocks(ast.blocks)?;
        Ok(self.result)
    }
}

/// Possible errors when writing to Typst
#[derive(Debug, Display)]
pub enum WriteError {
    /// Writing a [`Block`] or [`Inline`] that was not yet implemented
    NotImplemented(&'static str),
}

impl Error for WriteError {}

impl TypstWriter {
    fn push_str(&mut self, str: &str) { self.result.push_str(str) }

    fn push(&mut self, c: char) { self.result.push(c) }

    fn new_line(&mut self) {
        self.push('\n');
        self.result.push_str(&self.beginning);
    }

    fn write_blocks(&mut self, blocks: Vec<Block>) -> Result<(), WriteError> {
        for b in blocks {
            self.write_block(b)?;
        }
        Ok(())
    }

    fn write_block(&mut self, block: Block) -> Result<(), WriteError> {
        match block {
            Block::Plain(p) => self.write_inlines(p)?,
            Block::Para(p) => {
                self.new_line();
                self.write_inlines(p)?;
                self.new_line();
            },
            Block::CodeBlock((l, ..), t) => self.write_code_block(&l, &t),
            Block::BlockQuote(b) => {
                self.new_line();
                self.push_str("#quote(block: true)[");
                self.write_blocks(b)?;
                self.push(']');
                self.new_line();
            },
            Block::OrderedList((s, ..), items) => self.write_ordered_list(s, items)?,
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
        let max = content
            .lines()
            .map(|s| {
                let mut iter = s.chars();
                let mut count = 0;
                while iter.next() == Some('`') {
                    count += 1;
                }
                match iter.next() {
                    Some(_) => 0,
                    None => count,
                }
            })
            .max()
            .unwrap_or(0)
            .max(3);
        self.new_line();
        for _ in 0..max {
            self.push('`');
        }
        if !language.is_empty() {
            self.push_str(language);
        }
        for line in content.lines() {
            self.new_line();
            self.push_str(line);
        }
        self.new_line();
        for _ in 0..max {
            self.push('`');
        }
        self.new_line();
    }

    fn write_ordered_list(&mut self, start: i32, items: Vec<Vec<Block>>) -> Result<(), WriteError> {
        self.new_line();
        for (item, i) in items.into_iter().zip(start..) {
            let parsed = i.to_string();
            self.push_str(&parsed);
            self.push_str(". ");
            for _ in 0..parsed.len() + 2 {
                self.beginning.push(' ');
            }
            self.write_blocks(item)?;
            for _ in 0..parsed.len() + 2 {
                self.beginning.pop();
            }
            self.new_line();
        }
        self.new_line();
        Ok(())
    }

    fn write_bullet_list(&mut self, items: Vec<Vec<Block>>) -> Result<(), WriteError> {
        self.new_line();
        self.beginning.push_str("  ");
        for item in items {
            self.push_str("- ");
            self.write_blocks(item.clone())?;
        }
        self.beginning.pop();
        self.beginning.pop();
        self.new_line();
        Ok(())
    }

    fn write_header(&mut self, level: i32, content: Vec<Inline>) -> Result<(), WriteError> {
        self.new_line();
        for _ in 0..level {
            self.push('=');
        }
        self.push(' ');
        self.write_inlines(content)?;
        self.new_line();
        Ok(())
    }

    fn write_table(
        &mut self, spec: Vec<ColSpec>, head: Vec<Row>, body: Vec<TableBody>,
    ) -> Result<(), WriteError> {
        let size = spec.len();
        self.new_line();
        self.push_str("#table(\n");
        self.push_str("columns: ");
        self.push_str(&size.to_string());
        self.push_str("\nalign: (col, row) => (");
        for (c, _) in spec {
            match c {
                Alignment::Left => self.push_str("left,"),
                Alignment::Right => self.push_str("right,"),
                Alignment::Center => self.push_str("center,"),
                Alignment::Default => self.push_str("auto,"),
            }
        }
        self.push_str(").at(col),\n");
        for r in head.into_iter().chain(body.into_iter().next().into_iter().flat_map(|b| b.3)) {
            for c in r.1.into_iter().take(size) {
                self.push_str("[");
                let mut c_iter = c.4.into_iter();
                let (Some(Block::Plain(i)), None) = (c_iter.next(), c_iter.next()) else {
                    return Err(WriteError::NotImplemented(
                        "Tables with nested blocks aren't yet implemented",
                    ));
                };
                self.write_inlines(i)?;
                self.push_str("],\n");
            }
        }
        self.push(')');
        Ok(())
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
            Inline::Emph(i) =>
                if self.in_emph {
                    self.write_inlines(i)?;
                } else {
                    self.push('_');
                    self.in_emph = true;
                    self.write_inlines(i)?;
                    self.in_emph = false;
                    self.push('_');
                },
            Inline::Strong(i) =>
                if self.in_strong {
                    self.write_inlines(i)?;
                } else {
                    self.push('*');
                    self.in_strong = true;
                    self.write_inlines(i)?;
                    self.in_strong = false;
                    self.push('*');
                },
            Inline::Strikeout(i) => {
                self.push_str("#strike[");
                self.write_inlines(i)?;
                self.push_str("]");
            },
            Inline::Code(_, s) => {
                let mut longest = 0;
                let mut current = 0;
                for c in s.chars() {
                    if c == '`' {
                        current += 1;
                    } else {
                        longest = longest.max(current);
                        current = 0;
                    }
                }
                for _ in 0..longest {
                    self.push('`');
                }
                self.write_str(&s);
                for _ in 0..longest {
                    self.push('`');
                }
            },
            Inline::Space | Inline::SoftBreak => self.push(' '),
            Inline::LineBreak => self.push_str("\\\n"),
            Inline::Link(_, _, (u, t)) => {
                self.push_str("#link(");
                self.push_str(&u);
                self.push('[');
                self.push_str(&t);
                self.push(']');
            },
            Inline::Image(_, _, (u, _)) => {
                self.push_str("#figure(image(\"");
                self.push_str(&u);
                self.push_str("\", width: 100%))");
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
        let special =
            ['\\', '{', '}', '[', ']', '(', ')', '#', '$', '%', '^', '*', '_', '&', '~', '`'];
        if special.contains(&c) || c.is_ascii_digit() {
            self.push('\\');
        }
        self.push(c);
    }
}
