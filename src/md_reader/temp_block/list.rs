use std::iter;

use crate::ast::{new_list_attributes, Block};
use crate::md_reader::iters::SkipIndent;
use crate::md_reader::temp_block::{
    CheckResult, IndentedCodeBlock, LineResult, SkipIndentResult, TempBlock, ThematicBreak,
};
use crate::md_reader::Links;

/// Struct representing an unfinished list
#[derive(Debug)]
pub struct List {
    /// Type of the list
    #[allow(clippy::struct_field_names)]
    list_type: ListType,
    /// Finished items of the list
    items: Vec<Item>,
    /// Current open item of the list
    pub current: Option<Item>,
    /// Whether the list is loose
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
    /// Creates a new list with one given open [`Item`] and [`ListType`]
    fn new(current: Item, list_type: ListType) -> Self {
        Self { list_type, items: Vec::new(), current: Some(current), loose: false }
    }

    /// Checks if the line is the beginning of a list assuming the first char is a `'*'` or a `'-'`
    /// and the line doesn't come after a paragraph
    pub fn check_star_dash(line: SkipIndent) -> CheckResult {
        let c = line.first;
        Item::check_star_dash(line).into_check_result(c)
    }

    /// Checks if the line is the beginning of a list assuming the first char is a `'*'` and the
    /// line comes after a paragraph
    pub fn check_star_paragraph(line: SkipIndent) -> CheckResult {
        Item::check_star_paragraph(line).into_check_result('*')
    }

    /// Checks if the line is the beginning of a list assuming the first char is a `'-'` and the
    /// line comes after a paragraph
    pub fn check_dash_paragraph(line: SkipIndent) -> CheckOrSetextResult {
        Item::check_dash_paragraph(line)
    }

    /// Checks if the line is the beginning of a list assuming the first char is a `'+'` and the
    /// line doesn't come after a paragraph
    pub fn check_plus(line: SkipIndent) -> CheckResult {
        Item::check_plus(line).into_check_result('+')
    }

    /// Checks if the line is the beginning of a list assuming the first char is a `'+'` and the
    /// line comes after a paragraph
    pub fn check_plus_paragraph(line: SkipIndent) -> CheckResult {
        Item::check_plus_paragraph(line).into_check_result('+')
    }

    /// Checks if the line is the beginning of a list assuming the first char is a digit from and
    /// the line doesn't come after a paragraph
    pub fn check_number(line: SkipIndent) -> CheckResult {
        Item::check_number(line).into_check_result()
    }

    /// Checks if the line is the beginning of a list assuming the first char is `'1'`  and the line
    /// comes after a paragraph
    pub fn check_number_paragraph(line: SkipIndent) -> CheckResult {
        Item::check_number_paragraph(line)
    }

    /// Parses a non-blank line of a document
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
            // Check for list items, if matching the type
            let line = match &self.list_type {
                ListType::Unordered('+') if line.first == '+' => match Item::check_plus(line) {
                    NewItemResult::New(i) => {
                        self.add_item(i, links);
                        return LineResult::None;
                    },
                    NewItemResult::Text(s) => s,
                },
                ListType::Unordered(c) if line.first == *c => match Item::check_star_dash(line) {
                    NewItemBreakResult::New(i) => {
                        self.add_item(i, links);
                        return LineResult::None;
                    },
                    NewItemBreakResult::Break =>
                        return LineResult::DoneSelfAndOther(ThematicBreak.into()),
                    NewItemBreakResult::Text(s) => s,
                },
                ListType::Ordered(Ordered { closing, .. }) if line.first.is_ascii_digit() =>
                    match Item::check_number(line) {
                        NewOrderedItemResult::New(i, o) =>
                            return if o.closing == *closing {
                                self.add_item(i, links);
                                LineResult::None
                            } else {
                                LineResult::DoneSelfAndNew(
                                    Self::new(i, ListType::Ordered(o)).into(),
                                )
                            },
                        NewOrderedItemResult::Text(s) => s,
                    },
                _ => line,
            };
            match self.current.as_mut() {
                Some(s) => s.current.next_continuation(line),
                None => TempBlock::check_block_known_indent(line).into_line_result_paragraph(true),
            }
        }
    }

    /// Parses a blank line of a document
    pub fn next_blank(&mut self, indent: usize, links: &mut Links) {
        if self.current.as_mut().is_some_and(|i| i.next_blank(indent, links)) {
            self.items.push(self.current.take().unwrap());
        }
    }

    /// Finishes the list into a [`Block`]
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

    /// Returns whether the list ends with a blank line
    pub fn ends_with_blank(&self) -> bool {
        self.current.as_ref().map_or(true, Item::ends_with_blank)
    }

    /// Checks last item to see if the list should be loose
    fn check_end(&mut self) {
        if self.current.as_ref().is_some_and(|i| i.loose) {
            self.loose = true;
        }
    }

    /// Adds item to the list checking if the list should be loose
    fn add_item(&mut self, new: Item, links: &mut Links) {
        let old = self.current.replace(new);
        if !self.loose
            && (old.is_none() || old.as_ref().is_some_and(|i| i.loose || i.ends_with_blank()))
        {
            self.loose = true;
        }
        if let Some(mut old) = old {
            old.current.finish_links(links);
            self.items.push(old);
        }
    }
}

/// Struct representing a single [`List`] item
#[derive(Debug)]
pub struct Item {
    /// Finished blocks
    finished: Vec<TempBlock>,
    /// Current block
    pub current: Box<TempBlock>,
    /// Width of the list item marker (marker + spaces)
    width: usize,
    /// Indent of the list item marker
    indent: usize,
    /// Whether item ends with a blank line
    gap: bool,
    /// Whether item makes the [`List`] it's a part of loose
    loose: bool,
}

/// Result of checking a list item beginning with a `'-'` after a paragraph
pub enum CheckOrSetextResult<'a> {
    Check(CheckResult<'a>),
    Setext(usize),
}

/// Result of checking a list item
enum NewItemResult<'a> {
    New(Item),
    Text(SkipIndent<'a>),
}

impl<'a> NewItemResult<'a> {
    fn into_check_result(self, c: char) -> CheckResult<'a> {
        match self {
            NewItemResult::New(i) => CheckResult::New(List::new(i, ListType::Unordered(c)).into()),
            NewItemResult::Text(s) => CheckResult::Text(s),
        }
    }
}

/// Result of checking a list item when a thematic break is also possible
enum NewItemBreakResult<'a> {
    New(Item),
    Break,
    Text(SkipIndent<'a>),
}

impl<'a> NewItemBreakResult<'a> {
    fn into_check_result(self, c: char) -> CheckResult<'a> {
        match self {
            NewItemBreakResult::New(i) =>
                CheckResult::New(List::new(i, ListType::Unordered(c)).into()),
            NewItemBreakResult::Break => CheckResult::Done(ThematicBreak.into()),
            NewItemBreakResult::Text(s) => CheckResult::Text(s),
        }
    }
}

/// Result of checking an ordered list item
enum NewOrderedItemResult<'a> {
    New(Item, Ordered),
    Text(SkipIndent<'a>),
}

impl<'a> NewOrderedItemResult<'a> {
    fn into_check_result(self) -> CheckResult<'a> {
        match self {
            NewOrderedItemResult::New(i, o) =>
                CheckResult::New(List::new(i, ListType::Ordered(o)).into()),
            NewOrderedItemResult::Text(s) => CheckResult::Text(s),
        }
    }
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
    /// Creates a new item without any blocks
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

    /// Creates a new item parsing the first line into a block
    fn new(width: usize, indent: usize, content: SkipIndent) -> Self {
        let (current, finished) = TempBlock::new_empty_known_indent(content);
        Self { finished, current: Box::new(current), width, indent, gap: false, loose: false }
    }

    /// Creates a new item with the first block being a [`IndentedCodeBlock`]
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

    /// Checks if a line begins a list item assuming it starts with a `'*'` or a `'-'` and the line
    /// doesn't come after a paragraph
    fn check_star_dash(line: SkipIndent) -> NewItemBreakResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_star_dash_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemBreakResult::New(Self::new_empty(2, line.indent)),
        }
    }

    /// Checks if a line begins a list item assuming it starts with a `'*'` and the line comes after
    /// a paragraph
    fn check_star_paragraph(line: SkipIndent) -> NewItemBreakResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_star_dash_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemBreakResult::Text(line),
        }
    }

    /// Checks if a line begins a list item knowing the rest of the line is a non-blank line and
    /// assuming the line either starts with a `'*'` or it starts with a `'-'` and comes after the
    /// paragraph
    fn check_star_dash_known<'a>(
        line: SkipIndent<'a>, rest: SkipIndent<'a>,
    ) -> NewItemBreakResult<'a> {
        if Self::check_thematic(&line, &rest) {
            NewItemBreakResult::Break
        } else {
            Self::check_unordered_known(line, rest).into()
        }
    }

    /// Checks if a line begins a list item assuming it starts with a `'-'` and the line comes after
    /// a paragraph
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

    /// Checks if a line begins a list item assuming it starts with a `'+'` and the line doesn't
    /// come after a paragraph
    fn check_plus(line: SkipIndent) -> NewItemResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_unordered_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemResult::New(Self::new_empty(2, line.indent)),
        }
    }

    /// Checks if a line begins a list item assuming it starts with a `'+'` and the line comes after
    /// a paragraph
    fn check_plus_paragraph(line: SkipIndent) -> NewItemResult {
        match line.skip_indent_rest() {
            SkipIndentResult::Line(rest) => Self::check_unordered_known(line, rest),
            SkipIndentResult::Blank(_) => NewItemResult::Text(line),
        }
    }

    /// Checks if a line begins a list item knowing the rest of the line is not empty and assuming
    /// all other necessary checks that would prevent a list item from beginning were passed
    fn check_unordered_known<'a>(line: SkipIndent<'a>, rest: SkipIndent<'a>) -> NewItemResult<'a> {
        match rest.indent {
            0 => NewItemResult::Text(line),
            i @ 1..=4 => NewItemResult::New(Self::new(1 + i, line.indent, rest)),
            5.. => NewItemResult::New(Self::new_code(2, line.indent, rest)),
        }
    }

    /// Checks if a line begins a list item assuming it starts with a digit and the line doesn't
    /// come after a paragraph
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

    /// Checks if a line begins a list item assuming it starts with a `'1'` and the line comes after
    /// a paragraph
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

    /// Returns whether this item ends with a blank line
    fn ends_with_blank(&self) -> bool { self.gap || self.current.ends_with_gap() }

    /// Checks if a given line is a thematic break
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

    /// Checks if a given line is a thematic break or a setext heading underline
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

    /// Parses a non-blank line of the document
    fn next_line(&mut self, line: SkipIndent, links: &mut Links) {
        let result = self.current.next_line(line, links);
        if !self.loose
            && (result.is_done_or_new() && self.gap
                || result.is_done_self_and_new_or_other() && self.current.ends_with_gap())
        {
            self.loose = true;
        }
        self.gap = false;
        self.current.apply_result(result, &mut self.finished, links);
    }

    /// Parses a blank line of the document and returns whether this line ends a list item (an empty
    /// list item has to have content at it's second line)
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

    /// Finishes this item into a [`Vec`] of [`Block`] elements
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

#[cfg(test)]
mod tests {
    use super::*;

    fn new_dash(line: &str) -> List {
        match List::check_star_dash(SkipIndent::skip(line, 0).into_line()) {
            CheckResult::New(TempBlock::List(l)) => l,
            _ => panic!(),
        }
    }

    fn new_plus(line: &str) -> List {
        match List::check_plus(SkipIndent::skip(line, 0).into_line()) {
            CheckResult::New(TempBlock::List(l)) => l,
            _ => panic!(),
        }
    }

    fn new_number(line: &str) -> List {
        match List::check_number(SkipIndent::skip(line, 0).into_line()) {
            CheckResult::New(TempBlock::List(l)) => l,
            _ => panic!(),
        }
    }

    fn next(list: &mut List, line: &str) -> LineResult {
        list.next(SkipIndent::skip(line, 0).into_line(), &mut Links::new())
    }

    fn next_blank(list: &mut List) { list.next_blank(0, &mut Links::new()); }

    #[test]
    fn next_item_indent() {
        let mut list = new_dash("  -  line");
        assert!(matches!(next(&mut list, "     next"), LineResult::None));
        assert!(matches!(next(&mut list, "     - list"), LineResult::None));
        assert!(list.items.is_empty());
        assert!(list.current.is_some_and(
            |i| i.finished.len() == 1 && matches!(i.current.as_ref(), TempBlock::List(_))
        ));
        let mut list = new_dash("  -");
        next_blank(&mut list);
        assert!(matches!(next(&mut list, "     next"), LineResult::DoneSelfAndNew(_)));
    }

    #[test]
    fn next_indent() {
        let mut list = new_dash("   -  line");
        assert!(matches!(next(&mut list, "    text"), LineResult::None));
        assert!(matches!(next(&mut list, "    - list"), LineResult::None));
        assert!(list.items.is_empty());
        assert!(list.current.is_some_and(|i| i.finished.is_empty()));
        let mut list = new_dash("   -  ***");
        assert!(matches!(
            next(&mut list, "    text"),
            LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(_))
        ));
        let mut list = new_dash("   -  ***");
        assert!(matches!(
            next(&mut list, "    - list"),
            LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(_))
        ));
        let mut list = new_dash("   -");
        assert!(matches!(
            next(&mut list, "    text"),
            LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(_))
        ));
        let mut list = new_dash("   -");
        assert!(matches!(
            next(&mut list, "    - list"),
            LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(_))
        ));
        let mut list = new_dash("   -");
        next_blank(&mut list);
        assert!(matches!(
            next(&mut list, "    text"),
            LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(_))
        ));
        let mut list = new_dash("   -");
        next_blank(&mut list);
        assert!(matches!(
            next(&mut list, "    - list"),
            LineResult::DoneSelfAndNew(TempBlock::IndentedCodeBlock(_))
        ));
    }

    #[test]
    fn matching_list_item() {
        let mut list = new_plus("+ list");
        assert!(matches!(next(&mut list, "+ item"), LineResult::None));
        assert_eq!(list.items.len(), 1);
        assert!(matches!(
            next(&mut list, "- item"),
            LineResult::DoneSelfAndNew(TempBlock::List(_))
        ));
        let mut list = new_dash("- list");
        assert!(matches!(next(&mut list, "- item"), LineResult::None));
        assert_eq!(list.items.len(), 1);
        assert!(matches!(
            next(&mut list, "- - -"),
            LineResult::DoneSelfAndOther(TempBlock::ThematicBreak(_))
        ));
        let mut list = new_dash("- list");
        assert!(matches!(
            next(&mut list, "* item"),
            LineResult::DoneSelfAndNew(TempBlock::List(_))
        ));
        let mut list = new_number("123. list");
        assert!(matches!(next(&mut list, "456. item"), LineResult::None));
        assert_eq!(list.items.len(), 1);
        assert!(matches!(
            next(&mut list, "789) item"),
            LineResult::DoneSelfAndNew(TempBlock::List(_))
        ));
    }

    #[test]
    fn next_no_indent() {
        let mut list = new_dash("- list");
        assert!(matches!(next(&mut list, "paragraph"), LineResult::None));
        assert!(list.items.is_empty());
        assert!(matches!(
            next(&mut list, "2. list"),
            LineResult::DoneSelfAndNew(TempBlock::List(_))
        ));
        let mut list = new_dash("- ***");
        assert!(matches!(
            next(&mut list, "paragraph"),
            LineResult::DoneSelfAndNew(TempBlock::Paragraph(_))
        ));
        let mut list = new_dash("-");
        next_blank(&mut list);
        assert!(matches!(next(&mut list, "-"), LineResult::None));
        assert_eq!(list.items.len(), 1);
        next_blank(&mut list);
        assert!(matches!(
            next(&mut list, "paragraph"),
            LineResult::DoneSelfAndNew(TempBlock::Paragraph(_))
        ));
    }

    fn new_dash_all<I>(i: I) -> List
    where I: IntoIterator<Item = &'static str> {
        let mut iter = i.into_iter();
        let mut list = new_dash(iter.next().unwrap());
        for s in iter {
            if s.is_empty() {
                next_blank(&mut list);
            } else {
                assert!(matches!(next(&mut list, s), LineResult::None));
            }
        }
        list.check_end();
        list
    }

    #[test]
    fn test_loose() {
        assert!(!new_dash_all(["- list", "- item"]).loose);
        assert!(new_dash_all(["- list", "", "- item"]).loose);
        assert!(!new_dash_all(["- - nested", "- item"]).loose);
        assert!(new_dash_all(["- - nested", "", "- item"]).loose);
        assert!(!new_dash_all(["- - nested", "", "  - item", "- item"]).loose);
        assert!(new_dash_all(["-", "  -", "    -", "", "- next"]).loose);
        assert!(!new_dash_all(["-     code", "- item"]).loose);
        assert!(new_dash_all(["-     code", "", "- item"]).loose);
        assert!(new_dash_all(["-     code", "", "  content"]).loose);
        assert!(
            !new_dash_all(["- ***", "  - ***", "  - ***", "", "  - ***", "  - ***", "- ***"]).loose
        );
        assert!(
            new_dash_all(["- ***", "  - ***", "  - ***", "", "  + ***", "  + ***", "- ***"]).loose
        );
    }

    fn check<'a, F, M, T>(check: F, matches: M, line: &'a str)
    where
        F: FnOnce(SkipIndent<'a>) -> T,
        M: FnOnce(T) -> bool,
        T: 'a,
    {
        assert!(matches(check(SkipIndent::skip(line, 0).into_line())));
    }

    #[test]
    fn test_checks() {
        let plus_new = |c| matches!(c, NewItemResult::New(_));
        let plus_text = |c| matches!(c, NewItemResult::Text(_));
        let break_new = |c| matches!(c, NewItemBreakResult::New(_));
        let break_break = |c| matches!(c, NewItemBreakResult::Break);
        let break_text = |c| matches!(c, NewItemBreakResult::Text(_));
        let setext_new = |c| matches!(c, CheckOrSetextResult::Check(CheckResult::New(_)));
        let setext_break = |c| matches!(c, CheckOrSetextResult::Check(CheckResult::Done(_)));
        let setext_text = |c| matches!(c, CheckOrSetextResult::Check(CheckResult::Text(_)));
        let setext_setext = |c| matches!(c, CheckOrSetextResult::Setext(_));
        let number_new = |c| matches!(c, NewOrderedItemResult::New(..));
        let number_text = |c| matches!(c, NewOrderedItemResult::Text(_));
        let number_para_new = |c| matches!(c, CheckResult::New(..));
        let number_para_text = |c| matches!(c, CheckResult::Text(_));
        check(Item::check_plus, plus_new, "+");
        check(Item::check_plus, plus_text, "+a");
        check(Item::check_plus, plus_new, "+ a");
        check(Item::check_plus, plus_new, "+     a");
        
        check(Item::check_plus_paragraph, plus_text, "+");
        check(Item::check_plus_paragraph, plus_text, "+a");
        check(Item::check_plus_paragraph, plus_new, "+ a");
        check(Item::check_plus_paragraph, plus_new, "+     a");
        
        check(Item::check_star_dash, break_new, "*");
        check(Item::check_star_dash, break_text, "*a");
        check(Item::check_star_dash, break_break, "***");
        check(Item::check_star_dash, break_text, "*** a");
        check(Item::check_star_dash, break_new, "* *");
        check(Item::check_star_dash, break_break, "* * *");
        check(Item::check_star_dash, break_new, "* * * a");
        check(Item::check_star_dash, break_new, "*     a");
        check(Item::check_star_dash, break_break, "*     **");
        check(Item::check_star_dash, break_new, "*     ** a");

        check(Item::check_star_paragraph, break_text, "*");
        check(Item::check_star_paragraph, break_text, "*a");
        check(Item::check_star_paragraph, break_break, "***");
        check(Item::check_star_paragraph, break_text, "*** a");
        check(Item::check_star_paragraph, break_new, "* *");
        check(Item::check_star_paragraph, break_break, "* * *");
        check(Item::check_star_paragraph, break_new, "* * * a");
        check(Item::check_star_paragraph, break_new, "*     a");
        check(Item::check_star_paragraph, break_break, "*     **");
        check(Item::check_star_paragraph, break_new, "*     ** a");

        check(Item::check_dash_paragraph, setext_setext, "-");
        check(Item::check_dash_paragraph, setext_text, "-a");
        check(Item::check_dash_paragraph, setext_setext, "---");
        check(Item::check_dash_paragraph, setext_break, "-- -");
        check(Item::check_dash_paragraph, setext_text, "--- a");
        check(Item::check_dash_paragraph, setext_new, "- -");
        check(Item::check_dash_paragraph, setext_setext, "--");
        check(Item::check_dash_paragraph, setext_break, "- - -");
        check(Item::check_dash_paragraph, setext_new, "- - - a");
        check(Item::check_dash_paragraph, setext_new, "-     a");
        check(Item::check_dash_paragraph, setext_break, "-     --");
        check(Item::check_dash_paragraph, setext_new, "-     -- a");

        check(Item::check_number, number_new, "1.");
        check(Item::check_number, number_new, "1)");
        check(Item::check_number, number_text, "1]");
        check(Item::check_number, number_new, "1. a");
        check(Item::check_number, number_new, "1.    a");
        check(Item::check_number, number_text, "1234567890.");

        check(Item::check_number_paragraph, number_para_text, "1.");
        check(Item::check_number_paragraph, number_para_text, "1)");
        check(Item::check_number_paragraph, number_para_text, "1] a");
        check(Item::check_number_paragraph, number_para_new, "1. a");
        check(Item::check_number_paragraph, number_para_new, "1.    a");
        check(Item::check_number_paragraph, number_para_text, "11. a");
    }
}
