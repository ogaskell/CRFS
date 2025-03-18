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

pub fn get_named_arg(args: &Vec<String>, short: Option<&str>, long: Option<&str>) -> Option<String> {
    if short == None && long == None {panic!("Must provide short or long argument name.")}

    let mut short_pre = String::from("-");
    let mut long_pre = String::from("--");

    let search_s = match short {
        Some(s) => {short_pre.push_str(&s); args.iter().position(|x| *x == short_pre)},
        None => None,
    };

    let search_l = match long {
        Some(l) => {long_pre.push_str(&l); args.iter().position(|x| *x == long_pre)},
        None => None,
    };

    let i = match (search_s, search_l) {
        (Some(i), _) => i,
        (None, Some(i)) => i,
        (None, None) => return None,
    };

    let s = args.get(i + 1)?;
    return Some(s.clone());
}
