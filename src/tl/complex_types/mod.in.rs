pub mod auth;

#[derive(TLType)]
#[tl_id(_c4b9f9bb)]
pub struct Error {
    pub code: i32,
    pub text: String,
}

#[derive(TLType)]
pub enum InputPeer {
    #[tl_id(_7f3b18ea)] Empty,
    #[tl_id(_7da07ec9)] SelfPeer,
    #[tl_id(_1023dbe8)] Contact { user_id: i32 },
    #[tl_id(_9b447325)] Foreign { user_id: i32, access_hash: i64 },
    #[tl_id(_179be863)] Chat { chat_id: i32 },
}

#[derive(TLType)]
pub enum InputUser {
    #[tl_id(_b98886cf)] Empty,
    #[tl_id(_f7c1b13f)] SelfUser,
    #[tl_id(_86e94f65)] Contact { user_id: i32 },
    #[tl_id(_655e74ff)] Foreign { user_id: i32, access_hash: i64 },
}

#[derive(TLType)]
#[tl_id(_f392b7f4)]
pub struct InputContact {
    pub client_id: i64,
    pub phone: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(TLType)]
pub enum InputFile {
    #[tl_id(_f52ff27f)] Normal { id: i64, parts: i32, name: String, md5_checksum: String },
    #[tl_id(_fa4f0bb5)] Big { id: i64, parts: i32, name: String },
}
