use crate::errors;
use crate::types::{ReplicaID, Hash, RawData};

enum Type { Binary, Integer, Float, String }

// Look up where data is stored in a global data store, store/fetch under `id`
fn store_obj(replica_id: ReplicaID, id: Hash, data: RawData) -> Result< (), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
fn fetch_obj(replica_id: ReplicaID, id: Hash) -> Result< RawData, errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}

// Metadata Store
fn store_meta(replica_id: ReplicaID, key: String, data: &[u8], t: Type) -> Result< (), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
fn fetch_meta(replica_id: ReplicaID, key: String) -> Result< (&'static [u8], Type), errors::ErrorCode > {
    return Err(errors::CODE_NOT_IMPL);
}
