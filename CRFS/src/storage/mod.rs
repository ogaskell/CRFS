use std::path::PathBuf;

use serde::{Serialize, Deserialize};

pub const GLOBALCONF: &str = ".config/crfs/config.json"; // Appending to the user's home dir.
const OBJECTDIR: &str = ".crfs/objects/"; // Appended to the working dir.
const METADIR: &str = ".crfs/meta/"; // Appended to the working dir.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub working_dir: PathBuf,
}

pub mod object;
pub mod meta;
