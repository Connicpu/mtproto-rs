use std::io;

use chrono::{UTC, Timelike};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt, self};

pub mod encryption;
pub mod functions;

pub struct Session {
    session_id: u64,
    server_salt: u64,
    seq_no: u32,
    auth_key: Option<encryption::AuthKey>,
}

#[derive(Debug)]
pub struct DecryptedMessage {
    server_salt: u64,
    session_id: u64,
    message_id: u64,
    seq_no: u32,
    pub payload: Vec<u8>,
}

impl Session {
    pub fn new(session_id: u64) -> Session {
        Session {
            session_id: session_id,
            server_salt: 0,
            seq_no: 0,
            auth_key: None,
        }
    }

    fn next_content_seq_no(&mut self) -> u32 {
        let seq = self.seq_no * 2;
        self.seq_no += 1;
        seq
    }

    pub fn key_exchange_complete(&mut self, server_salt: u64, authorization_key: encryption::AuthKey) {
        self.server_salt = server_salt;
        self.auth_key = Some(authorization_key);
    }

    fn next_message_id(&mut self) -> u64 {
        let time = UTC::now();
        let timestamp = time.timestamp() as u64;
        let nano = time.nanosecond() as u64;
        (timestamp << 32) | (nano & !3)
    }

    pub fn encrypted_payload(&mut self, payload: &[u8]) -> Result<Vec<u8>, ::openssl::error::ErrorStack> {
        let key = self.auth_key.unwrap();
        let mut message = vec![];
        message.write_u64::<LittleEndian>(self.server_salt).unwrap();
        message.write_u64::<LittleEndian>(self.session_id).unwrap();
        message.write_u64::<LittleEndian>(self.next_message_id()).unwrap();
        message.write_u32::<LittleEndian>(self.next_content_seq_no() | 1).unwrap();
        message.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
        message.extend(payload);
        key.encrypt_message(&message)
    }

    pub fn plain_payload(&mut self, payload: &[u8]) -> Vec<u8> {
        let mut message = vec![];
        message.write_u64::<LittleEndian>(0).unwrap();
        message.write_u64::<LittleEndian>(self.next_message_id()).unwrap();
        message.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
        message.extend(payload);
        message
    }

    pub fn decrypt_message(&self, message: &[u8]) -> Result<DecryptedMessage, ::openssl::error::ErrorStack> {
        let decrypted = self.auth_key.unwrap().decrypt_message(message)?;
        let mut cursor = io::Cursor::new(&decrypted[..]);
        let server_salt = cursor.read_u64::<LittleEndian>().unwrap();
        let session_id = cursor.read_u64::<LittleEndian>().unwrap();
        let message_id = cursor.read_u64::<LittleEndian>().unwrap();
        let seq_no = cursor.read_u32::<LittleEndian>().unwrap();
        let len = cursor.read_u32::<LittleEndian>().unwrap() as usize;
        let pos = cursor.position() as usize;
        cursor.into_inner();
        let payload = &decrypted[pos..pos+len];
        //let computed_message_hash = sha1_bytes(&[&ret])?;
        //let computed_message_key = &computed_message_hash[4..20];
        Ok(DecryptedMessage {
            server_salt: server_salt,
            session_id: session_id,
            message_id: message_id,
            seq_no: seq_no,
            payload: payload.into(),
        })
    }
}

pub type RpcRes<T> = Result<T, RpcError>;

pub enum RpcError {
    Io(io::Error),
    WrongAuthKey,
    InvalidLength,
    Unknown,
}

impl From<io::Error> for RpcError {
    fn from(io: io::Error) -> RpcError {
        RpcError::Io(io)
    }
}

impl From<byteorder::Error> for RpcError {
    fn from(bo: byteorder::Error) -> RpcError {
        match bo {
            byteorder::Error::Io(io) => From::from(io),
            _ => RpcError::Unknown,
        }
    }
}
