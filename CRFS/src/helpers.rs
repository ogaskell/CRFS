use std::fs::create_dir_all;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};

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
