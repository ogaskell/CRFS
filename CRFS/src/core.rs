use crate::{storage, networking, conflict_res, errors, types};

use conflict_res::drivers::file_tree;

use std::fs;
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

    pub fn init(&self) -> Result<(), errors::Error> {
        let tree = file_tree::FileManager::read_or_init(&self.0, self.get_replica_id().unwrap())?;
        tree.write_out()?;

        Ok(())
    }

    pub fn sync(&self) -> Result<(), errors::Error> {
        let mut tree = file_tree::FileManager::read_or_init(&self.0, self.get_replica_id().unwrap())?;

        println!("-> File Tree loaded. Checking for local updates...");

        tree.update()?;

        println!("-> Internal state up-to-date. Syncing with server...");

        let remote_hashes = self.network_sync(&tree)?;

        if remote_hashes.len() > 0 {
            // println!("Applying {} ops...", remote_hashes.len());
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

    pub fn canonize(&self) -> errors::Result<()> {
        let mut tree = file_tree::FileManager::read_or_init(&self.0, self.get_replica_id().unwrap())?;

        println!("-> File Tree loaded. Checking for local updates...");

        tree.update()?;

        println!("-> Internal state up-to-date. Writing out canonical forms...");

        Ok(tree.canonize()?)
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
        let dir_abs = fs::canonicalize(dir).expect("Error finding absolute path of dir.");
        Some(self.replicas[self.replicas.iter().position(|x| x.0.working_dir == dir_abs)?].clone())
    }

    pub fn empty() -> Self {
        Self {
            replicas: Vec::new(),
        }
    }

    pub fn write_out(&self, path: &PathBuf) -> std::io::Result<()> {
        storage::meta::write_at(&storage::Config{working_dir: PathBuf::new(), }, path, false, self)
    }

    pub fn read(path: &PathBuf) -> std::io::Result<Self> {
        storage::meta::read_at(&storage::Config{working_dir: PathBuf::new(), }, path, false)
    }
}

pub fn init(path: &PathBuf) -> std::io::Result<()> {
    if !path.exists() {
        println!("Creating global config file...");
        return GlobalConfig::empty().write_out(path);
    } else {
        println!("Global config exists.");
        return Ok(());
    }
}

pub fn setup(conf: &mut GlobalConfig, conf_path: &PathBuf, server: &std::net::SocketAddr, user_id: &Option<Uuid>, fs_id: &Option<Uuid>, user_name: &Option<String>, fs_name: &Option<String>, dir: &Option<PathBuf>) {
    let working_dir = match dir {
        Some(d) => d.clone(),
        None => std::env::current_dir().expect("Error opening working directory. Move to a different directory, or specify a working directory."),
    };

    let working_dir = fs::canonicalize(working_dir).expect("Error getting absolute path of working dir.");

    let mut system_config = SystemConfig(
        storage::Config {working_dir}, networking::Config {
            server: Some(server.clone()),
            info: networking::ReplicaInfo {
                id: user_id.clone(),
                disp_name: None,
                fs: networking::FileSystemInfo {
                    id: fs_id.clone(),
                    disp_name: fs_name.clone(),
                    user: networking::UserInfo {
                        id: user_id.clone(),
                        disp_name: user_name.clone(),
                    }
                }
            }
        }
    );
    system_config.1.gen_blanks();

    let (user_ok, fs_ok) = system_config.1.check_info().expect("Error checking info with server.");

    if !user_ok {
        system_config.1.register_user().expect("Error registering user.");
    }

    if !fs_ok {
        system_config.1.register_fs().expect("Error registering FS");
    }

    println!("Identity confirmed with server.");

    system_config.init().expect("Error setting up replica config.");

    conf.replicas.push(system_config);
    conf.write_out(conf_path).expect("Error writing global config.");

    println!("Done!");
}

pub fn sync(conf: GlobalConfig, dir_: &Option<PathBuf>) {
    let dir = match dir_ {
        Some(d) => d.clone(),
        None => std::env::current_dir().expect("Error opening working directory. Move to a different directory, or specify a working directory."),
    };

    let system_config = conf.find_replica_by_dir(dir).expect("Replica not found. Please run the setup command first.");

    system_config.sync().expect("Sync error.");

    println!("Sync OK!");
}

pub fn canonize(conf: GlobalConfig, dir_: &Option<PathBuf>) {
    let dir = match dir_ {
        Some(d) => d.clone(),
        None => std::env::current_dir().expect("Error opening working directory. Move to a different directory, or specify a working directory."),
    };

    let system_config = conf.find_replica_by_dir(dir).expect("Replica not found. Please run the setup command first.");

    system_config.canonize().expect("Canonize error.");

    println!("Wrote out canonical forms.");
}
