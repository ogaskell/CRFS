use super::Config;

use std::{
    fs::{File, create_dir_all},
    io::{Read, Write},
    path::PathBuf,
};

use serde::{Serialize, Deserialize, de::DeserializeOwned};

fn get_root(config: &Config) -> PathBuf {
    let mut path = PathBuf::new();
    path.push(&config.working_dir);
    path.push(super::METADIR);
    return path;
}

fn get_path(config: &Config, name: &String) -> PathBuf {
    let mut path = get_root(config); path.push(name); path.set_extension("json");
    return path;
}

/// Returns `Ok(true)` if the directory existed, `Ok(false)` if we had to create it, `Err(_)` otherwise.
pub fn ensure_dir(config: &Config, name: &String) -> std::io::Result<bool> {
    let path = match get_path(config, name) {
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

pub fn read<T>(config: &Config, name: &String) -> std::io::Result<T> where T: DeserializeOwned {
    let path = get_path(config, name);
    return read_at(config, &path, false);
}

pub fn read_at<T>(config: &Config, path: &PathBuf, relative: bool) -> std::io::Result<T> where T: DeserializeOwned {
    let path = if relative {
        let mut _path = config.working_dir.clone(); _path.push(path); _path
    } else {
        path.clone()
    };

    let mut f = File::open(path)?;

    let mut json = String::new(); f.read_to_string(&mut json)?;

    return Ok(serde_json::from_str(&json)?);
}

pub fn write<T>(config: &Config, name: &String, data: &T) -> std::io::Result<()> where T: Serialize + std::fmt::Debug {
    ensure_dir(config, name)?;
    let path = get_path(config, name);
    return write_at(config, &path, false, data);
}

pub fn write_at<T>(config: &Config, path: &PathBuf, relative: bool, data: &T) -> std::io::Result<()> where T: Serialize + std::fmt::Debug {
    let path = if relative {
        let mut _path = config.working_dir.clone(); _path.push(path); _path
    } else {
        path.clone()
    };

    let json = serde_json::to_string(data)?;

    let mut f = File::create(&path)?;
    f.write_all(json.as_bytes())?;

    return Ok(());
}
