mod default_storage;
use crate::default_storage::storage;
use crate::default_storage::outputs;

mod default_networking;
use crate::default_networking::networking;
use crate::default_networking::networking::{UserInfo, FileSystemInfo, ReplicaInfo};

#[cfg(test)]
mod tests;

mod errors;
mod helpers;
mod types;

use std::env;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;

use homedir::my_home;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

type SystemConfig = (storage::Config, networking::Config);

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GlobalConfig {
    pub replicas: Vec<SystemConfig>,
}

impl GlobalConfig {
    pub fn clone(&self) -> Self {
        Self {
            replicas: self.replicas.clone(),
        }
    }

    pub fn find_replica_by_dir(&self, dir: PathBuf) -> Option<SystemConfig> {
        self.replicas.clone().into_iter().find(|x| *x.0.working_dir == dir)
    }

    pub fn replica_index_by_dir(&self, dir: PathBuf) -> Option<usize> {
        self.replicas.clone().into_iter().position(|x| *x.0.working_dir == dir)
    }

    pub fn empty() -> Self {
        Self {
            replicas: Vec::new(),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut networkconf = networking::Config::empty();

    let mut globalconfpath = my_home().unwrap().unwrap();
    globalconfpath.push(storage::GLOBALCONF);

    match args[1].as_str() {
        "setup" => {
            // CLI Args
            let addr_i = args.iter().position(|x| x == "-s" || x == "--server");
            let address = match addr_i {
                Some(i) => args[i + 1].parse().unwrap(),
                None => SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000),
            };
            networkconf.server = Some(SocketAddr::V4(address));

            networkconf.info.fs.user.id = match helpers::get_named_arg(&args, Some("u"), Some("user-uuid")) {
                Some(u) => Some(Uuid::parse_str(u.as_str()).unwrap()),
                None => None,
            };
            networkconf.info.fs.user.disp_name = helpers::get_named_arg(&args, None, Some("user-name"));
            networkconf.info.fs.id = match helpers::get_named_arg(&args, Some("f"), Some("fs-uuid")) {
                Some(f) => Some(Uuid::parse_str(f.as_str()).unwrap()),
                None => None,
            };
            networkconf.info.fs.disp_name = helpers::get_named_arg(&args, None, Some("fs-name"));
            networkconf.info.id = match helpers::get_named_arg(&args, Some("r"), Some("replica-uuid")) {
                Some(r) => Some(Uuid::parse_str(r.as_str()).unwrap()),
                None => None,
            };
            networkconf.info.disp_name = helpers::get_named_arg(&args, None, Some("replica-name"));

            let mut workingdir = helpers::get_named_arg(&args, Some("d"), Some("dir"));
            let workingdir_canonical = match &workingdir {
                Some(s) => PathBuf::from(s).canonicalize(),
                None => {workingdir = Some(String::from(".")); PathBuf::from(".").canonicalize()},
            };

            // Setup replica
            setup_replica(&mut networkconf).unwrap();

            // dbg!(netconfig.clone());

            println!(
                "Network Setup Done!\n\tUser: {:#?}\n\tFS: {:#?}\n\tReplica: {:#?}",
                networkconf.info.fs.user.id,
                networkconf.info.fs.id,
                networkconf.info.id
            );

            let storageconf = storage::Config {
                working_dir: match workingdir_canonical {
                    Ok(path) => path,
                    Err(_) => {println!("Malformed directory: {}", workingdir.unwrap()); return;}
                }
            };

            println!("Setting up local storage in {:?}", storageconf.working_dir);

            let concreplicainfo = outputs::ConcreteReplicaInfo::from_replicainfo(networkconf.info.clone()).unwrap();
            let mut infofile = storage::MetaFile::<outputs::ConcreteReplicaInfo>::create_at_path(
                PathBuf::from("replica.crfs")
            ).unwrap();
            infofile.write(&concreplicainfo).unwrap();

            add_replica_to_conf(globalconfpath, (storageconf, networkconf));

            println!("Setup Complete! replica.crfs contains details of your account, filesystem and replica.");
        },
        "check" => {
            let mut conffile = match storage::MetaFile::<GlobalConfig>::open_path(globalconfpath) {
                Ok(c) => c,
                Err(e) => {
                    println!("Failed to open global config file! Error: {:#?}", e);
                    return
                }
            };
            let conf = conffile.read().unwrap();

            let mut workingdir = helpers::get_named_arg(&args, Some("d"), Some("dir"));
            let workingdir_canonical = match &workingdir {
                Some(s) => PathBuf::from(s).canonicalize(),
                None => {workingdir = Some(String::from(".")); PathBuf::from(".").canonicalize()},
            }.unwrap();

            let replica = match conf.find_replica_by_dir(workingdir_canonical.clone()) {
                Some(config) => {
                    println!("Found config for replica at {:?}", config.clone().0.working_dir);
                    config
                },
                None => {
                    println!("Failed to find replica at {:?}", workingdir_canonical);
                    return;
                },
            };
        },
        "remove" => {
            let mut conf_read_file = match storage::MetaFile::<GlobalConfig>::open_path(globalconfpath.clone()) {
                Ok(c) => c,
                Err(e) => {
                    println!("Failed to open global config file! Error: {:#?}", e);
                    return
                },
            };
            let mut conf = conf_read_file.read().unwrap();

            let workingdir = helpers::get_named_arg(&args, Some("d"), Some("dir"));
            let workingdir_canonical = match &workingdir {
                Some(s) => PathBuf::from(s).canonicalize(),
                None => PathBuf::from(".").canonicalize(),
            }.unwrap();

            let replica_ind = match conf.replica_index_by_dir(workingdir_canonical.clone()) {
                Some(index) => index,
                None => {
                    println!("Failed to find replica at {:?}", workingdir_canonical);
                    return;
                },
            };

            conf.replicas.remove(replica_ind);
            let mut conf_write_file = storage::MetaFile::<GlobalConfig>::create_at_path(globalconfpath).unwrap();
            conf_write_file.write(&conf).unwrap();

            println!("Removed replica at {:?}", workingdir_canonical);
            println!("{:?} directory has been left intact, but this can now be safely removed.", workingdir_canonical.join(PathBuf::from(".crfs")))
        },
        _ => {
            println!("Unknown command!");
            return;
        },
    }
}

fn setup_replica(config: &mut networking::Config) -> Result<(), errors::Error> {
    setup_fs(config)?;

    config.info.id = match config.info.id {
        Some(v) => Some(v),
        None => Some(Uuid::now_v7()),
    };

    return Ok(());
}

fn setup_fs(config: &mut networking::Config) -> Result<(), errors::Error> {
    setup_user(config)?;

    config.info.fs.id = match config.info.fs.id {
        Some(v) => Some(v),
        None => Some(Uuid::now_v7()),
    };

    return Ok(());
}

fn setup_user(config: &mut networking::Config) -> Result<(), errors::Error> {
    config.info.fs.user.id = match config.info.fs.user.id {
        Some(v) => Some(v),
        None => Some(Uuid::now_v7()),
    };
    match networking::register_user(config) {
        Ok(_) => Ok(()),
        Err(neterr) => Err(errors::Error::from(neterr)),
    }
}

fn add_replica_to_conf(loc: PathBuf, config: SystemConfig) -> () {
    let read_file = storage::MetaFile::<GlobalConfig>::open_path(loc.clone());
    let mut conf = match read_file {
        Ok(mut f) => f.read().unwrap(),
        Err(_) => GlobalConfig::empty(),
    };

    conf.replicas.push(config);

    let mut write_file = storage::MetaFile::create_at_path(loc.clone()).unwrap();
    write_file.write(&conf).unwrap();
}
