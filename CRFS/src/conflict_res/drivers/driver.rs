/**
 * Traits and data structures used to define a driver.
 */

use super::CmRDT;
use crate::storage;
use crate::errors;

use uuid::Uuid;

pub trait Driver<Object> where Object: CmRDT::Object {
    /// Check if a file can be managed by this driver.
    fn check(loc: &storage::ObjectLocation) -> bool;

    /// Create a new driver instance for a given file.
    /// loc should be a file in the tree.
    fn new(loc: &storage::ObjectLocation, replica_id: Uuid) -> Self;

    /// Get the causal history.
    fn get_history(&self) -> CmRDT::History;

    /// Update the internal state based on the on-disk state.
    fn update(&mut self) -> Result<(), errors::Error>;

    /// Apply a number of operations fetched from the network or elsewhere.
    /// If unable to apply all the operations, return a Hashset of the indices of the applied operations.
    fn apply(&mut self, ops: Vec<&Object::Op>) -> Result<(), std::collections::HashSet<usize>>;
}
