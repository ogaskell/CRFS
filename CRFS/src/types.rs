// Provides useful type aliases.

use generic_array::GenericArray;
use generic_array::typenum::U32;

pub type Sha256Hash = GenericArray<u8, U32>;
pub type Hash = Sha256Hash;
