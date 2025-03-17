use crate::default_networking::networking;

use std::net::{Ipv4Addr, SocketAddrV4};

use json;

#[test]
pub fn check_user_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let status = networking::Status {
        server: Some(std::net::SocketAddr::V4(socket)),
        user: None,
        fs: None,
        replica: None,
        ready: false, data_ready: false,
        pull_changes: false, push_changes: false,
    };

    let body = json::parse(r#"

{
    "type": "check_user",
    "transaction_id": "0",
    "payload": {
        "user_uuid": "ad0ff637-87a8-4c64-a1a0-2ed08ec15e66"
    }
}

"#).unwrap();

    let (code, body) = match networking::send_json_message(status, body) {
        Ok(v) => v,
        Err(networking::Error::CRFSErr(code, msg)) => {panic!("CRFS Error: Code {}, Message:\n\t{}\n", code, msg);},
        Err(networking::Error::JsonErr(e)) => {panic!("JSON Error: \n\t{:?}\n", e);},
        Err(networking::Error::ReqwestErr(e)) => {panic!("Reqwest Error: \n\t{:?}\nDid you forget to start the server?\n", e);},
    };
    // println!("Code {}, body:\n{}\n> end body", code, body.to_string());

    assert_eq!(code, 200u16);

    let response = networking::res_body_to_response(body).unwrap();

    assert_eq!(response.version, (1u32, 0u32, 0u32));
    assert_eq!(response.transaction_id, 0i64);
    assert_eq!(response.reply, true);
    assert_eq!(response.message_type, "check_user");

    println!(
        "Payload:\n{}\n\nNotifications:\n{}\n",
        json::stringify_pretty(response.payload, 2),
        json::stringify_pretty(response.notifications, 2)
    );
}
