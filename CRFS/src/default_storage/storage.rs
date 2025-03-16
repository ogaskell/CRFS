use crate::errors;
use crate::types::{Hash, RawData};

use std::path::Path;

use uuid::Uuid;

enum Type { Binary, Integer, Float, String }

pub struct SystemStatus {
    storage_dir: &Path,
}

// Look up where data is stored in a global data store, store/fetch under `id`
fn store_obj(replica_id: Uuid, id: Hash, data: RawData) -> Result< (), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
fn fetch_obj(replica_id: Uuid, id: Hash) -> Result< RawData, errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}

// Metadata Store
fn store_meta(replica_id: Uuid, key: String, data: &[u8], t: Type) -> Result< (), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
fn fetch_meta(replica_id: Uuid, key: String) -> Result< (&'static [u8], Type), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
