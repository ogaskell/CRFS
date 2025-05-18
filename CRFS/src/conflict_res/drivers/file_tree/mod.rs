// Directed Graph CmRDT, adapted from https://pages.lip6.fr/Marc.Shapiro/papers/RR-7687.pdf#page=15

#[cfg(test)]
mod test;

use crate::storage;
// use crate::storage::{ObjectFile, ObjectLocation};
use storage::object;
use crate::{types, errors};
use super::driver::{AvailDrivers, DriverNames}; // AvailOps;
use super::ast_doc::yata;
use super::CmRDT::{self, Operation};

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use rand::Rng;
use serde::{Serialize, Deserialize};
use serde_with::serde_as;

use uuid::Uuid;

const IGNORED_DIRS: [&str; 1] = [".crfs"];

// Driver container
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriverID {
    Driver(u64),
    FileTree,
}
pub fn unique() -> DriverID {
    DriverID::Driver(rand::rng().random())
}
pub type DriverContainer = HashMap<DriverID, AvailDrivers>;

// CRDT State Format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    // driver: DriverID,
    paths: yata::Array<PathBuf, Uuid>,
    deleted: bool,
    // Potential to add file permissions, owners, etc. here in future.
    // This would however require considerations of how the program is run (i.e. setuid/setgid root may be required)
}
pub type FileState = HashMap<DriverID, FileInfo>;

impl FileInfo {
    /// Create a new FileInfo and driver, store the driver, and return a FileInfo
    /// init_path should be relative to config.working_directory
    pub fn new_file(
        container: &mut DriverContainer,
        driver: DriverNames,
        config: storage::Config,
        init_path: PathBuf,
        replica_id: Uuid
    ) -> (DriverID, Self) {
        let id = unique();
        container.insert(id, AvailDrivers::new_from_name(
            driver,
            config.clone(),
            &object::Location::Path(init_path.clone(), true),
            replica_id,
            id
        ));

        (id, Self {
            // driver: id,
            paths: yata::Array::from(([init_path].into_iter(), replica_id)),
            deleted: false,
        })
    }

    pub fn get_path(&self) -> &PathBuf {
        let tail = self.paths.tail.expect("paths list empty!");
        return &self.paths.items.get(&tail).expect("tail reference invalid!").content;
    }

    pub fn insert_path(&mut self, ins: yata::Insertion<PathBuf, Uuid>, id: yata::ID) {
        self.paths.insert(ins, Some(id));
    }
}

// Operations
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum FileOp {
    NewFile(DriverID, DriverNames, PathBuf, Uuid), // New ID, Type of Driver, Initial Location, Created by ID
    MoveFile(DriverID, yata::Insertion<PathBuf, Uuid>, yata::ID),
    DelFile(DriverID),
}

impl CmRDT::Operation for FileOp {
    fn get_driverid(&self) -> DriverID {
        DriverID::FileTree
    }
}

// Summary of the whole systems Causal Histories
#[derive(Debug)]
pub struct SystemHistory {
    tree: CmRDT::History, // for the file tree
    drivers: HashMap<DriverID, CmRDT::History>, // for all files
}

impl SystemHistory {
    pub fn all_hashes(&self) -> HashSet<types::Hash> {
        let mut hashes: Vec<HashSet<types::Hash>> = self.drivers.iter().map(|(_, h)| h.get_hashes()).collect();
        hashes.push(self.tree.get_hashes());

        let all_hashes= hashes.into_iter().reduce(|a, b| a.union(&b).cloned().collect());

        return match all_hashes {
            Some(h) => h,
            None => HashSet::new(),
        };
    }

    /// Return `Some(id)` if `hash` belongs to driver `id`, or `None` if `hash` belongs to the file tree CRDT.
    fn get_driver(&self, hash: &types::Hash) -> Option<DriverID> {
        if self.tree.contains(hash.clone()) {return None}
        for (id, h) in self.drivers.iter() {
            if h.contains(hash.clone()) {return Some(*id)}
        }
        panic!();
    }
}

// File Manager
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct FileManager {
    // CRDT
    #[serde_as(as = "HashMap<_, HashMap<serde_with::json::JsonString, _>>")]
    state: CmRDT::State<FileState>,
    hist: CmRDT::History,

    // Storage
    config: storage::Config,
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    drivers: DriverContainer,

    // Metadata
    replica_id: Uuid,
}

impl FileManager {
    pub fn init(config: storage::Config, replica_id: Uuid) -> Self {
        let mut new = Self {
            state: CmRDT::State::new(),
            hist: CmRDT::History::new(),
            config,
            drivers: DriverContainer::new(),
            replica_id,
        };

        new.state.insert(new.hist.k, FileState::new());

        return new;
    }

    fn list_dir(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut result = Vec::new();

        let mut path_stack = VecDeque::new(); path_stack.push_back(self.config.working_dir.clone());
        'outer: while let Some(path) = path_stack.pop_front() {
            // Skip any paths in IGNORED_DIRS
            for i in IGNORED_DIRS.iter() {if path.ends_with(i) {continue 'outer;}}

            if path.is_file() {
                result.push(path.strip_prefix(&self.config.working_dir).expect("Unable to strip file path prefix.").to_owned());
            } else if path.is_dir() {
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    path_stack.push_back(entry.path());
                }
            } else {
                panic!("Symlinks not supported!");
            }
        }

        return Ok(result);
    }

    pub fn query(&self) -> &FileState {
        let k = self.hist.k;
        let r = self.state.get(&k).expect(&format!("Internal error: State with k {} doesn't exist!", k));
        return r;
    }

    fn get_active_drivers(&self) -> Vec<DriverID> {
        let state = self.query();

        return state.iter().filter(|(_, info)| !info.deleted).map(|(id, _)| *id).collect();
    }

    fn prep(&self) -> std::io::Result<Option<FileOp>> {
        let old_state = self.query();

        let mut disk_files = self.list_dir()?;

        // Get "undeleted" drivers (i.e. drivers for which we expect a file to exist)
        let mut drivers: Vec<_> = self.query().iter().filter(|(_, info)| !info.deleted).map(|(id, ..)| *id).collect();

        let mut missing = Vec::new();

        // Match unmoved files.
        while let Some(f) = disk_files.pop() {
            if let Some((id, ..)) = old_state.iter().find(|(_, info)| info.get_path() == &f && !info.deleted) {
                drivers.retain(|d| d != id);
            } else {
                missing.push(f);
            }
        }

        // Rename detection
        while let Some(new_path) = missing.pop() {
            if let Some(driver_id) = self.rename_detection(&new_path) {
                let file_info = old_state.get(&driver_id).unwrap();
                let (id, ins) = file_info.paths.get_insertion(
                    file_info.paths.len_undel(), new_path, self.replica_id
                );

                return Ok(Some(FileOp::MoveFile(driver_id, ins, id)));
            } else {
                let driver = match AvailDrivers::get_name(
                    &object::Location::Path(new_path.clone(), true)
                ) {
                    Some(name) => name,
                    None => return Ok(None),
                };

                return Ok(Some(FileOp::NewFile(unique(), driver, new_path, self.replica_id)));
            }
        }

        // Drivers for which we found no file
        while let Some(d) = drivers.pop() {
            return Ok(Some(FileOp::DelFile(d)));
        }

        return Ok(None);
    }

    /// If a driver exists in the current state that has content similar to the contents of the file at `path`,
    /// return its id.
    fn rename_detection(&self, _path: &PathBuf) -> Option<DriverID> {
        None
        // TODO - not yet implemented.
    }

    fn apply<'a>(&mut self, ops: &Vec<&'a types::Hash>) -> std::io::Result<HashSet<&'a types::Hash>> {
        let mut applied = HashSet::new();
        let mut last_n_applied = 0usize;

        let n_ops = ops.len();

        while applied.len() < n_ops {
            'inner: for hash in ops.iter() {
                if !applied.contains(*hash) {
                    let op = match self.get_op(*hash) {
                        Ok(op) => op,
                        Err(_) => continue 'inner,
                    };

                    if op.get_driverid() != DriverID::FileTree {
                        continue 'inner;
                    }

                    self.apply_op(&op)?;
                    applied.insert(*hash);
                }
            }

            if applied.len() <= last_n_applied {
                return Ok(applied);
            }
            last_n_applied = applied.len();
        }

        Ok(applied)
    }

    fn apply_op(&mut self, op: &FileOp) -> std::io::Result<()> {
        let old_state = self.query();
        let mut new_state = old_state.clone();

        match op {
            FileOp::NewFile(id, name, path, creator_id) => {
                // Create Driver
                let driver = AvailDrivers::new_from_name(
                    *name, self.config.clone(), &object::Location::Path(path.clone(), true),
                    self.replica_id, // This ID is used for creating operations locally.
                    *id,
                );

                // Store Driver
                self.drivers.insert(*id, driver);

                let info = FileInfo {
                    paths: yata::Array::from((
                        [path.clone()].into_iter(),
                        *creator_id, // This ID is the ID where the paths was created, and needs to be consistent across all replicas.
                    )),
                    deleted: false,
                };

                new_state.insert(*id, info);
            },
            FileOp::MoveFile(id, ins, ins_id) => {let old_info = old_state.get(id).expect("No driver with given id.");
                let old_path = old_info.get_path();

                // Insert `ins`
                let new_info = new_state.get_mut(id).expect("No driver with given id.");
                new_info.insert_path(ins.clone(), *ins_id);
                let new_path = new_info.get_path();

                // Check if old_path != new_path
                //   If so, move the file
                if old_path != new_path && !new_info.deleted {
                    std::fs::rename(old_path, new_path)?;
                }
            },
            FileOp::DelFile(id) => {
                let file_info = new_state.get_mut(id).expect("No driver with given id.");

                let path = file_info.get_path(); let loc = object::Location::Path(path.clone(), true);
                if loc.exists(&self.config) {
                    // Delete file.
                    // std::fs::remove_file(path)?;
                    object::delete(&self.config, &loc)?;
                }

                file_info.deleted = true;
            }
        };

        let k = self.hist.add(op.to_history());
        self.state.insert(k, new_state);

        return Ok(());
    }

    fn update_drivers(&mut self) -> Result<(), errors::Error> {
        for id in self.get_active_drivers().iter() {
            self.drivers.get_mut(id).unwrap().update()?;
        }

        Ok(())
    }

    pub fn write_out(&self) -> std::io::Result<()> {
        storage::meta::write(&self.config, &String::from("filetree"), self)
    }

    pub fn read_in(config: &storage::Config) -> std::io::Result<Self> {
        storage::meta::read(config, &String::from("filetree"))
    }

    pub fn read_or_init(config: &storage::Config, replica_id: Uuid) -> std::io::Result<Self> {
        match Self::read_in(config) {
            Ok(m) => Ok(m),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::init(config.clone(), replica_id)),
            e => e,
        }
    }

    fn get_op(&self, hash: &types::Hash) -> std::io::Result<FileOp> {
        let loc = object::Location::Object(hash.clone());
        let mut json = String::new(); object::read_string(&self.config, &loc, &mut json)?;
        return FileOp::deserialize_from_str(json);
    }

    fn write_op(&self, op: FileOp) -> std::io::Result<types::Hash> {
        return object::write_op(&self.config, op);
    }

    fn update_self(&mut self) -> std::io::Result<()> {
        while let Some(op) = self.prep()? {
            self.apply_op(&op)?;
            self.write_op(op)?;
        }

        Ok(())
    }

    pub fn update(&mut self) -> Result<(), errors::Error> {
        self.update_self()?;
        self.update_drivers()?;
        Ok(())
    }

    pub fn get_history(&self) -> SystemHistory {
        return SystemHistory {
            tree: self.hist.clone(),
            drivers: self.drivers.iter().map(|(id, driver)| (*id, driver.get_history())).collect(),
        }
    }

    pub fn apply_ops(&mut self, hashes: &Vec<&types::Hash>) -> std::io::Result<()> {
        let mut applied_ops: HashSet<&types::Hash> = HashSet::new();

        applied_ops = applied_ops.union(&self.apply(hashes)?).cloned().collect();

        for id in self.get_active_drivers() {
            let driver = self.drivers.get_mut(&id).unwrap();
            applied_ops = applied_ops.union(
                &driver.apply(hashes)?
            ).cloned().collect();

            driver.write_out()?;
        }

        let n_unapplied = hashes.len() - applied_ops.len();

        if n_unapplied > 0 {
            println!("{} operations unable to be applied!", n_unapplied);
        }

        Ok(())
    }

    pub fn canonize(&mut self) -> std::io::Result<()> {
        for id in self.get_active_drivers() {
            self.drivers.get_mut(&id).unwrap().write_out()?;
        }

        self.write_out()?;

        Ok(())
    }
}
