use std::iter;

use super::{
    AtxHeading, DoneResult, FencedCodeBlock, IndentedCodeBlock, LineResult, List, ListBreakResult,
    NewResult, Paragraph, SkipIndent, SkipIndentResult, TempBlock, ThematicBreak, ToLineResult,
};
use crate::ast::Block;

#[derive(Debug)]
pub(crate) struct BlockQuote {
    current: Box<TempBlock>,
    finished: Vec<TempBlock>,
}

impl BlockQuote {
    pub(crate) fn new(line: SkipIndent) -> Self {
        let mut current = TempBlock::Empty;
        let mut finished = Vec::new();
        let mut content = line.skip_indent_rest();
        content.inspect(|s| s.move_indent_capped(1));
        current.apply_result(TempBlock::next_empty(content), &mut finished);
        Self { current: Box::new(current), finished }
    }

    pub(crate) fn next(&mut self, line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(line) => match line.indent {
                0..=3 => {
                    if line.first == '>' {
                        let mut content = line.skip_indent_rest();
                        content.inspect(|s| s.move_indent_capped(1));
                        self.current.next(content, &mut self.finished);
                        return LineResult::None;
                    }
                    if let Some(r) = self.continuation(line.clone()) {
                        return r;
                    }
                    match line.first {
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
                        '0'..='9' => match List::check_number(line) {
                            NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                            NewResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                        },
                        _ => Paragraph::new(line).done_self_and_new(),
                    }
                },
                4.. => match self.indented_continuation(line.clone()) {
                    true => LineResult::None,
                    false => IndentedCodeBlock::new(line).done_self_and_new(),
                },
            },
            SkipIndentResult::Blank(_) => LineResult::DoneSelf,
        }
    }

    pub(crate) fn next_blank(&mut self) -> LineResult { LineResult::DoneSelf }

    pub(crate) fn finish(self) -> Block {
        Block::BlockQuote(
            self.finished
                .into_iter()
                .chain(iter::once(*self.current))
                .filter_map(TempBlock::finish)
                .collect(),
        )
    }

    pub(crate) fn continuation(&mut self, line: SkipIndent) -> Option<LineResult> {
        match self.current.as_mut() {
            TempBlock::Paragraph(p) => Some(p.next_continuation(line)),
            TempBlock::BlockQuote(b) => b.continuation(line),
            TempBlock::List(l) => l.continuation(line),
            _ => None,
        }
    }

    pub(crate) fn indented_continuation(&mut self, line: SkipIndent) -> bool {
        match self.current.as_mut() {
            TempBlock::Paragraph(p) => {
                p.next_indented_continuation(line);
                true
            },
            TempBlock::BlockQuote(b) => b.indented_continuation(line),
            TempBlock::List(l) => l.indented_continuation(line),
            _ => false,
        }
    }
}
