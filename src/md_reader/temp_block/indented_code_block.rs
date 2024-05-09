use crate::ast::{attr_empty, Block};
use crate::md_reader::temp_block::list::ListBreakResult;

use super::{
    AtxHeading, BlockQuote, DoneResult, FencedCodeBlock, LineResult, List,
    NewResult, Paragraph, SkipIndent, SkipIndentResult, ThematicBreak, ToLineResult,
};

#[derive(Debug)]
pub(crate) struct IndentedCodeBlock(Vec<String>);

impl IndentedCodeBlock {
    pub(crate) fn new(mut line: SkipIndent) -> Self {
        line.move_indent(4);
        Self(vec![line.get_full()])
    }

    pub(crate) fn next(&mut self, line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(line) => match line.indent {
                0..=3 => match line.first {
                    '~' | '`' => match FencedCodeBlock::check(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                    },
                    '*' | '-' => match List::check_star_dash(line) {
                        ListBreakResult::List(l) => l.done_self_and_new(),
                        ListBreakResult::Break(b) => b.done_self_and_other(),
                        ListBreakResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                    },
                    '_' => match ThematicBreak::check(line) {
                        DoneResult::Done(b) => LineResult::DoneSelfAndOther(b),
                        DoneResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                    },
                    '+' => match List::check_plus(line) {
                        NewResult::New(l) => LineResult::DoneSelfAndNew(l),
                        NewResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                    },
                    '#' => match AtxHeading::check(line) {
                        DoneResult::Done(b) => LineResult::DoneSelfAndOther(b),
                        DoneResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                    },
                    '>' => BlockQuote::new(line).done_self_and_new(),
                    '0'..='9' => match List::check_number(line) {
                        NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                        NewResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                    },
                    _ => Paragraph::new(line).done_self_and_new(),
                },
                4.. => {
                    self.push(line);
                    LineResult::None
                },
            },
            SkipIndentResult::Blank(indent) => self.next_blank(indent),
        }
    }
    
    pub(crate) fn next_blank(&mut self, indent: usize) -> LineResult {
        self.push_empty(indent);
        LineResult::None
    }

    fn push(&mut self, mut line: SkipIndent) {
        line.move_indent(4);
        self.0.push(line.get_full());
    }

    fn push_empty(&mut self, indent: usize) { self.0.push(" ".repeat(indent.saturating_sub(4))); }

    pub(crate) fn finish(mut self) -> Block {
        while let Some(last) = self.0.last()
            && last.chars().all(char::is_whitespace)
        {
            self.0.pop();
        }
        let mut result = String::new();
        for l in self.0 {
            result.push_str(&l);
            result.push('\n');
        }
        result.pop();
        Block::CodeBlock(attr_empty(), result)
    }
}
