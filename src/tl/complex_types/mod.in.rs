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

