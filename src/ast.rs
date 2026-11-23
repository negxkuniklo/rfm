#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub frontmatter: Option<String>,
    pub blocks: Vec<Block>,
    pub footnotes: Vec<FootnoteDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootnoteDef {
    pub label: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    HorizontalRule,
    PageBreak,
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    QuoteSingle(Vec<Inline>),
    QuoteMulti {
        alt: Option<String>,
        blocks: Vec<Block>,
    },
    Details {
        alt: Vec<Inline>,
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    Table {
        alignments: Vec<Alignment>,
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    LineBreak,
    Italic(Vec<Inline>),
    Bold(Vec<Inline>),
    BoldItalic(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link {
        title: Option<Vec<Inline>>,
        url: String,
    },
    Image {
        alt: String,
        width: Option<u32>,
        path: String,
    },
    FootnoteRef(String),
    HtmlBr,
}
