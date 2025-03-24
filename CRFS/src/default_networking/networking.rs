use crate::errors;

use std::net;

use json;
use json::JsonValue;
use rand::Rng;
use regex::Regex;
use reqwest;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

macro_rules! crfserror {
    ($code: expr, $msg: expr) => {Err(Error::CRFSErr($code, $msg))};
}

#[derive(Debug, PartialEq)]
pub struct VersionNumber(pub u32, pub u32, pub u32);
pub const VERSION: VersionNumber = VersionNumber(0, 0, 1);

impl VersionNumber {
    pub fn from_str(version: String) -> Result<Self> {
        let malformed: Result<VersionNumber> = crfserror!(errors::CODE_MALFORMED, format!("Version String {version} malformed."));

        let re = Regex::new(r"(\d+)\.(\d+)(?:\.(\d+))?").unwrap();
        let Some(cap) = re.captures(&version) else {
            return malformed;
        };

        let ver: VersionNumber = VersionNumber(
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

    pub fn to_str(&self) -> String {
        format!("{}.{}.{}", self.0, self.1, self.2)
    }
}

#[derive(Debug)]
pub enum Error {
    CRFSErr(errors::ErrorCode, String),
    ReqwestErr(reqwest::Error),
    JsonErr(json::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {pub id: Option<Uuid>, pub disp_name: Option<String>}

impl UserInfo {
    pub fn clone(&self) -> Self {
        return Self { id: self.id.clone(), disp_name: self.disp_name.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSystemInfo {pub user: UserInfo, pub id: Option<Uuid>, pub disp_name: Option<String>}

impl FileSystemInfo {
    pub fn clone(&self) -> Self {
        return Self { user: self.user.clone(), id: self.id.clone(), disp_name: self.disp_name.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplicaInfo {pub fs: FileSystemInfo, pub id: Option<Uuid>, pub disp_name: Option<String>}

impl ReplicaInfo {
    pub fn clone(&self) -> Self {
        return Self { fs: self.fs.clone(), id: self.id.clone(), disp_name: self.disp_name.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub server: Option<net::SocketAddr>,
    pub info: ReplicaInfo,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            server: None,
            info: ReplicaInfo {
                id: None, disp_name: None, fs: FileSystemInfo {
                    id: None, disp_name: None, user: UserInfo {
                        id: None, disp_name: None,
                    }
                }
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Status {
    pub ready: bool, pub data_ready: bool,  // Is the local replica ready; if false, data_ready holds whether the server has the files ready
    pub pull_changes: bool,  // does the server have changes we don't?
    pub push_changes: bool,  // do we have changes the server doesn't?
}

impl Status {
    pub fn empty() -> Self {
        Status {
            ready: false, data_ready: false,
            pull_changes: false, push_changes: false,
        }
    }

    pub fn clone(&self) -> Self {
        Status {
            ready: self.ready, data_ready: self.data_ready,
            pull_changes: self.pull_changes, push_changes: self.push_changes,
        }
    }
}

pub struct Message {
    transaction_id: Option<i64>,
    message_type: String,
    payload: json::JsonValue,
}

impl Message {
    pub fn to_json(&self) -> JsonValue {
        let mut rng = rand::rng();

        let mut root = JsonValue::new_object();

        root["version"] = JsonValue::from(VERSION.to_str());
        root["transaction_id"] = JsonValue::from(match self.transaction_id {
            Some(v) => v,
            None => rng.random::<i64>(),
        });
        root["reply"] = JsonValue::from(false);
        root["type"] = JsonValue::from(self.message_type.clone());
        root["payload"] = JsonValue::from(self.payload.clone());

        return root;
    }
}

#[derive(Debug)]
pub struct Response {
    pub version: VersionNumber,
    pub transaction_id: i64,
    pub reply: bool,
    pub message_type: String,
    pub payload: json::object::Object,
    pub notifications: json::Array,
}

pub fn send_json_message(config: Config, body: JsonValue) -> Result<(u16, JsonValue)> {
    let host = config.server.unwrap();

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

pub fn res_body_to_response(res_body: JsonValue) -> Result<Response> {
    let malformed: Result<Response> = crfserror!(errors::CODE_MALFORMED, format!("Reponse malformed."));

    println!("{:?}", res_body);

    let ver = VersionNumber::from_str(
        String::from(res_body["version"].as_str().unwrap())
    )?;
    let Some(tid) = res_body["transaction_id"].as_i64() else {return malformed};
    let Some(reply) = res_body["reply"].as_bool() else {return malformed};

    let message_type = match res_body["message_type"].as_str() {
        Some(v) => String::from(v),
        None => return malformed,
    };
    let payload = match res_body["payload"].clone() {
        JsonValue::Object(o) => o,
        _ => return malformed,
    };
    let notifications = match res_body["notifications"].clone() {
        JsonValue::Array(a) => a,
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

pub fn ping_server(config: Config) -> Result<Response> {
    let message = Message {
        transaction_id: None,
        message_type: String::from("ping"),
        payload: JsonValue::new_object(),
    }.to_json();

    let (code, res_) = send_json_message(config, message)?;
    let res = res_body_to_response(res_)?;

    if code == 200 {return Ok(res)} else {
        return crfserror!(errors::CODE_ERROR, format!("Got code {code} from ping. Reply was:\n\t{:?}", res));
    };
}

pub fn check_user(config: Config) -> Result<Response> {
    let mut message = Message {
        transaction_id: None,
        message_type: String::from("check_user"),
        payload: JsonValue::new_object(),
    };

    let mut encode_buf = Uuid::encode_buffer();
    let uuid_str = config.info.fs.user.id.clone().unwrap().simple().encode_lower(&mut encode_buf);

    message.payload["user_uuid"] = JsonValue::from(String::from(uuid_str));
    let json_msg = message.to_json();

    let (code, res_) = send_json_message(config, json_msg)?;
    let res = res_body_to_response(res_)?;

    let error_code = res.payload["code"].as_u32().unwrap();

    if error_code > 0 {return crfserror!(error_code, String::from(""))}
    else {return Ok(res)}
}

pub fn register_user(config: &mut Config) -> Result<Response> {
    let mut message = Message {
        transaction_id: None,
        message_type: String::from("register_user"),
        payload: JsonValue::new_object(),
    };

    let mut encode_buf = Uuid::encode_buffer();
    let uuid_str = config.info.fs.user.id.clone().unwrap().simple().encode_lower(&mut encode_buf);

    message.payload["user_uuid"] = JsonValue::from(String::from(uuid_str));
    match config.info.fs.user.disp_name.clone() {
        Some(n) => {message.payload["display_name"] = JsonValue::from(n);},
        None => {},
    }

    let json_msg = message.to_json();

    let (code, res_) = send_json_message(config.clone(), json_msg)?;
    let res = res_body_to_response(res_)?;

    let error_code = res.payload["code"].as_u32().unwrap();

    if error_code > 0 {return crfserror!(error_code, String::from(""))}
    else {return Ok(res)}
}
