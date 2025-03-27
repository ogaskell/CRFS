use super::types::{GenericDoc, IDGenericDoc, TagLike};
use super::CmRDT;

use std::marker::PhantomData;

use serde::{Serialize, Deserialize, de::DeserializeOwned};

// == CmRDT Implementation ==
#[derive(Debug)]
pub struct GenericDocObject<NodeTag, LeafType> {
    state: CmRDT::State<IDGenericDoc<NodeTag, LeafType>>,
    hist: CmRDT::History,
    // conctype: PhantomData<ConcType>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum GenericDocOp<NodeTag, LeafType> {
    None(NodeTag, LeafType),  // TODO
}

impl<NodeTag, LeafType> CmRDT::Operation for GenericDocOp<NodeTag, LeafType>
where
    NodeTag: Clone + TagLike + Serialize + DeserializeOwned,
    LeafType: Clone + Serialize + DeserializeOwned
{}

impl<NodeTag, LeafType> CmRDT::Object for GenericDocObject<NodeTag, LeafType>
where
    NodeTag: Clone + TagLike + Serialize + DeserializeOwned,
    LeafType: Clone + Serialize + DeserializeOwned,
    GenericDoc<NodeTag, LeafType>: CmRDT::DiskType<StateFormat = IDGenericDoc<NodeTag, LeafType>>,
{
    type StateFormat = IDGenericDoc<NodeTag, LeafType>;
    type DiskFormat = GenericDoc<NodeTag, LeafType>;
    type Op = GenericDocOp<NodeTag, LeafType>;

    fn init() -> Self {
        Self {
            state: CmRDT::State::new(),
            hist: CmRDT::History::new(),
            // conctype: PhantomData,
        }
    }

    fn query_internal(&self) -> &Self::StateFormat {
        return &self.state[&self.hist.k];
    }

    fn prep(&self, data: &Self::DiskFormat) -> Option<Self::Op> {
        todo!()
    }

    fn apply(&mut self, op: &Self::Op) -> Result<Self::StateFormat, ()> {
        todo!()
    }

    fn precond(&self, op: &Self::Op) -> bool {
        todo!()
    }

    fn append_history(&mut self, hist_obj: CmRDT::HistoryItem) -> CmRDT::K {
        self.hist.add(hist_obj)
    }

    fn set_state(&mut self, k: CmRDT::K, state: Self::StateFormat) -> () {
        self.state.insert(k, state);
    }
}
