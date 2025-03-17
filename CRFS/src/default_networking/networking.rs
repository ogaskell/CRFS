use crate::errors;

use std::net;

use json;
use regex::Regex;
use reqwest;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

type VersionNumber = (u32, u32, u32);

#[derive(Debug)]
pub enum Error {
    CRFSErr(errors::ErrorCode, String),
    ReqwestErr(reqwest::Error),
    JsonErr(json::Error),
}

type Result<T> = std::result::Result<T, Error>;

macro_rules! crfserror {
    ($code: expr, $msg: expr) => {Err(Error::CRFSErr($code, $msg))};
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {pub id: Option<Uuid>, pub disp_name: Option<String>}

#[derive(Serialize, Deserialize)]
pub struct FileSystemInfo {pub user: UserInfo, pub id: Option<Uuid>, pub disp_name: Option<String>}

#[derive(Serialize, Deserialize)]
pub struct ReplicaInfo {pub fs: FileSystemInfo, pub id: Option<Uuid>, pub disp_name: Option<String>}

#[derive(Serialize, Deserialize)]
pub struct Status {
    pub server: Option< net::SocketAddr >,
    pub info: ReplicaInfo,
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

pub fn send_json_message(status: Status, body: json::JsonValue) -> Result<(u16, json::JsonValue)> {
    let host = status.server.unwrap();

    let body = json::stringify(body);

    let client = reqwest::blocking::Client::new();
    let res = match client.post(String::from("http://") + &host.to_string() + "/api/")
        .body(body)
        .send() {
            Ok(r) => r,
            Err(e) => return Err(Error::ReqwestErr(e)),
        };

    let status = res.status().as_u16();
    let res_body = match json::parse(res.text().unwrap().as_str()) {
        Ok(j) => j,
        Err(e) => return Err(Error::JsonErr(e)),
    };

    return Ok((status, res_body));
}

pub fn str_to_ver(version: String) -> Result<VersionNumber> {
    let malformed: Result<VersionNumber> = crfserror!(errors::CODE_MALFORMED, format!("Version String {version} malformed."));

    let re = Regex::new(r"(\d+)\.(\d+)(?:\.(\d+))?").unwrap();
    let Some(cap) = re.captures(&version) else {
        return malformed;
    };

    let ver: VersionNumber = (
        match cap[1].parse() {Ok(v) => v, Err(_) => return malformed},
        match cap[2].parse() {Ok(v) => v, Err(_) => return malformed},
        match cap.get(3) {
            Some(m) => match m.as_str().parse() {
                Ok(v) => v,
                Err(_) => return malformed,
            },
            None => 0u32,
        },
    );

    return Ok(ver);
}

pub fn res_body_to_response(res_body: json::JsonValue) -> Result<Response> {
    let malformed: Result<Response> = crfserror!(errors::CODE_MALFORMED, format!("Reponse malformed."));

    let ver = str_to_ver(
        String::from(res_body["version"].as_str().unwrap())
    )?;
    let Ok(tid) = res_body["transaction_id"].as_str().unwrap().parse() else {return malformed};
    let Some(reply) = res_body["reply"].as_bool() else {return malformed};

    let message_type = match res_body["message_type"].as_str() {
        Some(v) => String::from(v),
        None => return malformed,
    };
    let payload = match res_body["payload"].clone() {
        json::JsonValue::Object(o) => o,
        _ => return malformed,
    };
    let notifications = match res_body["notifications"].clone() {
        json::JsonValue::Array(a) => a,
        _ => return malformed,
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
