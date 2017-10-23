//! MTProto session.

use std::cmp;
use std::mem;

use chrono::{Timelike, Utc};
use serde::de::{DeserializeSeed, DeserializeOwned};
use serde_mtproto::{Boxed, Identifiable, MtProtoSized, WithSize};

use error::{self, ErrorKind};
use manual_types::Object;
use tl::TLObject;

use super::{AppInfo, Salt};
use super::encryption::AuthKey;
use super::message::{DecryptedData, Message, MessageSeed};


fn next_message_id() -> i64 {
    let time = Utc::now();
    let timestamp = time.timestamp();
    let nano = time.nanosecond() as i64; // from u32

    ((timestamp << 32) | (nano & 0x_ffff_fffc))
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MessagePurpose {
    Content,
    NonContent,
}

// We use signed integers here because that's the default integer representation in MTProto;
// by trying to match representations we can synchronize the range of allowed values
/// Represents a session attached to the client device and user key ID.
///
/// More information about sessions:
/// https://core.telegram.org/mtproto#high-level-component-rpc-query-language-api.
#[derive(Debug)]
pub struct Session {
    session_id: i64,
    //temp_session_id: Option<i64>    // Not used (yet)
    server_salts: Vec<Salt>,
    seq_no: i32,
    auth_key: Option<AuthKey>,
    to_ack: Vec<i64>,
    app_info: AppInfo,
}

impl Session {
    /// Construct a new `Session` from a unique session ID and app info.
    pub fn new(session_id: i64, app_info: AppInfo) -> Session {
        Session {
            session_id: session_id,
            server_salts: Vec::new(),
            seq_no: 0,
            auth_key: None,
            to_ack: Vec::new(),
            app_info: app_info,
        }
    }

    fn next_seq_no(&mut self, purpose: MessagePurpose) -> i32 {
        match purpose {
            MessagePurpose::Content => {
                let result = self.seq_no | 1;

                let (new_seq_no, overflowed) = self.seq_no.overflowing_add(2);
                self.seq_no = new_seq_no;

                if overflowed {
                    // TODO: log overflow
                }

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

    /// Adopt an `AuthKey` after successful authorization.
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

    /// Create a plain-text message tied to this session.
    pub fn create_plain_text_message<T>(&self, body: T) -> error::Result<Message<T>>
        where T: TLObject
    {
        Ok(Message::PlainText {
            message_id: next_message_id(),
            body: WithSize::new(Boxed::new(body))?,
        })
    }

    /// Create an encrypted message without acks.
    ///
    /// On success returns `Ok(message)` if there are no acks in this
    /// session and `Ok(None)` otherwise.
    pub fn create_encrypted_message_no_acks<T>(&mut self, body: T) -> error::Result<Option<Message<T>>>
        where T: TLObject
    {
        if !self.to_ack.is_empty() {
            return Ok(None);
        }

        let message = self.impl_create_decrypted_message(body, MessagePurpose::Content)?;

        Ok(Some(message))
    }

    /// Create an encrypted message with acks.
    ///
    /// On success returns `Ok(message)` if there are acks in this
    /// session and `Ok(None)` otherwise.
    pub fn create_encrypted_message_with_acks<T>(&mut self, body: T)
        -> error::Result<Option<Message<::schema::manual::MessageContainer>>>
        where T: TLObject
    {
        if self.to_ack.is_empty() {
            return Ok(None);
        }

        let acks = ::schema::MsgsAck {
            msg_ids: Boxed::new(mem::replace(&mut self.to_ack, vec![])),
        };

        let msg_container = ::schema::manual::MessageContainer {
            messages: vec![
                ::schema::manual::Message {
                    msg_id: next_message_id(),
                    seqno: self.next_seq_no(MessagePurpose::NonContent),
                    body: WithSize::new(Boxed::new(Box::new(acks) as Object))?,
                },
                ::schema::manual::Message {
                    msg_id: next_message_id(),
                    seqno: self.next_seq_no(MessagePurpose::Content),
                    body: WithSize::new(Boxed::new(Box::new(body) as Object))?,
                }
            ],
        };

        let msg_container_id = msg_container.messages[1].msg_id;
        let mut message = self.impl_create_decrypted_message(msg_container, MessagePurpose::Content)?;

        match *&mut message {
            Message::PlainText { .. } => unreachable!(),
            Message::Decrypted { ref mut decrypted_data } => decrypted_data.message_id = msg_container_id,
        }

        Ok(Some(message))
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

        let message = Message::Decrypted {
            decrypted_data: decrypted_data,
        };

        Ok(message)
    }

    /// Reads a `Message` from raw bytes.
    pub fn process_message<T>(&self, message_bytes: &[u8], encrypted_data_len: Option<u32>) -> error::Result<Message<T>>
        where T: DeserializeOwned
    {
        use serde_mtproto::Deserializer;

        let mut deserializer = Deserializer::new(message_bytes, None);
        let seed = MessageSeed::new(self.auth_key.clone(), encrypted_data_len);

        seed.deserialize(&mut deserializer).map_err(Into::into)
    }
}
