use super::{Session, RpcRes, RpcError};
use std::io::{Read, Write, Cursor};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use crypto::aessafe::{AesSafe256Encryptor, AesSafe256Decryptor};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt, ReadBytesExt};
use num::BigUint;

pub mod ige;

type MsgKey = [u8; 16];
const PRELUDE_LEN: usize = 32;

pub fn encrypt_message<W: Write>(session: &mut Session, payload: &[u8], stream: &mut W) -> RpcRes<()> {
    let unencrypted = Unencrypted {
        salt: session.get_salt(),
        session_id: session.get_session_id(),
        message_id: session.next_message_id(),
        seq_no: session.next_seq_no(),
        payload: payload,
    };

    do_encrypt_message(session, &unencrypted, stream)
}

pub fn encrypt_content_message<W: Write>(session: &mut Session, payload: &[u8], stream: &mut W) -> RpcRes<()> {
    let unencrypted = Unencrypted {
        salt: session.get_salt(),
        session_id: session.get_session_id(),
        message_id: session.next_message_id(),
        seq_no: session.next_content_seq_no(),
        payload: payload,
    };

    do_encrypt_message(session, &unencrypted, stream)
}

fn do_encrypt_message<W: Write>(session: &Session, unencrypted: &Unencrypted, stream: &mut W) -> RpcRes<()> {
    let (msg_key, msg_len) = try!(make_message_key(unencrypted));
    let AuthKey { key_id, aes_key, aes_iv } = make_client_auth_key(session, &msg_key);

    try!(stream.write_all(&key_id));
    try!(stream.write_all(&msg_key));
    try!(stream.write_u32::<LittleEndian>(msg_len as u32));

    let aes = AesSafe256Encryptor::new(&aes_key);
    let mut encryptor = ige::IgeStream::new(stream, ige::IgeEncryptor::new(aes, &aes_iv));
    try!(push_message(&mut encryptor, unencrypted));

    Ok(())
}

pub fn decrypt_message<R: Read>(session: &Session, stream: &mut R) -> RpcRes<Vec<u8>> {
    let mut key_id: [u8; 8] = Default::default();
    let mut msg_key: MsgKey = Default::default();

    try!(stream.read_exact(&mut key_id));
    try!(stream.read_exact(&mut msg_key));

    let AuthKey { key_id: my_key_id, aes_key, aes_iv } = make_client_auth_key(session, &msg_key);
    if key_id != my_key_id {
        return Err(RpcError::WrongAuthKey);
    }

    let payload_length = try!(stream.read_u32::<LittleEndian>()) as usize;
    if payload_length % ige::BLOCK_SIZE != 0 {
        return Err(RpcError::InvalidLength);
    }

    let mut enc_buf = [0; ige::BLOCK_SIZE];
    let mut prelude_buffer = [0; PRELUDE_LEN];
    let mut decrypted_buffer = vec![0; payload_length - PRELUDE_LEN];
    let mut decryptor = ige::IgeDecryptor::new(
        AesSafe256Decryptor::new(&aes_key),
        &aes_iv,
    );

    /* decrypt the data */ {
        let pre_chunks = prelude_buffer.chunks_mut(ige::BLOCK_SIZE);
        let dec_chunks = decrypted_buffer.chunks_mut(ige::BLOCK_SIZE);
        for decrypted in pre_chunks.chain(dec_chunks) {
            try!(stream.read_exact(&mut enc_buf));
            decryptor.decrypt_block(&enc_buf, decrypted);
        }
    }

    // TODO: Read the prelude and validate the message (plus remove padding)

    Ok(decrypted_buffer)
}

#[derive(Default)]
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

trait MessagePush {
    fn push(&mut self, bytes: &[u8]) -> RpcRes<()>;

    fn push_padding(&mut self, bytes: &[u8]) -> RpcRes<usize> {
        try!(self.push(bytes));
        Ok(bytes.len())
    }

    fn push_u32(&mut self, val: u32) -> RpcRes<()> {
        let mut temp_buf = [0; 4];
        LittleEndian::write_u32(&mut temp_buf, val);
        self.push(&temp_buf)
    }

    fn push_u64(&mut self, val: u64) -> RpcRes<()> {
        let mut temp_buf = [0; 8];
        LittleEndian::write_u64(&mut temp_buf, val);
        self.push(&temp_buf)
    }
}

impl MessagePush for Sha1 {
    fn push(&mut self, bytes: &[u8]) -> RpcRes<()> {
        self.input(bytes);
        Ok(())
    }

    fn push_padding(&mut self, bytes: &[u8]) -> RpcRes<usize> {
        Ok(bytes.len())
    }
}

impl<W: Write> MessagePush for ige::IgeStream<W, ige::IgeEncryptor<AesSafe256Encryptor>> {
    fn push(&mut self, bytes: &[u8]) -> RpcRes<()> {
        try!(self.write(bytes));
        Ok(())
    }
}

fn push_message<MS: MessagePush>(stream: &mut MS, payload: &Unencrypted) -> RpcRes<usize> {
    try!(stream.push_u64(payload.salt));
    try!(stream.push_u64(payload.session_id));
    try!(stream.push_u64(payload.message_id));
    try!(stream.push_u32(payload.seq_no));
    try!(stream.push_u32(payload.payload.len() as u32));
    try!(stream.push(payload.payload));

    let inv_pad = payload.payload.len() % 16;
    let mut extra_len = 0;
    if inv_pad != 0 {
        let padding = [0; 15];
        extra_len = try!(stream.push_padding(&padding[0..extra_len]));
    }

    Ok(32 + payload.payload.len() + extra_len)
}

fn make_message_key(payload: &Unencrypted) -> RpcRes<(MsgKey, usize)> {
    let mut sha1 = Sha1::new();
    let msg_len = try!(push_message(&mut sha1, payload));

    let mut temp_buf = [0; 20];
    let mut message_key = [0; 16];
    sha1.result(&mut temp_buf);
    message_key.clone_from_slice(&temp_buf[0..16]);

    Ok((message_key, msg_len))
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

    let mut result: AuthKey = Default::default();
    set_slice_parts(&mut result.key_id, &[ &key_id_raw[0..8] ]);
    set_slice_parts(&mut result.aes_key, &aes_key_raw);
    set_slice_parts(&mut result.aes_iv, &aes_iv_raw);
    result
}

#[derive(Debug)]
pub struct FactorizationFailure;

pub fn decompose_pq(pq: &BigUint) -> Result<(BigUint, BigUint), FactorizationFailure> {
    use num::{BigInt, Integer, One, Signed};
    use num::bigint::Sign;
    let one: BigUint = One::one();
    let two = &one + &one;
    let g = |x: &BigUint| -> BigUint {
        (x * x + &one) % pq
    };
    let mut x = two.clone();
    let mut y = two.clone();
    let mut d = one.clone();
    while d == one {
        x = g(&x);
        y = g(&g(&y));
        let delta = BigInt::from_biguint(Sign::Plus, x.clone()) - BigInt::from_biguint(Sign::Plus, y.clone());
        d = delta.abs().to_biguint().unwrap().gcd(&pq);
    }
    if &d == pq {
        Err(FactorizationFailure)
    } else {
        Ok((pq / &d, d))
    }
}
