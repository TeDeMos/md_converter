//! Module containing the [`LatexWriter`] type used for writing LaTeX
use std::error::Error;


use derive_more::Display;

use crate::ast::{Alignment, Block, ColSpec, Inline, Pandoc, Row, TableBody, TableHead};
use crate::traits::AstWriter;

/// Writes a [`Pandoc`] ast representation to LaTeX. For now only [`Block`] and `[Inline`] elements
/// available in GitHub Flavoured Markdown are supported
#[derive(Default)]
pub struct LatexWriter {
    result: String,
    enum_level: usize,
}

impl LatexWriter {
    /// Creates a new [`LatexWriter`]
    #[must_use]
    pub const fn new() -> Self { Self { result: String::new(), enum_level: 0 } }
}

impl AstWriter for LatexWriter {
    type WriteError = WriteError;

    fn write(mut self, ast: Pandoc) -> Result<String, Self::WriteError> {
        self.push_str("\\documentclass[]{article}\n");
        self.push_str("\\usepackage[utf8]{inputenc}\n");
        self.push_str("\\usepackage[normalem]{ulem}\n");
        self.push_str("\\usepackage{graphicx}\n");
        self.push_str("\\usepackage{listings}\n");
        self.push_str(
            "\\providecommand{\\tightlist}{\\setlength{\\itemsep}{0pt}\\setlength{\\parskip}{0pt}}\n",
        );
        self.push_str("\\begin{document}\n");
        self.write_blocks(ast.blocks)?;
        self.push_str("\n\\end{document}");
        Ok(self.result)
    }
}

/// Possible errors when writing to LaTeX
#[derive(Debug, Display)]
pub enum WriteError {
    /// Writing a [`Block`] or [`Inline`] that was not yet implemented
    NotImplemented(&'static str),
}

impl Error for WriteError {}

impl LatexWriter {
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
            Block::Plain(p) => {
                self.write_inlines(p)?;
            }
            Block::Para(p) => {
                self.push('\n');
                self.write_inlines(p)?;
                self.push('\n');
            },
            Block::CodeBlock((l, ..), t) => self.write_code_block(&l, &t),
            Block::BlockQuote(b) => {
                self.push_str("\n\\begin{quote}\n");
                self.write_blocks(b)?;
                self.push_str("\n\\end{quote}\n");
            },
            Block::OrderedList((s, ..), items) => {
                self.enum_level += 1;
                self.write_ordered_list(s, items)?;
                self.enum_level -= 1;
            },
            Block::BulletList(items) => self.write_bullet_list(items)?,
            Block::Header(l, _, i) => self.write_header(l, i)?,
            Block::HorizontalRule =>
                self.push_str("\n\\begin{center}\\rule{0.5\\linewidth}{0.5pt}\\end{center}\n"),
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
        self.push_str("\n\\begin{lstlisting}");
        if !language.is_empty() {
            self.push_str("[language=");
            self.push_str(language);
            self.push(']');
        }
        self.push('\n');
        self.push_str(content);
        self.push_str("\n\\end{lstlisting}\n");
    }

    fn write_ordered_list(&mut self, start: i32, items: Vec<Vec<Block>>) -> Result<(), WriteError> {
        self.push_str("\n\\begin{enumerate}");
        if start != 1 {
            self.push_str("\nsetcounter{enum");
            for _ in 0..self.enum_level {
                self.push('i');
            }
            self.push_str("}{");
            self.push_str(&start.saturating_sub(1).to_string());
            self.push('}');
        }
        if Self::is_list_loose(&items) {
            self.push_str("\n\\tightlist");
        }
        for i in items {
            self.push_str("\n\\item\n");
            self.write_blocks(i)?;
        }
        self.push_str("\n\\end{enumerate}\n");
        Ok(())
    }

    fn write_bullet_list(&mut self, items: Vec<Vec<Block>>) -> Result<(), WriteError> {
        self.push_str("\n\\begin{itemize}");
        if Self::is_list_loose(&items) {
            self.push_str("\n\\tightlist");
        }
        for i in items {
            self.push_str("\n\\item\n");
            self.write_blocks(i)?;
        }
        self.push_str("\n\\end{itemize}\n");
        Ok(())
    }

    fn write_header(&mut self, level: i32, content: Vec<Inline>) -> Result<(), WriteError> {
        match level {
            1 => self.push_str("\n\\section{"),
            2 => self.push_str("\n\\subsection{"),
            3 => self.push_str("\n\\subsubsection{"),
            4 => self.push_str("\n\\paragraph{"),
            5 => self.push_str("\n\\subparagraph{"),
            _ => self.push('\n'),
        }
        self.write_inlines(content)?;
        match level {
            1..=5 => self.push_str("}\n"),
            _ => self.push('\n'),
        }
        Ok(())
    }

    fn write_table(
        &mut self, spec: Vec<ColSpec>, head: Vec<Row>, body: Vec<TableBody>,
    ) -> Result<(), WriteError> {
        self.push_str("\n\\begin{tabular}{|");
        let width = spec.len();
        for (a, _) in spec {
            self.push_str(match a {
                Alignment::Left => "l|",
                Alignment::Right => "r|",
                Alignment::Center | Alignment::Default => "c|",
            });
        }
        self.push_str("} \\hline \n");
        for r in head.into_iter().chain(body.into_iter().next().into_iter().flat_map(|b| b.3)) {
            let row_length = r.1.len();
            for c in r.1.into_iter().take(width) {
                let mut c_iter = c.4.into_iter();
                let (Some(Block::Plain(i)), None) = (c_iter.next(), c_iter.next()) else {
                    return Err(WriteError::NotImplemented(
                        "Tables with nested blocks aren't yet implemented",
                    ));
                };
                self.write_inlines(i)?;
                self.push('&');
            }
            for _ in 0..width.saturating_sub(row_length) {
                self.push('&');
            }
            self.result.pop();
            self.push_str("\\\\\\hline\n");
        }
        self.push_str("\\end{tabular}\n");
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
                self.push_str("\\emph{");
                self.write_inlines(i)?;
                self.push('}');
            },
            Inline::Strong(i) => {
                self.push_str("\\textbf{");
                self.write_inlines(i)?;
                self.push('}');
            },
            Inline::Strikeout(i) => {
                self.push_str("\\sout{");
                self.write_inlines(i)?;
                self.push('}');
            },
            Inline::Code(_, s) => {
                self.push_str("\\texttt{");
                self.write_str(&s);
                self.push('}');
            },
            Inline::Space | Inline::SoftBreak => self.push(' '),
            Inline::LineBreak => self.push_str("\\\\\n"),
            Inline::Link(_, _, (u, t)) => {
                self.push_str("\\href{");
                self.push_str(&u);
                self.push_str("}{");
                self.push_str(&t);
                self.push('}');
            },
            Inline::Image(_, _, (u, _)) => {
                self.push_str("\n\\includegraphics[width=\\linewidth]{");
                self.push_str(&u);
                self.push_str("}\n");
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
            '~' => self.push_str("\\textasciitilde{}"),
            '^' => self.push_str("\\^{}"),
            '\\' => self.push_str("\\textbackslash{}"),
            '`' => self.push_str("\\textasciigrave{}"),
            _ => self.push(c),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::*;

    use super::*;

    fn get_content(document: &str) -> &str {
        let start_pattern = "\\begin{document}\n";
        let end_pattern = "\\end{document}";
        let start = document.find(start_pattern).unwrap() + start_pattern.len();
        let end = document.find(end_pattern).unwrap();
        document[start..end].trim()
    }

    #[test]
    fn special_chars() {
        let p = Pandoc {
            pandoc_api_version: Vec::new(),
            meta: Meta::default(),
            blocks: vec![Block::Plain(vec![Inline::Str(String::from("&%$#_{}~^\\`"))])],
        };
        let result = LatexWriter::new().write(p).unwrap();
        let content = get_content(&result);
        let expected =
            "\\&\\%\\$\\#\\_\\{\\}\\textasciitilde{}\\^{}\\textbackslash{}\\textasciigrave{}";
        assert_eq!(content, expected);
    }

    #[test]
    fn str() {
        let p = Pandoc {
            pandoc_api_version: Vec::new(),
            meta: Meta::default(),
            blocks: vec![Block::Plain(vec![Inline::Str(String::from("str"))])],
        };
        let result = LatexWriter::new().write(p).unwrap();
        let content = get_content(&result);
        let expected = "str";
        assert_eq!(content, expected);
    }
}
