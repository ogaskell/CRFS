use crate::{storage, networking, conflict_res, errors, types};

use conflict_res::drivers::file_tree;

use std::path::PathBuf;
use std::collections::HashSet;

use homedir::my_home;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// Single replica config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemConfig(pub storage::Config, pub networking::Config);

impl SystemConfig {
    pub fn get_replica_id(&self) -> Option<Uuid> { self.1.info.get_replica_id() }

    pub fn sync(&self) -> Result<(), errors::Error> {
        let mut tree = file_tree::FileManager::read_or_init(&self.0, self.1.info.get_replica_id().unwrap())?;

        println!("-> File Tree loaded. Checking for local updates...");

        tree.update()?;

        println!("-> Internal state up-to-date. Syncing with server...");

        let remote_hashes = self.network_sync(&tree)?;

        if remote_hashes.len() > 0 {
            println!("Applying {} ops...", remote_hashes.len());
            tree.apply_ops(&remote_hashes.iter().collect())?;
        }

        println!("-> Up-to-date with server.");

        tree.write_out()?;

        Ok(())
    }

    fn network_sync(&self, tree: &file_tree::FileManager) -> Result<HashSet<types::Hash>, errors::Error> {
        let remote_hashes = self.1.fetch_state()?;

        // Pull
        let local_hashes = tree.get_history().all_hashes();
        let new_hashes = self.1.pull(&self.0, &local_hashes, &remote_hashes)?;

        // Push
        self.1.push(&self.0, &local_hashes, &remote_hashes)?;

        return Ok(new_hashes);
    }
}


// Global (all replicas on device) config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalConfig {
    pub replicas: Vec<SystemConfig>,
}

impl GlobalConfig {
    pub fn get_conf_path() -> PathBuf {
        let mut globalconfpath = my_home().expect("Error getting home directory.").expect("User has no home directory.");
        globalconfpath.push(storage::GLOBALCONF);

        return globalconfpath;
    }

    pub fn find_replica_by_dir(&self, dir: PathBuf) -> Option<SystemConfig> {
        Some(self.replicas[self.replicas.iter().position(|x| x.0.working_dir == dir)?].clone())
    }

    pub fn replica_index_by_dir(&self, dir: PathBuf) -> Option<usize> {
        self.replicas.iter().position(|x| x.0.working_dir == dir)
    }

    pub fn empty() -> Self {
        Self {
            replicas: Vec::new(),
        }
    }

    pub fn write_out(&self) -> std::io::Result<()> {
        storage::meta::write_at(&storage::Config{working_dir: PathBuf::new(), }, &Self::get_conf_path(), false, self)
    }

    pub fn read() -> std::io::Result<Self> {
        storage::meta::read_at(&storage::Config{working_dir: PathBuf::new(), }, &Self::get_conf_path(), false)
    }
}
