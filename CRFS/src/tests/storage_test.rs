use crate::default_storage::storage;
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
    let hash = hex!("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    let location = storage::ObjectLocation::ObjectStore(GenericArray::from(hash));

    let path = PathBuf::from(".crfs/objects/b9/4d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9/");

    assert_eq!(path, location.get_path().unwrap());
}

#[test]
pub fn test_read_write_ondisk() {
    // Setup
    storage::ensure_dir(PathBuf::from(TESTFILEDIR)).unwrap();

    let mut path = PathBuf::new();
    path.push(TESTFILEDIR); path.push("ondisktest.txt");
    let loc = storage::ObjectLocation::OnDisk(path.clone());

    // Write Data
    let mut f = storage::ObjectFile::create_on_disk(path).unwrap();
    let write_buf = b"test data!\n";

    let written_bytes = f.write(write_buf).unwrap();
    println!("Wrote {written_bytes}B");

    // Read Data
    f = storage::ObjectFile::open(loc.clone()).unwrap();
    let mut read_buf = [0u8; 12];
    let read_bytes = f.read(&mut read_buf).unwrap();
    println!("Read {read_bytes}B");

    // Assert
    assert_eq!(read_buf[..read_bytes], write_buf[..written_bytes]);
}

#[test]
pub fn test_read_ondisk_doesntexist() {
    // Setup
    storage::ensure_dir(PathBuf::from(TESTFILEDIR)).unwrap();
    let mut path = PathBuf::new(); path.push(TESTFILEDIR); path.push("notexist.txt");

    match remove_file(path.clone()) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }.unwrap();

    // Try to open file
    let loc = storage::ObjectLocation::OnDisk(path.clone());
    let f = storage::ObjectFile::open(loc);
    match f {
        Ok(_) => panic!(),
        Err(_) => {},
    }
}

#[test]
pub fn test_read_write_object() {
    // Write Data
    let write_buf = b"test data!\n";
    let (hash, written_bytes) =
        storage::ObjectFile::create_object(write_buf).unwrap();
    println!("Wrote {written_bytes}B to object {:x}", hash);

    // Read Data
    let loc = storage::ObjectLocation::ObjectStore(hash);
    let mut f = storage::ObjectFile::open(loc.clone()).unwrap();
    let mut read_buf = [0u8; 12];
    let read_bytes = f.read(&mut read_buf).unwrap();
    println!("Read {read_bytes}B");

    assert_eq!(read_buf[..read_bytes], read_buf[..written_bytes]);
}

#[test]
pub fn test_read_write_meta() {
    // Open File
    let id = uuid::Uuid::from_bytes([0u8; 16]);
    let mut f = storage::MetaFile::<MetaTest>::create(id).unwrap();

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
    f = storage::MetaFile::<MetaTest>::open(id).unwrap();
    let result = f.read().unwrap();

    // Check
    assert_eq!(data, result);
}

#[test]
pub fn test_ensure_dir() {
    // Handle AlreadyExists error (since this is somewhat expected), but still raise other errors
    match create_dir(".testfiles") {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }.unwrap();
    let test_dir = PathBuf::from(".testfiles/testingdir");

    // Handle NotFound error (since this is somewhat expected), but still raise other errors
    match remove_dir(test_dir.clone()) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }.unwrap();

    // Test on non-existing dir
    assert_eq!(storage::ensure_dir(test_dir.clone()).unwrap(), false);

    // Test on existing dir
    assert_eq!(storage::ensure_dir(test_dir.clone()).unwrap(), true);
}
