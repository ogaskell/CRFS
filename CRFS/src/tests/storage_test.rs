use crate::default_storage::storage;
use crate::types::Hash;

use std::fs::{create_dir, remove_file, remove_dir};
use std::path::PathBuf;

use generic_array::GenericArray;
use hex_literal::hex;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const TESTFILEDIR: &str = ".testfiles";

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

    let hash = Hash::from(hex!("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"));
    // let location = storage::ObjectLocation::ObjectStore(config.clone(), Some(GenericArray::from(hash)));
    let loc = storage::object::Location::Object(hash);

    // Manually calculate path
    // let mut act_path = PathBuf::from(TESTFILEDIR);
    // act_path.push(".crfs/objects/b9/4d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9/");
    let act_path = PathBuf::from(".testfiles/.crfs/objects/b9/4d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9/");

    // Use `get_path` (which calls `hash_to_path`)
    let test_path = loc.get_path(&config);

    assert_eq!(act_path, test_path);
}

#[test]
pub fn test_read_write_ondisk() {
    // Setup
    let config = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };
    // ensure_dir(&config, &PathBuf::from(".")).unwrap();

    // let mut path = PathBuf::new();
    // path.push(TESTFILEDIR); path.push("ondisktest.txt");

    let path = PathBuf::from("ondisktest.txt");
    let loc = storage::object::Location::Path(path, true);

    // Write Data
    let write_buf = String::from("test data!\n");
    storage::object::write(&config, &loc, write_buf.as_bytes()).unwrap();

    let mut read_buf = String::new();
    storage::object::read_string(&config, &loc, &mut read_buf).unwrap();

    assert_eq!(write_buf, read_buf);
}

#[test]
pub fn test_read_ondisk_doesntexist() {
    // Setup
    let config = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };
    // ensure_dir(&config, &PathBuf::from(".")).unwrap();

    // let mut path = config.working_dir.clone(); path.push("notexist.txt");
    let path = PathBuf::from("notexist.txt");
    let loc = storage::object::Location::Path(path.clone(), true);

    match remove_file(&path) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }.unwrap();

    // Try to open file
    let mut buf = String::new();
    let f = storage::object::read_string(&config, &loc, &mut buf);
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
    let write_buf = String::from("test data!\n");

    // let mut loc = storage::ObjectLocation::ObjectStore(config.clone(), None);
    // storage::ObjectFile::create_mutloc(&mut loc, write_buf).unwrap();

    let hash = storage::object::write_obj(&config, write_buf.as_bytes()).unwrap();

    println!("Wrote to object {:x}", hash);

    // Read Data
    let loc = storage::object::Location::Object(hash);
    // let mut f = storage::ObjectFile::open(&loc).unwrap();
    let mut read_buf = String::new();
    // let read_bytes = f.read_to_string(&mut read_buf).unwrap();
    // println!("Read {read_bytes}B");

    storage::object::read_string(&config, &loc, &mut read_buf).unwrap();

    assert_eq!(write_buf, read_buf);
}

#[test]
pub fn test_read_write_meta() {
    // Setup
    let config = storage::Config {
        working_dir: PathBuf::from(TESTFILEDIR),
    };

    let name = String::from("meta_test");

    // Create Data
    let mut hasher = Sha256::new();
    hasher.update(b"Test data");
    let hash: Hash = hasher.finalize();

    let data = MetaTest {
        a: 0, b: -10, c: String::from("Test! :)"), d: Uuid::from_u128(0xdeadbeef), e: hash,
    };

    // Write
    storage::meta::write(&config, &name, &data).unwrap();

    // Read
    let result = storage::meta::read(&config, &name).unwrap();

    // Check
    assert_eq!(data, result);
}
