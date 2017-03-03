use std::io;

use chrono::{Duration, UTC, Timelike};
use byteorder::{LittleEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use openssl::hash;

pub mod encryption;
pub mod functions;

use error::Result;
use self::functions::authz::Nonce;
use self::functions::FutureSalt;

#[derive(Debug, Clone)]
pub struct AppId {
    pub api_id: i32,
    pub api_hash: String,
}

#[derive(Debug, Clone)]
pub struct Session {
    session_id: u64,
    server_salts: Vec<FutureSalt>,
    seq_no: u32,
    auth_key: Option<encryption::AuthKey>,
    pub app_id: AppId,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub message_id: u64,
    pub message: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct InboundPayload {
    pub message_id: u64,
    pub payload: Vec<u8>,
    pub was_encrypted: bool,
}

pub type InboundMessage = ::std::result::Result<InboundPayload, i32>;

impl Session {
    pub fn new(session_id: u64, app_id: AppId) -> Session {
        Session {
            session_id: session_id,
            server_salts: vec![],
            seq_no: 0,
            auth_key: None,
            app_id: app_id,
        }
    }

    fn next_content_seq_no(&mut self) -> u32 {
        let seq = self.seq_no * 2;
        self.seq_no += 1;
        seq
    }

    fn latest_server_salt(&mut self) -> u64 {
        let time = UTC::now();
        self.server_salts.retain(|s| &s.valid_until > &time);
        self.server_salts.first().unwrap().salt
    }

    pub fn add_server_salts<I>(&mut self, salts: I)
        where I: IntoIterator<Item = FutureSalt>,
    {
        self.server_salts.extend(salts);
        self.server_salts.sort_by(|a, b| a.valid_since.cmp(&b.valid_since));
    }

    pub fn adopt_negotiated_salt(&mut self, server_salt: u64) {
        let time = UTC::now();
        self.server_salts.push(FutureSalt {
            valid_until: time.clone() + Duration::minutes(10),
            valid_since: time,
            salt: server_salt,
        });
    }

    pub fn adopt_key(&mut self, authorization_key: encryption::AuthKey) {
        self.auth_key = Some(authorization_key);
    }

    fn next_message_id(&mut self) -> u64 {
        let time = UTC::now();
        let timestamp = time.timestamp() as u64;
        let nano = time.nanosecond() as u64;
        (timestamp << 32) | (nano & !3)
    }

    pub fn encrypted_payload(&mut self, payload: &[u8]) -> Result<OutboundMessage> {
        let key = self.auth_key.clone().unwrap();
        let mut ret = OutboundMessage {
            message_id: self.next_message_id(),
            message: vec![],
        };
        {
            let message = &mut ret.message;
            let salt = self.latest_server_salt();
            message.write_u64::<LittleEndian>(salt).unwrap();
            message.write_u64::<LittleEndian>(self.session_id).unwrap();
            message.write_u64::<LittleEndian>(ret.message_id).unwrap();
            message.write_u32::<LittleEndian>(self.next_content_seq_no() | 1).unwrap();
            message.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
            message.extend(payload);
        }
        ret.message = key.encrypt_message(&ret.message)?;
        Ok(ret)
    }

    pub fn plain_payload(&mut self, payload: &[u8]) -> OutboundMessage {
        let mut ret = OutboundMessage {
            message_id: self.next_message_id(),
            message: vec![],
        };
        {
            let message = &mut ret.message;
            message.write_u64::<LittleEndian>(0).unwrap();
            message.write_u64::<LittleEndian>(ret.message_id).unwrap();
            message.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
            message.extend(payload);
        }
        ret
    }

    pub fn assemble_payload(&mut self, payload: &[u8], encrypt: bool) -> Result<OutboundMessage> {
        if encrypt {
            self.encrypted_payload(payload)
        } else {
            Ok(self.plain_payload(payload))
        }
    }

    pub fn process_message(&self, message: &[u8]) -> Result<InboundMessage> {
        if message.len() == 4 {
            return Ok(Err(LittleEndian::read_i32(&message)));
        } else if message.len() < 8 {
            panic!("bad message");
        }

        let mut cursor = io::Cursor::new(message);
        let auth_key_id = cursor.read_u64::<LittleEndian>().unwrap();
        if auth_key_id != 0 {
            cursor.into_inner();
            return self.decrypt_message(message);
        }

        let message_id = cursor.read_u64::<LittleEndian>().unwrap();
        let len = cursor.read_u32::<LittleEndian>().unwrap() as usize;
        let pos = cursor.position() as usize;
        cursor.into_inner();
        let payload = &message[pos..pos+len];
        Ok(Ok(InboundPayload {
            message_id: message_id,
            payload: payload.into(),
            was_encrypted: false,
        }))
    }

    fn decrypt_message(&self, message: &[u8]) -> Result<InboundMessage> {
        let decrypted = self.auth_key.clone().unwrap().decrypt_message(message)?;
        let mut cursor = io::Cursor::new(&decrypted[..]);
        let server_salt = cursor.read_u64::<LittleEndian>().unwrap();
        let session_id = cursor.read_u64::<LittleEndian>().unwrap();
        let message_id = cursor.read_u64::<LittleEndian>().unwrap();
        let seq_no = cursor.read_u32::<LittleEndian>().unwrap();
        let len = cursor.read_u32::<LittleEndian>().unwrap() as usize;
        let pos = cursor.position() as usize;
        cursor.into_inner();
        let payload = &decrypted[pos..pos+len];
        let computed_message_hash = sha1_bytes(&[&decrypted[..pos+len]])?;
        let computed_message_key = &computed_message_hash[4..20];
        assert!(&message[8..24] == computed_message_key);
        assert!(self.server_salts.iter().any(|s| s.salt == server_salt));
        assert!(session_id == self.session_id);
        Ok(Ok(InboundPayload {
            message_id: message_id,
            payload: payload.into(),
            was_encrypted: true,
        }))
    }
}

fn sha1_bytes(parts: &[&[u8]]) -> Result<Vec<u8>> {
    let mut hasher = hash::Hasher::new(hash::MessageDigest::sha1())?;
    for part in parts {
        hasher.update(part)?;
    }
    Ok(hasher.finish()?)
}

fn sha1_nonces(nonces: &[Nonce]) -> Result<Vec<u8>> {
    let mut hasher = hash::Hasher::new(hash::MessageDigest::sha1())?;
    for nonce in nonces {
        let mut tmp = [0u8; 16];
        LittleEndian::write_u64(&mut tmp[..8], nonce.0);
        LittleEndian::write_u64(&mut tmp[8..], nonce.1);
        hasher.update(&tmp)?;
    }
    Ok(hasher.finish()?)
}

pub trait RpcFunction {
    type Reply;
}
