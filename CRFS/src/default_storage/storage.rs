use crate::types::Hash;

use std::fs::{File, create_dir_all};
use std::io::{BufReader, Error, ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde_json;
use sha2::{Sha256, Digest};
use uuid::Uuid;

const OBJECTDIR: &str = ".crfs/objects/";
const METADIR: &str = ".crfs/meta/";

// OBJECT FILE HANDLING
#[derive(Clone)]
pub enum ObjectLocation {
    ObjectStore(Hash),
    OnDisk(PathBuf),
}

impl ObjectLocation {
    pub fn get_path(&self) -> std::io::Result<PathBuf> {
        match self {
            ObjectLocation::OnDisk(p) =>
                Ok(p.clone()),
            ObjectLocation::ObjectStore(h) =>
                Ok(ObjectLocation::hash_to_path(h.clone())?),
        }
    }

    pub fn hash_to_path(hash: Hash) -> std::io::Result<PathBuf> {
        let hex = format!("{:x}", hash);
        let mut path = PathBuf::new();
        path.push(OBJECTDIR); path.push(&hex[0..2]); path.push(&hex[2..]);
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
    pub fn open(loc: ObjectLocation) -> std::io::Result<ObjectFile> {
        let p = loc.get_path()?;

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

    pub fn create_object(buf: &[u8]) -> std::io::Result<(Hash, usize)> {
        ensure_dir(PathBuf::from(OBJECTDIR))?;

        let mut hasher = Sha256::new();
        hasher.update(buf);
        let hash: Hash = hasher.finalize();

        let loc = ObjectLocation::ObjectStore(hash);
        let mut f = ObjectFile::create_on_disk(loc.get_path()?)?;
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
    id: Uuid, path: PathBuf,
    data_type: PhantomData<T>,
}

impl<T: serde::ser::Serialize + serde::de::DeserializeOwned> MetaFile<T> {
    pub fn get_path_from_id(id: Uuid) -> PathBuf {
        let mut p = PathBuf::new();
        let mut encode_buf = Uuid::encode_buffer();
        let uuid_str = id.simple().encode_lower(&mut encode_buf);
        p.push(METADIR); p.push(uuid_str); p.set_extension("json");
        println!("{}", p.clone().to_str().unwrap());

        return p;
    }

    pub fn open(id: Uuid) -> std::io::Result<MetaFile<T>> {
        let p = MetaFile::<T>::get_path_from_id(id);
        let f = File::open(p.clone())?;

        Ok(MetaFile::<T>{
            file: f, id: id.clone(), path: p.clone(), data_type: PhantomData,
        })
    }

    pub fn create(id: Uuid) -> std::io::Result<MetaFile<T>> {
        ensure_dir(PathBuf::from(METADIR))?;
        let path: PathBuf = MetaFile::<T>::get_path_from_id(id.clone());

        let f = File::create(path.clone())?;
        Ok(MetaFile::<T> {
            file: f, id: id.clone(), path: path.clone(), data_type: PhantomData,
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

// HELPER FUNCTIONS
pub fn ensure_dir(path: PathBuf) -> std::io::Result<bool> {
    // Return Ok(true) if the directory existed, Ok(false) if we had to create it, Err(_) otherwise.
    match path.try_exists() {
        Ok(true) => {
            if !path.is_dir() {Err(Error::new(ErrorKind::Other, "Path exists, but is file."))}
            else {Ok(true)}
        },
        Ok(false) => {
            match create_dir_all(path) {
                Ok(()) => Ok(false),
                Err(e) => Err(e),
            }
        },
        Err(e) => Err(e),
    }
}
