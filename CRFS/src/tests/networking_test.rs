use crate::default_networking::networking;

use std::net::{Ipv4Addr, SocketAddrV4};

use json;
use uuid::{uuid, Uuid};

#[test]
pub fn check_user_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut status = networking::Status::empty();
    status.server = Some(std::net::SocketAddr::V4(socket));
    status.info.fs.user.id = Some(uuid!("ad0ff637-87a8-4c64-a1a0-2ed08ec15e66"));

//     let body = json::parse(r#"

// {
//     "type": "check_user",
//     "transaction_id": 0,
//     "payload": {
//         "user_uuid": "ad0ff637-87a8-4c64-a1a0-2ed08ec15e66"
//     }
// }

// "#).unwrap();

//     let (code, body) = match networking::send_json_message(status, body) {
//         Ok(v) => v,
//         Err(networking::Error::CRFSErr(code, msg)) => {panic!("CRFS Error: Code {}, Message:\n\t{}\n", code, msg);},
//         Err(networking::Error::JsonErr(e)) => {panic!("JSON Error: \n\t{:?}\n", e);},
//         Err(networking::Error::ReqwestErr(e)) => {panic!("Reqwest Error: \n\t{:?}\nDid you forget to start the server?\n", e);},
//     };
//     // println!("Code {}, body:\n{}\n> end body", code, body.to_string());

    let response = networking::check_user(status).unwrap();

    assert_eq!(response.version, networking::VERSION);
    assert_eq!(response.reply, true);
    assert_eq!(response.message_type, "check_user");

    println!(
        "Payload:\n{}\n\nNotifications:\n{}\n",
        json::stringify_pretty(response.payload, 2),
        json::stringify_pretty(response.notifications, 2)
    );
}

#[test]
pub fn test_ping() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let mut status = networking::Status::empty();
    status.server = Some(std::net::SocketAddr::V4(socket));

    networking::ping_server(status).unwrap();
}
