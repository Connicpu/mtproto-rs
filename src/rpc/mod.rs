use std::io;
use chrono::{UTC, Timelike};
use byteorder;

pub mod encryption;
pub mod functions;

pub struct Session {
    session_id: u64,
    server_salt: u64,
    message_id_seq: u16,
    message_id_last_nano: u16,
    seq_no: u32,
    auth_key: [u8; 256],
}

impl Session {
    pub fn new(authorization_key: &[u8; 256]) -> Session {
        Session {
            session_id: 0,
            server_salt: 0,
            message_id_seq: 0,
            message_id_last_nano: 0,
            seq_no: 0,
            auth_key: *authorization_key,
        }
    }
    
    pub fn get_session_id(&self) -> u64 {
        self.session_id
    }
    
    pub fn get_salt(&self) -> u64 {
        self.server_salt
    }
    
    pub fn next_seq_no(&self) -> u32 {
        self.seq_no * 2
    }
    
    pub fn next_content_seq_no(&mut self) -> u32 {
        let seq = self.seq_no * 2;
        self.seq_no += 1;
        seq
    }
    
    pub fn get_authorization_key(&self) -> &[u8; 256] {
        &self.auth_key
    }
    
    pub fn next_message_id(&mut self) -> u64 {
        let time = UTC::now();
        let timestamp = time.timestamp();
        let nano = time.nanosecond();
        
        let nano_bits = (nano >> 14) as u16 & 0xFFFC;
        // For the highly unlikely case that the nanosecond is the same
        if nano_bits == self.message_id_last_nano {
            self.message_id_seq += 1;
        } else {
            self.message_id_last_nano = nano_bits;
            self.message_id_seq = 0b0101010101010101; // too many zeroes = ignored, so
        }
        let id = self.message_id_seq;
        
        // [ lower 32-bits of unix time | bits 13..30 of the nanosecond (14 total) | id (16 total) | 0b00 ]
        ((timestamp as u64) << 32) |
        ((nano_bits as u64) << 16) |
        ((id as u64) << 2)
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

