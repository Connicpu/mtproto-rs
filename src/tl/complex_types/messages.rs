use super::{contacts, Dialog, Message, Chat, User, EncryptedFile, Document};
use tl::Vector;

#[derive(Debug, TLType)]
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

#[derive(Debug, TLType)]
pub enum Messages {
    #[tl_id(_8c718e87)] Messages {
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
    },
    #[tl_id(_b446ae3)] Slice {
        count: i32,
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
    },
}

#[derive(Debug, TLType)]
pub enum StatedMessages {
    #[tl_id(_969478bb)] Messages {
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
        pts: i32,
        seq: i32,
    },
    #[tl_id(_3e74f5c6)] Links {
        messages: Vector<Message>,
        chats: Vector<Chat>,
        users: Vector<User>,
        links: Vector<contacts::Link>,
        pts: i32,
        seq: i32,
    },
}

#[derive(Debug, TLType)]
pub enum StatedMessage {
    #[tl_id(_d07ae726)] Message {
        message: Message,
        chats: Vector<Chat>,
        users: Vector<User>,
        pts: i32,
        seq: i32,
    },
    #[tl_id(_a9af2881)] Link {
        message: Message,
        chats: Vector<Chat>,
        users: Vector<User>,
        links: Vector<contacts::Link>,
        pts: i32,
        seq: i32,
    },
}

#[derive(Debug, TLType)]
pub enum SentMessage {
    #[tl_id(_d1f4d35c)] Message {
        id: i32,
        date: i32,
        pts: i32,
        seq: i32,
    },
    #[tl_id(_e9db4a3f)] Link {
        id: i32,
        date: i32,
        pts: i32,
        seq: i32,
        links: Vector<contacts::Link>,
    },
}

#[derive(Debug, TLType)]
#[tl_id(_8150cbd8)]
pub struct Chats {
    pub chats: Vector<Chat>,
    pub users: Vector<User>,
}

#[derive(Debug, TLType)]
#[tl_id(_e5d7d19c)]
pub struct ChatFull {
    pub full_chat: super::ChatFull,
    pub chats: Vector<Chat>,
    pub users: Vector<User>,
}

#[derive(Debug, TLType)]
#[tl_id(_b7de36f2)]
pub struct AffectedHistory {
    pub pts: i32,
    pub seq: i32,
    pub offset: i32,
}

#[derive(Debug, TLType)]
pub enum DhConfig {
    #[tl_id(_c0e24635)] NotModified {
        random: Vec<u8>,
    },
    #[tl_id(_2c221edd)] Config {
        g: i32,
        p: Vec<u8>,
        version: i32,
        random: Vec<u8>,
    },
}

#[derive(Debug, TLType)]
pub enum SentEncryptedMessage {
    #[tl_id(_560f8935)] Message {
        date: i32,
    },
    #[tl_id(_9493ff32)] File {
        date: i32,
        file: EncryptedFile,
    },
}

#[derive(Debug, TLType)]
pub enum Stickers {
    #[tl_id(_f1749a22)] NotModified,
    #[tl_id(_8a8ecd32)] Stickers {
        hash: String,
        stickers: Vector<Document>,
    },
}

#[derive(Debug, TLType)]
#[tl_id(_12b299d4)]
pub struct StickerPack {
    pub emoticon: String,
    pub documents: Vector<i64>,
}

#[derive(Debug, TLType)]
pub enum AllStickers {
    #[tl_id(_e86602c3)] NotModified,
    #[tl_id(_dcef3102)] All {
        hash: String,
        packs: Vector<StickerPack>,
        documents: Vector<Document>,
    },
}

// #[tl_id(_)]
