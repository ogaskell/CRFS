// Directed Graph CmRDT, adapted from https://pages.lip6.fr/Marc.Shapiro/papers/RR-7687.pdf#page=15

use crate::storage;
use storage::object;

use super::file_tree::DriverID;
use super::CmRDT::{self, StateType};

use std::collections::HashSet;
use std::hash;

use rand::Rng;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_json;
use uuid::Uuid;

type Tag = u64;

fn unique() -> Tag {
    rand::rng().random()
}

// == Data Types ==
// Disk Format
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Graph<T: Clone + Eq + hash::Hash> {
    pub v: HashSet<T>,
    pub a: HashSet<(T, T)>,
}

// State Format
#[derive(Clone, Debug)]
pub struct TaggedGraph<T: Clone> {
    v: HashSet<(T, Tag)>,
    a: HashSet<((T, T), Tag)>,
}

// Operation Format
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum GraphOp<T: Clone + Eq + hash::Hash> {
    AddVertex(T, Tag),
    RemoveVertex(T, HashSet<(T, Tag)>),
    AddArc((T, T), Tag),
    RemoveArc((T, T), HashSet<((T, T), Tag)>),
}

// == CmRDT Trait Impls ==
impl<T> CmRDT::DiskType for Graph<T> where T: Clone + Eq + hash::Hash + Serialize + DeserializeOwned {
    fn new() -> Self {
        Self {
            v: HashSet::new(),
            a: HashSet::new(),
        }
    }

    fn read(config: &storage::Config, loc: &object::Location) -> std::io::Result<Box<Self>> {
        let mut buf = String::new();
        object::read_string(config, loc, &mut buf)?;

        let obj = serde_json::from_str(&buf)?;
        return Ok(Box::new(obj));
    }

    fn write(&self, config: &storage::Config, loc: &object::Location) -> std::io::Result<()> {
        let json = serde_json::to_string(self)?;
        return object::write(config, loc, json.as_bytes());
    }

    type StateFormat = TaggedGraph<T>;
    fn from_state(graph: &Self::StateFormat) -> Self {
        Self {
            v: graph.v.clone().into_iter().map(|(v, _)| v).collect(),
            a: graph.a.clone().into_iter().map(|(a, _)| a).collect(),
        }
    }
}

impl<T> CmRDT::StateType for TaggedGraph<T> where T: Clone {
    fn new() -> Self {
        Self {
            v: HashSet::new(),
            a: HashSet::new(),
        }
    }
}

impl<T> CmRDT::Operation for GraphOp<T> where T: Clone + Eq + hash::Hash + Serialize + DeserializeOwned {
    fn get_driverid(&self) -> DriverID {todo!();}
}

// == CmRDT Implementation ==
#[derive(Debug)]
pub struct GraphObject<T: Clone> {
    state: CmRDT::State<TaggedGraph<T>>,
    hist: CmRDT::History,
    driverid: DriverID,
}

impl<T> CmRDT::Object for GraphObject<T> where T: Clone + Eq + hash::Hash + Serialize + DeserializeOwned + std::fmt::Debug {
    type StateFormat = TaggedGraph<T>;
    type DiskFormat = Graph<T>;
    type Op = GraphOp<T>;

    fn init(driverid: DriverID) -> Self {
        Self {
            state: CmRDT::State::from([(0, Self::StateFormat::new())]),
            hist: CmRDT::History::new(),
            driverid,
        }
    }

    fn get_driverid(&self) -> DriverID {
        return self.driverid;
    }

    fn query_internal(&self) -> &Self::StateFormat {
        return &self.state[&self.hist.k];
    }

    fn prep(&self, data: &Self::DiskFormat, _: Uuid) -> Option<Self::Op> {
        let state = self.query_internal().clone();
        let untagged_state = self.query();

        // dbg!(&untagged_state);
        // dbg!(&data);

        // Look for new vertices
        let mut new_vertices = data.v.difference(&untagged_state.v);

        match new_vertices.next() {
            Some(new_vertex) => {return Some(GraphOp::AddVertex(new_vertex.clone(), unique()));},
            None => {},
        };

        // Look for removed vertices
        let mut rem_vertices = untagged_state.v.difference(&data.v);

        match rem_vertices.next() {
            Some(rem_vertex) => {
                return Some(GraphOp::RemoveVertex(
                    rem_vertex.clone(),
                    state.clone().v.into_iter().filter(
                        |(v, _)| v == rem_vertex
                    ).collect()
                ));
            },
            None => {},
        };

        // Look for new arcs
        let mut new_arcs = data.a.difference(&untagged_state.a);

        match new_arcs.next() {
            Some(new_arc) => {return Some(GraphOp::AddArc(new_arc.clone(), unique()))},
            None => {},
        };

        // Look for removed arcs

        let mut rem_arcs = untagged_state.a.difference(&data.a);

        match rem_arcs.next() {
            Some(rem_arc) => {
                return Some(GraphOp::RemoveArc(
                    rem_arc.clone(),
                    state.clone().a.into_iter().filter(
                        |(a, _)| a == rem_arc
                    ).collect()
                ))
            },
            None => {},
        };

        return None;
    }

    fn apply(&mut self, op: &Self::Op) -> Option<Self::StateFormat> {
        if !self.precond(op) {
            return None;
        }

        let mut s = self.query_internal().clone();

        match op {
            GraphOp::AddVertex(v, w) => {
                s.v.insert((v.clone(), *w));
            },
            GraphOp::RemoveVertex(_, set) => {
                s.v = s.v.difference(&set).cloned().collect();
            },
            GraphOp::AddArc((v1, v2), w) => {
                s.a.insert(((v1.clone(), v2.clone()), *w));
            },
            GraphOp::RemoveArc(_, set) => {
                s.a = s.a.difference(&set).cloned().collect();
            },
        };

        return Some(s);
    }

    fn precond(&self, op: &Self::Op) -> bool {
        let state = self.query();
        match op {
            GraphOp::AddVertex(_, _) => true,
            GraphOp::RemoveVertex(v, _) => {
                state.v.iter().any(|v2| v == v2) &&
                state.a.iter().all(|(v2, _)| v != v2)
            },
            GraphOp::AddArc((v1, _), _) => {
                state.v.iter().any(|v2| v1 == v2)
            },
            GraphOp::RemoveArc(a, _) => {
                state.a.iter().any(|a2| a == a2)
            },
        }
    }

    fn append_history(&mut self, hist_obj: CmRDT::HistoryItem) -> CmRDT::K {
        self.hist.add(hist_obj)
    }

    fn set_state(&mut self, k: CmRDT::K, state: Self::StateFormat) -> () {
        self.state.insert(k, state);
    }
}
