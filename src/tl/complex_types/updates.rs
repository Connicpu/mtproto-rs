use super::*;
use tl::Vector;

#[derive(TLType)]
#[tl_id(_a56c2a3e)]
pub struct State {
    pub pts: i32,
    pub qts: i32,
    pub date: i32,
    pub seq: i32,
    pub unread_count: i32,
}

#[derive(TLType)]
pub enum Difference {
    #[tl_id(_5d75a138)] Empty {
        date: i32,
        seq: i32,
    },
    #[tl_id(_f49ca0)] Difference {
        new_messages: Vector<Message>,
        new_encrypted_messages: Vector<messages::Messages>,
        other_updates: Vector<Update>,
        chats: Vector<Chat>,
        users: Vector<User>,
        state: State,
    },
    #[tl_id(_a8fb1981)] Slice {
        new_messages: Vector<Message>,
        new_encrypted_messages: Vector<messages::Messages>,
        other_updates: Vector<Update>,
        chats: Vector<Chat>,
        users: Vector<User>,
        intermediate_state: State,
    },
}

// #[tl_id(_)]
