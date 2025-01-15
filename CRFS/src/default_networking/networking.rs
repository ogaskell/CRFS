use crate::errors;
use crate::types::{Hash, RawData};

use std::net;

use json;
use uuid::Uuid;

use reqwest;

pub struct SystemStatus {
    pub server: Option< net::SocketAddr >,
    pub user: Option< Uuid >, pub user_dn: Option< String >,
    pub filesystem: Option< Uuid >, pub filesystem_dn: Option< String >,
    pub replica: Option< Uuid >,
    pub ready: bool, pub data_ready: bool,  // Is the local replica ready; if false, data_ready holds whether the server has the files ready
    pub pull_changes: bool,  // does the server have changes we don't?
    pub push_changes: bool,  // do we have changes the server doesn't?

}

pub fn send_json_message(status: SystemStatus, body: json::JsonValue) -> Result<(u16, json::JsonValue), ()> {
    let host = status.server.unwrap();

    let body = json::stringify(body);

    let client = reqwest::blocking::Client::new();
    let res = client.post(String::from("http://") + &host.to_string() + "/api/")
        .body(body)
        .send()
        .unwrap();

    let status = res.status().as_u16();
    let res_body = json::parse(res.text().unwrap().as_str()).unwrap();

    return Ok((status, res_body));
}
