use super::{DriverID, FileManager};
use crate::storage;
use crate::tests::storage_test::TESTFILEDIR;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use uuid::Uuid;

#[test]
fn test_listdir() {
    let mut path = PathBuf::from(TESTFILEDIR); path.push("managertest");

    let config = storage::Config {working_dir: path};
    let manager = FileManager::init(config, Uuid::from_u128(1));

    dbg!(manager.list_dir().unwrap());

    let mut result = manager.list_dir().unwrap(); result.sort();
    let mut expected: Vec<_> = vec!(
        "a.md",
        "1/b.md",
        "1/c.md",
        "2/d.md",
        "2/3/e.md",
    ).into_iter().map(|x| PathBuf::from(x)).collect(); expected.sort();

    assert_eq!(result, expected);
}

#[test]
fn test_filemanager() {
    let mut path = PathBuf::from(TESTFILEDIR); path.push("managertest");

    let config = storage::Config {working_dir: path}; let uuid = Uuid::from_u128(1);
    let mut manager = FileManager::init(config, uuid);

    while let Ok(Some(op)) = manager.prep() {
        dbg!(&op);

        manager.apply_op(&op).unwrap();
    }

    manager.write_out().unwrap();
}

#[test]
fn test_persistent_fm() {
    let mut path = PathBuf::from(TESTFILEDIR); path.push("managertest");
    let config = storage::Config {working_dir: path};
    let mut manager = FileManager::read_or_init(&config, Uuid::from_u128(1)).unwrap();

    while let Ok(Some(op)) = manager.prep() {
        dbg!(&op);

        manager.apply_op(&op).unwrap();
    }

    manager.update_drivers().unwrap();

    dbg!(manager.get_history());

    manager.write_out().unwrap();
}

#[test]
fn test_apply() {
    let mut path1 = PathBuf::from(TESTFILEDIR); path1.push("apply1");
    let mut path2 = PathBuf::from(TESTFILEDIR); path2.push("apply2");

    let conf1 = storage::Config {working_dir: path1}; let conf2 = storage::Config {working_dir: path2};

    let mut manager1 = FileManager::read_or_init(&conf1, Uuid::from_u128(1)).unwrap();
    let mut manager2 = FileManager::read_or_init(&conf2, Uuid::from_u128(2)).unwrap();

    println!("Setup ok;");

    manager1.update().unwrap(); manager2.update().unwrap();

    let ops1 = manager1.get_history().all_hashes();
    let ops2 = manager2.get_history().all_hashes();

    let needs1 = ops2.difference(&ops1).collect();
    let needs2 = ops1.difference(&ops2).collect();

    manager1.apply_ops(&needs1).unwrap();
    manager2.apply_ops(&needs2).unwrap();

    manager1.write_out().unwrap(); manager2.write_out().unwrap();
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SerdeDriverIDTest {
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    map: HashMap<DriverID, String>,
}

#[test]
fn test_driverid_serde() {
    let ids = vec!(
        DriverID::FileTree, DriverID::Driver(0), DriverID::Driver(1), DriverID::Driver(2613584385057160389u64),
    );

    let json: Vec<_> = ids.iter().map(|id| serde_json::to_string(id).unwrap()).collect();

    let de_ids: Vec<DriverID> = json.iter().map(|j| {
        dbg!(j); let res = serde_json::from_str(j).unwrap(); dbg!(&res); res
    }).collect();

    assert_eq!(ids, de_ids);


    let mut map: SerdeDriverIDTest = SerdeDriverIDTest{map: HashMap::new()};
    map.map.insert(DriverID::Driver(2613584385057160389), "huh".to_owned());

    let json = serde_json::to_string(&map).unwrap();

    println!("{}", &json);

    let res = serde_json::from_str(&json).unwrap();
    assert_eq!(&map, &res);
}
