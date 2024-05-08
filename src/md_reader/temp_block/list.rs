use std::iter;
use std::iter::Peekable;
use std::str::Chars;

use super::{
    skip_indent, skip_indent_iter, AtxHeading, FencedCodeBlock, IndentedCodeBlock, LineResult,
    NewResult, Paragraph, SkipIndent, TempBlock, ThematicBreak, ToLineResult,
};
use crate::ast::{new_list_attributes, Block};

#[derive(Debug)]
pub struct List {
    list_type: ListType,
    items: Vec<ListItem>,
    current: ListItem,
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
    fn new2(current: ListItem, list_type: ListType) -> Self {
        Self { list_type, items: Vec::new(), current, loose: false, gap: false }
    }

    pub fn check_star_dash2(line: SkipIndent) -> StarDashResult {
        ListItem::check_star_dash2(line).into()
    }

    pub fn check_plus2(line: SkipIndent) -> NewResult { ListItem::check_plus2(line).into() }

    pub fn check_number2(line: SkipIndent) -> NewResult { ListItem::check_number2(line) }

    fn new_unordered_empty(indent: usize, marker: char) -> Self {
        Self {
            list_type: ListType::Unordered(marker),
            items: Vec::new(),
            current: ListItem::new_empty(2, indent),
            loose: false,
            gap: false,
        }
    }

    fn new_ordered_empty(indent: usize, width: usize, starting: i32, closing: char) -> Self {
        Self {
            list_type: ListType::Ordered(Ordered { starting, closing }),
            items: Vec::new(),
            current: ListItem::new_empty(width + 1, indent),
            loose: false,
            gap: false,
        }
    }

    fn new_unordered(indent: usize, marker: char, spaces: usize, first: &str) -> Self {
        Self {
            list_type: ListType::Unordered(marker),
            items: Vec::new(),
            current: ListItem::new(1 + spaces, indent, first),
            loose: false,
            gap: false,
        }
    }

    fn new_ordered(
        indent: usize, width: usize, starting: i32, closing: char, spaces: usize, first: &str,
    ) -> Self {
        Self {
            list_type: ListType::Ordered(Ordered { starting, closing }),
            items: Vec::new(),
            current: ListItem::new(width + spaces, indent, first),
            loose: false,
            gap: false,
        }
    }

    fn new_unordered_code(indent: usize, marker: char, rest: &mut Peekable<Chars>) -> Self {
        Self {
            list_type: ListType::Unordered(marker),
            items: Vec::new(),
            current: ListItem::new_code(2, indent, rest),
            loose: false,
            gap: false,
        }
    }

    fn new_ordered_code(
        indent: usize, width: usize, starting: i32, closing: char, rest: &mut Peekable<Chars>,
    ) -> Self {
        Self {
            list_type: ListType::Ordered(Ordered { starting, closing }),
            items: Vec::new(),
            current: ListItem::new_code(width + 1, indent, rest),
            loose: false,
            gap: false,
        }
    }

    pub fn check_number(
        first: char, indent: usize, line: &str, rest: &mut Peekable<Chars>,
    ) -> Option<Self> {
        let mut width = 1;
        while matches!(rest.peek(), Some('0'..='9')) {
            width += 1;
            if width == 10 {
                return None;
            }
            rest.next();
        }
        let closing = rest.next_if(|x| matches!(x, '.' | ')'))?;
        let starting = line[indent..indent + width].parse().unwrap();
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            match rest.next() {
                Some(_) => None,
                None => Some(Self::new_ordered_empty(indent, width + 1, starting, closing)),
            }
        } else if spaces <= 4 {
            match rest.next() {
                Some(_) => Some(Self::new_ordered(
                    indent,
                    width + 1,
                    starting,
                    closing,
                    spaces,
                    &line[indent + width + 1 + spaces..],
                )),
                None => Some(Self::new_ordered_empty(indent, width + 1, starting, closing)),
            }
        } else {
            Some(match rest.clone().all(char::is_whitespace) {
                true => Self::new_ordered_empty(indent, width + 1, starting, closing),
                false => Self::new_ordered_code(indent, width + 1, starting, closing, rest),
            })
        }
    }

    pub fn check_number_paragraph(
        indent: usize, line: &str, rest: &mut Peekable<Chars>,
    ) -> Option<Self> {
        let closing = rest.next_if(|x| matches!(x, '.' | ')'))?;
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            None
        } else if spaces <= 4 {
            rest.next().map(|_| {
                Self::new_ordered(indent, 2, 1, closing, spaces, &line[indent + 1 + spaces..])
            })
        } else {
            (!rest.clone().all(char::is_whitespace))
                .then(|| Self::new_ordered_code(indent, 2, 1, closing, rest))
        }
    }

    pub fn check_plus(indent: usize, line: &str, rest: &mut Peekable<Chars>) -> Option<Self> {
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            match rest.next() {
                Some(_) => None,
                None => Some(Self::new_unordered_empty(indent, '+')),
            }
        } else if spaces <= 4 {
            match rest.next() {
                Some(_) =>
                    Some(Self::new_unordered(indent, '+', spaces, &line[indent + 1 + spaces..])),
                None => Some(Self::new_unordered_empty(indent, '+')),
            }
        } else {
            match rest.clone().all(char::is_whitespace) {
                true => Some(Self::new_unordered_empty(indent, '+')),
                false => Some(Self::new_unordered_code(indent, '+', rest)),
            }
        }
    }

    pub fn check_other(
        first: char, indent: usize, line: &str, rest: &mut Peekable<Chars>,
    ) -> ListResult {
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            match rest.next() {
                Some(c) => match c == first && Self::check_thematic(first, rest) {
                    true => ListResult::Break(ThematicBreak),
                    false => ListResult::None,
                },
                None => ListResult::List(Self::new_unordered_empty(indent, first)),
            }
        } else if spaces <= 4 {
            match rest.next() {
                Some(c) => match c == first && Self::check_thematic(first, rest) {
                    true => ListResult::Break(ThematicBreak),
                    false => ListResult::List(Self::new_unordered(
                        indent,
                        first,
                        spaces,
                        &line[indent + 1 + spaces..],
                    )),
                },
                None => ListResult::List(Self::new_unordered_empty(indent, first)),
            }
        } else {
            let mut third = false;
            for c in rest.clone() {
                third = false;
                match c {
                    ' ' | '\t' => continue,
                    c if c == first => third = true,
                    _ => return ListResult::List(Self::new_unordered_code(indent, first, rest)),
                }
            }
            match third {
                true => ListResult::Break(ThematicBreak),
                false => ListResult::List(Self::new_unordered_empty(indent, first)),
            }
        }
    }

    pub fn check_plus_non_empty(
        indent: usize, line: &str, rest: &mut Peekable<Chars>,
    ) -> Option<Self> {
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            None
        } else if spaces <= 4 {
            rest.next()
                .map(|_| Self::new_unordered(indent, '+', spaces, &line[indent + 1 + spaces..]))
        } else {
            (!rest.clone().all(char::is_whitespace))
                .then(|| Self::new_unordered_code(indent, '+', rest))
        }
    }

    pub fn check_star_non_empty(
        indent: usize, line: &str, rest: &mut Peekable<Chars>,
    ) -> ListResult {
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            match rest.next() {
                Some(c) => match c == '*' && Self::check_thematic('*', rest) {
                    true => ListResult::Break(ThematicBreak),
                    false => ListResult::None,
                },
                None => ListResult::None,
            }
        } else if spaces <= 4 {
            match rest.next() {
                Some(c) => match c == '*' && Self::check_thematic('*', rest) {
                    true => ListResult::Break(ThematicBreak),
                    false => ListResult::List(Self::new_unordered(
                        indent,
                        '*',
                        spaces,
                        &line[indent + 1 + spaces..],
                    )),
                },
                None => ListResult::None,
            }
        } else {
            let mut third = false;
            for c in rest.clone() {
                third = false;
                match c {
                    ' ' | '\t' => continue,
                    '*' => third = true,
                    _ => return ListResult::List(Self::new_unordered_code(indent, '*', rest)),
                }
            }
            match third {
                true => ListResult::Break(ThematicBreak),
                false => ListResult::None,
            }
        }
    }

    pub fn check_dash_paragraph(
        indent: usize, line: &str, rest: &mut Peekable<Chars>,
    ) -> ParagraphListResult {
        let spaces = skip_indent_iter(rest, 5);
        if spaces == 0 {
            match rest.next() {
                Some('-') => Self::check_thematic_setext(rest),
                Some(_) | None => ParagraphListResult::None,
            }
        } else if spaces <= 4 {
            match rest.next() {
                Some(c) => match c == '-' && Self::check_thematic('-', rest) {
                    true => ParagraphListResult::Break(ThematicBreak),
                    false => ParagraphListResult::List(Self::new_unordered(
                        indent,
                        '-',
                        spaces,
                        &line[indent + 1 + spaces..],
                    )),
                },
                None => ParagraphListResult::None,
            }
        } else {
            let mut third = false;
            for c in rest.clone() {
                third = false;
                match c {
                    ' ' | '\t' => continue,
                    '-' => third = true,
                    _ =>
                        return ParagraphListResult::List(Self::new_unordered_code(
                            indent, '-', rest,
                        )),
                }
            }
            match third {
                true => ParagraphListResult::Break(ThematicBreak),
                false => ParagraphListResult::None,
            }
        }
    }

    const fn matches_type(&self, c: char) -> bool {
        match self.list_type {
            ListType::Unordered(m) => m == c,
            ListType::Ordered(_) => c.is_ascii_digit(),
        }
    }

    const fn matches_closing(&self, other: &Self) -> bool {
        let ListType::Ordered(Ordered { closing: a, .. }) = self.list_type else { return false };
        let ListType::Ordered(Ordered { closing: b, .. }) = other.list_type else { return false };
        a == b
    }

    fn update_self(&mut self, new: Self) {
        let old = std::mem::replace(&mut self.current, new.current);
        if self.gap
            || old.done
            || matches!(old.current.as_ref(), TempBlock::List(Self { gap: true, loose: false, .. }))
        {
            self.loose = true;
        }
        self.items.push(old);
    }

    #[allow(clippy::too_many_lines)]
    pub fn next(&mut self, line: &str) -> LineResult {
        let total = self.current.indent + self.current.width;
        let (indent, mut iter) = skip_indent(line, total);
        if indent >= total && !self.current.done {
            let result = self.current.current.next_no_apply(&line[total..]);
            match &result {
                LineResult::DoneSelf =>
                    if iter.all(char::is_whitespace) {
                        self.gap = true;
                    },
                LineResult::New(_) | LineResult::Done(_) if self.gap => self.loose = true,
                LineResult::DoneSelfAndNew(_) | LineResult::DoneSelfAndOther(_) => {
                    if matches!(
                        self.current.current.as_ref(),
                        TempBlock::List(Self { gap: true, loose: false, .. })
                    ) {
                        self.loose = true;
                    }
                },
                _ =>
                    if iter.all(char::is_whitespace) && self.current.check_empty() {
                        self.current.done = true;
                    },
            }
            self.current.current.apply_result(result, &mut self.current.finished);
            LineResult::None
        } else if indent >= 4 {
            let empty = iter.clone().all(char::is_whitespace);
            if empty {
                if self.current.check_empty() {
                    self.current.done = true;
                } else {
                    self.gap = true;
                }
                LineResult::None
            } else {
                match !self.current.done && self.over_indented_continuation(line) {
                    true => LineResult::None,
                    false =>
                        IndentedCodeBlock::new(&mut skip_indent(line, 4).1).done_self_and_new(),
                }
            }
            // match self.over_indented_continuation(line) {
            //     true => {
            //         if iter.clone().all(char::is_whitespace) {
            //             self.gap = true;
            //         }
            //         LineResult::None
            //     },
            //     false => IndentedCodeBlock::new(&mut skip_indent(line, 4).1).done_self_and_new(),
            // }
        } else {
            match iter.next() {
                Some('+') if self.matches_type('+') =>
                    match Self::check_plus(indent, line, &mut iter) {
                        Some(b) => {
                            self.update_self(b);
                            LineResult::None
                        },
                        None => Paragraph::new(line).done_self_and_new(),
                    },
                Some(c @ ('*' | '-')) if self.matches_type(c) =>
                    match Self::check_other(c, indent, line, &mut iter) {
                        ListResult::List(b) => {
                            self.update_self(b);
                            LineResult::None
                        },
                        ListResult::Break(b) => b.done_self_and_other(),
                        ListResult::None => Paragraph::new(line).done_self_and_new(),
                    },
                Some(c @ '0'..='9') if self.matches_type(c) =>
                    match Self::check_number(c, indent, line, &mut iter) {
                        Some(l) =>
                            if self.matches_closing(&l) {
                                let old = std::mem::replace(&mut self.current, l.current);
                                self.items.push(old);
                                if self.gap {
                                    self.loose = true;
                                }
                                LineResult::None
                            } else {
                                l.done_self_and_new()
                            },
                        None => Paragraph::new(line).done_self_and_new(),
                    },
                // f if let Some(r) = self.continuation(indent, f, line, &mut iter.clone()) => match
                // r {
                // LineResult::DoneSelf => {
                //     self.current
                //         .current
                //         .apply_result(LineResult::DoneSelf, &mut self.current.finished);
                //     self.gap = true;
                //     LineResult::None
                // },
                // r => r,
                // },
                Some(f)
                    if let Some(r) =
                        self.continuation(indent, Some(f), line, &mut iter.clone()) =>
                    r,
                Some(c @ ('~' | '`')) => match FencedCodeBlock::check(indent, c, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(c @ ('*' | '-')) => match List::check_other(c, indent, line, &mut iter) {
                    ListResult::List(b) => b.done_self_and_new(),
                    ListResult::Break(b) => b.done_self_and_other(),
                    ListResult::None => Paragraph::new(line).done_self_and_new(),
                },
                Some('_') => match ThematicBreak::check('_', &mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('+') => match List::check_plus(indent, line, &mut iter) {
                    Some(l) => l.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some('#') => match AtxHeading::check(&mut iter) {
                    Some(b) => b.done_self_and_other(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(c @ '0'..='9') => match List::check_number(c, indent, line, &mut iter) {
                    Some(b) => b.done_self_and_new(),
                    None => Paragraph::new(line).done_self_and_new(),
                },
                Some(_) => LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line))),
                // _ if !iter.all(char::is_whitespace) =>
                //     LineResult::DoneSelfAndNew(TempBlock::Paragraph(Paragraph::new(line))),
                _ => {
                    if self.current.check_empty() {
                        self.current.done = true;
                    }
                    self.next_blank()
                },
            }
        }
    }

    pub fn next_blank(&mut self) -> LineResult {
        let result = self.current.current.next_blank();
        if matches!(result, LineResult::DoneSelf)
            || matches!(self.current.current.as_ref(), TempBlock::Empty)
        {
            self.gap = true;
        }
        self.current.current.apply_result(result, &mut self.current.finished);
        LineResult::None
    }

    pub fn finish(self) -> Block {
        let done = self
            .items
            .into_iter()
            .chain(iter::once(self.current))
            .map(|i| i.finish(self.loose))
            .collect();
        match self.list_type {
            ListType::Unordered(_) => Block::BulletList(done),
            ListType::Ordered(Ordered { starting, closing }) =>
                Block::OrderedList(new_list_attributes(starting, closing), done),
        }
    }

    fn check_thematic(first: char, rest: &mut Peekable<Chars>) -> bool {
        let mut third = false;
        for c in rest {
            match c {
                ' ' | '\t' => continue,
                c if c == first => third = true,
                _ => return false,
            }
        }
        third
    }

    fn check_thematic_setext(rest: &mut Peekable<Chars>) -> ParagraphListResult {
        let mut space = false;
        let mut gap = false;
        for c in rest {
            match c {
                ' ' | '\t' => space = true,
                '-' =>
                    if space {
                        gap = true
                    },
                _ => return ParagraphListResult::None,
            }
        }
        match gap {
            true => ParagraphListResult::Break(ThematicBreak),
            false => ParagraphListResult::Setext,
        }
    }

    pub fn continuation(
        &mut self, indent: usize, first: Option<char>, line: &str, rest: &mut Peekable<Chars>,
    ) -> Option<LineResult> {
        // if first.is_none() && rest.clone().all(char::is_whitespace) {
        //     return None
        // }
        self.current.current.continuation(indent, first, line, rest)
    }

    pub fn over_indented_continuation(&mut self, line: &str) -> bool {
        self.current.current.over_indented_continuation(line)
    }
}

#[derive(Debug)]
struct ListItem {
    finished: Vec<TempBlock>,
    current: Box<TempBlock>,
    done: bool,
    width: usize,
    indent: usize,
}

pub enum StarDashResult<'a> {
    List(List),
    Break(ThematicBreak),
    Text(SkipIndent<'a>),
}

enum StarDashItemResult<'a> {
    Item(ListItem, char),
    Break(ThematicBreak),
    Text(SkipIndent<'a>),
}

impl<'a> From<StarDashItemResult<'a>> for StarDashResult<'a> {
    fn from(value: StarDashItemResult<'a>) -> Self {
        match value {
            StarDashItemResult::Item(l, c) => Self::List(List::new2(l, ListType::Unordered(c))),
            StarDashItemResult::Break(b) => Self::Break(b),
            StarDashItemResult::Text(s) => Self::Text(s),
        }
    }
}

enum PlusResult<'a> {
    New(ListItem),
    Text(SkipIndent<'a>),
}

impl<'a> From<PlusResult<'a>> for NewResult<'a> {
    fn from(value: PlusResult<'a>) -> Self {
        match value {
            PlusResult::New(l) => Self::New(List::new2(l, ListType::Unordered('+')).into()),
            PlusResult::Text(s) => Self::Text(s),
        }
    }
}

impl ListItem {
    fn new_empty2(width: usize, indent: usize) -> Self {
        Self {
            finished: Vec::new(),
            current: Box::new(TempBlock::Empty),
            done: false,
            width,
            indent,
        }
    }

    fn new2(width: usize, indent: usize, content: SkipIndent) -> Self {
        let mut current = TempBlock::Empty;
        let mut finished = Vec::new();
        current.apply_result(TempBlock::next_empty_known_indent(content), &mut finished);
        Self { finished, current: Box::new(current), done: false, width, indent }
    }

    fn new_code2(width: usize, indent: usize, mut content: SkipIndent) -> Self {
        content.move_indent(1);
        Self {
            finished: Vec::new(),
            current: Box::new(IndentedCodeBlock::new2(content).into()),
            done: false,
            width,
            indent,
        }
    }

    fn check_star_dash2(line: SkipIndent) -> StarDashItemResult {
        match line.skip_indent_rest() {
            Some(rest) => {
                if Self::check_thematic2(&line, &rest) {
                    return StarDashItemResult::Break(ThematicBreak);
                }
                match rest.indent {
                    0 => StarDashItemResult::Text(line),
                    i @ 1..=4 =>
                        StarDashItemResult::Item(Self::new2(1 + i, line.indent, rest), line.first),
                    5.. =>
                        StarDashItemResult::Item(Self::new_code2(2, line.indent, rest), line.first),
                }
            },
            None => StarDashItemResult::Item(Self::new_empty2(2, line.indent), line.first),
        }
    }

    pub fn check_plus2(line: SkipIndent) -> PlusResult {
        match line.skip_indent_rest() {
            Some(rest) => match rest.indent {
                0 => PlusResult::Text(line),
                i @ 1..=4 => PlusResult::New(Self::new2(1 + i, line.indent, rest)),
                5.. => PlusResult::New(Self::new_code2(2, line.indent, rest)),
            },
            None => PlusResult::New(Self::new_empty2(2, line.indent)),
        }
    }

    pub fn check_number2(line: SkipIndent) -> NewResult {
        let mut iter = line.indent_iter_rest();
        let Some((starting, width)) = iter.get_number(line.first) else {
            return NewResult::Text(line);
        };
        let Some(closing) = iter.get_closing() else {
            return NewResult::Text(line);
        };
        match iter.skip_indent() {
            Some(rest) => match rest.indent {
                0 => NewResult::Text(line),
                i @ 1..=4 => NewResult::New(
                    List::new2(
                        Self::new2(width + 1 + i, line.indent, rest),
                        ListType::Ordered(Ordered { starting: starting as _, closing }),
                    )
                    .into(),
                ),
                5.. => NewResult::New(
                    List::new2(
                        Self::new_code2(width + 2, line.indent, rest),
                        ListType::Ordered(Ordered { starting: starting as _, closing }),
                    )
                    .into(),
                ),
            },
            None => NewResult::New(
                List::new2(
                    Self::new_empty2(width + 2, line.indent),
                    ListType::Ordered(Ordered { starting: starting as _, closing }),
                )
                .into(),
            ),
        }
    }

    fn check_thematic2(line: &SkipIndent, rest: &SkipIndent) -> bool {
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

    fn new_empty(width: usize, indent: usize) -> Self {
        Self {
            finished: Vec::new(),
            current: Box::new(TempBlock::default()),
            width,
            indent,
            done: false,
        }
    }

    fn new(width: usize, indent: usize, first: &str) -> Self {
        let mut result = Self::new_empty(width, indent);
        result.current.next(first, &mut result.finished);
        result
    }

    fn new_code(width: usize, indent: usize, rest: &mut Peekable<Chars>) -> Self {
        Self {
            finished: Vec::new(),
            current: Box::new(IndentedCodeBlock::new(rest).into()),
            done: false,
            width,
            indent,
        }
    }

    fn check_empty(&self) -> bool {
        self.finished.is_empty() && matches!(self.current.as_ref(), TempBlock::Empty)
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

pub enum ListResult {
    List(List),
    Break(ThematicBreak),
    None,
}

pub enum ParagraphListResult {
    List(List),
    Break(ThematicBreak),
    Setext,
    None,
}

// trait ToListLineResult {
// fn new(self, loose: bool) -> ListLineResult;
// fn done(self, loose: bool) -> ListLineResult;
// fn done_self_and_new(self, loose: bool) -> ListLineResult;
// fn done_self_and_other(self, loose: bool) -> ListLineResult;
// }
//
// impl<T> ToListLineResult for T where T : ToLineResult {
// fn new(self, loose: bool) -> ListLineResult {
// ListLineResult { line_result: self.new(), loose }
// }
//
// fn done(self, loose: bool) -> ListLineResult {
// ListLineResult { line_result: self.done(), loose }
// }
//
// fn done_self_and_new(self, loose: bool) -> ListLineResult {
// ListLineResult { line_result: self.done_self_and_new(), loose }
// }
//
// fn done_self_and_other(self, loose: bool) -> ListLineResult {
// ListLineResult { line_result: self.done_self_and_other(), loose }
// }
// }
