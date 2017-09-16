pub mod asymm;


use std::fmt;
use std::io::{Cursor, Write};

use byteorder::{ByteOrder, LittleEndian};
use openssl::{aes, symm};

use error::{self, ErrorKind};
use rpc::{sha1_bytes};


#[derive(Clone, Copy, Debug, Default)]
pub struct AesParams {
    key: [u8; 32],
    iv: [u8; 32],
}

impl AesParams {
    pub fn ige_encrypt(self, decrypted: &[u8], prepend_sha1: bool) -> error::Result<Vec<u8>> {
        let input = sha1_and_or_pad(decrypted, prepend_sha1, Padding::Mod16)?;
        self.run_ige(&input, symm::Mode::Encrypt)
    }

    pub fn ige_decrypt(self, encrypted: &[u8]) -> error::Result<Vec<u8>> {
        self.run_ige(encrypted, symm::Mode::Decrypt)
    }

    fn run_ige(mut self, input: &[u8], mode: symm::Mode) -> error::Result<Vec<u8>> {
        let key = match mode {
            // self.key is 256-bit, so can unwrap here
            symm::Mode::Encrypt => aes::AesKey::new_encrypt(&self.key).unwrap(),
            symm::Mode::Decrypt => aes::AesKey::new_decrypt(&self.key).unwrap(),
        };

        let mut output = vec![0; input.len()];

        // Must not panic because:
        // - input.len() == output.len() by declaration of output
        // - input.len() % 16 == 0
        // - iv.len() == 32 >= 32
        aes::aes_ige(input, &mut output, &key, &mut self.iv, mode);

        Ok(output)
    }

    // TODO: uncomment after getting rpc functions working
    /*pub fn from_pq_inner_data(data: &P_Q_inner_data) -> Result<AesParams> {
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
    }*/
}


const AUTH_KEY_SIZE: usize = 256;

pub struct AuthKey {
    auth_key: [u8; AUTH_KEY_SIZE],
    aux_hash: i64,
    fingerprint: i64,
}

// FIXME: wait until compiler-generated Clone impls for [T; N] where N > 32 is stable
impl Clone for AuthKey {
    fn clone(&self) -> AuthKey {
        let mut auth_key = [0; AUTH_KEY_SIZE];
        auth_key.copy_from_slice(&self.auth_key);

        AuthKey {
            auth_key: auth_key,
            aux_hash: self.aux_hash,
            fingerprint: self.fingerprint,
        }
    }
}

impl fmt::Debug for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AuthKey")
            .field("auth_key", &self.auth_key.as_ref())
            .field("aux_hash", &self.aux_hash)
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

impl PartialEq for AuthKey {
    fn eq(&self, other: &AuthKey) -> bool {
        self.auth_key.as_ref() == other.auth_key.as_ref()
            && self.aux_hash == other.aux_hash
            && self.fingerprint == other.fingerprint
    }
}


// FIXME: is that the right default?
// well, zeroing the buffer for sensitive data seems fine...
impl Default for AuthKey {
    fn default() -> AuthKey {
        AuthKey {
            auth_key: [0; AUTH_KEY_SIZE],
            aux_hash: 0,
            fingerprint: 0,
        }
    }
}

impl AuthKey {
    pub fn new(key_in: &[u8]) -> error::Result<AuthKey> {
        let mut key = [0u8; AUTH_KEY_SIZE];
        // TODO: handle cases when key_in > MAX_ISIZE (low priority)
        let size_diff = (AUTH_KEY_SIZE as isize) - (key_in.len() as isize);

        if size_diff >= 0 {
            // key longer than or same length as key_in
            (&mut key[size_diff as usize..]).copy_from_slice(key_in);
        } else {
            // key shorter than key_in
            bail!(ErrorKind::AuthKeyTooLong(key_in.to_vec()));
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

    pub fn encrypt_message(&self, message: &[u8]) -> error::Result<Vec<u8>> {
        let message_hash = sha1_bytes(&[message])?;
        let message_key = &message_hash[4..20];
        let aes = self.generate_message_aes_params(message_key, symm::Mode::Encrypt)?;

        // TODO: optimize precalculated length for vector allocation
        let mut ret = vec![0u8; 8];

        LittleEndian::write_i64(&mut ret, self.fingerprint);
        ret.extend(message_key);
        // TODO: replace prepend_sha1 bool parameter with an enum
        ret.extend(aes.ige_encrypt(message, false)?);

        Ok(ret)
    }

    pub fn decrypt_message(&self, message: &[u8]) -> error::Result<Vec<u8>> {
        let input_fingerprint = LittleEndian::read_i64(&message[0..8]);

        if input_fingerprint != self.fingerprint {
            bail!(ErrorKind::WrongFingerprint(input_fingerprint));
        }

        let message_key = &message[8..24];
        let aes = self.generate_message_aes_params(message_key, symm::Mode::Decrypt)?;

        aes.ige_decrypt(&message[24..])
    }

    fn generate_message_aes_params(&self, msg_key: &[u8], mode: symm::Mode) -> error::Result<AesParams> {
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
}


// Utils

enum Padding {
    Total255,
    Mod16,
}

fn sha1_and_or_pad(input: &[u8], prepend_sha1: bool, padding: Padding) -> error::Result<Vec<u8>> {
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
        // In case if new variants will be added
        _ => (),
    }

    Ok(ret)
}

fn set_slice_parts(result: &mut [u8], parts: &[&[u8]]) {
    let parts_len = parts.iter().map(|x| x.len()).sum();
    assert_eq!(result.len(), parts_len);

    let mut cursor = Cursor::new(result);
    for part in parts {
        // Can unwrap here safely since we've already checked for length mismatch
        cursor.write(part).unwrap();
    }
}
