use super::types::{GenericDoc, IDGenericDoc, TagLike};
use super::crdt::GenericDocObject;
use super::CmRDT;

use crate::storage;

use markdown_ast as mdast;
use markdown_ast::{CodeBlockKind, HeadingLevel};
use pulldown_cmark::{Alignment, BlockQuoteKind, LinkType};

const BUF_SIZE: usize = 100 * (2 ^ 10); // 100KB

// == Data Structures ==
#[derive(Clone, Debug)]
pub enum Style {
    Emphasis,
    Strong,
    Strikethrough,
}

#[derive(Clone, Debug)]
pub enum MDLeaf {
    InlineText(String),
    InlineCode(String),
    SoftBreak,
    HardBreak,
    Rule,
    CodeBlock(CodeBlockKind, String),
}

#[derive(Clone, Debug)]
pub enum MDTag {
    Root,
    Paragraph,
    List,
    ListItem,
    Heading(HeadingLevel),
    BlockQuote(Option<BlockQuoteKind>),
    Table(Vec<Alignment>),
    TableRow,
    TableCell,
    StyledText(Style),
    Link{link_type: LinkType, dest: String, title: String, label: String},
}

pub type MDDoc = GenericDoc<MDTag, MDLeaf>;
pub type MDObject = GenericDocObject<MDTag, MDLeaf>;

// == Implementations ==
impl TagLike for MDTag {
    fn root() -> Self {
        Self::Root
    }
}

impl MDDoc {
    // Conversion from markdown_ast
    fn from_blocks(blocks: Vec<mdast::Block>) -> Vec<Self> {
        blocks.into_iter().map(|x| Self::from_block(x)).collect()
    }

    fn from_block(block: mdast::Block) -> Self {
        use mdast::Block as B;
        use MDTag as M;
        match block {
            B::Paragraph(inlines) => Self::Node{tag: M::Paragraph, children: Self::from_inlines(inlines)},
            B::List(items) => Self::Node{tag: M::List, children: Self::from_listitems(items)},
            B::Heading(l, inlines) => Self::Node{tag: M::Heading(l), children: Self::from_inlines(inlines)},
            B::CodeBlock{kind, code} => Self::Leaf{content: MDLeaf::CodeBlock(kind, code)},
            B::BlockQuote{kind, blocks} => Self::Node{tag: M::BlockQuote(kind), children: Self::from_blocks(blocks)},
            B::Table{alignments, headers, rows} => Self::Node{
                tag: M::Table(alignments),
                children: {
                    let mut v = rows.clone();
                    v.insert(0, headers);
                    v.into_iter().map(
                        |x| Self::Node{
                            tag: M::TableRow,
                            children: x.into_iter().map(
                                |y| Self::Node{
                                    tag: M::TableCell,
                                    children: Self::from_inlines(y)
                                }
                            ).collect()
                        }
                    ).collect()
                }
            },
            B::Rule => Self::Leaf{content: MDLeaf::Rule},
        }
    }

    fn from_listitems(items: Vec<mdast::ListItem>) -> Vec<Self> {
        items.into_iter().map(|x| Self::from_listitem(x)).collect()
    }

    fn from_listitem(item: mdast::ListItem) -> Self {
        Self::Node {
            tag: MDTag::ListItem,
            children: Self::from_blocks(item.0),
        }
    }

    fn from_inlines(inlines: mdast::Inlines) -> Vec<Self> {
        inlines.0.clone().into_iter().map(|x| Self::from_inline(x)).collect()
    }

    fn from_inline(inline: mdast::Inline) -> Self {
        use mdast::Inline as I;
        match inline {
            I::Text(string) => Self::Leaf{content: MDLeaf::InlineText(string)},
            I::Emphasis(inlines) => Self::Node{
                tag: MDTag::StyledText(Style::Emphasis),
                children: Self::from_inlines(inlines),
            },
            I::Strong(inlines) => Self::Node{
                tag: MDTag::StyledText(Style::Strong),
                children: Self::from_inlines(inlines),
            },
            I::Strikethrough(inlines) => Self::Node{
                tag: MDTag::StyledText(Style::Strikethrough),
                children: Self::from_inlines(inlines),
            },
            I::Code(string) => Self::Leaf{content: MDLeaf::InlineCode(string)},
            I::Link{link_type, dest_url, title, id, content_text} => Self::Node{
                tag: MDTag::Link{link_type, dest: dest_url, title, label: id},
                children: Self::from_inlines(content_text),
            },
            I::SoftBreak => Self::Leaf{content: MDLeaf::SoftBreak},
            I::HardBreak => Self::Leaf{content: MDLeaf::HardBreak},
        }
    }

    pub fn from_mdast(blocks: Vec<mdast::Block>) -> Self {
        Self::Node{
            tag: MDTag::Root,
            children: Self::from_blocks(blocks)
        }
    }

    // Conversion to markdown_ast
    fn to_inline(&self) -> mdast::Inline {
        match self {
            Self::Leaf{content} => match content {
                MDLeaf::InlineText(s) => mdast::Inline::Text(s.clone()),
                MDLeaf::InlineCode(s) => mdast::Inline::Code(s.clone()),
                MDLeaf::SoftBreak => mdast::Inline::SoftBreak,
                MDLeaf::HardBreak => mdast::Inline::HardBreak,
                _ => panic!("markdown: to_inline used on a Leaf that cannot be converted to an Inline."),
            },
            Self::Node{tag, children} => match tag {
                MDTag::StyledText(style) => match style {
                    Style::Emphasis => mdast::Inline::Emphasis(Self::to_inlines(children)),
                    Style::Strong => mdast::Inline::Strong(Self::to_inlines(children)),
                    Style::Strikethrough => mdast::Inline::Strikethrough(Self::to_inlines(children)),
                },
                MDTag::Link{link_type, dest, title, label} => mdast::Inline::Link {
                    link_type: link_type.clone(),
                    dest_url: dest.clone(),
                    title: title.clone(),
                    id: label.clone(),
                    content_text: Self::to_inlines(children),
                },
                _ => panic!("markdown: to_inline used on a Node that cannot be converted to an Inline."),
            },
        }
    }

    fn to_inlines(selves: &Vec<Self>) -> mdast::Inlines {
        mdast::Inlines(
            selves.into_iter().map(|x| x.to_inline()).collect()
        )
    }

    fn to_block(&self) -> mdast::Block {
        use mdast::Block as B;
        match self {
            Self::Node{tag, children} => match tag {
                MDTag::Paragraph => B::Paragraph(Self::to_inlines(children)),
                MDTag::List => B::List(Self::to_listitems(children)),
                MDTag::Heading(level) => B::Heading(*level, Self::to_inlines(children)),
                MDTag::BlockQuote(kind) => B::BlockQuote{kind: *kind, blocks: Self::to_blocks(children)},
                MDTag::Table(alignments) => B::Table{
                    alignments: alignments.clone(),
                    headers: children[0].row_to_vec(),
                    rows: children[1..].into_iter().map(|x| x.row_to_vec()).collect(),
                },
                _ => panic!(),
            },
            Self::Leaf{content} => match content {
                MDLeaf::Rule => B::Rule,
                MDLeaf::CodeBlock(kind, string) => B::CodeBlock{
                    kind: kind.clone(), code: string.clone(),
                },
                _ => panic!(),
            },
        }
    }

    fn to_blocks(selves: &Vec<Self>) -> Vec<mdast::Block> {
        selves.into_iter().map(|x| x.to_block()).collect()
    }

    fn to_listitem(&self) -> mdast::ListItem {
        if let Self::Node{tag: MDTag::ListItem, children} = self {
            mdast::ListItem(Self::to_blocks(children))
        } else {panic!("markdown: to_listitem can only be used on a Node{{ListItem, _}}.")}
    }

    fn to_listitems(selves: &Vec<Self>) -> Vec<mdast::ListItem> {
        selves.into_iter().map(|x| x.to_listitem()).collect()
    }

    fn row_to_vec(&self) -> Vec<mdast::Inlines> {
        if let Self::Node{tag: MDTag::TableRow, children} = self {
            children.into_iter().map(|x| x.cell_to_inlines()).collect()
        } else {panic!("markdown: row_to_vec can only be used on a Node{{TableRow, _}}.")}
    }

    fn cell_to_inlines(&self) -> mdast::Inlines {
        if let Self::Node{tag: MDTag::TableCell, children} = self {
            Self::to_inlines(children)
        } else {panic!("markdown: cell_to_inlines can only be used on a Node{{TableCell, _}}.")}
    }

    pub fn to_mdast(&self) -> Vec<mdast::Block> {
        if let Self::Node{tag: MDTag::Root, children} = self {
            Self::to_blocks(children)
        } else {panic!("markdown: to_mdast can only be used on a Root Node.")}
    }


}

impl CmRDT::DiskType for MDDoc {
    fn new() -> Self {
        Self::new()
    }

    fn read(config: &crate::default_storage::storage::Config, loc: &crate::default_storage::storage::ObjectLocation) -> Result<Box<Self>, std::io::Error> {
        let mut file = storage::ObjectFile::open(config, loc.clone())?;
        let mut buf = [0u8; BUF_SIZE];
        let bytes = file.read(&mut buf)?;

        if bytes == BUF_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "markdown: Buffer too small."));
        }

        let mdast = mdast::markdown_to_ast(&String::from_utf8_lossy(&buf));

        let md = Self::from_mdast(mdast);

        return Ok(Box::from(md));
    }

    fn write(&self, loc: &crate::default_storage::storage::ObjectLocation) -> Result<(), std::io::Error> {
        let mut f = if let storage::ObjectLocation::OnDisk(diskloc) = loc {
            storage::ObjectFile::create_on_disk(diskloc.clone())?
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "loc parameter must be an OnDisk."));
        };

        let mdast = self.to_mdast();
        let raw_md = mdast::ast_to_markdown(&mdast);
        f.write(raw_md.as_bytes());

        Ok(())
    }

    fn from_state(state: &Self::StateFormat) -> Self {
        state.strip_id()
    }

    type StateFormat = IDGenericDoc<MDTag, MDLeaf>;
}


