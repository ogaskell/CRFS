use crate::errors;
use crate::types::{Hash, RawData};

use std::net;

use json;
use regex::Regex;
use uuid::Uuid;

use reqwest;

type VersionNumber = (u32, u32, u32);

pub struct SystemStatus {
    pub server: Option< net::SocketAddr >,
    pub user: Option< Uuid >, pub user_dn: Option< String >,
    pub filesystem: Option< Uuid >, pub filesystem_dn: Option< String >,
    pub replica: Option< Uuid >,
    pub ready: bool, pub data_ready: bool,  // Is the local replica ready; if false, data_ready holds whether the server has the files ready
    pub pull_changes: bool,  // does the server have changes we don't?
    pub push_changes: bool,  // do we have changes the server doesn't?

}

pub struct Response {
    pub version: VersionNumber,
    pub transaction_id: i64,
    pub reply: bool,
    pub message_type: String,
    pub payload: json::object::Object,
    pub notifications: json::Array,
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

pub fn str_to_ver(version: String) -> Result<VersionNumber, ()> {
    let re = Regex::new(r"(\d+)\.(\d+)(?:\.(\d+))?").unwrap();
    let Some(cap) = re.captures(&version) else {return Err(());};

    let ver: VersionNumber = (
        cap[1].parse().unwrap(),
        cap[2].parse().unwrap(),
        match cap.get(3) {
            Some(m) => m.as_str().parse().unwrap(),
            None => 0u32,
        },
    );

    return Ok(ver);
}

pub fn res_body_to_response(res_body: json::JsonValue) -> Result<Response, ()> {
    let ver = str_to_ver(String::from(res_body["version"].as_str().unwrap())).unwrap();
    let tid = res_body["transaction_id"].as_str().unwrap().parse().unwrap();
    let reply = res_body["reply"].as_bool().unwrap();
    let message_type = String::from(res_body["message_type"].as_str().unwrap());
    let payload = match res_body["payload"].clone() {
        json::JsonValue::Object(o) => o,
        _ => panic!(),
    };
    let notifications = match res_body["notifications"].clone() {
        json::JsonValue::Array(a) => a,
        _ => panic!(),
    };

    return Ok(Response{
        version: ver,
        transaction_id: tid,
        reply: reply,
        message_type: message_type,
        payload: payload,
        notifications: notifications,
    })
}
