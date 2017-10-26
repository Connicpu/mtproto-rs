//! Message-related definitions.

use std::fmt;
use std::marker::PhantomData;

use extprim::i128::i128;
use serde::ser::{self, Error as SerError, Serialize};
use serde::de::{self, DeserializeOwned, DeserializeSeed, Error as DeError, SeqAccess, Visitor};
use serde_mtproto::{self, Boxed, Identifiable, MtProtoSized, WithSize, UnsizedByteBuf, UnsizedByteBufSeed, size_hint_from_unsized_byte_seq_len};

use error::{self, ErrorKind};

use super::encryption::AuthKey;
use super::utils::EitherRef;


/// Possible kinds of MTProto messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageType {
    PlainText,
    Encrypted,
}


/// Holds data relevant to a specific MTProto message in a type-safe
/// manner.
#[derive(Debug, PartialEq)]
pub enum Message<T> {
    /// Data transmitted in plain-text.
    PlainText {
        message_id: i64,
        body: WithSize<Boxed<T>>,
    },
    /// Either data to encrypt for sending or data alredy decrypted
    /// after receiving raw bytes.
    Decrypted {
        decrypted_data: DecryptedData<T>,
    },
}

// We use signed integers here because that's the default integer representation in MTProto;
// by trying to match representations we can synchronize the range of allowed values
/// Holds data either to be encrypted (in requests) or after decryption (in responses).
#[derive(Debug, PartialEq, Serialize, Deserialize, MtProtoSized)]
pub struct DecryptedData<T> {
    pub(super) salt: i64,
    pub(super) session_id: i64,
    pub(super) message_id: i64,
    pub(super) seq_no: i32,
    pub(super) body: WithSize<Boxed<T>>,

    #[serde(skip)]
    #[mtproto_sized(skip)]
    pub(super) key: AuthKey,
}

#[derive(Debug, Serialize)]
enum RawMessage<'msg, T: 'msg> {
    PlainText {
        auth_key_id: i64,
        message_id: i64,
        ref_body: EitherRef<'msg, WithSize<Boxed<T>>>,
    },
    Encrypted {
        auth_key_id: i64,
        msg_key: i128,
        encrypted_data: UnsizedByteBuf,
    },
}


impl<T: MtProtoSized> MtProtoSized for Message<T> {
    fn size_hint(&self) -> serde_mtproto::Result<usize> {
        // just a dummy value, not an actual one
        let auth_key_id_size = i64::size_hint(&0)?;

        let size_hint = match *self {
            Message::PlainText { message_id, ref body } => {
                let message_id_size = message_id.size_hint()?;
                let body_size = body.size_hint()?;

                auth_key_id_size + message_id_size + body_size
            },
            Message::Decrypted { ref decrypted_data } => {
                // just a dummy value, not an actual one
                let msg_key_size = i128::size_hint(&i128::new(0))?;
                let minimum_encrypted_data_size = decrypted_data.size_hint()?;
                let actual_encrypted_data_size =
                    size_hint_from_unsized_byte_seq_len(minimum_encrypted_data_size)?;

                auth_key_id_size + msg_key_size + actual_encrypted_data_size
            },
        };

        Ok(size_hint)
    }
}


impl<T: Identifiable + MtProtoSized> Message<T> {
    /// Returns `Some(body)` if the message was plain-text.
    /// Otherwise returns `None`.
    pub fn into_plain_text_body(self) -> Option<T> {
        match self {
            Message::PlainText { body, .. } => Some(body.into_inner().into_inner()),
            Message::Decrypted { .. } => None,
        }
    }

    /// Unwraps the body of the plain-text message.
    ///
    /// # Panics
    ///
    /// Panics if the message was encrypted.
    pub fn unwrap_plain_text_body(self) -> T {
        self.into_plain_text_body().expect("`Message::PlainText` variant")
    }

    /// Returns `Some(body)` if the message was encrypted.
    /// Otherwise returns `None`.
    pub fn into_decrypted_body(self) -> Option<T> {
        match self {
            Message::PlainText { .. } => None,
            Message::Decrypted { decrypted_data } => Some(decrypted_data.body.into_inner().into_inner()),
        }
    }

    /// Unwraps the body of the encrypted message.
    ///
    /// # Panics
    ///
    /// Panics if the message was plain-text.
    pub fn unwrap_decrypted_body(self) -> T {
        self.into_decrypted_body().expect("`Message::Decrypted` variant")
    }
}

impl<T> Message<T> {
    fn to_raw_message<'msg>(&'msg self) -> error::Result<RawMessage<'msg, T>>
        where T: fmt::Debug + Serialize
    {
        let raw_message = match *self {
            Message::PlainText { message_id, ref body } => {
                RawMessage::PlainText {
                    auth_key_id: 0,
                    message_id: message_id,
                    ref_body: EitherRef::Ref(body),
                }
            },
            Message::Decrypted { ref decrypted_data } => {
                let decrypted_data_serialized = serde_mtproto::to_bytes(decrypted_data)?;
                debug!("Serialized data to be encrypted: {:?}", &decrypted_data_serialized);

                let (auth_key_id, msg_key, encrypted_data) = decrypted_data.key
                    .encrypt_message_bytes(&decrypted_data_serialized)?;

                RawMessage::Encrypted {
                    auth_key_id: auth_key_id,
                    msg_key: msg_key,
                    encrypted_data: UnsizedByteBuf::new(encrypted_data),
                }
            },
        };

        debug!("Resulting raw message: {:?}", &raw_message);

        Ok(raw_message)
    }

    fn from_raw_message<'msg>(raw_message: RawMessage<'msg, T>,
                              opt_key: Option<AuthKey>)
                             -> error::Result<Message<T>>
        where T: fmt::Debug + DeserializeOwned
    {
       let message =  match raw_message {
            RawMessage::PlainText { auth_key_id, message_id, ref_body } => {
                assert_eq!(auth_key_id, 0);

                Message::PlainText {
                    message_id: message_id,
                    body: ref_body.into_owned().unwrap(), // FIXME
                }
            },
            RawMessage::Encrypted { auth_key_id, msg_key, encrypted_data } => {
                let key = opt_key.ok_or(ErrorKind::NoAuthKey)?;
                let decrypted_data_serialized = key
                    .decrypt_message_bytes(auth_key_id, msg_key, &encrypted_data.into_inner())?;
                debug!("Decrypted data to be deserialized: {:?}", &decrypted_data_serialized);

                let mut decrypted_data: DecryptedData<T> =
                    serde_mtproto::from_reader(decrypted_data_serialized.as_slice(), None)?;

                decrypted_data.key = key;

                Message::Decrypted {
                    decrypted_data: decrypted_data,
                }
            },
        };

        debug!("Message obtained from raw message: {:?}", &message);

        Ok(message)
    }
}

impl<T: fmt::Debug + Serialize> Serialize for Message<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        self.to_raw_message()
            .map_err(S::Error::custom)?
            .serialize(serializer)
            .map_err(S::Error::custom)
    }
}


#[derive(Debug)]
pub struct MessageSeed<T> {
    opt_key: Option<AuthKey>,
    encrypted_data_len: Option<u32>,
    phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> MessageSeed<T> {
    pub fn new(opt_key: Option<AuthKey>, encrypted_data_len: Option<u32>) -> MessageSeed<T> {
        MessageSeed {
            opt_key: opt_key,
            encrypted_data_len: encrypted_data_len,
            phantom: PhantomData,
        }
    }
}

impl<'de, T: fmt::Debug + DeserializeOwned> DeserializeSeed<'de> for MessageSeed<T> {
    type Value = Message<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Message<T>, D::Error>
        where D: de::Deserializer<'de>
    {
        struct MessageVisitor<T> {
            opt_key: Option<AuthKey>,
            encrypted_data_len: Option<u32>,
            phantom: PhantomData<T>,
        }

        impl<'de, T: fmt::Debug + DeserializeOwned> Visitor<'de> for MessageVisitor<T> {
            type Value = Message<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a message")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Message<T>, A::Error>
                where A: SeqAccess<'de>
            {
                //TODO: add more info to error data
                let errconv = |kind: ErrorKind| A::Error::custom(error::Error::from(kind));

                let auth_key_id = seq.next_element()?
                    .ok_or(errconv(ErrorKind::NotEnoughFields("Message::?", 0)))?;

                let raw_message = if auth_key_id == 0 {
                    let message_id = seq.next_element()?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::PlainText", 1)))?;
                    let ref_body: EitherRef<WithSize<Boxed<T>>> = seq.next_element()?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::PlainText", 2)))?;

                    RawMessage::PlainText {
                        auth_key_id: 0,
                        message_id: message_id,
                        ref_body: ref_body,
                    }
                } else {
                    if self.encrypted_data_len.is_none() {
                        bail!(errconv(ErrorKind::NoEncryptedDataLengthProvided));
                    }

                    let msg_key = seq.next_element()?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::Decrypted", 1)))?;

                    let seed = UnsizedByteBufSeed::new(self.encrypted_data_len.unwrap());
                    let encrypted_data = seq.next_element_seed(seed)?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::Decrypted", 2)))?;

                    let raw_message = RawMessage::Encrypted {
                        auth_key_id: auth_key_id,
                        msg_key: msg_key,
                        encrypted_data: encrypted_data,
                    };

                    raw_message
                };

                let message = Message::from_raw_message(raw_message, self.opt_key)
                    .map_err(A::Error::custom)?;

                Ok(message)
            }
        }

        let visitor = MessageVisitor {
            opt_key: self.opt_key,
            encrypted_data_len: self.encrypted_data_len,
            phantom: PhantomData,
        };

        deserializer.deserialize_tuple(3, visitor)
    }
}
