use std::iter;

use super::{
    AtxHeading, BlockQuote, DoneResult, FencedCodeBlock, IndentedCodeBlock, LineResult, NewResult,
    Paragraph, SkipIndent, SkipIndentResult, TempBlock, ThematicBreak, ToLineResult,
};
use crate::ast::{new_list_attributes, Block};

#[derive(Debug)]
pub struct List {
    list_type: ListType,
    items: Vec<ListItem>,
    current: Option<ListItem>,
    loose: bool,
    gap: bool,
}

#[derive(Debug)]
enum ListType {
    Unordered(char),
    Ordered(Ordered),
}

#[derive(Debug)]
struct Ordered {
    starting: i32,
    closing: char,
}

impl List {
    fn new(current: ListItem, list_type: ListType) -> Self {
        Self { list_type, items: Vec::new(), current: Some(current), loose: false, gap: false }
    }

    pub(crate) fn check_star_dash(line: SkipIndent) -> ListBreakResult {
        let c = line.first;
        match ListItem::check_star_dash(line) {
            NewItemBreakResult::New(i) =>
                ListBreakResult::List(Self::new(i, ListType::Unordered(c))),
            NewItemBreakResult::Break => ListBreakResult::Break(ThematicBreak),
            NewItemBreakResult::Text(s) => ListBreakResult::Text(s),
        }
    }

    pub(crate) fn check_star_paragraph(line: SkipIndent) -> ListBreakResult {
        match ListItem::check_star_paragraph(line) {
            NewItemBreakResult::New(i) =>
                ListBreakResult::List(Self::new(i, ListType::Unordered('*'))),
            NewItemBreakResult::Break => ListBreakResult::Break(ThematicBreak),
            NewItemBreakResult::Text(s) => ListBreakResult::Text(s),
        }
    }

    pub(crate) fn check_dash_paragraph(line: SkipIndent) -> ListBreakSetextResult {
        ListItem::check_dash_paragraph(line)
    }

    pub(crate) fn check_plus(line: SkipIndent) -> NewResult {
        match ListItem::check_plus(line) {
            NewItemResult::New(i) => NewResult::New(Self::new(i, ListType::Unordered('+')).into()),
            NewItemResult::Text(s) => NewResult::Text(s),
        }
    }

    pub(crate) fn check_plus_paragraph(line: SkipIndent) -> NewResult {
        match ListItem::check_plus_paragraph(line) {
            NewItemResult::New(i) => NewResult::New(Self::new(i, ListType::Unordered('+')).into()),
            NewItemResult::Text(s) => NewResult::Text(s),
        }
    }

    pub(crate) fn check_number(line: SkipIndent) -> NewResult {
        match ListItem::check_number(line) {
            NewOrderedItemResult::New(i, o) =>
                NewResult::New(Self::new(i, ListType::Ordered(o)).into()),
            NewOrderedItemResult::Text(s) => NewResult::Text(s),
        }
    }

    pub(crate) fn check_number_paragraph(line: SkipIndent) -> NewResult {
        ListItem::check_number_paragraph(line)
    }

    pub(crate) fn continuation(&mut self, line: SkipIndent) -> Option<LineResult> {
        match self.current.as_mut()?.current.as_mut() {
            TempBlock::Paragraph(p) => Some(p.next_continuation(line)),
            TempBlock::BlockQuote(b) => b.continuation(line),
            TempBlock::List(l) => l.continuation(line),
            _ => None,
        }
    }

    pub(crate) fn indented_continuation(&mut self, line: SkipIndent) -> bool {
        let Some(current) = self.current.as_mut() else { return false };
        current.indented_continuation(line)
    }

    pub(crate) fn next(&mut self, line: SkipIndentResult) -> LineResult {
        match line {
            SkipIndentResult::Line(mut line) => {
                if let Some(current) = self.current.as_mut()
                    && line.indent >= current.indent + current.width
                {
                    line.move_indent(current.indent + current.width);
                    current.next(line);
                    LineResult::None
                } else if line.indent > 3 {
                    match self.indented_continuation(line.clone()) {
                        true => LineResult::None,
                        false =>
                            IndentedCodeBlock::new(line).done_self_and_new(),
                        
                    }
                } else {
                    let line = match self.check_matching(line) {
                        CheckMatchingResult::LineResult(r) => return r,
                        CheckMatchingResult::Text(s) => s,
                    };
                    match self.continuation(line.clone()) {
                        Some(r) => {
                            return r;
                        },
                        None => {},
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
                        '>' => BlockQuote::new(line).done_self_and_new(),
                        '0'..='9' => match List::check_number(line) {
                            NewResult::New(b) => LineResult::DoneSelfAndNew(b),
                            NewResult::Text(s) => Paragraph::new(s).done_self_and_new(),
                        },
                        _ => Paragraph::new(line).done_self_and_new(),
                    }
                }
            },
            SkipIndentResult::Blank(indent) => self.next_blank(indent),
        }
    }

    fn check_end(&mut self) {
        if self.current.as_ref().is_some_and(|i| i.loose) {
            self.loose = true;
        }
    }

    fn add_item(&mut self, new: ListItem) {
        match self.current.replace(new) {
            Some(old) => {
                if self.gap || old.makes_loose() {
                    self.loose = true;
                }
                self.items.push(old);
            },
            None =>
                if self.gap {
                    self.loose = true;
                },
        }
    }

    fn check_matching<'a>(&mut self, line: SkipIndent<'a>) -> CheckMatchingResult<'a> {
        match &self.list_type {
            ListType::Unordered('+') => match line.first {
                '+' => match ListItem::check_plus(line) {
                    NewItemResult::New(i) => {
                        self.add_item(i);
                        CheckMatchingResult::LineResult(LineResult::None)
                    },
                    NewItemResult::Text(s) => CheckMatchingResult::Text(s),
                },
                _ => CheckMatchingResult::Text(line),
            },
            ListType::Unordered(c) =>
                if line.first == *c {
                    match ListItem::check_star_dash(line) {
                        NewItemBreakResult::New(i) => {
                            self.add_item(i);
                            CheckMatchingResult::LineResult(LineResult::None)
                        },
                        NewItemBreakResult::Break =>
                            CheckMatchingResult::LineResult(ThematicBreak.done_self_and_other()),
                        
                        NewItemBreakResult::Text(s) => CheckMatchingResult::Text(s),
                    }
                } else {
                    CheckMatchingResult::Text(line)
                },
            ListType::Ordered(Ordered { closing, .. }) => match line.first {
                '0'..='9' => match ListItem::check_number(line) {
                    NewOrderedItemResult::New(i, o) =>
                        if o.closing == *closing {
                            self.add_item(i);
                            CheckMatchingResult::LineResult(LineResult::None)
                        } else {
                            CheckMatchingResult::LineResult(
                                Self::new(i, ListType::Ordered(o)).done_self_and_new(),
                            )
                        },
                    NewOrderedItemResult::Text(s) => CheckMatchingResult::Text(s),
                },
                _ => CheckMatchingResult::Text(line),
            },
        }
    }

    fn next_blank(&mut self, indent: usize) -> LineResult {
        let Some(current) = self.current.as_mut() else { return LineResult::None };
        if current.check_empty(indent) {
            self.items.push(self.current.take().unwrap());
            self.gap = true;
        }
        LineResult::None
    }

    pub(crate) fn finish(mut self) -> Block {
        self.check_end();
        let done =
            self.items.into_iter().chain(self.current).map(|i| i.finish(self.loose)).collect();
        match self.list_type {
            ListType::Unordered(_) => Block::BulletList(done),
            ListType::Ordered(Ordered { starting, closing }) =>
                Block::OrderedList(new_list_attributes(starting, closing), done),
        }
    }
}

enum CheckMatchingResult<'a> {
    LineResult(LineResult),
    Text(SkipIndent<'a>),
}

#[derive(Debug)]
struct ListItem {
    finished: Vec<TempBlock>,
    current: Box<TempBlock>,
    width: usize,
    indent: usize,
    gap: bool,
    loose: bool,
}

pub(crate) enum ListBreakResult<'a> {
    List(List),
    Break(ThematicBreak),
    Text(SkipIndent<'a>),
}

pub(crate) enum ListBreakSetextResult<'a> {
    List(List),
    Break(ThematicBreak),
    Text(SkipIndent<'a>),
    Setext,
}

enum NewItemResult<'a> {
    New(ListItem),
    Text(SkipIndent<'a>),
}

enum NewItemBreakResult<'a> {
    New(ListItem),
    Break,
    Text(SkipIndent<'a>),
}

enum NewOrderedItemResult<'a> {
    New(ListItem, Ordered),
    Text(SkipIndent<'a>),
}

impl<'a> From<NewItemResult<'a>> for NewItemBreakResult<'a> {
    fn from(value: NewItemResult<'a>) -> Self {
        match value {
            NewItemResult::New(i) => NewItemBreakResult::New(i),
            NewItemResult::Text(s) => NewItemBreakResult::Text(s),
        }
    }
}

impl ListItem {
    fn new_empty(width: usize, indent: usize) -> Self {
        Self {
            finished: Vec::new(),
            current: Box::new(TempBlock::Empty),
            width,
            indent,
            gap: false,
            loose: false,
        }
    }

    fn new(width: usize, indent: usize, content: SkipIndent) -> Self {
        let mut current = TempBlock::Empty;
        let mut finished = Vec::new();
        current.apply_result(TempBlock::next_empty_known_indent(content), &mut finished);
        Self { finished, current: Box::new(current), width, indent, gap: false, loose: false }
    }

    fn new_code(width: usize, indent: usize, mut content: SkipIndent) -> Self {
        content.move_indent(1);
        Self {
            finished: Vec::new(),
            current: Box::new(IndentedCodeBlock::new(content).into()),
            width,
            indent,
            gap: false,
            loose: false,
        }
    }

    fn check_star_dash(line: SkipIndent) -> NewItemBreakResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_star_dash_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemBreakResult::New(Self::new_empty(2, line.indent)),
        }
    }

    fn check_star_paragraph(line: SkipIndent) -> NewItemBreakResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_star_dash_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemBreakResult::Text(line),
        }
    }

    fn check_star_dash_known<'a>(
        line: SkipIndent<'a>, rest: SkipIndent<'a>,
    ) -> NewItemBreakResult<'a> {
        match Self::check_thematic(&line, &rest) {
            true => NewItemBreakResult::Break,
            false => Self::check_unordered_known(line, rest).into(),
        }
    }

    fn check_dash_paragraph(line: SkipIndent) -> ListBreakSetextResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) =>
                if rest.indent == 0 {
                    Self::check_thematic_setext(line, rest)
                } else if Self::check_thematic(&line, &rest) {
                    ListBreakSetextResult::Break(ThematicBreak)
                } else {
                    let item = if rest.indent < 5 {
                        Self::new(1 + rest.indent, line.indent, rest)
                    } else {
                        Self::new_code(2, line.indent, rest)
                    };
                    ListBreakSetextResult::List(List::new(item, ListType::Unordered('-')))
                },
            SkipIndentResult::Blank(_) => ListBreakSetextResult::Setext,
        }
    }

    fn check_plus(line: SkipIndent) -> NewItemResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_unordered_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemResult::New(Self::new_empty(2, line.indent)),
        }
    }

    fn check_plus_paragraph(line: SkipIndent) -> NewItemResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_unordered_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemResult::Text(line),
        }
    }

    fn check_unordered_known<'a>(line: SkipIndent<'a>, rest: SkipIndent<'a>) -> NewItemResult<'a> {
        match rest.indent {
            0 => NewItemResult::Text(line),
            i @ 1..=4 => NewItemResult::New(Self::new(1 + i, line.indent, rest)),
            5.. => NewItemResult::New(Self::new_code(2, line.indent, rest)),
        }
    }

    fn check_number(line: SkipIndent) -> NewOrderedItemResult {
        let mut iter = line.indent_iter_rest();
        let (Some((starting, width)), Some(closing)) =
            (iter.get_number(line.first), iter.get_closing())
        else {
            return NewOrderedItemResult::Text(line);
        };
        let list_type = Ordered { starting: starting as _, closing };
        match iter.skip_indent() {
            SkipIndentResult::Line(rest) => match rest.indent {
                0 => NewOrderedItemResult::Text(line),
                i @ 1..=4 => NewOrderedItemResult::New(
                    Self::new(width + 1 + i, line.indent, rest),
                    list_type,
                ),
                5.. => NewOrderedItemResult::New(
                    Self::new_code(width + 2, line.indent, rest),
                    list_type,
                ),
            },
            SkipIndentResult::Blank(_) =>
                NewOrderedItemResult::New(Self::new_empty(width + 2, line.indent), list_type),
        }
    }

    fn check_number_paragraph(line: SkipIndent) -> NewResult {
        let mut iter = line.indent_iter_rest();
        let Some(closing) = iter.get_closing() else {
            return NewResult::Text(line);
        };
        let list_type = ListType::Ordered(Ordered { starting: 1, closing });
        match iter.skip_indent() {
            SkipIndentResult::Line(rest) => match rest.indent {
                0 => NewResult::Text(line),
                i @ 1..=4 =>
                    NewResult::New(List::new(Self::new(2 + i, line.indent, rest), list_type).into()),
                5.. => NewResult::New(
                    List::new(Self::new_code(3, line.indent, rest), list_type).into(),
                ),
            },
            SkipIndentResult::Blank(_) => NewResult::Text(line),
        }
    }

    fn check_thematic(line: &SkipIndent, rest: &SkipIndent) -> bool {
        if line.first != rest.first {
            return false;
        }
        let mut third = false;
        for c in rest.get_rest().chars() {
            match c {
                ' ' | '\t' => continue,
                c if c == rest.first => third = true,
                _ => return false,
            }
        }
        third
    }

    fn check_empty(&mut self, indent: usize) -> bool {
        let result = match self.current.as_mut() {
            TempBlock::Empty =>
                return if self.finished.is_empty() {
                    true
                } else {
                    self.gap = true;
                    false
                },
            TempBlock::Paragraph(p) => {
                self.gap = true;
                p.next_blank()
            },
            TempBlock::IndentedCodeBlock(i) => i.next_blank(indent),
            TempBlock::FencedCodeBlock(f) => f.next_blank(indent),
            TempBlock::Table(t) => {
                self.gap = true;
                t.next_blank()
            },
            TempBlock::BlockQuote(b) => {
                self.gap = true;
                b.next_blank()
            },
            TempBlock::List(l) => l.next_blank(indent),
            _ => unreachable!(),
        };
        self.current.apply_result(result, &mut self.finished);
        false
    }

    fn check_thematic_setext<'a>(
        line: SkipIndent<'a>, rest: SkipIndent,
    ) -> ListBreakSetextResult<'a> {
        let mut space = false;
        let mut thematic = false;
        for c in rest.line.chars() {
            match c {
                ' ' | '\t' => space = true,
                '-' =>
                    if space {
                        thematic = true;
                    },
                _ => return ListBreakSetextResult::Text(line),
            }
        }
        match thematic {
            true => ListBreakSetextResult::Break(ThematicBreak),
            false => ListBreakSetextResult::Setext,
        }
    }

    fn next(&mut self, line: SkipIndent) {
        let result = self.current.next_no_apply(SkipIndentResult::Line(line));
        if self.gap && matches!(result, LineResult::New(_) | LineResult::Done(_)) {
            self.loose = true;
        }
        self.current.apply_result(result, &mut self.finished);
    }

    fn indented_continuation(&mut self, line: SkipIndent) -> bool {
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

    fn makes_loose(&self) -> bool {
        if self.gap || self.loose { return true; }
        let TempBlock::List(l) = self.current.as_ref() else { return false };
        if l.gap && !l.loose { return true; }
        let Some(last) = l.current.as_ref().or_else(|| l.items.last()) else { return false };
        last.gap && !last.loose
    }
    
    fn finish(self, loose: bool) -> Vec<Block> {
        let temp = self
            .finished
            .into_iter()
            .chain(iter::once(*self.current))
            .filter_map(TempBlock::finish);
        if loose {
            temp.collect()
        } else {
            temp.map(|b| match b {
                Block::Para(v) => Block::Plain(v),
                b => b,
            })
            .collect()
        }
    }
}
