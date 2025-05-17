use super::CmRDT;
// use crate::storage;

use super::yata;

use std::collections::HashMap;
use rand::Rng;

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use uuid::Uuid;

// == Type Definitions ==
pub type ID = u128;

/// Generate a unique ID.
/// Assumes that ID has a large enough range of values that randomly choosing a value is enough to ensure uniqueness.
/// TODO: improve this
pub fn unique() -> ID {
    rand::rng().random()
}

/// Generate an ID in the range `(lower, upper)`.
/// This function is deterministic, unlike unique.
/// Given there will be `n` items between `lower, upper`, get an ID for the `i`th.
pub fn between(lower: ID, upper: ID, n: u128, i: u128) -> ID {
    let space = upper - lower;
    let delta = space / (n + 1);
    return lower + ((i + 1) * delta);
}

/// Container type for holding the set of Nodes in a document.
pub type Container<TagType, LeafType> = HashMap<ID, Node<TagType, LeafType>>;

pub type Children = yata::Array<ID, Uuid>;

// == Data Structures ==
// /// A "reference" to a child node.
// /// (i.e. not a reference in the technical sense, but does refer to a particular child node)
// #[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
// pub struct YATAChild{origin: ID, pub w: ID, deleted: bool}

// /// An ordered set of children. Designed to mask the underlying container.
// #[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Hash)]
// pub struct Children{children: Vec<YATAChild>}

/// A single node in the tree-like document.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Node<TagType, LeafType> {
    /// A Node which may have children. Contains no content itself, only a tag, of type `TagType`.
    Parent{
        id: ID,
        tag: TagType,
        children: Children,
    },
    /// A leaf node which contains content, of type `LeafType`.
    Leaf{
        id: ID,
        content: LeafType,
    },
}

/// A tree-like document.
/// Contains `items` - the container of every node in the document - and `root`, the ID of the root node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc<TagType, LeafType> {
    pub items: Container<TagType, LeafType>,
    pub root: ID,
}

// == Traits ==
/// Trait for Doc's TagType.
pub trait TagLike {
    fn root() -> Self;
}

/// Used to provide a single file on disk to a CRDT object.
pub trait FileInterface : CmRDT::DiskType<StateFormat = Doc<Self::TagType, Self::LeafType>> {
    type TagType: TagLike + Clone + Serialize + DeserializeOwned + Eq + std::hash::Hash;
    type LeafType: Clone + Serialize + DeserializeOwned + Eq + std::hash::Hash;

    /// Generate a StateFormat (usually a Doc) from the DiskInterface
    fn generate(&self, creator: Uuid) -> Self::StateFormat;

    /// Similar to `generate`, but with IDs matching those from `against`.
    fn generate_against(&self, against: &Self::StateFormat, creator: Uuid) -> Self::StateFormat;
}

// == Implementations ==
impl<TagType, LeafType> Node<TagType, LeafType> where TagType: Clone + TagLike + Eq + std::fmt::Debug, LeafType: Clone + Eq {
    pub fn root() -> Self {
        Self::Parent {
            id: 0, tag: TagType::root(), children: Children::empty(),
        }
    }

    pub fn get_id(&self) -> ID {
        match self {
            Self::Parent{id, tag: _, children: _} => *id,
            Self::Leaf{id, content: _} => *id,
        }
    }

    pub fn set_id(&mut self, w: ID) {
        match self {
            Self::Leaf {id, content: _} => {*id = w},
            Self::Parent {id, tag: _, children: _} => {*id = w},
        }
    }

    pub fn unwrap_parent(&self) -> (ID, &TagType, Vec<ID>) {
        match self {
            Self::Parent{id, tag, children} => (*id, tag, children.in_order_content_undel()),
            Self::Leaf{..} => panic!(),
        }
    }

    pub fn get_children(&self) -> &Children {
        match self {
            Self::Parent{id: _, tag: _, children} => children,
            Self::Leaf{..} => panic!(),
        }
    }

    pub fn get_mut_children(&mut self) -> &mut Children {
        match self {
            Self::Parent{id: _, tag: _, children} => children,
            Self::Leaf{..} => panic!(),
        }
    }

    pub fn get_children_owned(&self) -> Children {
        match self {
            Self::Parent{id: _, tag: _, children} => children.clone(),
            Self::Leaf{..} => Children::empty(),
        }
    }

    /// Check if equal to `other`, ignoring the ID field.
    pub fn eq_content(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Parent {id: _, tag: t1, children: _c1},
                Self::Parent {id: _, tag: t2, children: _c2}
            ) => t1 == t2, // && (c1.eq_content(c2)),
            (
                Self::Leaf {id: _, content: c1},
                Self::Leaf {id: _, content: c2}
            ) =>
                c1 == c2,
            _ => false,
        }
    }

    pub fn rename(&mut self, w_old: ID, w_new: ID) {
        if self.get_id() == w_old {self.set_id(w_new)}
        match self {
            Self::Leaf {..} => {},
            Self::Parent {children , ..} => {
                for id in children.in_order() {
                    if children[id].content == w_old {children[id].content = w_new;}
                }
            }
        }
    }
}

impl<TagType, LeafType> Doc<TagType, LeafType> where TagType: Clone + TagLike + Eq + std::fmt::Debug, LeafType: Clone + Eq {
    pub fn get_root_children(&self) -> &Children {
        self.items.get(&self.root).unwrap().get_children()
    }

    pub fn get_mut_root_children(&mut self) -> &mut Children {
        self.items.get_mut(&self.root).unwrap().get_mut_children()
    }

    fn add_node(&mut self, node: &Node<TagType, LeafType>) {
        self.items.insert(node.get_id(), node.clone());
    }

    pub fn rename_node(&mut self, w_old: ID, w_new: ID) {
        for (_, n) in self.items.iter_mut() {
            n.rename(w_old, w_new);
        }

        if let Some(n) = self.items.remove(&w_old) {
            self.items.insert(w_new, n);
        }
    }

    /// Find a node in self matching the content of `node`, and whose ID is not in `exclude`, and return its ID.
    pub fn match_node(&self, node: &Node<TagType, LeafType>, exclude: &Vec<ID>) -> Option<ID> {
        let mut ids = self.bottom_up(); // Use bottom up to ensure nodes are in-order.

        for id in self.items.keys() {
            if !ids.contains(id) {ids.push(*id);}
        }

        ids.retain(
            |id| self.items.get(id).unwrap().eq_content(node) && !exclude.contains(id)
        );

        return ids.into_iter().next();
    }

    /// Return all the IDs in the document tree, ordered bottom up and left-to-right.
    /// That is:
    /// - Any node with children is guaranteed to appear after its children.
    /// - Any node before another in the document, is guaranteed to appear before the other in this list.
    pub fn bottom_up(&self) -> Vec<ID> {
        let mut result = Vec::<ID>::new();
        let mut stack = Vec::<ID>::new();
        stack.push(self.root);

        'top: while !stack.is_empty() {
            let node = stack[stack.len() - 1];
            let children = self.items[&node].get_children_owned();

            // Check if node has unvisited children
            for c in children.in_order() {
                if !result.contains(&children[c].content) {
                    stack.push(children[c].content);
                    continue 'top;
                }
            }

            let top = stack.pop().unwrap();
            result.push(top);
        }
        return result;
    }

    pub fn bottom_up_refs(&self) -> Vec<(ID, &Node<TagType, LeafType>)> {
        let mut result = Vec::<ID>::new();
        let mut stack = Vec::<ID>::new();
        stack.push(self.root);

        'top: while !stack.is_empty() {
            let node = stack[stack.len() - 1];
            let children = self.items[&node].get_children_owned();

            // Check if node has unvisited children
            for c in children.in_order() {
                if !result.contains(&children[c].content) {
                    stack.push(children[c].content);
                    continue 'top;
                }
            }

            let top = stack.pop().unwrap();
            result.push(top);
        }

        return result.into_iter().map(
            |id| (id, self.items.get(&id).unwrap())
        ).collect();
    }
}

impl<TagType, LeafType> CmRDT::StateType for Doc<TagType, LeafType> where TagType: Clone + TagLike + Eq + std::fmt::Debug, LeafType: Clone + Eq {
    fn new() -> Self {
        let root = Node::root();
        let mut doc = Self{
            items: Container::new(), root: root.get_id(),
        };
        doc.items.insert(root.get_id(), root);
        return doc;
    }
}
