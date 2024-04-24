use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, AtxHeading, BlockQuote, FencedCodeBlock, IndentedCodeBlock, LineResult, Paragraph,
    TempBlock, ThematicBreak,
};

pub struct Empty;

impl Empty {
    pub fn next(line: &str) -> LineResult {
        let (indent, mut iter) = skip_indent(line, 4);
        if indent >= 4 {
            match iter.clone().all(char::is_whitespace) {
                true => LineResult::None,
                false =>
                    LineResult::New(TempBlock::IndentedCodeBlock(IndentedCodeBlock::new(&mut iter))),
            }
        } else {
            match iter.next() {
                Some(c @ ('~' | '`')) => Self::check_code_block(indent, c, line, &mut iter),
                Some(c @ ('*' | '_' | '-')) => Self::check_thematic_break(c, line, &mut iter),
                Some('#') => Self::check_atx_heading(line, &mut iter),
                Some('>') =>
                    LineResult::New(TempBlock::BlockQuote(BlockQuote::new(indent, line, &mut iter))),
                _ if !iter.all(char::is_whitespace) =>
                    LineResult::New(TempBlock::Paragraph(Paragraph::new(line))),
                _ => LineResult::None,
            }
        }
    }

    fn check_code_block(
        indent: usize, first: char, line: &str, rest: &mut Peekable<Chars>,
    ) -> LineResult {
        LineResult::New(
            FencedCodeBlock::check(indent, first, rest)
                .map(TempBlock::FencedCodeBlock)
                .unwrap_or_else(|| TempBlock::Paragraph(Paragraph::new(line))),
        )
    }

    fn check_thematic_break(first: char, line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        ThematicBreak::check(first, rest)
            .map(|t| LineResult::Done(TempBlock::ThematicBreak(t)))
            .unwrap_or_else(|| LineResult::New(TempBlock::Paragraph(Paragraph::new(line))))
    }

    fn check_atx_heading(line: &str, rest: &mut Peekable<Chars>) -> LineResult {
        AtxHeading::check(rest)
            .map(|a| LineResult::Done(TempBlock::AtxHeading(a)))
            .unwrap_or_else(|| LineResult::New(TempBlock::Paragraph(Paragraph::new(line))))
    }
}
