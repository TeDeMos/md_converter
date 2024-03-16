use std::collections::HashMap;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

type Bool = bool;
type Int = i32;
type Double = f64;
type Text = String;
type Map<T, K> = HashMap<T, K>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
    CodeBlock(Attr, Text),
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

type Attr = (Text, Vec<Text>, Vec<(Text, Text)>);

pub fn attr_empty() -> Attr { ("".into(), vec![], vec![]) }

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Format(pub Text);

type ListAttributes = (Int, ListNumberStyle, ListNumberDelim);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Caption(pub Option<ShortCaption>, pub Vec<Block>);

type ColSpec = (Alignment, ColWidth);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TableHead(pub Attr, pub Vec<Row>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TableBody(pub Attr, pub RowHeadColumns, pub Vec<Row>, pub Vec<Row>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TableFoot(pub Attr, pub Vec<Row>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum MathType {
    DisplayMath,
    InlineMath,
}

type Target = (Text, Text);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum ListNumberDelim {
    DefaultDelim,
    Period,
    OneParen,
    TwoParens,
}

type ShortCaption = Vec<Inline>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RowHeadColumns(pub Int);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum CitationMode {
    AuthorInText,
    SuppressAuthor,
    NormalCitation,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Cell(pub Attr, pub Alignment, pub RowSpan, pub ColSpan, pub Vec<Block>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RowSpan(pub Int);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ColSpan(pub Int);
