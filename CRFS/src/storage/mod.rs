use crate::conflict_res::drivers::CmRDT;
use crate::types::Hash;

use std::fs::{File, create_dir_all};
use std::io::{BufReader, Error, ErrorKind, Read, Write};
use std::option::Option;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use serde_json;
use sha2::{Sha256, Digest};
use uuid::Uuid;

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

    fn clone(&self) -> Self {
        match self {
            ObjectLocation::ObjectStore(c, h) =>
                ObjectLocation::ObjectStore(c.clone(), h.clone()),
            ObjectLocation::OnDisk(c, p) =>
                ObjectLocation::OnDisk(c.clone(), p.clone()),
        }
    }

    pub fn extension(&self) -> Option<String> {
        match self {
            ObjectLocation::OnDisk(_, p) => Some(p.extension()?.to_owned().into_string().unwrap()),
            ObjectLocation::ObjectStore(..) => None,
        }
    }
}

/*
pub struct ObjectFile {
    file: File,
    loc: ObjectLocation,
}

impl ObjectFile {
    pub fn open(loc: &ObjectLocation) -> std::io::Result<ObjectFile> {
        let p = loc.get_path();

        match p.try_exists() {
            Ok(true) => {
                let f = File::open(p.clone())?;
                Ok(ObjectFile {
                    file: f,
                    loc: loc.clone(),
                })
            },
            Ok(false) => Err(Error::new(ErrorKind::NotFound, format!("Broken symlink in path {:#?}.", p))),
            Err(e) => Err(e),
        }
    }

    /// Create a file, writing the hash back to loc if loc is an ObjectStore.
    pub fn create_mutloc(loc: &mut ObjectLocation, buf: &[u8]) -> std::io::Result<()> {
        match loc {
            ObjectLocation::ObjectStore(config, hash) => {
                *hash = Some(Self::create_object(config, buf)?);
                Ok(())
            },
            ObjectLocation::OnDisk(config, path) => Self::create_on_disk(config, path, buf)
        }
    }

    /// Create a file, but don't write the hash back to loc.
    pub fn create(loc: &ObjectLocation, buf: &[u8]) -> std::io::Result<()> {
        match loc {
            ObjectLocation::ObjectStore(config, _) => {
                Self::create_object(config, buf)?; Ok(())
            },
            ObjectLocation::OnDisk(config, path) => Self::create_on_disk(config, path, buf)
        }
    }

    pub fn create_on_disk(config: &Config, _path: &PathBuf, buf: &[u8]) -> std::io::Result<()> {
        let path = if _path.is_relative() {
            let mut path = config.working_dir.clone(); path.push(_path); path
        } else {
            _path.clone()
        };

        match path.parent() {
            Some(parent) => {ensure_dir(config, &PathBuf::from(parent))?;},
            None => {},
        }

        let mut f = File::create(path)?;

        return f.write_all(buf);
    }

    pub fn create_object(config: &Config, buf: &[u8]) -> std::io::Result<Hash> {
        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(buf);
        let hash: Hash = hasher.finalize();

        // Find location
        let path = ObjectLocation::hash_to_path(config, &hash);

        // Ensure directory exists
        match path.parent() {
            Some(dir) => {ensure_dir(config, &PathBuf::from(dir))?;},
            None => {},
        }

        // Write data
        ObjectFile::create_on_disk(config, &path, buf)?;

        return Ok(hash);
    }

    pub fn create_op_object<T>(config: &Config, op: T) -> std::io::Result<Hash> where T: CmRDT::Operation {
        let hash = op.get_hash();
        let data = op.serialize_to_str()?;

        // Find location
        let path = ObjectLocation::hash_to_path(config, &hash);

        // Ensure directory exists
        match path.parent() {
            Some(dir) => {ensure_dir(config, &PathBuf::from(dir))?;},
            None => {},
        }

        ObjectFile::create_on_disk(config, &path, data.as_bytes())?;

        return Ok(hash);
    }

    pub fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }

    pub fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.file.read_to_string(buf)
    }
}
*/
