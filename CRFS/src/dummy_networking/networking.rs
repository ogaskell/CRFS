use crate::errors;
use crate::types::{Hash, RawData};

use std::net;

use uuid::Uuid;

pub struct SystemStatus {
    server: Option< net::SocketAddr >,
    user: Option< Uuid >, user_dn: Option< String >,
    filesystem: Option< Uuid >, filesystem_dn: Option< String >,
    replica: Option< Uuid >,
    ready: bool, data_ready: bool,  // Is the local replica ready; if false, data_ready holds whether the server has the files ready
    pull_changes: bool,  // does the server have changes we don't?
    push_changes: bool,  // do we have changes the server doesn't?

}

fn setup_user(status: SystemStatus, new: bool) -> Result< SystemStatus, errors::ErrorCode > {
    // Setup a user with the server. If !new, use the UUID in status. Else, generate a new UUID, and register as a new user.
    return Err(errors::CODE_NOT_IMPL);
}

fn setup_filesystem(status: SystemStatus, new: bool) -> Result< SystemStatus, errors::ErrorCode > {
    // Setup a filesystem with the server. If new, generate a UUID and register it with the server. Else, set up assuming the filesystem exists.
    return Err(errors::CODE_NOT_IMPL);
}

fn setup_replica(status: SystemStatus) -> Result< SystemStatus, errors::ErrorCode > {
    // Generate a UUID for this replica, and notify the server.
    // Will also submit a request for all files in the FS.
    return Err(errors::CODE_NOT_IMPL);
}

fn poll(status: SystemStatus) -> Result< SystemStatus, errors::ErrorCode > {
    // Based on current system status, make one or more queries, and return an updated system status.
    return Err(errors::CODE_NOT_IMPL);
}

fn pull_files(status: SystemStatus, uuid: Uuid) -> Result< (SystemStatus, RawData), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}

fn push_operation(status: SystemStatus, op: RawData) -> Result< SystemStatus, errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}

fn pull_operation(status: SystemStatus, hash: Hash) -> Result< (SystemStatus, RawData), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
