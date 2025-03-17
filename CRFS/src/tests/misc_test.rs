use crate::helpers::ensure_dir;

use std::fs::{create_dir, remove_dir};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

#[test]
pub fn test_ensure_dir() {
    // Handle AlreadyExists error (since this is somewhat expected), but still raise other errors
    match create_dir(".testfiles") {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }.unwrap();
    let test_dir = PathBuf::from(".testfiles/testingdir");

    // Handle NotFound error (since this is somewhat expected), but still raise other errors
    match remove_dir(test_dir.clone()) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }.unwrap();

    // Test on non-existing dir
    assert_eq!(ensure_dir(test_dir.clone()).unwrap(), false);

    // Test on existing dir
    assert_eq!(ensure_dir(test_dir.clone()).unwrap(), true);
}
