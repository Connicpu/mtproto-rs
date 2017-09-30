use std::fmt;
use std::marker::PhantomData;

use extprim::i128::i128;
use serde::ser::{self, Error as SerError, Serialize};
use serde::de::{self, DeserializeOwned, DeserializeSeed, Error as DeError, SeqAccess, Visitor};
use serde_mtproto::{self, Boxed, Identifiable, MtProtoSized, WithSize, UnsizedByteBuf, UnsizedByteBufSeed};

use error::{self, ErrorKind};

use super::AuthKey;
use super::utils::EitherRef;


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageType {
    PlainText,
    Encrypted,
}


#[derive(Debug, PartialEq)]
pub enum Message<T> {
    PlainText {
        message_id: i64,
        body: WithSize<Boxed<T>>,
    },
    Decrypted {
        decrypted_data: DecryptedData<T>,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DecryptedData<T> {
    pub(super) salt: i64,
    pub(super) session_id: i64,
    pub(super) message_id: i64,
    pub(super) seq_no: i32,
    pub(super) body: WithSize<Boxed<T>>,

    #[serde(skip)]
    pub(super) key: AuthKey,
}

impl<T: Identifiable + MtProtoSized> Message<T> {
    pub fn into_plain_text_body(self) -> Option<T> {
        match self {
            Message::PlainText { body, .. } => Some(body.into_inner().into_inner()),
            Message::Decrypted { .. } => None,
        }
    }

    pub fn unwrap_plain_text_body(self) -> T {
        self.into_plain_text_body().expect("`Message::PlainText` variant")
    }

    pub fn into_decrypted_body(self) -> Option<T> {
        match self {
            Message::PlainText { .. } => None,
            Message::Decrypted { decrypted_data } => Some(decrypted_data.body.into_inner().into_inner()),
        }
    }

    pub fn unwrap_decrypted_body(self) -> T {
        self.into_decrypted_body().expect("`Message::Decrypted` variant")
    }
}

impl<T: Serialize> Serialize for Message<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        match *self {
            Message::PlainText { ref message_id, ref body } => {
                let msg_to_serialize = RawMessage::PlainText {
                    auth_key_id: 0,
                    message_id: *message_id,
                    body: EitherRef::Ref(&body),
                };

                msg_to_serialize.serialize(serializer)
            },
            Message::Decrypted { ref decrypted_data } => {
                let decrypted_data_serialized = serde_mtproto::to_bytes(decrypted_data)
                    .map_err(S::Error::custom)?;
                let (auth_key_id, msg_key, encrypted_data) = decrypted_data.key
                    .encrypt_message_bytes(&decrypted_data_serialized)
                    .map_err(S::Error::custom)?;

                let msg_to_serialize: RawMessage<()> = RawMessage::Encrypted {
                    auth_key_id: auth_key_id,
                    msg_key: msg_key,
                    encrypted_data: UnsizedByteBuf::new(encrypted_data),
                };

                msg_to_serialize.serialize(serializer)
            },
        }
    }
}


#[derive(Debug)]
pub struct MessageSeed<T> {
    opt_key: Option<AuthKey>,
    encrypted_data_len: u32,
    phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> MessageSeed<T> {
    pub fn new(opt_key: Option<AuthKey>, encrypted_data_len: u32) -> MessageSeed<T> {
        MessageSeed {
            opt_key: opt_key,
            encrypted_data_len: encrypted_data_len,
            phantom: PhantomData,
        }
    }
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for MessageSeed<T> {
    type Value = Message<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Message<T>, D::Error>
        where D: de::Deserializer<'de>
    {
        struct MessageVisitor<T> {
            opt_key: Option<AuthKey>,
            encrypted_data_len: u32,
            phantom: PhantomData<T>,
        }

        impl<'de, T: DeserializeOwned> Visitor<'de> for MessageVisitor<T> {
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
                    .ok_or(errconv(ErrorKind::NotEnoughFields("Message", 0)))?;

                let message = if auth_key_id == 0 {
                    let message_id = seq.next_element()?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::PlainText", 1)))?;
                    let body: EitherRef<WithSize<Boxed<T>>> = seq.next_element()?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::PlainText", 2)))?;

                    Message::PlainText {
                        message_id: message_id,
                        body: body.into_owned().unwrap(),    // we know it's owned here
                    }
                } else {
                    let message_key = seq.next_element()?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::Decrypted", 1)))?;

                    let seed = UnsizedByteBufSeed::new(self.encrypted_data_len);
                    let encrypted_data = seq.next_element_seed(seed)?
                        .ok_or(errconv(ErrorKind::NotEnoughFields("Message::Decrypted", 2)))?;

                    let key = self.opt_key.ok_or(errconv(ErrorKind::NoAuthKey))?;
                    let decrypted_data_serialized = key
                        .decrypt_message_bytes(auth_key_id, message_key, &encrypted_data.into_inner())
                        .map_err(A::Error::custom)?;
                    let mut decrypted_data: DecryptedData<T> = serde_mtproto::from_reader(decrypted_data_serialized.as_slice(), None)
                        .map_err(A::Error::custom)?;

                    decrypted_data.key = key;

                    Message::Decrypted {
                        decrypted_data: decrypted_data,
                    }
                };

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


#[derive(Debug, Serialize)]
enum RawMessage<'a, T: 'a> {
    PlainText {
        auth_key_id: i64,
        message_id: i64,
        body: EitherRef<'a, WithSize<Boxed<T>>>,
    },
    Encrypted {
        auth_key_id: i64,
        msg_key: i128,
        encrypted_data: UnsizedByteBuf,
    },
}
