/**
 * Traits and data structures used to define a driver.
 */

use super::CmRDT;
use CmRDT::Operation;
use super::ast_doc;
use super::file_tree;
use file_tree::DriverID;

use ast_doc::md;
use md::MDDriver;

use crate::storage::{Config, object};
use crate::errors;
use crate::types::Hash;

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub trait Driver {
    type Object: CmRDT::Object;

    /// Check if a file can be managed by this driver.
    fn check(loc: &object::Location) -> bool;

    /// Create a new driver instance for a given file.
    /// loc should be a file in the tree.
    fn new(config: Config, loc: &object::Location, replica_id: Uuid, driverid: DriverID) -> Self;

    fn get_driverid(&self) -> DriverID;

    /// Get/set internal storage config.
    fn get_config(&self) -> Config;
    fn set_config(&mut self, config: Config);

    /// Get the causal history.
    fn get_history(&self) -> CmRDT::History;

    /// Update the internal state based on the on-disk state.
    /// Should also write out operations to disk
    fn update(&mut self) -> Result<(), errors::Error>;

    /// Apply a number of operations fetched from the network or elsewhere.
    /// Not all the ops referred to in `ops` need be for this driver.
    /// As such, drivers should perform two checks:
    /// 1. That the operation is of the expected type. Do not apply it if it is not.
    /// 2. That the operation's driverid matches the driver's. Do not apply it if it does not.
    /// Finally, the return value should be a vector of all the applied operations.
    fn apply<'a>(&mut self, ops: &Vec<&'a Hash>) -> std::io::Result<HashSet<&'a Hash>>;

    /// Write out the internal state to disk
    fn write_out(&self) -> std::io::Result<()>;

    fn get_path(&self) -> PathBuf;

    fn get_op(&self, hash: Hash) -> std::io::Result<<<Self as Driver>::Object as CmRDT::Object>::Op> {
        let loc = object::Location::Object(hash);
        let mut json = String::new();
        object::read_string(&self.get_config(), &loc, &mut json)?;
        return Ok(<<Self as Driver>::Object as CmRDT::Object>::Op::deserialize_from_str(json)?);
    }

    fn write_op(&self, op: <<Self as Driver>::Object as CmRDT::Object>::Op) -> std::io::Result<Hash> {
        object::write_op(&self.get_config(), op)
    }
}

/// `AvailDrivers` provides a generic way to interact with any instlaled driver, without using `dyn` or similar mechanisms.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AvailDrivers {
    Markdown(MDDriver),
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum DriverNames {
    Markdown,
}

impl AvailDrivers {
    pub fn get_name(loc: &object::Location) -> Option<DriverNames> {
        if MDDriver::check(loc) {
            return Some(DriverNames::Markdown);
        }

        // Add other drivers here when added.

        return None;
    }

    pub fn get(config: Config, loc: &object::Location, replica_id: Uuid, driverid: DriverID) -> Option<Self> {
        let name = Self::get_name(loc)?;
        return Some(Self::new_from_name(name, config, loc, replica_id, driverid));
    }

    pub fn new_from_name(name: DriverNames, config: Config, loc: &object::Location, replica_id: Uuid, driverid: DriverID) -> Self {
        match name {
            DriverNames::Markdown => Self::Markdown(MDDriver::new(config, loc, replica_id, driverid)),
            _ => todo!(),
        }
    }

    pub fn get_history(&self) -> CmRDT::History {
        match self{
            Self::Markdown(md) => md.get_history(),
            _ => todo!(),
        }
    }

    pub fn update(&mut self) -> Result<(), errors::Error> {
        match self {
            Self::Markdown(md) => md.update(),
            _ => todo!(),
        }
    }

    pub fn apply<'a>(&mut self, ops: &Vec<&'a Hash>) -> std::io::Result<HashSet<&'a Hash>> {
        match self {
            Self::Markdown(driver) => driver.apply(ops),
            _ => todo!(),
        }
    }

    pub fn write_out(&self) -> std::io::Result<()> {
        match self {
            Self::Markdown(driver) => driver.write_out(),
            _ => todo!(),
        }
    }

    pub fn get_path(&self) -> PathBuf {
        match self {
            Self::Markdown(md) => md.get_path(),
            _ => todo!(),
        }
    }
}

// #[derive(Clone, Serialize, Deserialize, Debug)]
// pub enum AvailOps {
//     Markdown(ast_doc::crdt::DocOp<md::MDTag, md::MDLeaf>),
//     FileOp(file_tree::FileOp),
// }

// impl From<ast_doc::crdt::DocOp<md::MDTag, md::MDLeaf>> for AvailOps {
//     fn from(item: ast_doc::crdt::DocOp<md::MDTag, md::MDLeaf>) -> Self {
//         Self::Markdown(item)
//     }
// }

// impl Into<ast_doc::crdt::DocOp<md::MDTag, md::MDLeaf>> for AvailOps {
//     fn into(self) -> ast_doc::crdt::DocOp<md::MDTag, md::MDLeaf> {
//         match self {
//             Self::Markdown(item) => item,
//             _ => panic!(),
//         }
//     }
// }

// impl From<file_tree::FileOp> for AvailOps {
//     fn from(item: file_tree::FileOp) -> Self {
//         Self::FileOp(item)
//     }
// }
