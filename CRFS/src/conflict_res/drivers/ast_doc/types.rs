use super::CmRDT;
// use crate::storage;

use std::collections::HashMap;
use rand::{distr::uniform::SampleBorrow, Rng};

use serde::{Serialize, Deserialize, de::DeserializeOwned};

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
    dbg!(lower, upper, n, i);
    let space = upper - lower; dbg!(space);
    let delta = space / (n + 1); dbg!(delta);
    dbg!(lower + ((i + 1) * delta));
    return lower + ((i + 1) * delta);
}

/// Container type for holding the set of Nodes in a document.
type Container<TagType, LeafType> = HashMap<ID, Node<TagType, LeafType>>;

// == Data Structures ==
/// A "reference" to a child node.
/// (i.e. not a reference in the technical sense, but does refer to a particular child node)
#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct Child{i: ID, pub w: ID}

/// An ordered set of children. Designed to mask the underlying container.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Hash)]
pub struct Children{children: Vec<Child>}

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
#[derive(Clone, Debug, PartialEq)]
pub struct Doc<TagType, LeafType> {
    pub items: Container<TagType, LeafType>,
    pub root: ID,
}

// == Traits ==
/// Trait for Doc's TagType.
pub trait TagLike {
    fn root() -> Self;
}

/// Used to interface between a CRDT object and files on disk.
pub trait DiskInterface : CmRDT::DiskType<StateFormat = Doc<Self::TagType, Self::LeafType>> {
    type TagType: TagLike + Clone + Serialize + DeserializeOwned + Eq + std::hash::Hash;
    type LeafType: Clone + Serialize + DeserializeOwned + Eq + std::hash::Hash;

    fn generate(&self) -> Self::StateFormat;
    fn generate_against(&self, doc: &Self::StateFormat) -> Self::StateFormat;
}

// == Implementations ==
impl Children {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    pub fn in_order(&self) -> Vec<Child> {
        let mut vec: Vec<Child> = self.children.iter().map(|x|x.clone()).collect();
        vec.sort();
        return vec;
    }

    pub fn remove(&mut self, w: ID) {
        self.children.retain(|child| child.w != w);
    }

    pub fn insert(&mut self, i: ID, w: ID) {
        self.children.push(Child{i, w});
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }
}

impl<TagType, LeafType> Node<TagType, LeafType> where TagType: Clone + TagLike, LeafType: Clone {
    pub fn root() -> Self {
        Self::Parent {
            id: 0, tag: TagType::root(), children: Children::new(),
        }
    }

    pub fn get_id(&self) -> ID {
        match self {
            Self::Parent{id, tag: _, children: _} => *id,
            Self::Leaf{id, content: _} => *id,
        }
    }

    pub fn unwrap_parent(&self) -> (ID, &TagType, Vec<Child>) {
        match self {
            Self::Parent{id, tag, children} => (*id, tag, children.in_order()),
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
}

impl<TagType, LeafType> Doc<TagType, LeafType> where TagType: Clone + TagLike, LeafType: Clone {
    pub fn new() -> Self {
        let root = Node::root();
        let mut doc = Self{
            items: Container::new(), root: root.get_id(),
        };
        doc.items.insert(root.get_id(), root);
        return doc;
    }

    pub fn get_root_children(&self) -> &Children {
        self.items.get(&self.root).unwrap().get_children()
    }

    pub fn get_mut_root_children(&mut self) -> &mut Children {
        self.items.get_mut(&self.root).unwrap().get_mut_children()
    }

    fn add_node(&mut self, node: &Node<TagType, LeafType>) {
        self.items.insert(node.get_id(), node.clone());
    }
}

impl<TagType, LeafType> CmRDT::StateType for Doc<TagType, LeafType> where TagType: Clone + TagLike, LeafType: Clone {
    fn new() -> Self {
        Self::new()
    }
}

// == Trait Impls ==
impl PartialOrd for Child {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for Child {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.i.cmp(&other.i) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Greater =>  std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => self.w.cmp(&other.w),
        }
    }
}
