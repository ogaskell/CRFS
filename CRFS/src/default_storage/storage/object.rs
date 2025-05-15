use std::{
    fs::{create_dir_all, File},
    io::{Error, ErrorKind, Read, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use trash;

use super::{Config, OBJECTDIR};
use crate::types::Hash;
use crate::conflict_res::drivers::CmRDT;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Location {
    Object(Hash),
    Path(PathBuf, bool) // second item is true if path is relative to the working directory
}

impl Location {
    pub fn get_path(&self, config: &Config) -> PathBuf {
        match self {
            Self::Object(h) => {
                let hex = format!("{:x}", h);

                let mut result = PathBuf::new();
                result.push(&config.working_dir);
                result.push(OBJECTDIR);
                result.push(&hex[0..2]);
                result.push(&hex[2..]);

                return result;
            },
            Self::Path(p, true) => {
                let mut result = PathBuf::new();
                result.push(&config.working_dir); result.push(p);
                result
            },
            Self::Path(p, false) => {
                p.clone()
            }
        }
    }

    pub fn extension(&self) -> Option<String> {
        match self {
            Self::Path(p, _) => Some(p.extension()?.to_str()?.to_owned()),
            Self::Object(_) => None,
        }
    }
}

/// Returns `Ok(true)` if the directory existed, `Ok(false)` if we had to create it, `Err(_)` otherwise.
pub fn ensure_dir(config: &Config, loc: &Location) -> std::io::Result<bool> {
    let path = match loc.get_path(config) {
        dir if dir.is_dir() => dir,

        // If loc is not a dir, or doesn't exist, assume it is a file name.
        file => match file.parent() {
            Some(p) => p.to_owned(),
            None => return Ok(true),
        },
    };

    match path.try_exists() {
        Ok(true) => {
            return Ok(true);
        },
        Ok(false) => {
            create_dir_all(path)?; return Ok(false);
        },
        Err(e) => {
            return Err(e);
        },
    }
}

fn create(config: &Config, loc: &Location) -> std::io::Result<File> {
    ensure_dir(config, loc)?;
    let path = loc.get_path(config);
    return File::create(&path);
}

fn open(config: &Config, loc: &Location) -> std::io::Result<File> {
    let path = loc.get_path(config);
    return File::open(&path);
}

pub fn read_bytes(config: &Config, loc: &Location, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    let mut f = open(config, loc)?;
    return f.read_to_end(buf);
}

pub fn read_string(config: &Config, loc: &Location, buf: &mut String) -> std::io::Result<usize> {
    let mut f = open(config, loc)?;
    return f.read_to_string(buf);
}

pub fn write(config: &Config, loc: &Location, buf: &[u8]) -> std::io::Result<()> {
    let mut f = create(config, loc)?;
    return f.write_all(buf);
}

pub fn write_obj(config: &Config, buf: &[u8]) -> std::io::Result<Hash> {
    let mut hasher = Sha256::new();
    hasher.update(buf);
    let hash: Hash = hasher.finalize();
    let loc = Location::Object(hash);

    write(config, &loc, buf)?;
    return Ok(hash);
}

pub fn write_op<T>(config: &Config, op: T) -> std::io::Result<Hash> where T: CmRDT::Operation {
    let hash = op.get_hash();
    let loc = Location::Object(hash);

    write(config, &loc, op.serialize_to_str()?.as_bytes())?;
    return Ok(hash);
}

pub fn delete(config: &Config, loc: &Location) -> std::io::Result<()> {
    let path = loc.get_path(config);
    Ok(trash::delete(&path).expect("Error deleting file."))
}
