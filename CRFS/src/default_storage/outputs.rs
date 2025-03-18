use crate::default_networking::networking::{UserInfo, FileSystemInfo, ReplicaInfo};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConcreteUserInfo {
    pub id: Uuid,
    pub disp_name: String,
}

impl ConcreteUserInfo {
    pub fn from_userinfo(info: UserInfo) -> Option<Self> {
        // Return Some if all the necessary fields are Some. Else return None
        match info {
            UserInfo { id: Some(id), disp_name: Some(dn) } => Some(Self {id: id, disp_name: dn}),
            UserInfo { id: Some(id), disp_name: None } => Some(Self {id: id, disp_name: String::from("")}),
            UserInfo { id: None, disp_name: _ } => None,
        }
    }

    pub fn clone(&self) -> Self {
        return Self { id: self.id.clone(), disp_name: self.disp_name.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone,Debug)]
pub struct ConcreteFSInfo {
    pub user: ConcreteUserInfo,
    pub id: Uuid,
    pub disp_name: String,
}

impl ConcreteFSInfo {
    pub fn from_fsinfo(info: FileSystemInfo) -> Option<Self> {
        let user = match ConcreteUserInfo::from_userinfo(info.user.clone()) {
            Some(u) => u,
            None => return None,
        };
        match info {
            FileSystemInfo { user: _, id: Some(id), disp_name: Some(dn) } => Some(Self {user: user, id: id, disp_name: dn}),
            FileSystemInfo { user: _, id: Some(id), disp_name: None } => Some(Self {user: user, id: id, disp_name: String::from("")}),
            FileSystemInfo { user: _, id: None, disp_name: _ } => None,
        }
    }

    pub fn clone(&self) -> Self {
        return Self { user: self.user.clone(), id: self.id.clone(), disp_name: self.disp_name.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConcreteReplicaInfo {
    pub fs: ConcreteFSInfo,
    pub id: Uuid,
    pub disp_name: String
}

impl ConcreteReplicaInfo {
    pub fn from_replicainfo(info: ReplicaInfo) -> Option<Self> {
        let fs = match ConcreteFSInfo::from_fsinfo(info.fs.clone()) {
            Some(fs) => fs,
            None => return None,
        };
        match info {
            ReplicaInfo { fs: _, id: Some(id), disp_name: Some(dn) } => Some(Self {fs: fs, id: id, disp_name: dn}),
            ReplicaInfo { fs: _, id: Some(id), disp_name: None } => Some(Self {fs: fs, id: id, disp_name: String::from("")}),
            ReplicaInfo { fs: _, id: None, disp_name: _ } => None,
        }
    }

    pub fn clone(&self) -> Self {
        return Self { fs: self.fs.clone(), id: self.id.clone(), disp_name: self.disp_name.clone() }
    }
}
