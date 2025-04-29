use crate::default_storage::storage;
use crate::helpers::ensure_dir;
use crate::types::Hash;

use std::fs::{create_dir, remove_file, remove_dir};
use std::path::PathBuf;

use generic_array::GenericArray;
use hex_literal::hex;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const TESTFILEDIR: &str = ".testfiles";

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct MetaTest {
    a: u8, b: i16, c: String, d: Uuid, e: Hash,
}

#[test]
pub fn test_hash_to_path() {
    // Setup
    let config = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };

    let hash = hex!("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    let location = storage::ObjectLocation::ObjectStore(config.clone(), Some(GenericArray::from(hash)));

    // Manually calculate path
    let mut act_path = PathBuf::from(TESTFILEDIR);
    act_path.push(".crfs/objects/b9/4d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9/");

    // Use `get_path` (which calls `hash_to_path`)
    let test_path = location.get_path();

    assert_eq!(act_path, test_path);
}

#[test]
pub fn test_read_write_ondisk() {
    // Setup
    let stat = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };
    ensure_dir(stat.working_dir.clone()).unwrap();

    let mut path = PathBuf::new();
    path.push(TESTFILEDIR); path.push("ondisktest.txt");
    let loc = storage::ObjectLocation::OnDisk(path.clone());

    // Write Data
    let write_buf = b"test data!\n";

    storage::ObjectFile::create(&loc, write_buf).unwrap();

    // Read Data
    let mut f = storage::ObjectFile::open(&loc).unwrap();
    let mut read_buf = String::new();
    let read_bytes = f.read_to_string(&mut read_buf).unwrap();
    println!("Read {read_bytes}B");

    assert_eq!(String::from_utf8(Vec::from(write_buf)).unwrap(), read_buf);
}

#[test]
pub fn test_read_ondisk_doesntexist() {
    // Setup
    let stat = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };
    ensure_dir(stat.working_dir.clone()).unwrap();

    let mut path = stat.working_dir.clone(); path.push("notexist.txt");

    match remove_file(path.clone()) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }.unwrap();

    // Try to open file
    let loc = storage::ObjectLocation::OnDisk(path.clone());
    let f = storage::ObjectFile::open(&loc);
    match f {
        Ok(_) => panic!(),
        Err(_) => {},
    }
}

#[test]
pub fn test_read_write_object() {
    // Setup
    let config = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };

    // Write Data
    let write_buf = b"test data!\n";
    let mut loc = storage::ObjectLocation::ObjectStore(config.clone(), None);
    storage::ObjectFile::create_mutloc(&mut loc, write_buf).unwrap();
    println!("Wrote to object {:x}", loc.get_hash().unwrap());

    // Read Data
    let mut f = storage::ObjectFile::open(&loc).unwrap();
    let mut read_buf = String::new();
    let read_bytes = f.read_to_string(&mut read_buf).unwrap();
    println!("Read {read_bytes}B");

    assert_eq!(String::from_utf8(Vec::from(write_buf)).unwrap(), read_buf);
}

#[test]
pub fn test_read_write_meta() {
    // Setup
    let stat = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };

    // Open File
    let id = uuid::Uuid::from_bytes([0u8; 16]);
    let mut f = storage::MetaFile::<MetaTest>::create(&stat, id).unwrap();

    // Create Data
    let mut hasher = Sha256::new();
    hasher.update(b"Test data");
    let hash: Hash = hasher.finalize();
    let data = MetaTest {
        a: 0, b: -10, c: String::from("Test! :)"), d: id, e: hash,
    };

    // Write
    let written_bytes = f.write(&data).unwrap();
    println!("Wrote {written_bytes}B");

    // Read
    f = storage::MetaFile::<MetaTest>::open(&stat, id).unwrap();
    let result = f.read().unwrap();

    // Check
    assert_eq!(data, result);
}
