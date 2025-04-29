// Provides Traits, types, etc. needed to implement a CmRDT-based driver.

use crate::types::Hash;
use crate::storage::{ObjectLocation, ObjectFile};
use crate::storage;

use std::collections::{HashMap, HashSet};

use serde::{Serialize, de::DeserializeOwned};
use sha2::{Sha256, Digest};
use uuid::Uuid;

// k in the CmRDT paper
// Used as a key for state history and causal history
pub type K = usize;

const BUF_SIZE: usize = 1024;

// == Data Formats ==
// On Disk Format
pub trait DiskType {
    fn new() -> Self;

    fn read(loc: &ObjectLocation) -> Result<Box<Self>, std::io::Error>;
    fn write(&self, loc: &ObjectLocation) -> Result<(), std::io::Error>;

    fn from_state(state: &Self::StateFormat) -> Self;

    type StateFormat: StateType;
}

// Internal (state) Format
pub type State<T> = HashMap<K, T>;
pub trait StateType {
    fn new() -> Self;  // s^0
}

// Operation Format
pub trait Operation: Serialize + DeserializeOwned {
    fn serialize_to_bytes(&self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let json = serde_json::to_string(self)?;
        let bytes = json.as_bytes();
        buf[0..bytes.len()].clone_from_slice(bytes);
        Ok(bytes.len())
    }

    fn deserialize(data: &[u8]) -> Result<Self, std::io::Error> {
        let json = String::from_utf8_lossy(data);
        Ok(serde_json::from_str(&json)?)
    }

    fn get_hash(&self) -> Hash {
        let mut buf = [0u8; BUF_SIZE];
        let res = self.serialize_to_bytes(&mut buf);

        let mut hasher = Sha256::new();
        hasher.update(buf);
        return hasher.finalize();
    }

    fn to_history(&self) -> HistoryItem {
        Some(self.get_hash())
    }
}

// History Format
pub type HistoryItem = Option<Hash>;

#[derive(Clone, Debug)]
pub struct History {
    data: Vec<HistoryItem>,
    pub k: K,
}

impl History {
    pub fn new() -> Self {
        Self {
            data: Vec::from([None]), k: 0,
        }
    }

    pub fn add(&mut self, item: HistoryItem) -> K {
        self.data.push(item); self.k += 1; self.k
    }

    pub fn contains(&self, hash: Hash) -> bool {
        return self.data.contains(&Some(hash));
    }

    pub fn k_contains(&self, hash: Hash, k: K) -> bool {
        if k > self.k {panic!()}
        return self.data[..k+1].contains(&Some(hash));
    }

    pub fn happened_before(&self, hash1: Hash, hash2: Hash) -> bool {
        // Did hash1 happen before hash2?
        let k2 = self.data.iter().position(
            |h| match h {Some(hash) => *hash == hash2, None => false}
        ).unwrap();
        return self.k_contains(hash1, k2 - 1);
    }

    pub fn get_set(&self, k: K) -> HashSet<(K, Hash)> {  // Get c^k
        if k > self.k {panic!()}
        self.data[..k+1].iter().enumerate().fold(
            HashSet::new(),
            |s, (i, h)| match h {
                None => s,
                Some(hash) => {let mut s_ = s.clone(); s_.insert((i, *hash)); s},
            }
        )
    }
}


// == Main CmRDT Object ==
// General flow of using this is as follows:
// - Instantiate an Object with init to create it in the initial state
// - Call prep to get an operation if possible
// - If prep returned an update, apply it with apply_op
// - Write out the operation to disk
// - Repeat until prep returns None
pub trait Object {
    // CmRDT signature is (S, s^0, q, t, u, P)

    // Data formats
    type StateFormat: StateType;  // S
    type DiskFormat: DiskType<StateFormat = Self::StateFormat>;  // Format for data read from the real tree
    type Op: Operation + Clone;  // Format of an operation

    // Create an object in state s^0, with empty history
    fn init() -> Self;

    // Get the current state (q)
    fn query_internal(&self) -> &Self::StateFormat;
    fn query(&self) -> Self::DiskFormat {
        Self::DiskFormat::from_state(self.query_internal())
    }
    fn query_into_buf(&self, buf: &mut Self::DiskFormat) -> () {
        *buf = Self::DiskFormat::from_state(self.query_internal());
    }

    // Prepare updates.
    // Will return a single update, which must then immediately be applied.
    // If data matches the current state, None will be returned.
    // Otherwise, a single update as Some(op) will be returned.
    // Note that one call to this function may not be sufficient - it will calculate a single operation and prepare it,
    //   but it makes no guarantee that all the outstanding changes can be encoded in one operation.
    fn prep(&self, data: &Self::DiskFormat, replica_id: Uuid) -> Option<Self::Op>;  // t

    // Apply a single update.
    // This should be called immediately after `prep` if prep returned a Some value.
    // Should simply return the updated state if possible.
    // Checks for the precondition - will return None if it is not applied.
    fn apply(&mut self, op: &Self::Op) -> Option<Self::StateFormat>;  // u

    // Check if the preconditions of the operation are satisfied.
    // If this returns false, then the operation cannot yet be applied!
    // - This is not up to the driver to deal with.
    fn precond(&self, op: &Self::Op) -> bool;  // P

    fn apply_op(&mut self, op: &Self::Op) -> Option<()> {
        let new_state = self.apply(op)?;
        self.log_op(op.to_history(), new_state);
        Some(())
    }

    fn log_op(&mut self, hist_obj: HistoryItem, new_state: Self::StateFormat) -> () {
        let k = self.append_history(hist_obj);
        self.set_state(k, new_state);
    }

    fn append_history(&mut self, hist_obj: HistoryItem) -> K;
    fn set_state(&mut self, k: K, state: Self::StateFormat) -> ();
}
