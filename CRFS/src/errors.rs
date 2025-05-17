use crate::networking;

pub type ErrorCode = u32;

#[derive(Debug)]
pub struct Error(pub ErrorCode, pub String);
pub type Result<T> = std::result::Result<T, Error>;

impl From<networking::NetError> for Error {
    fn from(e: networking::NetError) -> Self {
        match e {
            networking::NetError::CRFSErr(code, msg) => Self(code, msg),
            networking::NetError::ReqwestErr(e) => Self(CODE_NET_ERR, format!("Reqwest Error: {:#?}", e)),
            networking::NetError::SerdeErr(e) => Self(CODE_JSON_ERR, format!("Serde JSON Decode Error: {:#?}", e)),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(CODE_IO_ERR, e.to_string())
    }
}

impl From<()> for Error {
    fn from(e: ()) -> Self {
        Self(CODE_ERROR, String::from("Unknown error."))
    }
}

pub fn ok() -> ErrorCode { CODE_OK }

// Errors in the range 0x0000---- are networking protocol errors
pub const CODE_OK: ErrorCode = 0;
pub const CODE_ERROR: ErrorCode = 1;
pub const CODE_COLLISION: ErrorCode = 2;
pub const CODE_NO_USER: ErrorCode = 3;
pub const CODE_NO_FS: ErrorCode = 4;
pub const CODE_WAITING: ErrorCode = 5;
pub const CODE_NOT_FOUND: ErrorCode = 6;
pub const CODE_NOT_IMPL: ErrorCode = 7;
pub const CODE_MALFORMED: ErrorCode = 8;
pub const CODE_AUTH_ERR: ErrorCode = 9;

// Errors in the range 0x0001---- are specific to this Client
pub const CODE_JSON_ERR: ErrorCode = 0x00010001; // Includes Serde errors
pub const CODE_NET_ERR: ErrorCode = 0x00010002;
pub const CODE_IO_ERR: ErrorCode = 0x00010003;
pub const CODE_INVALID_DATA: ErrorCode = 0x00010004; // Data doesn't match hash.
