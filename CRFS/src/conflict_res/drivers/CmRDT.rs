// Provides Traits, types, etc. needed to implement a CmRDT-based driver.

use crate::types::{Hash, calculate_hash};
use crate::storage;
use crate::storage::object;

use super::file_tree::DriverID;

use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize, de};
use de::DeserializeOwned;
use sha2::{Sha256, Digest};
use uuid::Uuid;

// k in the CmRDT paper
// Used as a key for state history and causal history
pub type K = usize;

// == Data Formats ==
// On Disk Format
pub trait DiskType {
    fn new() -> Self;

    fn read(config: &storage::Config, loc: &object::Location) -> Result<Box<Self>, std::io::Error>;
    fn write(&self, config: &storage::Config, loc: &object::Location) -> Result<(), std::io::Error>;

    fn from_state(state: &Self::StateFormat) -> Self;

    type StateFormat: StateType;
}

// Internal (state) Format
pub type State<T> = HashMap<K, T>;
pub trait StateType {
    fn new() -> Self;  // s^0
}

// Operation Format

/// Struct types implementing this trait should have at least one field which is guaranteed to make its signature unique.
/// Enum types should aim to use unique variant names.
/// This guarantees it cannot be deserialized into any other type.
pub trait Operation: Serialize + DeserializeOwned + Clone {
    fn serialize_to_str(&self) -> std::io::Result<String> {
        return Ok(serde_json::to_string(self)?);
    }

    #[deprecated]
    fn deserialize(data: &[u8]) -> std::io::Result<Self> {
        let json = String::from_utf8_lossy(data);
        Ok(serde_json::from_str(&json)?)
    }

    fn deserialize_from_str(json: String) -> std::io::Result<Self> {
        Ok(serde_json::from_str(&json)?)
    }

    fn get_hash(&self) -> Hash {
        let str = self.serialize_to_str().expect("Serialization error.");

        return calculate_hash(&str);
    }

    fn to_history(&self) -> HistoryItem {
        Some(self.get_hash())
    }

    fn get_driverid(&self) -> super::file_tree::DriverID;
}

// History Format
pub type HistoryItem = Option<Hash>;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        self.data.push(item); self.k += 1;
        assert_eq!(self.data.len(), self.k + 1);
        return self.k;
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

    pub fn get_hashes(&self) -> HashSet<Hash> {
        self.data.iter().filter(|x: &&Option<Hash>| x.is_some()).map(|x| x.unwrap()).collect()
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
    fn init(driver_id: DriverID) -> Self;

    fn get_driverid(&self) -> DriverID;

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
