use std::collections::HashMap;
use std::iter;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::inline_parser::InlineParser;

type Bool = bool;
type Int = i32;
type Double = f64;
type Text = String;
type Map<T, K> = HashMap<T, K>;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct Pandoc {
    pub meta: Meta,
    pub blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct Meta(pub Map<Text, MetaValue>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum MetaValue {
    Map(Map<Text, MetaValue>),
    List(Vec<MetaValue>),
    Bool(Bool),
    String(Text),
    Inlines(Vec<Inline>),
    Blocks(Vec<Block>),
}

#[derive(Serialize, Deserialize, Debug, Derivative)]
#[serde(tag = "t", content = "c")]
#[derivative(PartialEq)]
pub enum Block {
    Plain(Vec<Inline>),
    Para(Vec<Inline>),
    LineBlock(Vec<Vec<Inline>>),
    CodeBlock(/* #[derivative(PartialEq = "ignore")] */ Attr, Text),
    RawBlock(Format, Text),
    BlockQuote(Vec<Block>),
    OrderedList(ListAttributes, Vec<Vec<Block>>),
    BulletList(Vec<Vec<Block>>),
    DefinitionList(Vec<(Vec<Inline>, Vec<Vec<Block>>)>),
    Header(Int, #[derivative(PartialEq = "ignore")] Attr, Vec<Inline>),
    HorizontalRule,
    Table(Attr, Caption, Vec<ColSpec>, TableHead, Vec<TableBody>, TableFoot),
    Figure(Attr, Caption, Vec<Block>),
    Div(Attr, Vec<Block>),
}

impl Block {
    #[must_use]
    pub fn new_header(level: usize, inlines: Vec<Inline>) -> Self {
        #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
        Self::Header(level as Int, attr_empty(), inlines)
    }

    #[must_use]
    pub fn new_table(rows: Vec<Vec<String>>, alignments: Vec<Alignment>, size: usize) -> Self {
        let mut iter = rows.into_iter();
        Self::Table(
            attr_empty(),
            Caption::empty(),
            alignments.into_iter().map(|a| (a, ColWidth::ColWidthDefault)).collect(),
            TableHead::new(iter.next().unwrap(), size),
            vec![TableBody::new(iter, size)],
            TableFoot::empty(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t", content = "c")]
pub enum Inline {
    Str(Text),
    Emph(Vec<Inline>),
    Underline(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikeout(Vec<Inline>),
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    SmallCaps(Vec<Inline>),
    Quoted(QuoteType, Vec<Inline>),
    Cite(Vec<Citation>, Vec<Inline>),
    Code(Attr, Text),
    Space,
    SoftBreak,
    LineBreak,
    Math(MathType, Text),
    RawInline(Format, Text),
    Link(Attr, Vec<Inline>, Target),
    Image(Attr, Vec<Inline>, Target),
    Note(Vec<Block>),
    Span(Attr, Vec<Inline>),
}

pub type Attr = (Text, Vec<Text>, Vec<(Text, Text)>);

#[must_use]
pub fn attr_empty() -> Attr { (String::new(), vec![], vec![]) }

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Format(pub Text);

type ListAttributes = (Int, ListNumberStyle, ListNumberDelim);

#[must_use]
pub fn new_list_attributes(starting: usize, closing: char) -> ListAttributes {
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    (starting as Int, ListNumberStyle::Decimal, match closing {
        '.' => ListNumberDelim::Period,
        ')' => ListNumberDelim::OneParen,
        _ => unreachable!(),
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Caption(pub Option<ShortCaption>, pub Vec<Block>);

impl Caption {
    #[must_use]
    pub fn empty() -> Self { Self(None, Vec::new()) }
}

pub type ColSpec = (Alignment, ColWidth);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TableHead(pub Attr, pub Vec<Row>);

impl TableHead {
    #[must_use]
    pub fn new(row: Vec<String>, size: usize) -> Self {
        Self(attr_empty(), vec![Row::new(row, size)])
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TableBody(pub Attr, pub RowHeadColumns, pub Vec<Row>, pub Vec<Row>);

impl TableBody {
    pub fn new<I>(rows: I, size: usize) -> Self
    where I: Iterator<Item = Vec<String>> {
        Self(attr_empty(), RowHeadColumns(0), Vec::new(), rows.map(|r| Row::new(r, size)).collect())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TableFoot(pub Attr, pub Vec<Row>);

impl TableFoot {
    #[must_use]
    pub fn empty() -> Self { Self(attr_empty(), Vec::new()) }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum QuoteType {
    SingleQuote,
    DoubleQuote,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub struct Citation {
    #[serde(rename = "citationId")]
    pub id: Text,
    #[serde(rename = "citationPrefix")]
    pub prefix: Vec<Inline>,
    #[serde(rename = "citationSuffix")]
    pub suffix: Vec<Inline>,
    #[serde(rename = "citationMode")]
    pub mode: CitationMode,
    #[serde(rename = "citationNoteNum")]
    pub note_num: Int,
    #[serde(rename = "citationHash")]
    pub hash: Int,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum MathType {
    DisplayMath,
    InlineMath,
}

type Target = (Text, Text);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum ListNumberStyle {
    DefaultStyle,
    Example,
    Decimal,
    LowerRoman,
    UpperRoman,
    LowerAlpha,
    UpperAlpha,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum ListNumberDelim {
    DefaultDelim,
    Period,
    OneParen,
    TwoParens,
}

type ShortCaption = Vec<Inline>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "t")]
pub enum Alignment {
    #[serde(rename = "AlignLeft")]
    Left,
    #[serde(rename = "AlignRight")]
    Right,
    #[serde(rename = "AlignCenter")]
    Center,
    #[serde(rename = "AlignDefault")]
    Default,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum ColWidth {
    ColWidth(Double),
    ColWidthDefault,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Row(pub Attr, pub Vec<Cell>);

impl Row {
    pub fn new(row: Vec<String>, size: usize) -> Self {
        let rest = size - row.len();
        Self(
            attr_empty(),
            row.into_iter()
                .map(Cell::new)
                .chain(iter::repeat_with(Cell::empty).take(rest))
                .collect(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RowHeadColumns(pub Int);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum CitationMode {
    AuthorInText,
    SuppressAuthor,
    NormalCitation,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Cell(pub Attr, pub Alignment, pub RowSpan, pub ColSpan, pub Vec<Block>);

impl Cell {
    #[must_use]
    pub fn new(content: String) -> Self {
        let inlines = InlineParser::parse_lines(&content);
        Self(
            attr_empty(),
            Alignment::Default,
            RowSpan(1),
            ColSpan(1),
            if inlines.is_empty() { Vec::new() } else { vec![Block::Plain(inlines)] },
        )
    }

    #[must_use]
    pub fn empty() -> Self {
        Self(attr_empty(), Alignment::Default, RowSpan(1), ColSpan(1), Vec::new())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RowSpan(pub Int);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ColSpan(pub Int);
