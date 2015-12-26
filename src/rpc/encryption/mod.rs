use super::Session;
use std::mem;
use std::io::{Write, Cursor};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use byteorder::{ByteOrder, LittleEndian};
use crypto::{symmetriccipher};

pub mod ige;

pub type CipherError = symmetriccipher::SymmetricCipherError;

struct AuthKey {
    pub key_id: [u8; 8],
    pub aes_key: [u8; 32],
    pub aes_iv: [u8; 32],
}

struct Unencrypted<'a> {
    salt: u64,
    session_id: u64,
    message_id: u64,
    seq_no: u32,
    payload: &'a [u8],
}

trait MessageState {
    fn push(&mut self, bytes: &[u8]);
    
    fn push_u32(&mut self, val: u32) {
        let mut temp_buf = [0; 4];
        LittleEndian::write_u32(&mut temp_buf, val);
        self.push(&temp_buf);
    }
    
    fn push_u64(&mut self, val: u64) {
        let mut temp_buf = [0; 8];
        LittleEndian::write_u64(&mut temp_buf, val);
        self.push(&temp_buf);
    }
}

impl MessageState for Sha1 {
    fn push(&mut self, bytes: &[u8]) {
        <Sha1 as Digest>::input(self, bytes)
    }
}

fn push_message<MS: MessageState>(hash: &mut MS, payload: &Unencrypted) {
    hash.push_u64(payload.salt);
    hash.push_u64(payload.session_id);
    hash.push_u64(payload.message_id);
    hash.push_u32(payload.seq_no);
    hash.push_u32(payload.payload.len() as u32);
    hash.push(payload.payload);
    
    let inv_pad = payload.payload.len() % 16;
    if inv_pad != 0 {
        let padding = [0; 15];
        hash.push(&padding[0..16-inv_pad]);
    }
}

fn make_message_key(payload: &Unencrypted) -> [u8; 16] {
    let mut sha1 = Sha1::new();
    push_message(&mut sha1, payload);
    
    let mut temp_buf = [0; 20];
    let mut message_key = [0; 16];
    sha1.result(&mut temp_buf);
    message_key.clone_from_slice(&temp_buf[0..16]);
    
    message_key
}

fn sha1(parts: &[&[u8]]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    
    for part in parts {
        hasher.input(*part);
    }
    
    let mut result = [0; 20];
    hasher.result(&mut result);
    result
}

fn set_slice_parts(result: &mut [u8], parts: &[&[u8]]) {
    let mut cursor = Cursor::new(result);
    for part in parts {
        cursor.write(part).unwrap();
    }
}

fn make_client_auth_key(session: &Session, msg_key: &[u8; 16]) -> AuthKey {
    let auth_key = session.get_authorization_key();
    
    let sha1_a = sha1(&[ msg_key, &auth_key[0..32] ]);
    let sha1_b = sha1(&[ &auth_key[32..48], msg_key, &auth_key[48..64] ]);
    let sha1_c = sha1(&[ &auth_key[64..96], msg_key ]);
    let sha1_d = sha1(&[ msg_key, &auth_key[96..128] ]);
    
    let key_id_raw = sha1(&[ &auth_key[..] ]);
    let aes_key_raw = [ &sha1_a[0..8], &sha1_b[8..20], &sha1_c[4..16] ];
    let aes_iv_raw = [ &sha1_a[8..20], &sha1_b[0..8], &sha1_c[16..20], &sha1_d[0..8] ];
    
    let mut result = AuthKey { ..unsafe{ mem::uninitialized() } };
    set_slice_parts(&mut result.key_id, &[ &key_id_raw[0..8] ]);
    set_slice_parts(&mut result.aes_key, &aes_key_raw);
    set_slice_parts(&mut result.aes_iv, &aes_iv_raw);
    result
}

fn do_encrypt_message(session: &Session, unencrypted: &Unencrypted) -> Result<Vec<u8>, CipherError> {
    let msg_key = make_message_key(unencrypted);
    let AuthKey { key_id, aes_key, aes_iv } = make_client_auth_key(session, &msg_key);
    
    Ok(vec![])
}

pub fn encrypt_message(session: &mut Session, payload: &[u8]) -> Result<Vec<u8>, CipherError> {
    let unencrypted = Unencrypted {
        salt: session.get_salt(),
        session_id: session.get_session_id(),
        message_id: session.next_message_id(),
        seq_no: session.next_seq_no(),
        payload: payload,
    };
    
    do_encrypt_message(session, &unencrypted)
}
