use crate::types::Hash;

use std::option::Option;
use std::path::PathBuf;

use serde::{Serialize, Deserialize};

pub const GLOBALCONF: &str = ".config/crfs/config.json"; // Appending to the user's home dir.
const OBJECTDIR: &str = ".crfs/objects/"; // Appended to the working dir.
const METADIR: &str = ".crfs/meta/"; // Appended to the working dir.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub working_dir: PathBuf,
}

impl Config {
    pub fn clone(&self) -> Self {
        Self {
            working_dir: self.working_dir.clone(),
        }
    }
}

pub mod object;
pub mod meta;

// OBJECT FILE HANDLING
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ObjectLocation {
    ObjectStore(Config, Option<Hash>),
    OnDisk(Config, PathBuf),
}

impl ObjectLocation {
    pub fn get_path(&self) -> PathBuf {
        match self {
            ObjectLocation::OnDisk(c, p) => {
                if p.is_relative() {
                    let mut res = c.working_dir.clone(); res.push(p);
                    res
                } else {
                    p.clone()
                }
            },
            ObjectLocation::ObjectStore(c, h) =>
                ObjectLocation::hash_to_path(c, &(h.unwrap())),
        }
    }

    pub fn get_hash(&self) -> Option<&Hash> {
        match self {
            ObjectLocation::OnDisk(..) => None,
            ObjectLocation::ObjectStore(_, hash) => match hash {
                Some(h) => Some(h),
                None => None,
            },
        }
    }

    pub fn hash_to_path(config: &Config, hash: &Hash) -> PathBuf {
        let hex = format!("{:x}", hash);

        let mut path = PathBuf::new();
        path.push(&config.working_dir);
        path.push(OBJECTDIR);
        path.push(&hex[0..2]);
        path.push(&hex[2..]);

        return path;
    }

    pub fn extension(&self) -> Option<String> {
        match self {
            ObjectLocation::OnDisk(_, p) => Some(p.extension()?.to_owned().into_string().unwrap()),
            ObjectLocation::ObjectStore(..) => None,
        }
    }
}
