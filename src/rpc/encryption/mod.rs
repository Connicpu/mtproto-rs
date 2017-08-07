use std::fmt;
use std::io::{Cursor, Write};

use byteorder::{LittleEndian, ByteOrder, WriteBytesExt};
use openssl::{aes, symm};
use rand::Rng;

use error::Result;
use rpc::{sha1_bytes, sha1_nonces};
use schema::manual::BindAuthKeyInner;
use schema::rpc::auth::bindTempAuthKey;
use schema::{Int128, Object, P_Q_inner_data};
use tl::serialize_message;

pub mod asymm;

enum Padding {
    Total255,
    Mod16,
}

fn sha1_and_or_pad(input: &[u8], prepend_sha1: bool, padding: Padding) -> Result<Vec<u8>> {
    let mut ret = if prepend_sha1 {
        sha1_bytes(&[input])?
    } else {
        vec![]
    };
    ret.extend(input);
    match padding {
        Padding::Total255 => {
            while ret.len() < 255 {
                ret.push(0);
            }
        },
        Padding::Mod16 if ret.len() % 16 != 0 => {
            for _ in 0..16 - (ret.len() % 16) {
                ret.push(0);
            }
        },
        _ => (),
    }
    Ok(ret)
}

#[derive(Default, Clone, Copy)]
pub struct AesParams {
    key: [u8; 32],
    iv: [u8; 32],
}

impl fmt::Debug for AesParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AesParams")
    }
}

impl AesParams {
    fn run_ige(mut self, input: &[u8], mode: symm::Mode) -> Result<Vec<u8>> {
        let key = match mode {
            symm::Mode::Encrypt => aes::AesKey::new_encrypt(&self.key).unwrap(),
            symm::Mode::Decrypt => aes::AesKey::new_decrypt(&self.key).unwrap(),
        };
        let mut output = vec![0; input.len()];
        aes::aes_ige(input, &mut output, &key, &mut self.iv, mode);
        Ok(output)
    }

    pub fn ige_encrypt(self, decrypted: &[u8], prepend_sha1: bool) -> Result<Vec<u8>> {
        let input = sha1_and_or_pad(decrypted, prepend_sha1, Padding::Mod16)?;
        self.run_ige(&input, symm::Mode::Encrypt)
    }

    pub fn ige_decrypt(self, encrypted: &[u8]) -> Result<Vec<u8>> {
        self.run_ige(encrypted, symm::Mode::Decrypt)
    }

    pub fn from_pq_inner_data(data: &P_Q_inner_data) -> Result<AesParams> {
        let new_nonce = data.new_nonce();
        let server_nonce = data.server_nonce();
        let sha1_a = sha1_nonces(&[new_nonce.0, new_nonce.1, server_nonce])?;
        let sha1_b = sha1_nonces(&[server_nonce, new_nonce.0, new_nonce.1])?;
        let sha1_c = sha1_nonces(&[new_nonce.0, new_nonce.1, new_nonce.0, new_nonce.1])?;
        let mut tmp = [0u8; 8];
        LittleEndian::write_i64(&mut tmp, (new_nonce.0).0);
        let mut ret: AesParams = Default::default();
        set_slice_parts(&mut ret.key, &[&sha1_a, &sha1_b[..12]]);
        set_slice_parts(&mut ret.iv, &[&sha1_b[12..], &sha1_c, &tmp[..4]]);
        Ok(ret)
    }
}

fn set_slice_parts(result: &mut [u8], parts: &[&[u8]]) {
    let mut cursor = Cursor::new(result);
    for part in parts {
        cursor.write(part).unwrap();
    }
}

const AUTH_KEY_SIZE: usize = 256;

pub struct AuthKey {
    auth_key: [u8; AUTH_KEY_SIZE],
    aux_hash: i64,
    fingerprint: i64,
}

impl Clone for AuthKey {
    fn clone(&self) -> AuthKey {
        AuthKey {
            auth_key: self.auth_key,
            aux_hash: self.aux_hash,
            fingerprint: self.fingerprint,
        }
    }
}

impl fmt::Debug for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AuthKey(#{:08x})", self.fingerprint)
    }
}

impl AuthKey {
    pub fn new(key_in: &[u8]) -> Result<AuthKey> {
        let mut key = [0u8; AUTH_KEY_SIZE];
        let size_diff = (AUTH_KEY_SIZE as isize) - (key_in.len() as isize);
        if size_diff > 0 {
            // key longer than key_in
            (&mut key[size_diff as usize..]).copy_from_slice(key_in);
        } else if size_diff < 0 {
            // key_in longer than key
            unimplemented!()
        } else {
            key.copy_from_slice(key_in);
        }
        let sha1 = sha1_bytes(&[&key])?;
        let aux_hash = LittleEndian::read_i64(&sha1[0..8]);
        let fingerprint = LittleEndian::read_i64(&sha1[12..20]);
        Ok(AuthKey {
            auth_key: key,
            aux_hash: aux_hash,
            fingerprint: fingerprint,
        })
    }

    fn generate_message_aes_params(&self, msg_key: &[u8], mode: symm::Mode) -> Result<AesParams> {
        let mut pos = match mode {
            symm::Mode::Encrypt => 0,
            symm::Mode::Decrypt => 8,
        };
        let mut auth_key_take = |len| {
            let ret = &self.auth_key[pos..pos+len];
            pos += len;
            ret
        };
        let sha1_a = sha1_bytes(&[msg_key, auth_key_take(32)])?;
        let sha1_b = sha1_bytes(&[auth_key_take(16), msg_key, auth_key_take(16)])?;
        let sha1_c = sha1_bytes(&[auth_key_take(32), msg_key])?;
        let sha1_d = sha1_bytes(&[msg_key, auth_key_take(32)])?;

        let mut ret: AesParams = Default::default();
        set_slice_parts(&mut ret.key, &[&sha1_a[0..8], &sha1_b[8..20], &sha1_c[4..16]]);
        set_slice_parts(&mut ret.iv, &[&sha1_a[8..20], &sha1_b[0..8], &sha1_c[16..20], &sha1_d[0..8]]);
        Ok(ret)
    }

    pub fn new_nonce_hash(&self, which: u8, new_nonce: (Int128, Int128)) -> Result<Int128> {
        let mut input = [0u8; 41];
        {
            let mut cursor = Cursor::new(&mut input[..]);
            cursor.write_i64::<LittleEndian>((new_nonce.0).0).unwrap();
            cursor.write_i64::<LittleEndian>((new_nonce.0).1).unwrap();
            cursor.write_i64::<LittleEndian>((new_nonce.1).0).unwrap();
            cursor.write_i64::<LittleEndian>((new_nonce.1).1).unwrap();
            cursor.write_u8(which).unwrap();
            cursor.write_i64::<LittleEndian>(self.aux_hash).unwrap();
        }
        let sha1 = sha1_bytes(&[&input])?;
        Ok((LittleEndian::read_i64(&sha1[4..12]), LittleEndian::read_i64(&sha1[12..20])))
    }

    pub fn encrypt_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        let message_hash = sha1_bytes(&[message])?;
        let message_key = &message_hash[4..20];
        let aes = self.generate_message_aes_params(message_key, symm::Mode::Encrypt)?;
        let mut ret = vec![0u8; 8];
        LittleEndian::write_i64(&mut ret, self.fingerprint);
        ret.extend(message_key);
        ret.extend(aes.ige_encrypt(message, false)?);
        Ok(ret)
    }

    pub fn decrypt_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        assert!(LittleEndian::read_i64(&message[..8]) == self.fingerprint);
        let message_key = &message[8..24];
        let aes = self.generate_message_aes_params(message_key, symm::Mode::Decrypt)?;
        aes.ige_decrypt(&message[24..])
    }

    pub fn into_inner(self) -> [u8; AUTH_KEY_SIZE] {
        self.auth_key
    }

    pub fn bind_temp_auth_key<R: Rng>(self, temp_key: &AuthKey, expires_at: i32, message_id: i64, rng: &mut R)
                                      -> Result<(i64, bindTempAuthKey)> {
        let nonce: i64 = rng.gen();
        let temp_session_id: i64 = rng.gen();
        let inner = ::schema::manual::Encrypted {
            salt: rng.gen(),
            session_id: rng.gen(),
            message_id: message_id,
            seq_no: 0,
            payload: (Box::new(BindAuthKeyInner {
                nonce: nonce,
                temp_auth_key_id: temp_key.fingerprint,
                perm_auth_key_id: self.fingerprint,
                temp_session_id: temp_session_id,
                expires_at: expires_at,
            }) as Object).into(),
        };
        Ok((temp_session_id, bindTempAuthKey {
            perm_auth_key_id: self.fingerprint,
            nonce: nonce,
            expires_at: expires_at,
            encrypted_message: self.encrypt_message(&serialize_message(inner)?)?,
        }))
    }
}
