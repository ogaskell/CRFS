use super::types::{ID, Node, Doc, TagLike, FileInterface};
use super::CmRDT;
use CmRDT::{StateType, Operation};
use super::yata;
use super::super::file_tree::DriverID;
use crate::types::Hash;

use std::collections::VecDeque;

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use uuid::Uuid;

// == Data Structures ==
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocObject<DocFile> where DocFile: FileInterface {
    pub state: CmRDT::State<Doc<DocFile::TagType, DocFile::LeafType>>,
    pub hist: CmRDT::History,
    /// The last operation created locally.
    /// Used to satisfy in-order delivery.
    last_op: Option<Hash>,
    driverid: DriverID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DocOp<TagType, LeafType>
{
    // AddNode{node: Node<TagType, LeafType>, dep: Option<Hash>},
    DocAddParent{driverid: DriverID, w: ID, tag: TagType, w_parent: ID, i: yata::ID, ins: yata::Insertion<ID, Uuid>, dep: Option<Hash>},
    DocAddLeaf{driverid: DriverID, w: ID, content: LeafType, w_parent: ID, i: yata::ID, ins: yata::Insertion<ID, Uuid>, dep: Option<Hash>},
    DocInsChild{driverid: DriverID, w_parent: ID, i: yata::ID, ins: yata::Insertion<ID, Uuid>, dep: Option<Hash>},
    DocDelChild{driverid: DriverID, w_parent: ID, i: yata::ID, dep: Option<Hash>}, // i is a YATA ID
}

// == Implementations ==
impl<TagType, LeafType> DocOp<TagType, LeafType>
where
    TagType: TagLike + Serialize,
    LeafType: Serialize,
{
    fn get_dep(&self) -> Option<Hash> {
        match self {
            Self::DocAddParent{dep, ..} => *dep,
            Self::DocAddLeaf{dep, ..} => *dep,
            Self::DocInsChild{dep, ..} => *dep,
            Self::DocDelChild{dep, ..} => *dep,
        }
    }
}


// == Trait Impls ==
impl<TagType, LeafType> CmRDT::Operation for DocOp<TagType, LeafType>
where
    TagType: Clone + TagLike + Serialize + DeserializeOwned,
    LeafType: Clone + Serialize + DeserializeOwned
{
    fn get_driverid(&self) -> DriverID {
        match self {
            Self::DocAddParent {driverid, ..} => *driverid,
            Self::DocAddLeaf {driverid, ..} => *driverid,
            Self::DocInsChild {driverid, ..} => *driverid,
            Self::DocDelChild {driverid, ..} => *driverid,
        }
    }
}

impl<Interface> CmRDT::Object for DocObject<Interface> where Interface: FileInterface, <Interface as FileInterface>::TagType: std::fmt::Debug, <Interface as FileInterface>::LeafType: std::fmt::Debug {
    type StateFormat = Doc<Interface::TagType, Interface::LeafType>;
    type DiskFormat = Interface;
    type Op = DocOp<Interface::TagType, Interface::LeafType>;

    fn init(driverid: DriverID) -> Self {
        let mut new = Self {
            state: CmRDT::State::new(),
            hist: CmRDT::History::new(),
            last_op: None,
            driverid,
        };

        new.state.insert(new.hist.k, Self::StateFormat::new());

        return new;
    }

    fn get_driverid(&self) -> DriverID {
        return self.driverid;
    }

    fn query_internal(&self) -> &Self::StateFormat {
        return &self.state[&self.hist.k];
    }

    fn prep(&self, data: &Self::DiskFormat, replica_id: Uuid) -> Option<Self::Op> {
        // Tree Diff
        let old_state = self.query_internal();
        let mut new_state = data.generate_against(old_state, replica_id);

        let mut queue = VecDeque::new(); queue.push_back(old_state.root);

        while let Some(current) = queue.pop_front() {
            // Assume current exists in both states, as it has been created by a previous call to this method.
            // Provable inductively - base case root (always exists in both), inductive case
            // since we ensure all children of a node exist before moving down the tree (BFS)

            match &old_state.items[&current] {
                Node::Parent {children, ..} => {
                    let new_children = new_state.items.get_mut(&current).unwrap().get_mut_children();

                    let op = children.get_op(new_children, replica_id);

                    match op {
                        None => {for c in children.in_order_content_undel() {queue.push_back(c);}},
                        Some(yata::Op::Deletion(i)) => {return Some(
                            Self::Op::DocDelChild {w_parent: current, i, dep: self.last_op, driverid: self.driverid}
                        )},
                        Some(yata::Op::Insertion(i, ins)) => {
                            let new_node = &new_state.items[&ins.content];
                            match new_node {
                                Node::Parent {id: new_id, tag: new_tag, ..} => {
                                    return Some(Self::Op::DocAddParent {
                                        w_parent: current, tag: new_tag.clone(), w: *new_id, i, ins, dep: self.last_op, driverid: self.driverid,
                                    })
                                },
                                Node::Leaf {id: new_id, content} => {
                                    return Some(Self::Op::DocAddLeaf {
                                        w: *new_id, content: content.clone(), w_parent: current, i, ins, dep: self.last_op, driverid: self.driverid,
                                    })
                                },
                            }
                        },
                    }
                },
                Node::Leaf {..} => {},
            }
        }

        return None;
    }

    fn apply(&mut self, op: &Self::Op) -> Option<Self::StateFormat> {
        if !self.precond(op) {return None}

        let mut new_state = self.query_internal().clone();

        match op {
            DocOp::DocAddParent {w, tag, w_parent, i, ins, ..} => {
                new_state.items.insert(*w, Node::Parent {
                    id: *w, tag: tag.clone(),
                    children: super::types::Children::empty(),
                });

                let children = new_state.items.get_mut(w_parent).unwrap().get_mut_children();
                children.insert(*ins, Some(*i));
            },
            DocOp::DocAddLeaf {w, content, w_parent, i, ins, ..} => {
                new_state.items.insert(*w, Node::Leaf {
                    id: *w, content: content.clone(),
                });

                let children = new_state.items.get_mut(w_parent).unwrap().get_mut_children();
                children.insert(*ins, Some(*i));
            }
            DocOp::DocInsChild {w_parent, i, ins, ..} => {
                let children = new_state.items.get_mut(w_parent).unwrap().get_mut_children();
                children.insert(*ins, Some(*i));
            },
            DocOp::DocDelChild {w_parent, i, ..} => {
                let children = new_state.items.get_mut(w_parent).unwrap().get_mut_children();
                children.delete(*i);
            }
        };

        return Some(new_state);
    }

    fn apply_op(&mut self, op: &Self::Op) -> Option<()> {
        let new_state = self.apply(op)?;
        self.log_op(op.to_history(), new_state);
        self.last_op = Some(op.get_hash());
        Some(())
    }

    fn precond(&self, op: &Self::Op) -> bool {
        if let Some(hash) = op.get_dep() {
            if !self.hist.contains(hash) {
                return false;
            }
        }

        return true;
    }

    fn append_history(&mut self, hist_obj: CmRDT::HistoryItem) -> CmRDT::K {
        self.hist.add(hist_obj)
    }

    fn set_state(&mut self, k: CmRDT::K, state: Self::StateFormat) -> () {
        self.state.insert(k, state);
    }
}
