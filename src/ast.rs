//! Module containing the [`Pandoc`] type for representing parsed documents

use std::collections::HashMap;
use std::iter;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::md_reader::inline_parser::InlineParser;

type Bool = bool;
type Int = i32;
type Double = f64;
type Text = String;
type Map<T, K> = HashMap<T, K>;

/// Struct representing a parsed document. Implements [`Serialize`] and
/// [`Deserialize`] traits. This type is compatible with Pandoc AST.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Pandoc {
    /// Metadata of a parsed document
    pub meta: Meta,
    /// Block elements of a parsed document
    pub blocks: Vec<Block>,
}

/// Metadata for the document: title, authors, date.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Meta(pub Map<Text, MetaValue>);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum MetaValue {
    Map(Map<Text, MetaValue>),
    List(Vec<MetaValue>),
    Bool(Bool),
    String(Text),
    Inlines(Vec<Inline>),
    Blocks(Vec<Block>),
}

/// Enum representing a single block element of a parsed document
#[derive(Serialize, Deserialize, Debug, PartialOrd, Clone, Derivative)]
#[serde(tag = "t", content = "c")]
#[derivative(PartialEq)]
pub enum Block {
    /// Plain text - list of [`Inline`] elements
    Plain(Vec<Inline>),
    /// Paragraph - list of [`Inline`] elements
    Para(Vec<Inline>),
    /// List of non-breaking lines, each a list of [`Inline`] elements
    LineBlock(Vec<Vec<Inline>>),
    /// Code block ([`String`]) with [`Attr`]
    CodeBlock(Attr, Text),
    /// Raw block as [`String`] with a specified [`Format`]
    RawBlock(Format, Text),
    /// Block quote (list of [`Block`] elements)
    BlockQuote(Vec<Block>),
    /// Ordered list ([`Attr`] and a list of items, each a list of [`Block`] elements)
    OrderedList(ListAttributes, Vec<Vec<Block>>),
    /// Bullet list (list of items, each a list of [`Block`] elements)
    BulletList(Vec<Vec<Block>>),
    /// Definition list. Each list item is a pair consisting of a term (a list of [`Inline`]
    /// elements) and one or more definitions (each a list of [`Block`] elements)
    DefinitionList(Vec<(Vec<Inline>, Vec<Vec<Block>>)>),
    /// Header - level [`i32`] and text - list of [`Inline`] elements
    Header(Int, #[derivative(PartialEq = "ignore")] Attr, Vec<Inline>),
    /// Horizontal rule
    HorizontalRule,
    /// Table with [`Attr`], [`Caption`], a list of [`ColSpec`] for each column, [`TableHead`], a
    /// list of [`TableBody`] elements and a [`TableFoot`]
    Table(
        Attr,
        Caption,
        Vec<ColSpec>,
        TableHead,
        Vec<TableBody>,
        TableFoot,
    ),
    /// Figure with [`Attr`], [`Caption`] and content as a list of [`Block`] elements
    Figure(Attr, Caption, Vec<Block>),
    /// Generic [`Block`] container with [`Attr`]
    Div(Attr, Vec<Block>),
}

impl Block {
    /// Creates a header from a level and list of [`Inline`] elements with empty [`Attr`]
    /// # Panics
    /// If `level` cannot fit into an [`i32`]
    #[must_use]
    pub fn new_header(level: usize, inlines: Vec<Inline>) -> Self {
        Self::Header(Int::try_from(level).unwrap(), attr_empty(), inlines)
    }

    /// Creates a table with the amount of columns given by the length of the `alignments`
    /// argument. Each column will have a default [`ColWidth`]. Content is given by the `rows`
    /// argument. Each row is defined by a list of [`String`] elements, each representing one
    /// [`Cell`]. If a row contains too many elements the excess will be ignored and if a row
    /// contains too little elements empty cells will be added. Each [`String`] is parsed as a
    /// [`Block::Plain`] element. The table will have empty [`Attr`], no [`Caption`] a single
    /// row in [`TableHead`], a single [`TableBody`] element with the remaining rows in its
    /// intermediate body and an empty [`TableFoot`]
    /// # Panics
    /// If `rows` is empty.
    #[must_use]
    pub fn new_table(rows: Vec<Vec<String>>, alignments: Vec<Alignment>) -> Self {
        let mut iter = rows.into_iter();
        let size = alignments.len();
        Self::Table(
            attr_empty(),
            Caption::default(),
            alignments
                .into_iter()
                .map(|a| (a, ColWidth::ColWidthDefault))
                .collect(),
            TableHead::new(iter.next().unwrap(), size),
            vec![TableBody::new(iter, size)],
            TableFoot::default(),
        )
    }
}

/// Enum representing a single inline element of a document
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone)]
#[serde(tag = "t", content = "c")]
pub enum Inline {
    /// String
    Str(Text),
    /// Emphasized text (list of [`Inline`] elements)
    Emph(Vec<Inline>),
    /// Underlined text (list of [`Inline`] elements)
    Underline(Vec<Inline>),
    /// Strongly emphasized text (list of [`Inline`] elements)
    Strong(Vec<Inline>),
    /// Strikeout text (list of [`Inline`] elements)
    Strikeout(Vec<Inline>),
    /// Superscripted text (list of [`Inline`] elements)
    Superscript(Vec<Inline>),
    /// Subscripted text (list of [`Inline`] elements)
    Subscript(Vec<Inline>),
    /// Small caps text (list of [`Inline`] elements)
    SmallCaps(Vec<Inline>),
    /// Quoted text (a [`QuoteType`] and a list of [`Inline`] elements)
    Quoted(QuoteType, Vec<Inline>),
    /// Citation (a list of [`Citation`] elements and a list of [`Inline`] elements)
    Cite(Vec<Citation>, Vec<Inline>),
    /// Inline code ([`Attr`] and raw [`String`])
    Code(Attr, Text),
    /// Inner-word space
    Space,
    /// Soft line break
    SoftBreak,
    /// Hard line break
    LineBreak,
    /// TeX math ([`MathType`] and a raw [`String`])
    Math(MathType, Text),
    /// Raw inline as a [`String`] with a specified [`Format`]
    RawInline(Format, Text),
    /// Hyperlink: alt text (list of [`Inline`] elements) and a [`Target`]
    Link(Attr, Vec<Inline>, Target),
    /// Image: alt text (list of [`Inline`] elements) and a [`Target`]
    Image(Attr, Vec<Inline>, Target),
    /// Footnote or endnote (list of [`Block`] elements)
    Note(Vec<Block>),
    /// Generic [`Inline`] container with [`Attr`]
    Span(Attr, Vec<Inline>),
}

/// Attributes: identifier, classes, key-value pairs
pub type Attr = (Text, Vec<Text>, Vec<(Text, Text)>);

/// Creates empty [`Attr`]
#[must_use]
pub fn attr_empty() -> Attr {
    (String::new(), Vec::new(), Vec::new())
}

/// Format for [`Block::RawBlock`] and [`Inline::RawInline`]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Default)]
pub struct Format(pub Text);

/// Starting number, [`ListNumberStyle`] and [`ListNumberDelim`]
pub type ListAttributes = (Int, ListNumberStyle, ListNumberDelim);

/// Creates [`ListAttributes`] with a given starting number, [`ListNumberStyle::Decimal`] and
/// [`ListNumberDelim`] based on a given closing char.
/// # Panics
/// If `starting` cannot fit into an [`i32`] or if closing char is not `'.'` or `')'`
#[must_use]
pub fn new_list_attributes(starting: usize, closing: char) -> ListAttributes {
    (
        Int::try_from(starting).unwrap(),
        ListNumberStyle::Decimal,
        match closing {
            '.' => ListNumberDelim::Period,
            ')' => ListNumberDelim::OneParen,
            _ => panic!(),
        },
    )
}

/// Caption of a [`Block::Table`] or [`Block::Figure`] with an optional [`ShortCaption`]
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
pub struct Caption(pub Option<ShortCaption>, pub Vec<Block>);

/// Specification of a single [`Block::Table`] column
pub type ColSpec = (Alignment, ColWidth);

/// Head of a `[Block::Table`]
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
pub struct TableHead(pub Attr, pub Vec<Row>);

impl TableHead {
    /// Creates a [`TableHead`] from a row as a list of [`String`] where each represents one
    /// [`Cell`] and the amount of columns. Each [`String`] is parsed as a [`Block::Plain`] element.
    /// If the row contains too many elements, the excess will be ignored and if it contains too
    /// little elements, empty cells will be added.
    #[must_use]
    pub fn new(row: Vec<String>, size: usize) -> Self {
        Self(attr_empty(), vec![Row::new(row, size)])
    }
}

/// A body of a [`Block::Table`] with an intermediate head and the specified number of row header
/// columns in the intermediate body.
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
pub struct TableBody(pub Attr, pub RowHeadColumns, pub Vec<Row>, pub Vec<Row>);

impl TableBody {
    /// Creates a [`TableBody`] from an [`Iterator`] of rows each a list of [`String`] where each
    /// represents one [`Cell`]. The body will have empty [`Attr`], no head columns and all the rows
    /// in the intermediate body. Each [`String`] is parsed as a [`Block::Plain`] element. If
    /// the row contains too many elements, the excess will be ignored and if it contains too
    /// little elements, empty cells will be added.
    pub fn new<I>(rows: I, size: usize) -> Self
    where
        I: Iterator<Item = Vec<String>>,
    {
        Self(
            attr_empty(),
            RowHeadColumns(0),
            Vec::new(),
            rows.map(|r| Row::new(r, size)).collect(),
        )
    }
}

/// A foot of a [`Block::Table`]
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
pub struct TableFoot(pub Attr, pub Vec<Row>);

/// Type of quotation marks to use in [`Inline::Quoted`]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
#[serde(tag = "t")]
pub enum QuoteType {
    /// Single quotation marks
    SingleQuote,
    /// Double quotation marks
    DoubleQuote,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
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

/// Type of math element
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
#[serde(tag = "t")]
pub enum MathType {
    DisplayMath,
    InlineMath,
}

/// Link target - a [`String`] for URL and a [`String`] for title
pub type Target = (Text, Text);

/// Style of a [`Block::OrderedList`] numbers
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Default,
)]
#[serde(tag = "t")]
pub enum ListNumberStyle {
    #[default]
    DefaultStyle,
    Example,
    Decimal,
    LowerRoman,
    UpperRoman,
    LowerAlpha,
    UpperAlpha,
}

/// Delimiter of a [`Block::OrderedList`] numbers
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Default,
)]
#[serde(tag = "t")]
pub enum ListNumberDelim {
    #[default]
    DefaultDelim,
    Period,
    OneParen,
    TwoParens,
}

/// Short caption for use in [`Block::Table`] and [`Block::Figure`]
pub type ShortCaption = Vec<Inline>;

/// Alignment of a [`Block::Table`] column
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Default,
)]
#[serde(tag = "t")]
pub enum Alignment {
    #[serde(rename = "AlignLeft")]
    Left,
    #[serde(rename = "AlignRight")]
    Right,
    #[serde(rename = "AlignCenter")]
    Center,
    #[serde(rename = "AlignDefault")]
    #[default]
    Default,
}

/// The width of a [`Block::Table`] column as a percentage of the text width
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Copy, Clone, Default)]
#[serde(tag = "t")]
pub enum ColWidth {
    ColWidth(Double),
    #[default]
    ColWidthDefault,
}

/// A [`Block::Table`] row
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
pub struct Row(pub Attr, pub Vec<Cell>);

impl Row {
    /// Crates a new row from a list of [`String`] where each represents one [`Cell`] and the amount
    /// of table columns. Each [`String`] is parsed as a [`Block::Plain`] element. If the row
    /// contains too many elements, the excess will be ignored and if it contains too
    /// little elements, empty cells will be added. The row will have empty [`Attr`]
    pub fn new(row: Vec<String>, size: usize) -> Self {
        let rest = size - row.len();
        Self(
            attr_empty(),
            row.into_iter()
                .map(|s| Cell::new(&s))
                .chain(iter::repeat_with(Cell::default).take(rest))
                .collect(),
        )
    }
}

/// The number of columns taken up by the row head of each row of a [`TableBody`]. The row body
/// takes up the remaining columns.
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Default,
)]
pub struct RowHeadColumns(pub Int);

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Default,
)]
#[serde(tag = "t")]
pub enum CitationMode {
    AuthorInText,
    SuppressAuthor,
    #[default]
    NormalCitation,
}

/// A [`Block::Table`] cell
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Default)]
pub struct Cell(
    pub Attr,
    pub Alignment,
    pub RowSpan,
    pub ColSpan,
    pub Vec<Block>,
);

impl Cell {
    /// Creates a new [`Cell`]. The [`String`] will be parsed as a `[Block::Inline`]. The cell will
    /// have empty [`Attr`], `Alignment::Default` and [`RowSpan`] and [`ColSpan`] set to 1.
    #[must_use]
    pub fn new(content: &str) -> Self {
        let inlines = InlineParser::parse_lines(content);
        Self(
            attr_empty(),
            Alignment::Default,
            RowSpan(1),
            ColSpan(1),
            if inlines.is_empty() {
                Vec::new()
            } else {
                vec![Block::Plain(inlines)]
            },
        )
    }
}

/// The number of rows occupied by a cell; the height of a cell.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct RowSpan(pub Int);

impl Default for RowSpan {
    fn default() -> Self {
        Self(1)
    }
}

/// The number of columns occupied by a cell; the width of a cell.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct ColSpan(pub Int);

impl Default for ColSpan {
    fn default() -> Self {
        Self(1)
    }
}
