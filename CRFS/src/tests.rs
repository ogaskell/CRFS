use crate::default_networking::networking;

use std::net::{Ipv4Addr, SocketAddrV4};

use json;

#[test]
pub fn check_user_test() {
    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000);

    let status = networking::SystemStatus {
        server: Some(std::net::SocketAddr::V4(socket)),
        user: None, user_dn: None,
        filesystem: None, filesystem_dn: None,
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

    let (code, body) = networking::send_json_message(status, body).unwrap();
    println!("Code {}, body:\n{}\n> end body", code, body.to_string());

    assert_eq!(code, 200u16);
}
