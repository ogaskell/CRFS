use std::{collections::HashSet, net::{Ipv4Addr, SocketAddr, SocketAddrV4}, path::PathBuf};

use uuid::uuid;

use crate::{storage, networking, tests::storage_test::TESTFILEDIR, types};
use networking::api;

const TEST_USER: &'static str = "ad0ff637-87a8-4c64-a1a0-2ed08ec15e66";
const TEST_FS: &'static str = "00000000-0000-0000-0000-000000000001"; // Will not be a "valid" FS!

#[test]
fn ping_test() {
    let mut config = networking::Config::empty();
    config.server = Some(SocketAddr::V4(
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000)
    ));

    let message = api::Message::new(
        api::MessagePayload::Ping {  }
    );

    let (code, res) = message.send(&config).unwrap();
    res.unwrap(&message);

    assert_eq!(code, 200);
}

#[test]
fn register_user_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut config = networking::Config::empty();
    config.server = Some(std::net::SocketAddr::V4(socket));
    config.info.fs.user.id = Some(uuid!(TEST_USER));

    let message = api::Message::new(api::MessagePayload::RegisterUser {
        user_uuid: config.info.get_user_id().unwrap(),
        display_name: "Test User".to_owned()
    });
    println!("{}", serde_json::to_string_pretty(&message).unwrap());

    let (code, res) = message.send(&config).unwrap();
    assert_eq!(code, 200);

    dbg!(&res);
}

#[test]
fn check_user_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut config = networking::Config::empty();
    config.server = Some(std::net::SocketAddr::V4(socket));
    config.info.fs.user.id = Some(uuid!(TEST_USER));

    let message = api::Message::new(api::MessagePayload::CheckUser { user_uuid: config.info.get_user_id().unwrap() });
    println!("{}", serde_json::to_string_pretty(&message).unwrap());

    let (code, res) = message.send(&config).unwrap();
    assert_eq!(code, 200);

    dbg!(&res);
}

#[test]
fn register_fs_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut config = networking::Config::empty();
    config.server = Some(std::net::SocketAddr::V4(socket));
    config.info.fs.user.id = Some(uuid!(TEST_USER));
    config.info.fs.id = Some(uuid!(TEST_FS));

    let message = api::Message::new(api::MessagePayload::RegisterFs {
        user_uuid: config.info.get_user_id().unwrap(),
        fs_uuid: config.info.get_fs_id().unwrap(),
        display_name: "Test FS".to_owned(),
        fs_opts: Vec::new(),
    });
    println!("{}", serde_json::to_string_pretty(&message).unwrap());

    let (code, res) = message.send(&config).unwrap();
    dbg!(&res);

    assert_eq!(code, 200);
}

#[test]
fn check_fs_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut config = networking::Config::empty();
    config.server = Some(std::net::SocketAddr::V4(socket));
    config.info.fs.user.id = Some(uuid!(TEST_USER));
    config.info.fs.id = Some(uuid!(TEST_FS));

    let message = api::Message::new(api::MessagePayload::CheckFs {
        user_uuid: config.info.get_user_id().unwrap(),
        fs_uuid: config.info.get_fs_id().unwrap(),
    });
    println!("{}", serde_json::to_string_pretty(&message).unwrap());

    let (code, res) = message.send(&config).unwrap();
    dbg!(&res);

    assert_eq!(code, 200);
}

#[test]
fn put_get_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut netconfig = networking::Config::empty();
    netconfig.server = Some(std::net::SocketAddr::V4(socket));
    netconfig.info.fs.id = Some(uuid!(TEST_FS));

    let storageconfig = storage::Config {working_dir: PathBuf::from(TESTFILEDIR)};

    let data = "test operation data";
    let hash = storage::object::write_obj(&storageconfig, data.as_bytes()).expect("Write error");

    netconfig.push_op(&storageconfig, &hash).expect("Push error");
    netconfig.fetch_op(&storageconfig, &hash).expect("Pull error");

    let mut read_buf = String::new();
    storage::object::read_string(&storageconfig, &storage::object::Location::Object(hash), &mut read_buf).expect("Read error");

    assert_eq!(data, read_buf);
}

#[test]
fn push_fetch_state_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut config = networking::Config::empty();
    config.server = Some(std::net::SocketAddr::V4(socket));
    config.info.fs.user.id = Some(uuid!(TEST_USER));
    config.info.fs.id = Some(uuid!(TEST_FS));

    let push_state: HashSet<types::Hash> = vec!(types::calculate_hash("one"), types::calculate_hash("two"), types::calculate_hash("three")).into_iter().collect();
    config.push_state(push_state.clone()).expect("Push error");

    let pull_state = config.fetch_state().expect("Fetch error");

    for h in push_state.iter() {
        assert!(pull_state.contains(h))
    }
}
