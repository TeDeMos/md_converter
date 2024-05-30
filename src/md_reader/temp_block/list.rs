use std::iter;

use crate::ast::{new_list_attributes, Block};
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{
    CheckResult, IndentedCodeBlock, LineResult, SkipIndentResult, TempBlock,
    ThematicBreak,
};
use crate::md_reader::Links;

#[derive(Debug)]
pub struct List {
    #[allow(clippy::struct_field_names)]
    list_type: ListType,
    items: Vec<Item>,
    pub current: Option<Item>,
    loose: bool,
}

#[derive(Debug)]
enum ListType {
    Unordered(char),
    Ordered(Ordered),
}

#[derive(Debug)]
struct Ordered {
    starting: usize,
    closing: char,
}

impl List {
    fn new(current: Item, list_type: ListType) -> Self {
        Self { list_type, items: Vec::new(), current: Some(current), loose: false }
    }

    pub fn check_star_dash(line: SkipIndent) -> CheckResult {
        let c = line.first;
        match Item::check_star_dash(line) {
            NewItemBreakResult::New(i) =>
                CheckResult::New(Self::new(i, ListType::Unordered(c)).into()),
            NewItemBreakResult::Break => CheckResult::Done(ThematicBreak.into()),
            NewItemBreakResult::Text(s) => CheckResult::Text(s),
        }
    }

    pub fn check_star_paragraph(line: SkipIndent) -> CheckResult {
        match Item::check_star_paragraph(line) {
            NewItemBreakResult::New(i) =>
                CheckResult::New(Self::new(i, ListType::Unordered('*')).into()),
            NewItemBreakResult::Break => CheckResult::Done(ThematicBreak.into()),
            NewItemBreakResult::Text(s) => CheckResult::Text(s),
        }
    }

    pub fn check_dash_paragraph(line: SkipIndent) -> CheckOrSetextResult {
        Item::check_dash_paragraph(line)
    }

    pub fn check_plus(line: SkipIndent) -> CheckResult {
        match Item::check_plus(line) {
            NewItemResult::New(i) =>
                CheckResult::New(Self::new(i, ListType::Unordered('+')).into()),
            NewItemResult::Text(s) => CheckResult::Text(s),
        }
    }

    pub fn check_plus_paragraph(line: SkipIndent) -> CheckResult {
        match Item::check_plus_paragraph(line) {
            NewItemResult::New(i) =>
                CheckResult::New(Self::new(i, ListType::Unordered('+')).into()),
            NewItemResult::Text(s) => CheckResult::Text(s),
        }
    }

    pub fn check_number(line: SkipIndent) -> CheckResult {
        match Item::check_number(line) {
            NewOrderedItemResult::New(i, o) =>
                CheckResult::New(Self::new(i, ListType::Ordered(o)).into()),
            NewOrderedItemResult::Text(s) => CheckResult::Text(s),
        }
    }

    pub fn check_number_paragraph(line: SkipIndent) -> CheckResult {
        Item::check_number_paragraph(line)
    }

    pub fn next(&mut self, mut line: SkipIndent, links: &mut Links) -> LineResult {
        if let Some(current) = self.current.as_mut()
            && line.indent >= current.indent + current.width
        {
            line.move_indent(current.indent + current.width);
            current.next_line(line, links);
            LineResult::None
        } else if line.indent > 3 {
            match self.current.as_mut() {
                Some(current) => current.current.next_indented_continuation(line),
                None => LineResult::DoneSelfAndNew(IndentedCodeBlock::new(line).into()),
            }
        } else {
            let line = match self.check_matching(line) {
                CheckMatchingResult::LineResult(r) => return r,
                CheckMatchingResult::Text(s) => s,
            };
            match self.current.as_mut() {
                Some(s) => s.current.next_continuation(line),
                None => TempBlock::check_block_known_indent(line).into_line_result_paragraph(true),
            }
        }
    }

    fn ends_with_gap(&self) -> bool { self.current.as_ref().map_or(true, Item::ends_with_gap) }

    fn check_end(&mut self) {
        if self.current.as_ref().is_some_and(|i| i.loose) {
            self.loose = true;
        }
    }

    fn add_item(&mut self, new: Item) {
        let old = self.current.replace(new);
        if !self.loose
            && (old.is_none() || old.as_ref().is_some_and(|i| i.loose || i.ends_with_gap()))
        {
            self.loose = true;
        }
        if let Some(old) = old {
            self.items.push(old);
        }
    }

    fn check_matching<'a>(&mut self, line: SkipIndent<'a>) -> CheckMatchingResult<'a> {
        match &self.list_type {
            ListType::Unordered('+') => match line.first {
                '+' => match Item::check_plus(line) {
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
                    match Item::check_star_dash(line) {
                        NewItemBreakResult::New(i) => {
                            self.add_item(i);
                            CheckMatchingResult::LineResult(LineResult::None)
                        },
                        NewItemBreakResult::Break => CheckMatchingResult::LineResult(
                            LineResult::DoneSelfAndOther(ThematicBreak.into()),
                        ),
                        NewItemBreakResult::Text(s) => CheckMatchingResult::Text(s),
                    }
                } else {
                    CheckMatchingResult::Text(line)
                },
            ListType::Ordered(Ordered { closing, .. }) => match line.first {
                '0'..='9' => match Item::check_number(line) {
                    NewOrderedItemResult::New(i, o) =>
                        if o.closing == *closing {
                            self.add_item(i);
                            CheckMatchingResult::LineResult(LineResult::None)
                        } else {
                            CheckMatchingResult::LineResult(LineResult::DoneSelfAndNew(
                                Self::new(i, ListType::Ordered(o)).into(),
                            ))
                        },
                    NewOrderedItemResult::Text(s) => CheckMatchingResult::Text(s),
                },
                _ => CheckMatchingResult::Text(line),
            },
        }
    }

    pub fn next_blank(&mut self, indent: usize, links: &mut Links) {
        if self.current.as_mut().is_some_and(|i| i.next_blank(indent, links)) {
            self.items.push(self.current.take().unwrap());
        }
    }

    pub fn finish(mut self) -> Block {
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
pub struct Item {
    finished: Vec<TempBlock>,
    pub current: Box<TempBlock>,
    width: usize,
    indent: usize,
    gap: bool,
    loose: bool,
}

pub enum CheckOrSetextResult<'a> {
    Check(CheckResult<'a>),
    Setext(usize),
}

enum NewItemResult<'a> {
    New(Item),
    Text(SkipIndent<'a>),
}

enum NewItemBreakResult<'a> {
    New(Item),
    Break,
    Text(SkipIndent<'a>),
}

enum NewOrderedItemResult<'a> {
    New(Item, Ordered),
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

impl Item {
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
        let (current, finished) = TempBlock::new_empty_known_indent(content);
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
        if Self::check_thematic(&line, &rest) {
            NewItemBreakResult::Break
        } else {
            Self::check_unordered_known(line, rest).into()
        }
    }

    fn check_dash_paragraph(line: SkipIndent) -> CheckOrSetextResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) =>
                if rest.indent == 0 {
                    Self::check_thematic_setext(line, &rest)
                } else if Self::check_thematic(&line, &rest) {
                    CheckOrSetextResult::Check(CheckResult::Done(ThematicBreak.into()))
                } else {
                    let item = if rest.indent < 5 {
                        Self::new(1 + rest.indent, line.indent, rest)
                    } else {
                        Self::new_code(2, line.indent, rest)
                    };
                    CheckOrSetextResult::Check(CheckResult::New(
                        List::new(item, ListType::Unordered('-')).into(),
                    ))
                },
            SkipIndentResult::Blank(_) => CheckOrSetextResult::Setext(1),
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
        match iter.skip_indent() {
            SkipIndentResult::Line(rest) => match rest.indent {
                0 => NewOrderedItemResult::Text(line),
                i @ 1..=4 => NewOrderedItemResult::New(
                    Self::new(width + 1 + i, line.indent, rest),
                    Ordered { starting, closing },
                ),
                5.. => NewOrderedItemResult::New(
                    Self::new_code(width + 2, line.indent, rest),
                    Ordered { starting, closing },
                ),
            },
            SkipIndentResult::Blank(_) =>
                NewOrderedItemResult::New(Self::new_empty(width + 2, line.indent), Ordered {
                    starting,
                    closing,
                }),
        }
    }

    fn check_number_paragraph(line: SkipIndent) -> CheckResult {
        let mut iter = line.indent_iter_rest();
        let Some(closing) = iter.get_closing() else {
            return CheckResult::Text(line);
        };
        let list_type = ListType::Ordered(Ordered { starting: 1, closing });
        match iter.skip_indent() {
            SkipIndentResult::Line(rest) => match rest.indent {
                0 => CheckResult::Text(line),
                i @ 1..=4 => CheckResult::New(
                    List::new(Self::new(2 + i, line.indent, rest), list_type).into(),
                ),
                5.. => CheckResult::New(
                    List::new(Self::new_code(3, line.indent, rest), list_type).into(),
                ),
            },
            SkipIndentResult::Blank(_) => CheckResult::Text(line),
        }
    }

    fn ends_with_gap(&self) -> bool {
        self.gap || matches!(self.current.as_ref(), TempBlock::List(l) if l.ends_with_gap())
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

    fn check_thematic_setext<'a>(
        line: SkipIndent<'a>, rest: &SkipIndent,
    ) -> CheckOrSetextResult<'a> {
        let mut space = false;
        let mut thematic = false;
        let mut count = 1;
        for c in rest.line.chars() {
            match c {
                ' ' | '\t' => space = true,
                '-' => {
                    if space {
                        thematic = true;
                    }
                    count += 1;
            },
                _ => return CheckOrSetextResult::Check(CheckResult::Text(line)),
            }
        }
        if thematic {
            CheckOrSetextResult::Check(CheckResult::Done(ThematicBreak.into()))
        } else {
            CheckOrSetextResult::Setext(count)
        }
    }

    fn next_line(&mut self, line: SkipIndent, links: &mut Links) {
        let result = self.current.next_line(line, links);
        if !self.loose
            && (result.is_done_or_new() && self.gap
                || result.is_done_self_and_new_or_other()
                    && self.current.as_list().is_some_and(List::ends_with_gap))
        {
            self.loose = true;
        }
        self.gap = false;
        self.current.apply_result(result, &mut self.finished, links);
    }

    fn next_blank(&mut self, indent: usize, links: &mut Links) -> bool {
        if self.current.is_empty() && self.finished.is_empty() {
            return true;
        }
        let result;
        (result, self.gap) =
            self.current.next_blank(indent.saturating_sub(self.indent + self.width), links);
        self.current.apply_result(result, &mut self.finished, links);
        false
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
