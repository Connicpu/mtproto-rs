use super::*;

#[derive(TLType)]
pub enum AppUpdate {
    #[tl_id(_c45a6536)] NoUpdate,
    #[tl_id(_8987f311)] Update {
        id: i32,
        critical: bool,
        url: String,
        text: String,
    },
}

#[derive(TLType)]
#[tl_id(_18cb9f78)]
pub struct InviteText {
    pub message: String,
}

#[derive(TLType)]
#[tl_id(_17c6b5f6)]
pub struct Support {
    pub phone_number: String,
    pub user: User,
}

// #[tl_id(_)]
