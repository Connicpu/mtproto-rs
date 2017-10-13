use std::fmt;

use byteorder::{ByteOrder, LittleEndian};
use extprim::i128::i128;
use openssl::{aes, symm};

use error::{self, ErrorKind};
use rpc::utils::sha1_bytes;

use super::AUTH_KEY_SIZE;
use super::utils::{Padding, sha1_and_or_pad, set_slice_parts};


#[derive(Clone, Copy, Debug, Default)]
pub struct AesParams {
    pub(super) key: [u8; 32],
    pub(super) iv: [u8; 32],
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


pub struct AuthKey {
    auth_key: [u8; AUTH_KEY_SIZE],
    aux_hash: i64,
    fingerprint: i64,
}

// FIXME: wait until compiler-generated trait impls for [T; N] where N > 32 is stable
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
// zeroing the buffer for sensitive data seems fine...
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

        if key_in.len() <= AUTH_KEY_SIZE {
            // key longer than or same length as key_in
            let len_diff = AUTH_KEY_SIZE - key_in.len();
            (&mut key[len_diff..]).copy_from_slice(key_in);
        } else {
            // key shorter than key_in
            bail!(ErrorKind::AuthKeyTooLong(AUTH_KEY_SIZE, key_in.to_vec()));
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

    pub fn encrypt_message_bytes(&self, message_bytes: &[u8]) -> error::Result<(i64, i128, Vec<u8>)> {
        let auth_key_id = self.fingerprint;

        let message_hash = sha1_bytes(&[message_bytes])?;
        let mut message_key_bytes = [0; 16];
        message_key_bytes.copy_from_slice(&message_hash[4..20]);

        let message_key_lo = LittleEndian::read_u64(&message_key_bytes[0..8]);
        let message_key_hi = LittleEndian::read_i64(&message_key_bytes[8..16]);
        let message_key = i128::from_parts(message_key_hi, message_key_lo);

        let aes = self.generate_message_aes_params(message_key, symm::Mode::Encrypt)?;
        let encrypted_data = aes.ige_encrypt(message_bytes, false)?;

        Ok((auth_key_id, message_key, encrypted_data))
    }

    pub fn decrypt_message_bytes(&self,
                                 auth_key_id: i64,
                                 message_key: i128,
                                 message_bytes: &[u8])
                                -> error::Result<Vec<u8>> {
        if auth_key_id != self.fingerprint {
            bail!(ErrorKind::WrongFingerprint(self.fingerprint, auth_key_id));
        }

        let aes = self.generate_message_aes_params(message_key, symm::Mode::Decrypt)?;
        aes.ige_decrypt(message_bytes)
    }

    fn generate_message_aes_params(&self, msg_key: i128, mode: symm::Mode) -> error::Result<AesParams> {
        let mut msg_key_bytes = [0; 16];
        LittleEndian::write_u64(&mut msg_key_bytes[0..8], msg_key.low64());
        LittleEndian::write_i64(&mut msg_key_bytes[8..16], msg_key.high64());

        let mut pos = match mode {
            symm::Mode::Encrypt => 0,
            symm::Mode::Decrypt => 8,
        };

        let mut auth_key_take = |len| {
            let ret = &self.auth_key[pos..pos+len];
            pos += len;
            ret
        };

        let sha1_a = sha1_bytes(&[&msg_key_bytes, auth_key_take(32)])?;
        let sha1_b = sha1_bytes(&[auth_key_take(16), &msg_key_bytes, auth_key_take(16)])?;
        let sha1_c = sha1_bytes(&[auth_key_take(32), &msg_key_bytes])?;
        let sha1_d = sha1_bytes(&[&msg_key_bytes, auth_key_take(32)])?;

        let mut ret: AesParams = Default::default();
        set_slice_parts(&mut ret.key, &[&sha1_a[0..8], &sha1_b[8..20], &sha1_c[4..16]]);
        set_slice_parts(&mut ret.iv, &[&sha1_a[8..20], &sha1_b[0..8], &sha1_c[16..20], &sha1_d[0..8]]);

        Ok(ret)
    }
}
