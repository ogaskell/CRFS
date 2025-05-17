use crate::{errors, storage, types::{self, calculate_hash}};

use std::net;
use std::collections::HashSet;

use reqwest;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub mod api;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum NetError {
    CRFSErr(errors::ErrorCode, String),
    ReqwestErr(reqwest::Error),
    SerdeErr(serde_json::Error),
}

type NetResult<T> = std::result::Result<T, NetError>;

impl From<reqwest::Error> for NetError {
    fn from(item: reqwest::Error) -> Self {
        return Self::ReqwestErr(item);
    }
}

impl From<serde_json::Error> for NetError {
    fn from(item: serde_json::Error) -> Self {
        return Self::SerdeErr(item);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {pub id: Option<Uuid>, pub disp_name: Option<String>}

impl UserInfo {
    pub fn empty() -> Self {
        Self { id: None, disp_name: None }
    }

    pub fn get_user_id(&self) -> Option<Uuid> { self.id }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSystemInfo {pub user: UserInfo, pub id: Option<Uuid>, pub disp_name: Option<String>}

impl FileSystemInfo {
    pub fn empty() -> Self {
        Self { id: None, disp_name: None, user: UserInfo::empty() }
    }

    pub fn get_fs_id(&self) -> Option<Uuid> { self.id }
    pub fn get_user_id(&self) -> Option<Uuid> { self.user.get_user_id() }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplicaInfo {pub fs: FileSystemInfo, pub id: Option<Uuid>, pub disp_name: Option<String>}

impl ReplicaInfo {
    pub fn empty() -> Self {
        Self { id: None, disp_name: None, fs: FileSystemInfo::empty() }
    }

    pub fn get_replica_id(&self) -> Option<Uuid> { self.id }
    pub fn get_fs_id(&self) -> Option<Uuid> { self.fs.get_fs_id() }
    pub fn get_user_id(&self) -> Option<Uuid> { self.fs.get_user_id() }

    pub fn gen_blanks(&mut self) {
        if self.id == None {self.id = Some(Uuid::now_v7());}
        if self.fs.id == None {self.fs.id = Some(Uuid::now_v7());}
        if self.fs.user.id == None {self.fs.user.id = Some(Uuid::now_v7());}
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
            info: ReplicaInfo::empty(),
        }
    }

    /// Generate a new UUID for any that are set to None.
    pub fn gen_blanks(&mut self) {
        self.info.gen_blanks();
    }

    /// Returns 2 bools, represting if the server holds info on the user, and fs, respectively.
    pub fn check_info(&self) -> errors::Result<(bool, bool)> {
        let (user_uuid, fs_uuid) = (
            self.info.get_user_id().expect("No user UUID configured!"),
            self.info.get_fs_id().expect("No FS UUID configured!"),
        );

        // Check User
        let user_msg = api::Message::new(api::MessagePayload::CheckUser {
            user_uuid,
        });

        let (_, res) = user_msg.send(&self)?;
        match res.unwrap(&user_msg) {
            api::ReplyPayload::CheckUser {code, ..} => {
                if code != 0 {return Ok((false, false))};
            },
            _ => {panic!();} // Unreachable
        }

        // Check FS
        let fs_msg = api::Message::new(api::MessagePayload::CheckFs {
            user_uuid, fs_uuid,
        });

        let (_, res) = fs_msg.send(&self)?;
        match res.unwrap(&fs_msg) {
            api::ReplyPayload::CheckFs {code, ..} => {
                if code != 0 {return Ok((true, false))};
            },
            _ => {panic!();} // Unreachable
        }

        return Ok((true, true));
    }

    pub fn register_user(&self) -> errors::Result<()> {
        let user_uuid = self.info.fs.user.id.expect("No user UUID configured!");
        let display_name = self.info.fs.user.disp_name.clone().unwrap_or("Unnamed User".to_owned());

        let message = api::Message::new(api::MessagePayload::RegisterUser {
            user_uuid,
            display_name,
        });

        let (_, res) = message.send(&self)?;
        let (code, err_msg) = match res.unwrap(&message) {
            api::ReplyPayload::RegisterUser {code, err_msg} => (code, err_msg),
            _ => panic!(), // Unreachable
        };

        if code == 0 {
            return Ok(())
        } else {
            return Err(errors::Error(code, err_msg));
        }
    }

    pub fn register_fs(&self) -> errors::Result<()> {
        let user_uuid = self.info.fs.user.id.expect("No user UUID configured!");
        let fs_uuid = self.info.fs.id.expect("No FS UUID configured!");
        let display_name = self.info.fs.disp_name.clone().unwrap_or("Unnamed Filesystem".to_owned());

        let message = api::Message::new(api::MessagePayload::RegisterFs {
            user_uuid,
            fs_uuid,
            display_name,
            fs_opts: Vec::new(),
        });

        let (_, res) = message.send(&self)?;
        let (code, err_msg) = match res.unwrap(&message) {
            api::ReplyPayload::RegisterFs {code, err_msg} => (code, err_msg),
            _ => panic!(), // Unreachable
        };

        if code == 0 {
            return Ok(())
        } else {
            return Err(errors::Error(code, err_msg));
        }
    }

    pub fn get_endpoint(&self, endpoint: &str) -> Option<url::Url> {
        let server = self.server?;
        let base_url = format!("http://{}/{}/", server, endpoint);
        return Some(url::Url::parse(&base_url).expect("Malformed URL."));
    }

    pub fn pull(&self, storage: &storage::Config, local_hashes: &HashSet<types::Hash>, remote_hashes: &HashSet<types::Hash>) -> errors::Result<HashSet<types::Hash>> {
        // let remote_hashes = self.fetch_state()?;
        let new_hashes: HashSet<_> = remote_hashes.difference(local_hashes).cloned().collect();

        for op in new_hashes.iter() {
            self.fetch_op(&storage, op)?;
        }

        return Ok(new_hashes);
    }

    pub fn push(&self, storage: &storage::Config, local_hashes: &HashSet<types::Hash>, remote_hashes: &HashSet<types::Hash>) -> errors::Result<()> {
        // let remote_hashes = self.fetch_state()?; // De-duplicate this?
        let new_hashes: HashSet<_> = local_hashes.difference(&remote_hashes).cloned().collect();

        for op in new_hashes.iter() {
            self.push_op(&storage, op)?;
        }

        self.push_state(new_hashes)?;

        return Ok(());
    }

    pub fn fetch_state(&self) -> errors::Result<HashSet<types::Hash>> {
        let message = api::Message::new(api::MessagePayload::FetchState {
            user_uuid: self.info.get_user_id().expect("No User UUID configured"),
            fs_uuid: self.info.get_fs_id().expect("No FS UUID configured"),
        });

        let (_, reply) = message.send(self)?;
        let payload = reply.unwrap(&message);

        match payload {
            api::ReplyPayload::FetchState {code, err_msg, state} => {
                if code == errors::CODE_OK {return Ok(state)}
                else {return Err(errors::Error(code, err_msg))}
            },
            _ => {panic!()} // should never be reached due to reply.unwrap() handling unexpected reply types.
        }
    }

    pub fn push_state(&self, ops: HashSet<types::Hash>) -> errors::Result<()> {
        let message = api::Message::new(api::MessagePayload::PushState {
            user_uuid: self.info.get_user_id().expect("No User UUID configured"),
            fs_uuid: self.info.get_fs_id().expect("No FS UUID configured"),
            ops,
        });

        let (_, reply) = message.send(self)?;
        let payload = reply.unwrap(&message);

        match payload {
            api::ReplyPayload::PushState { code, err_msg } => {
                if code == errors::CODE_OK {return Ok(())}
                else {return Err(errors::Error(code, err_msg))}
            },
            _ => {panic!()} // should never be reached due to reply.unwrap() handling unexpected reply types.
        }
    }

    fn fetch_op(&self, storage: &storage::Config, op: &types::Hash) -> errors::Result<()> {
        let endpoint = self.get_endpoint("operation").unwrap();
        let fs_uuid: String = self.info.get_fs_id().expect("No FS UUID configured").into();

        let full_url = endpoint
            .join(&format!("{}/", &fs_uuid)).unwrap()
            .join(&types::hash_to_str(op)).unwrap();

        let (code, data) = api::get(full_url)?;

        if !code.is_success() {
            return Err(errors::Error(errors::CODE_NET_ERR, format!("Received code {}", code)));
        }

        if calculate_hash(&data) != *op {
            return Err(errors::Error(errors::CODE_INVALID_DATA, "Hash doesn't match downloaded data.".to_owned()));
        }

        let loc = storage::object::Location::Object(op.clone());
        storage::object::write(storage, &loc, data.as_bytes())?;

        return Ok(());
    }

    fn push_op(&self, storage: &storage::Config, op: &types::Hash) -> errors::Result<()> {
        let endpoint = self.get_endpoint("operation").unwrap();
        let fs_uuid: String = self.info.get_fs_id().expect("No FS UUID configured").into();

        let full_url = endpoint
            .join(&format!("{}/", &fs_uuid)).unwrap()
            .join(&types::hash_to_str(op)).unwrap();

        // dbg!(&full_url, &fs_uuid);

        let loc = storage::object::Location::Object(op.clone());
        let mut buf = String::new(); storage::object::read_string(storage, &loc, &mut buf)?;

        let (code, res) = api::put(full_url, buf)?;

        if !code.is_success() {
            return Err(errors::Error(errors::CODE_NET_ERR, format!("Received code {}, message {}", code, res)));
        }

        return Ok(());
    }
}
