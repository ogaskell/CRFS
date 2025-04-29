use super::types::{Doc, FileInterface, TagLike, Children, ID, unique, between, Node};
use super::crdt::DocObject;

use super::CmRDT;
use super::CmRDT::{StateType, DiskType, Object};
use super::super::driver::Driver;
use crate::storage;

use markdown_ast as mdast;
use markdown_ast::{CodeBlockKind as CodeBlockKind_, HeadingLevel};
use pulldown_cmark::{Alignment as Alignment_, BlockQuoteKind, LinkType as LinkType_};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

// Re-implementations of types from markdown_ast and pulldown_cmark
// Needed so I can add traits like Hash
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeBlockKind {
    Fenced(String),
    Indented,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LinkType {
    Inline,
    Reference,
    ReferenceUnknown,
    Collapsed,
    CollapsedUnknown,
    Shortcut,
    ShortcutUnknown,
    Autolink,
    Email,
}

impl From<CodeBlockKind_> for CodeBlockKind {
    fn from(item: CodeBlockKind_) -> Self {
        match item {
            CodeBlockKind_::Indented => Self::Indented,
            CodeBlockKind_::Fenced(s) => Self::Fenced(s),
        }
    }
}
impl Into<CodeBlockKind_> for CodeBlockKind {
    fn into(self) -> CodeBlockKind_ {
        match self {
            Self::Indented => CodeBlockKind_::Indented,
            Self::Fenced(s) => CodeBlockKind_::Fenced(s),
        }
    }
}
impl From<Alignment_> for Alignment {
    fn from(item: Alignment_) -> Self {
        match item {
            Alignment_::None => Self::None,
            Alignment_::Left => Self::Left,
            Alignment_::Center => Self::Center,
            Alignment_::Right => Self::Right,
        }
    }
}
impl Into<Alignment_> for Alignment {
    fn into(self) -> Alignment_ {
        match self {
            Self::None => Alignment_::None,
            Self::Left => Alignment_::Left,
            Self::Center => Alignment_::Center,
            Self::Right => Alignment_::Right,
        }
    }
}
impl From<LinkType_> for LinkType {
    fn from(item: LinkType_) -> Self {
        match item {
            LinkType_::Inline => Self::Inline,
            LinkType_::Reference => Self::Reference,
            LinkType_::ReferenceUnknown => Self::ReferenceUnknown,
            LinkType_::Collapsed => Self::Collapsed,
            LinkType_::CollapsedUnknown => Self::CollapsedUnknown,
            LinkType_::Shortcut => Self::Shortcut,
            LinkType_::ShortcutUnknown => Self::ShortcutUnknown,
            LinkType_::Autolink => Self::Autolink,
            LinkType_::Email => Self::Email,
        }
    }
}
impl Into<LinkType_> for LinkType {
    fn into(self) -> LinkType_ {
        match self {
            Self::Inline => LinkType_::Inline,
            Self::Reference => LinkType_::Reference,
            Self::ReferenceUnknown => LinkType_::ReferenceUnknown,
            Self::Collapsed => LinkType_::Collapsed,
            Self::CollapsedUnknown => LinkType_::CollapsedUnknown,
            Self::Shortcut => LinkType_::Shortcut,
            Self::ShortcutUnknown => LinkType_::ShortcutUnknown,
            Self::Autolink => LinkType_::Autolink,
            Self::Email => LinkType_::Email,
        }
    }
}

// == Markdown Documents ==
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Style {
    Emphasis,
    Strong,
    Strikethrough,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MDLeaf {
    InlineText(String),
    InlineCode(String),
    SoftBreak,
    HardBreak,
    Rule,
    CodeBlock(CodeBlockKind, String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

type MDDoc = Doc<MDTag, MDLeaf>;

#[derive(Debug)]
pub struct MDInterface {
    pub mdast: Vec<mdast::Block>,
}

impl TagLike for MDTag {
    fn root() -> Self {
        Self::Root
    }
}

impl DiskType for MDInterface {
    type StateFormat = MDDoc;

    fn new() -> Self {
        Self {
            mdast: Vec::new(),
        }
    }

    fn read(loc: &storage::ObjectLocation) -> Result<Box<Self>, std::io::Error> {
        let mut file = storage::ObjectFile::open(loc)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        let mdast = mdast::markdown_to_ast(&buf);

        return Ok(Box::new(Self{mdast}));
    }

    fn write(&self, loc: &storage::ObjectLocation) -> Result<(), std::io::Error> {
        let raw_md = mdast::ast_to_markdown(&self.mdast);
        return storage::ObjectFile::create(&loc, raw_md.as_bytes());
    }

    fn from_state(state: &Self::StateFormat) -> Self {
        let children = state.get_root_children();
        return Self {
            mdast: Self::to_blocks(children, state),
        };
    }
}

impl FileInterface for MDInterface {
    type TagType = MDTag;
    type LeafType = MDLeaf;

    fn generate(&self, creator: Uuid) -> Self::StateFormat {
        let mut doc = MDDoc::new();
        let blocks = Self::gen_blocks(&self.mdast, &mut doc, creator);

        let children = doc.get_mut_root_children();
        (*children) = blocks;

        return doc;
    }

    fn generate_against(&self, against: &Self::StateFormat, creator: Uuid) -> Self::StateFormat {
        let mut new_doc = self.generate(creator);

        for w in new_doc.bottom_up().iter() {
            let ids = new_doc.bottom_up();
            if let Some(w_new) = against.match_node(new_doc.items.get(w).unwrap(), &ids) {
                new_doc.rename_node(*w, w_new);
            }
        }

        for (_, node) in new_doc.items.iter_mut() {
            match node {
                Node::Parent{id, children, ..} => {
                    if let Some(other) = against.items.get(&id) {
                        children.rename_against(other.get_children());
                        children.rename_creators(other.get_children());
                    }
                },
                _ => {},
            }
        }

        // dbg!(&against.items.keys());
        // dbg!(&new_doc.items.keys());

        return new_doc;
    }
}

impl MDInterface {
    pub fn get_canon(&self) -> String {
        return markdown_ast::ast_to_markdown(&self.mdast);
    }

    /// Given a vector of MDAST blocks, convert them to Nodes, add these to `doc`, and return a Children object.
    fn gen_blocks(blocks: &Vec<mdast::Block>, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> Children {
        let ids: Vec<ID> = blocks.iter().map(|b| Self::from_block(b, doc, uuid)).collect();
        let children = Children::from((ids.into_iter(), uuid));

        return children;
    }

    /// Given an MDAST block, convert it to a Node, add it to `doc`, and return its ID.
    fn from_block(block: &mdast::Block, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> ID {
        use mdast::Block as B;
        use MDTag as M;

        let id = unique();

        let node: Node<MDTag, MDLeaf> = match block {
            B::Paragraph(inlines) => Node::Parent{
                id, tag: M::Paragraph, children: Self::gen_inlines(inlines, doc, uuid)
            },
            B::List(items) => Node::Parent{
                id, tag: M::List, children: Self::gen_listitems(items, doc, uuid)
            },
            B::Heading(l, inlines) => Node::Parent{
                id, tag: M::Heading(*l), children: Self::gen_inlines(inlines, doc, uuid)
            },
            B::CodeBlock{kind, code} => Node::Leaf{
                id, content: MDLeaf::CodeBlock(kind.clone().into(), code.clone())
            },
            B::BlockQuote{kind, blocks} => Node::Parent{
                id, tag: M::BlockQuote(*kind), children: Self::gen_blocks(blocks, doc, uuid)
            },
            B::Table{alignments, headers, rows} => Node::Parent{
                id,
                tag: M::Table(alignments.into_iter().map(|x|(*x).into()).collect()),
                children: Self::gen_table(headers, rows, doc, uuid)
            },
            B::Rule => Node::Leaf{
                id, content: MDLeaf::Rule
            },
        };

        doc.items.insert(id, node);

        return id;
    }

    /// Given an MDAST Inlines (collection of `Inline` objects), convert them to Nodes, add these to `doc`, and return a Children object.
    fn gen_inlines(inlines: &mdast::Inlines, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> Children {
        let items: Vec<_> = inlines.0.iter().map(|i| Self::from_inline(i, doc, uuid)).collect();
        let children = Children::from((items.into_iter(), uuid));

        return children;
    }

    fn from_inline(inline: &mdast::Inline, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> ID {
        use mdast::Inline as I;

        let id = unique();

        let node = match inline {
            I::Text(string) => Node::Leaf{
                id, content: MDLeaf::InlineText(string.clone())
            },
            I::Emphasis(inlines) => Node::Parent{
                id,
                tag: MDTag::StyledText(Style::Emphasis),
                children: Self::gen_inlines(inlines, doc, uuid),
            },
            I::Strong(inlines) => Node::Parent{
                id,
                tag: MDTag::StyledText(Style::Strong),
                children: Self::gen_inlines(inlines, doc, uuid),
            },
            I::Strikethrough(inlines) => Node::Parent{
                id,
                tag: MDTag::StyledText(Style::Strikethrough),
                children: Self::gen_inlines(inlines, doc, uuid),
            },
            I::Code(string) => Node::Leaf{
                id, content: MDLeaf::InlineCode(string.clone())
            },
            I::Link{link_type, dest_url, title, id: link_id, content_text} => Node::Parent{
                id,
                tag: MDTag::Link{link_type: (*link_type).into(), dest: dest_url.clone(), title: title.clone(), label: link_id.clone()},
                children: Self::gen_inlines(content_text, doc, uuid),
            },
            I::SoftBreak => Node::Leaf{
                id, content: MDLeaf::SoftBreak
            },
            I::HardBreak => Node::Leaf{
                id, content: MDLeaf::HardBreak
            },
        };

        doc.items.insert(id, node);

        return id;
    }

    /// Given a vec of MDAST ListItems, convert them to Nodes, add these to `doc`, and return a Children object.
    fn gen_listitems(items: &Vec<mdast::ListItem>, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> Children {
        let ids: Vec<_> = items.iter().map(|i| Self::from_listitem(i, doc, uuid)).collect();
        let children = Children::from((ids.into_iter(), uuid));

        return children;
    }

    fn from_listitem(item: &mdast::ListItem, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> ID {
        let id = unique();
        let children = Self::gen_blocks(&item.0, doc, uuid);
        doc.items.insert(id, Node::Parent {
            id,
            tag: MDTag::ListItem,
            children,
        });
        return id;
    }

    /// Given the headers and rows of a table, convert to Nodes as necessary, add these to `doc`, and return a Children object.
    fn gen_table(headers: &Vec<mdast::Inlines>, rows: &Vec<Vec<mdast::Inlines>>, doc: &mut <Self as DiskType>::StateFormat, uuid: Uuid) -> Children {
        let mut children = Vec::new();

        let header_iter = Vec::from([headers]).into_iter();
        let rows_iter = rows.iter();

        let all_iter = header_iter.chain(rows_iter);

        for row in all_iter {
            let mut row_children = Vec::new();

            for cell in row.iter() {
                // Create node for this cell
                let cell_id = unique();
                let cell_node = Node::Parent{
                    id: cell_id, tag: MDTag::TableCell, children: Self::gen_inlines(&cell, doc, uuid),
                };

                // Store in doc
                doc.items.insert(cell_id, cell_node);

                // Add to row's children
                row_children.push(cell_id);
            }

            let row_id = unique();
            let row_node = Node::Parent{
                id: row_id, tag: MDTag::TableRow, children: Children::from((row_children.into_iter(), uuid)),
            };

            doc.items.insert(row_id, row_node);
            children.push(row_id);
        }

        return Children::from((children.into_iter(), uuid));
    }

    /// Convert a Children object to a Vec of MDAST blocks.
    pub fn to_blocks(children: &Children, doc: &<Self as DiskType>::StateFormat) -> Vec<mdast::Block> {
        children.in_order_content_undel().into_iter()
            .map(|x| Self::to_block(doc.items.get(&x).unwrap(), doc))
            .collect()
    }

    fn to_block(node: &Node<MDTag, MDLeaf>, doc: &<Self as DiskType>::StateFormat) -> mdast::Block {
        use mdast::Block as B;
        match node {
            Node::Parent{id: _, tag, children} => match tag {
                MDTag::Paragraph => B::Paragraph(Self::to_inlines(children, doc)),
                MDTag::List => B::List(Self::to_listitems(children, doc)),
                MDTag::Heading(level) => B::Heading(*level, Self::to_inlines(children, doc)),
                MDTag::BlockQuote(kind) => B::BlockQuote{kind: *kind, blocks: Self::to_blocks(children, doc)},
                MDTag::Table(alignments) => {
                    let (headers, rows) = Self::to_table(children, doc);
                    B::Table{
                        alignments: alignments.into_iter().map(|x| x.clone().into()).collect(),
                        headers, rows,
                        // headers: children[0].row_to_vec(),
                        // rows: children[1..].into_iter().map(|x| x.row_to_vec()).collect(),
                    }
                },
                _ => panic!(),
            },
            Node::Leaf{id: _, content} => match content {
                MDLeaf::Rule => B::Rule,
                MDLeaf::CodeBlock(kind, string) => B::CodeBlock{
                    kind: kind.clone().into(), code: string.clone(),
                },
                _ => panic!(),
            },
        }
    }

    fn to_inlines(children: &Children, doc: &<Self as DiskType>::StateFormat) -> mdast::Inlines {
        mdast::Inlines(
            children.in_order_content_undel().into_iter()
                .map(|x| Self::to_inline(doc.items.get(&x).unwrap(), doc))
                .collect()
        )
    }

    fn to_inline(node: &Node<MDTag, MDLeaf>, doc: &<Self as DiskType>::StateFormat) -> mdast::Inline {
        match node {
            Node::Leaf{id: _, content} => match content {
                MDLeaf::InlineText(s) => mdast::Inline::Text(s.clone()),
                MDLeaf::InlineCode(s) => mdast::Inline::Code(s.clone()),
                MDLeaf::SoftBreak => mdast::Inline::SoftBreak,
                MDLeaf::HardBreak => mdast::Inline::HardBreak,
                _ => panic!("markdown: to_inline used on a Leaf that cannot be converted to an Inline."),
            },
            Node::Parent{id: _, tag, children} => match tag {
                MDTag::StyledText(style) => match style {
                    Style::Emphasis => mdast::Inline::Emphasis(Self::to_inlines(children, doc)),
                    Style::Strong => mdast::Inline::Strong(Self::to_inlines(children, doc)),
                    Style::Strikethrough => mdast::Inline::Strikethrough(Self::to_inlines(children, doc)),
                },
                MDTag::Link{link_type, dest, title, label} => mdast::Inline::Link {
                    link_type: link_type.clone().into(),
                    dest_url: dest.clone(),
                    title: title.clone(),
                    id: label.clone(),
                    content_text: Self::to_inlines(children, doc),
                },
                _ => panic!("markdown: to_inline used on a Node that cannot be converted to an Inline."),
            },
        }
    }

    fn to_listitems(children: &Children, doc: &<Self as DiskType>::StateFormat) -> Vec<mdast::ListItem> {
        children.in_order_content_undel().into_iter()
            .map(|x| Self::to_listitem(doc.items.get(&x).unwrap(), doc))
            .collect()
    }

    fn to_listitem(node: &Node<MDTag, MDLeaf>, doc: &<Self as DiskType>::StateFormat) -> mdast::ListItem {
        if let Node::Parent{id: _, tag: MDTag::ListItem, children} = node {
            mdast::ListItem(Self::to_blocks(children, doc))
        } else {panic!("markdown: to_listitem can only be used on a Node{{ListItem, _}}.")}
    }

    fn to_table(children: &Children, doc: &<Self as DiskType>::StateFormat) -> (Vec<mdast::Inlines>, Vec<Vec<mdast::Inlines>>) {
        let mut v = children.in_order_content_undel();
        let rows = v.split_off(1);
        let heading = doc.items.get(&(v[0])).unwrap();

        let heading_ = Self::row_to_vec(heading, doc);
        let rows_ = rows.iter()
            .map(
                |x| Self::row_to_vec(doc.items.get(&x).unwrap(), doc)
            ).collect();

        return (heading_, rows_);
    }

    fn row_to_vec(row: &Node<MDTag, MDLeaf>, doc: &<Self as DiskType>::StateFormat) -> Vec<mdast::Inlines> {
        if let Node::Parent{id: _, tag: MDTag::TableRow, children} = row {
            children.in_order_content_undel().iter().map(
                |x| Self::cell_to_inlines(doc.items.get(&x).unwrap(), doc)
            ).collect()
        } else {panic!("markdown: row_to_vec can only be used on a Node{{TableRow, _}}.")}
    }

    fn cell_to_inlines(cell: &Node<MDTag, MDLeaf>, doc: &<Self as DiskType>::StateFormat) -> mdast::Inlines {
        if let Node::Parent{id: _, tag: MDTag::TableCell, children} = cell {
            Self::to_inlines(children, doc)
        } else {panic!("markdown: cell_to_inlines can only be used on a Node{{TableCell, _}}.")}
    }
}

pub type MDObject = DocObject<MDInterface>;

pub struct MDDriver {
    object: MDObject,
    loc: storage::ObjectLocation,
    uuid: Uuid,
}

impl Driver<MDObject> for MDDriver {
    fn check(loc: &storage::ObjectLocation) -> bool {
        loc.extension() == Some(String::from("md"))
    }

    fn new(loc: &storage::ObjectLocation, uuid: Uuid) -> Self {
        Self {
            object: MDObject::init(),
            loc: loc.clone(),
            uuid,
        }
    }

    fn get_history(&self) -> CmRDT::History {
        return self.object.hist.clone();
    }

    fn update(&mut self) -> Result<(), crate::errors::Error> {
        let latest_state = *MDInterface::read(&self.loc)?;

        while let Some(op) = self.object.prep(&latest_state, self.uuid) {
            self.object.apply_op(&op).unwrap(); // Use unwrap here, since there is no reason a just-prepped update doesn't apply.
            // If a just-prepped update doesn't apply, then something has gone very wrong!! We *must* always immediately `apply` after a `prep`.
        }

        return Ok(());
    }

    /// Operations may have dependencies that do not align with their order in `ops`.
    /// As such, we iterate over `ops`, attempting to apply every operation, until they are all applied.
    /// If in a single iteration, nothing applies, then stop and throw an error.
    /// This error will include a hashset of the indices of the applied operations.
    fn apply(&mut self, ops: Vec<&<DocObject<MDInterface> as CmRDT::Object>::Op>) -> Result<(), std::collections::HashSet<usize>> {
        let mut applied = std::collections::HashSet::new();
        let mut last_n_applied = 0usize;

        while applied.len() < ops.len() {
            for (i, op) in ops.iter().enumerate() {
                // If op hasn't been applied yet
                if !applied.contains(&i) {
                    // Attempt to apply op
                    match self.object.apply_op(op) {
                        Some(()) => {applied.insert(i);},
                        None => {},
                    }
                }
            }

            if applied.len() <= last_n_applied {
                return Err(applied);
            }
            last_n_applied = applied.len();
        }

        Ok(())
    }
}
