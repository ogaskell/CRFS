use crate::helpers::ensure_dir;
use crate::types::Hash;

use std::fs::{File, create_dir_all};
use std::io::{BufReader, Error, ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::option::Option;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use serde_json;
use sha2::{Sha256, Digest};
use uuid::Uuid;

pub const GLOBALCONF: &str = ".config/crfs/config.json";
const OBJECTDIR: &str = ".crfs/objects/";
const METADIR: &str = ".crfs/meta/";

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

// OBJECT FILE HANDLING
#[derive(Clone)]
pub enum ObjectLocation {
    ObjectStore(Hash),
    OnDisk(PathBuf),
}

impl ObjectLocation {
    pub fn get_path(&self, stat: &Config) -> std::io::Result<PathBuf> {
        match self {
            ObjectLocation::OnDisk(p) =>
                Ok(p.clone()),
            ObjectLocation::ObjectStore(h) =>
                Ok(ObjectLocation::hash_to_path(stat, h.clone())?),
        }
    }

    pub fn hash_to_path(stat: &Config, hash: Hash) -> std::io::Result<PathBuf> {
        let hex = format!("{:x}", hash);

        let mut path = PathBuf::new();
        path.push(stat.working_dir.clone());
        path.push(OBJECTDIR);
        path.push(&hex[0..2]);
        path.push(&hex[2..]);

        return Ok(path);
    }

    fn clone(&self) -> Self {
        match self {
            ObjectLocation::ObjectStore(h) =>
                ObjectLocation::ObjectStore(h.clone()),
            ObjectLocation::OnDisk(p) =>
                ObjectLocation::OnDisk(p.clone()),
        }
    }
}

pub struct ObjectFile {
    file: File,
    loc: ObjectLocation,
    path: PathBuf,
}

impl ObjectFile {
    pub fn open(stat: &Config, loc: ObjectLocation) -> std::io::Result<ObjectFile> {
        let p = loc.get_path(stat)?;

        match p.try_exists() {
            Ok(true) => {
                let f = File::open(p.clone())?;
                Ok(ObjectFile {
                    file: f,
                    loc, path: p,
                })
            },
            Ok(false) => Err(Error::new(ErrorKind::NotFound, "Broken symlink.")),
            Err(e) => Err(e),
        }
    }

    pub fn create_on_disk(path: PathBuf) -> std::io::Result<ObjectFile> {
        let loc = ObjectLocation::OnDisk(path.clone());
        match path.parent() {
            Some(parent) => {ensure_dir(PathBuf::from(parent))?;},
            None => {},
        }

        let f = File::create(path.clone())?;
        Ok(ObjectFile {
            file: f,
            loc, path,
        })
    }

    pub fn create_object(stat: &Config, buf: &[u8]) -> std::io::Result<(Hash, usize)> {
        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(buf);
        let hash: Hash = hasher.finalize();

        // Find location
        let loc = ObjectLocation::ObjectStore(hash);
        let path = loc.get_path(stat)?; let parent = path.parent().unwrap();

        // Ensure directory exists
        ensure_dir(PathBuf::from(parent)).unwrap();

        // Write data
        let mut f = ObjectFile::create_on_disk(path)?;
        let bytes = f.write(buf);

        return match bytes {
            Ok(b) => Ok((hash, b)),
            Err(e) => Err(e),
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
}

// METADATA FILE HANDLING
pub struct MetaFile<T: serde::ser::Serialize + serde::de::DeserializeOwned> {
    file: File,
    id: Option<Uuid>, path: PathBuf,
    data_type: PhantomData<T>,
}

impl<T: serde::ser::Serialize + serde::de::DeserializeOwned> MetaFile<T> {
    pub fn get_path_from_id(config: &Config, id: Uuid) -> PathBuf {
        // Stringify UUID
        let mut encode_buf = Uuid::encode_buffer();
        let uuid_str = id.simple().encode_lower(&mut encode_buf);

        // Compute path
        let mut p = PathBuf::new();
        p.push(config.working_dir.clone());
        p.push(METADIR);
        p.push(uuid_str);
        p.set_extension("json");

        return p;
    }

    pub fn open_path(path: PathBuf) -> std::io::Result<MetaFile<T>> {
        let f = File::open(path.clone())?;
        Ok(MetaFile::<T>{
            file: f, id: None, path: path.clone(), data_type: PhantomData,
        })
    }

    pub fn open(config: &Config, id: Uuid) -> std::io::Result<MetaFile<T>> {
        let p = MetaFile::<T>::get_path_from_id(config, id);
        MetaFile::<T>::open_path(p)
    }

    pub fn create(config: &Config, id: Uuid) -> std::io::Result<MetaFile<T>> {
        let path: PathBuf = MetaFile::<T>::get_path_from_id(config, id.clone());

        let parent = path.parent().unwrap();
        ensure_dir(PathBuf::from(parent))?;

        let f = File::create(path.clone())?;
        Ok(MetaFile::<T> {
            file: f, id: Some(id.clone()), path: path.clone(), data_type: PhantomData,
        })
    }

    pub fn create_at_path(path: PathBuf) -> std::io::Result<MetaFile<T>> {
        ensure_dir(PathBuf::from(path.parent().unwrap())).unwrap();
        let f = File::create(path.clone())?;
        Ok(MetaFile::<T> {
            file: f, id: None, path: path.clone(), data_type: PhantomData,
        })
    }

    pub fn write(&mut self, object: &T) -> std::io::Result<usize> {
        let raw_json = serde_json::to_string(object).unwrap();
        let buf: &[u8] = raw_json.as_bytes();
        self.file.write(buf)
    }

    pub fn read(&mut self) -> std::io::Result<T> {
        let reader = BufReader::new(&self.file);
        let object = serde_json::from_reader(reader).unwrap();

        Ok(object)
    }
}
