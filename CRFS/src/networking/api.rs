use std::collections::HashSet;

use rand::Rng;
use regex::Regex;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// use time::serde::iso8601;
use url;

use crate::{errors, types};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct VersionNumber(pub u32, pub u32, pub u32);
pub const VERSION: VersionNumber = VersionNumber(0, 0, 1);

impl From<String> for VersionNumber {
    fn from(item: String) -> Self {
        let re = Regex::new(r"(\d+)\.(\d+)(?:\.(\d+))?").unwrap();
        let Some(cap) = re.captures(&item) else {
            panic!("Malformed version.");
        };

        let ver: VersionNumber = VersionNumber(
            match cap[1].parse() {Ok(v) => v, Err(_) => panic!("Malformed version.")},
            match cap[2].parse() {Ok(v) => v, Err(_) => panic!("Malformed version.")},
            match cap.get(3) {
                Some(m) => match m.as_str().parse() {
                    Ok(v) => v,
                    Err(_) => panic!("Malformed version."),
                },
                None => 0u32,
            },
        );

        return ver;
    }
}

impl Into<String> for VersionNumber {
    fn into(self) -> String {
        format!("{}.{}.{}", self.0, self.1, self.2)
    }
}

pub type TID = u64;
fn unique() -> TID {
    rand::rng().random()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
pub enum MessagePayload {
    Ping {  },
    RegisterUser { user_uuid: Uuid, display_name: String },
    CheckUser { user_uuid: Uuid },
    RegisterFs { user_uuid: Uuid, fs_uuid: Uuid, display_name: String, fs_opts: Vec<String> },
    CheckFs { user_uuid: Uuid, fs_uuid: Uuid },
    Enrol { user_uuid: Uuid, fs_uuid: Uuid, replica_uuid: Uuid },
    // FetchData {  },
    // PostData,
    // AckData,
    FetchState { user_uuid: Uuid, fs_uuid: Uuid },
    PushState { user_uuid: Uuid, fs_uuid: Uuid, ops: HashSet<types::Hash> },
    // AckOperation,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
pub enum ReplyPayload {
    Ping {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    RegisterUser {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    CheckUser {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    RegisterFs {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    CheckFs {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    Enrol {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    // FetchData {  },
    // PostData,
    // AckData,
    FetchState {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,

        #[serde(default)]
        state: HashSet<types::Hash>,
    },
    PushState {
        #[serde(default = "errors::ok")]
        code: errors::ErrorCode,

        #[serde(default)]
        err_msg: String,
    },
    // AckOperation,
}

pub fn correct_reply_type(msg: &MessagePayload, reply: &ReplyPayload) -> bool {
    use MessagePayload as M; use ReplyPayload as R;
    match (msg, reply) {
        (M::Ping {..}, R::Ping {..}) |
        (M::RegisterUser {..}, R::RegisterUser {..}) |
        (M::CheckUser {..}, R::CheckUser {..}) |
        (M::RegisterFs {..}, R::RegisterFs {..}) |
        (M::CheckFs {..}, R::CheckFs {..}) |
        (M::Enrol {..}, R::Enrol {..}) |
        (M::FetchState {..}, R::FetchState {..}) |
        (M::PushState {..}, R::PushState {..})
            => true,
        _ => false,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub version: VersionNumber,
    pub transaction_id: TID,
    pub reply: bool,

    #[serde(flatten)]
    pub payload: MessagePayload,
}

impl Message {
    pub fn new(payload: MessagePayload) -> Self {
        return Self {
            version: VERSION,
            transaction_id: unique(),
            reply: false,
            payload,
        }
    }

    pub fn send(&self, config: &super::Config) -> super::NetResult<(reqwest::StatusCode, Reply)> {
        let json = serde_json::to_string(&self)?;
        let endpoint = config.get_endpoint("api").expect("Server hostname not configured.");

        let (status, res_body) = post(endpoint, json)?;
        // println!("{}", &res_body);
        let reply = serde_json::from_str(&res_body)?;

        return Ok((status, reply));
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Reply {
    pub version: VersionNumber,
    pub transaction_id: TID,
    pub reply: bool,

    #[serde(flatten)]
    pub payload: ReplyPayload,
}

impl Reply {
    pub fn unwrap(self, message: &Message) -> ReplyPayload {
        if !correct_reply_type(&message.payload, &self.payload) {panic!("Unexpected reply type.")}
        if message.version != self.version {panic!("API version mismatch.")}
        if message.transaction_id != self.transaction_id {panic!("TID mismatch.")}
        if !self.reply {panic!("Not a reply!")}

        return self.payload;
    }
}

pub fn get(endpoint: url::Url) -> super::NetResult<(reqwest::StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let res = client.get(endpoint).send()?;
    return Ok((res.status(), res.text()?));
}

pub fn post(endpoint: url::Url, body: String) -> super::NetResult<(reqwest::StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let res = client.post(endpoint).body(body).send()?;
    return Ok((res.status(), res.text()?));
}

pub fn put(endpoint: url::Url, body: String) -> super::NetResult<(reqwest::StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let res = client.put(endpoint).body(body).send()?;
    return Ok((res.status(), res.text()?));
}
