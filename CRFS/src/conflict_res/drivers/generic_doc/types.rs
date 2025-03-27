use super::CmRDT;
use crate::storage;
use storage::ObjectLocation;

use rand::Rng;

type ID = u128;
fn unique() -> ID {
    let mut i = rand::rng().random();
    while i == 0 {i = rand::rng().random()}
    return i;
}

// == Data Types ==
#[derive(Debug, Clone)]
pub enum GenericDoc<NodeTag, LeafType> {
    Node{tag: NodeTag, children: Vec<Self>},
    Leaf{content: LeafType},
}

#[derive(Debug, Clone)]
pub enum IDGenericDoc<NodeTag, LeafType> {
    Node{id: ID, tag: NodeTag, children: Vec<Self>},
    Leaf{id: ID, content: LeafType},
}

// == Traits ==
pub trait TagLike {
    // Get a tag representing a root node.
    fn root() -> Self;
}

// pub trait ToGenericDoc<NodeTag, LeafType> : CmRDT::DiskType {
//     fn to_generic_doc(&self) -> GenericDoc<NodeTag, LeafType>;
//     fn from_generic_doc(doc: &GenericDoc<NodeTag, LeafType>) -> Box<Self>;

//     fn read_genericdoc(config: &storage::Config, loc: &ObjectLocation) -> Result<GenericDoc<NodeTag, LeafType>, std::io::Error> {
//         let read_self = Self::read(config, loc)?;
//         Ok((*read_self).to_generic_doc())
//     }

//     fn write_genericdoc(doc: &GenericDoc<NodeTag, LeafType>, loc: &ObjectLocation) -> Result<(), std::io::Error> {
//         let convdoc = Self::from_generic_doc(doc);
//         return (*convdoc).write(loc);
//     }
// }

// == Implementations ==
impl<NodeTag, LeafType> GenericDoc<NodeTag, LeafType> where NodeTag: TagLike {
    pub fn new() -> Self {
        Self::Node {
            tag: NodeTag::root(),
            children: Vec::new(),
        }
    }
}

impl<NodeTag, LeafType> IDGenericDoc<NodeTag, LeafType> where NodeTag: Clone, LeafType: Clone {
    pub fn strip_id(&self) -> GenericDoc<NodeTag, LeafType> {
        match self {
            IDGenericDoc::Node{id: _, tag, children} => GenericDoc::Node{
                tag: tag.clone(),
                children: children.clone().into_iter().map(|x| x.strip_id()).collect(),
            },
            IDGenericDoc::Leaf{id: _, content} => GenericDoc::Leaf{
                content: content.clone()
            },
        }
    }
}

impl<NodeTag, LeafType> CmRDT::StateType for IDGenericDoc<NodeTag, LeafType> where NodeTag: Clone + TagLike, LeafType: Clone {

    fn new() -> Self {
        Self::Node{
            id: 0,
            tag: NodeTag::root(),
            children: Vec::new(),
        }
    }
}
