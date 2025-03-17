pub type ErrorCode = u32;

pub const CODE_SUCCESS: ErrorCode = 0;
pub const CODE_ERROR: ErrorCode = 1;
pub const CODE_COLLISION: ErrorCode = 2;
pub const CODE_NO_USER: ErrorCode = 3;
pub const CODE_NO_FS: ErrorCode = 4;
pub const CODE_WAITING: ErrorCode = 5;
pub const CODE_NOT_FOUND: ErrorCode = 6;
pub const CODE_NOT_IMPL: ErrorCode = 7;
pub const CODE_MALFORMED: ErrorCode = 8;
pub const CODE_AUTH_ERR: ErrorCode = 9;
