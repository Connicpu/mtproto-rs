use super::*;
use tl::Vector;

#[derive(TLType)]
pub enum Dialogs {
    #[tl_id(_15ba6c40)] Dialogs {
        dialogs: Vector<Dialog>,
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
    },
    #[tl_id(_71e094f3)] Slice {
        count: i32,
        dialogs: Vector<Dialog>,
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
    },
}

#[derive(TLType)]
pub enum Messages {
    Messages {
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
    }
}

// #[tl_id(_)] 
