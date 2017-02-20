use super::*;

#[derive(TLType)]
#[tl_id(_e300cc3b)]
pub struct CheckedPhone {
    pub phone_registered: bool,
    pub phone_invited: bool,
}

#[derive(TLType)]
pub enum SentCode {
    #[tl_id(_efed51d9)] Code {
        phone_registered: bool,
        phone_code_hash: String,
        send_call_timeout: i32,
        is_password: bool,
    },
    #[tl_id(_e325edcf)] AppCode {
        phone_registered: bool,
        phone_code_hash: String,
        send_call_timeout: i32,
        is_password: bool,
    },
}

#[derive(Debug, TLType)]
#[tl_id(_f6b673a4)]
pub struct Authorization {
    pub expires: i32,
    pub user: User,
}

#[derive(TLType)]
#[tl_id(_df969c2d)]
pub struct ExportedAuthorization {
    pub id: i32,
    pub bytes: Vec<u8>,
}

// #[tl_id(_)]
