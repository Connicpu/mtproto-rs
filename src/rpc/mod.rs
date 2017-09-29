use std::cmp;

use chrono::{DateTime, Timelike, TimeZone, Utc};
use erased_serde::Serialize as ErasedSerialize;
use openssl::hash;
use serde::de::{Deserialize, DeserializeSeed, DeserializeOwned};
use serde_mtproto::{Boxed, Identifiable, MtProtoSized, WithSize};

use error::{self, ErrorKind};
use schema::FutureSalt;


pub mod encryption;
pub mod message;
pub mod utils;

use rpc::encryption::AuthKey;
use rpc::message::{DecryptedData, Message, MessageSeed, MessageType};
use tl::dynamic::TLObject;


pub trait RpcFunction: ErasedSerialize {
    type Reply: TLObject + 'static;
}

fn sha1_bytes(parts: &[&[u8]]) -> error::Result<Vec<u8>> {
    let mut hasher = hash::Hasher::new(hash::MessageDigest::sha1())?;
    for part in parts {
        hasher.update(part)?;
    }

    let bytes = hasher.finish2().map(|b| b.to_vec())?;

    Ok(bytes)
}


fn next_message_id() -> i64 {
    let time = Utc::now();
    let timestamp = time.timestamp() as i64;
    let nano = time.nanosecond() as i64;

    ((timestamp << 32) | (nano & 0x_7fff_fffc))
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppId {
    api_id: i32,
    // FIXME: use &'static str or Cow<'static, str> here
    api_hash: String,
}

impl AppId {
    pub fn new(api_id: i32, api_hash: String) -> AppId {
        AppId {
            api_id: api_id,
            api_hash: api_hash,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Salt {
    valid_since: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    salt: i64,
}

impl From<FutureSalt> for Salt {
    fn from(fs: FutureSalt) -> Self {
        Salt {
            valid_since: Utc.timestamp(fs.valid_since as i64, 0),
            valid_until: Utc.timestamp(fs.valid_until as i64, 0),
            salt: fs.salt,
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MessagePurpose {
    Content,
    NonContent,
}

#[derive(Debug)]
pub struct Session {
    session_id: i64,
    //temp_session_id: Option<i64>    // Not used
    server_salts: Vec<Salt>,
    seq_no: i32,
    auth_key: Option<AuthKey>,
    to_ack: Vec<i64>,
    app_id: AppId,
}

impl Session {
    pub fn new(session_id: i64, app_id: AppId) -> Session {
        Session {
            session_id: session_id,
            server_salts: Vec::new(),
            seq_no: 0,
            auth_key: None,
            to_ack: Vec::new(),
            app_id: app_id,
        }
    }

    fn next_seq_no(&mut self, purpose: MessagePurpose) -> i32 {
        match purpose {
            MessagePurpose::Content => {
                let result = self.seq_no | 1;
                // FIXME: Resolve (im?)possible overlow panics here
                self.seq_no += 2;
                result
            },
            MessagePurpose::NonContent => {
                self.seq_no
            },
        }
    }

    fn latest_server_salt(&mut self) -> error::Result<i64> {
        let time = {
            let last_salt: &Salt = self.server_salts.last().ok_or(error::Error::from(ErrorKind::NoServerSalts))?;

            // Make sure at least one salt is retained.
            cmp::min(Utc::now(), last_salt.valid_until.clone())
        };

        self.server_salts.retain(|s| &s.valid_until >= &time);
        assert!(self.server_salts.len() >= 1);
        let salt = self.server_salts[0].salt;

        Ok(salt)
    }

    pub fn add_server_salts<S, I>(&mut self, salts: I)
        where S: Into<Salt>,
              I: IntoIterator<Item = S>
    {
        self.server_salts.extend(salts.into_iter().map(Into::into));
        self.server_salts.sort_by(|a, b| a.valid_since.cmp(&b.valid_since));
    }

    pub fn adopt_key(&mut self, auth_key: AuthKey) {
        self.auth_key = Some(auth_key);
    }

    pub fn ack_id(&mut self, id: i64) {
        self.to_ack.push(id);
    }

    fn fresh_auth_key(&self) -> error::Result<AuthKey> {
        match self.auth_key {
            Some(ref key) => Ok(key.clone()),
            None => bail!(ErrorKind::NoAuthKey),
        }
    }

    pub fn create_message<T>(&mut self, body: T, msg_type: MessageType) -> error::Result<Message<T>>
        where T: Identifiable + MtProtoSized
    {
        let message = match msg_type {
            MessageType::PlainText => {
                Message::PlainText {
                    message_id: next_message_id(),
                    body: WithSize::new(Boxed::new(body))?,
                }
            },
            MessageType::Encrypted => {
                if self.to_ack.is_empty() {
                    self.impl_create_decrypted_message(body, MessagePurpose::Content)?
                } else {
                    /*let acks = ::schema::MsgsAck {
                        msg_ids: mem::replace(&mut self.to_ack, vec![]),
                    };*/

                    unimplemented!()
                }
            },
        };

        Ok(message)
    }

    fn impl_create_decrypted_message<T>(&mut self, body: T, purpose: MessagePurpose) -> error::Result<Message<T>>
        where T: Identifiable + MtProtoSized
    {
        let decrypted_data = DecryptedData {
            salt: self.latest_server_salt()?,
            session_id: self.session_id,
            message_id: next_message_id(),
            seq_no: self.next_seq_no(purpose),
            body: WithSize::new(Boxed::new(body))?,

            key: self.fresh_auth_key()?,
        };

        // FIXME: implement
        let message = Message::Decrypted {
            decrypted_data: decrypted_data,
        };

        Ok(message)
    }

    pub fn process_message<T>(&self, message_bytes: &[u8]) -> error::Result<Message<T>>
        where T: DeserializeOwned
    {
        use serde_mtproto::Deserializer;

        let mut deserializer = Deserializer::new(message_bytes, None);

        if message_bytes.len() == 4 {
            // Error codes seem to be represented as negative i32
            let code = i32::deserialize(&mut deserializer)?;
            bail!(ErrorKind::ErrorCode(-code));
        } else if message_bytes.len() < 24 {
            bail!(ErrorKind::BadMessage(message_bytes.len()));
        }

        // FIXME: use safe casts here
        let seed = MessageSeed::new(self.auth_key.clone(), (message_bytes.len() - 24) as u32);
        seed.deserialize(&mut deserializer).map_err(Into::into)
    }
}
