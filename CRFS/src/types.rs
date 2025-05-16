// Provides useful type aliases.

use generic_array::GenericArray;
use generic_array::typenum::U32;
use sha2::{Sha256, Digest};

pub type Sha256Hash = GenericArray<u8, U32>;
pub type Hash = Sha256Hash;

pub fn hash_to_str(h: &Hash) -> String {
    return format!("{:x}", h);
}

pub fn calculate_hash(str: &String) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(str.as_bytes());
    return hasher.finalize();
}
