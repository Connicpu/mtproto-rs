//! Everything necessary for dynamic typing in the context of Type
//! Language.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use erased_serde::{self, Serialize as ErasedSerialize, Deserializer as ErasedDeserializer};
use serde::ser::{Serialize, Serializer};
use serde::de::{self, DeserializeOwned, DeserializeSeed, Deserializer, Error as DeError};
use serde_mtproto::{Identifiable, MtProtoSized};

use error::{self, ErrorKind};


/// \[**IMPLEMENTATION DETAIL**]
/// Helper trait to implement Clone for trait objects.
///
/// Idea taken from:
///
/// * https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714
/// * https://stackoverflow.com/questions/30353462/how-to-clone-a-struct-storing-a-trait-object
#[doc(hidden)]
pub trait TLObjectCloneToBox {
    fn clone_to_box(&self) -> Box<TLObject>;
}

impl<T: 'static + Clone + TLObject> TLObjectCloneToBox for T {
    fn clone_to_box(&self) -> Box<TLObject> {
        Box::new(self.clone())
    }
}


/// For any object whose type is representable in Type Language.
pub trait TLObject: Any + ErasedSerialize + Identifiable + MtProtoSized + TLObjectCloneToBox {
    fn as_any(&self) -> &Any;
    fn as_box_any(self: Box<Self>) -> Box<Any>;
}

// TLObject impls

impl Serialize for TLObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        erased_serde::serialize(self, serializer)
    }
}

impl fmt::Debug for TLObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("TLObject [trait object]")
    }
}

// &TLObject impls

impl<'a> Identifiable for &'a TLObject {
    fn type_id(&self) -> u32 {
        (**self).type_id()
    }

    fn enum_variant_id(&self) -> Option<&'static str> {
        (**self).enum_variant_id()
    }
}

// Box<TLObject> impls

impl Clone for Box<TLObject> {
    fn clone(&self) -> Box<TLObject> {
        self.clone_to_box()
    }
}

impl Identifiable for Box<TLObject> {
    fn type_id(&self) -> u32 {
        (**self).type_id()
    }

    fn enum_variant_id(&self) -> Option<&'static str> {
        (**self).enum_variant_id()
    }
}

// impl TLObject for types

impl<T: Clone + Any + Serialize + Identifiable + MtProtoSized> TLObject for T {
    fn as_any(&self) -> &Any { self }
    fn as_box_any(self: Box<Self>) -> Box<Any> { self }
}


pub(crate) type TLConstructorType = Box<Fn(&mut ErasedDeserializer) -> Result<Box<TLObject>, erased_serde::Error>>;

/// A single TL constructor body (i.e. without its id).
pub struct TLConstructor(TLConstructorType);

impl fmt::Debug for TLConstructor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("TLConstructor")
            .field(&"TL constructor [boxed closure]")
            .finish()
    }
}

/// A mapping between TL constructor ids and corresponding TL constructor bodies.
#[derive(Debug)]
pub struct TLConstructorsMap(pub(crate) HashMap<u32, TLConstructor>);

impl TLConstructorsMap {
    pub fn new() -> TLConstructorsMap {
        TLConstructorsMap(HashMap::new())
    }

    pub fn add<T: TLObject + DeserializeOwned>(&mut self, type_id: u32) {
        self.0.insert(type_id, TLConstructor(Box::new(|deserializer| {
            erased_serde::deserialize::<T>(deserializer)
                .map(|obj| Box::new(obj) as Box<TLObject>)
        })));
    }

    pub fn get(&self, type_id: u32) -> Option<&TLConstructor> {
        self.0.get(&type_id)
    }
}

impl<'de> DeserializeSeed<'de> for TLConstructorsMap {
    type Value = Box<TLObject>;

    fn deserialize<D>(self, deserializer: D) -> Result<Box<TLObject>, D::Error>
        where D: Deserializer<'de>
    {
        fn errconv<E: DeError>(kind: ErrorKind) -> E {
            E::custom(error::Error::from(kind))
        }

        struct BoxTLObjectVisitor(TLConstructorsMap);

        impl<'de> de::Visitor<'de> for BoxTLObjectVisitor {
            type Value = Box<TLObject>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a boxed dynamically-typed value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Box<TLObject>, A::Error>
                where A: de::SeqAccess<'de>
            {
                struct BoxTLObjectSeed(TLConstructorsMap, u32);

                impl<'de> DeserializeSeed<'de> for BoxTLObjectSeed {
                    type Value = Box<TLObject>;

                    fn deserialize<D>(self, deserializer: D) -> Result<Box<TLObject>, D::Error>
                        where D: Deserializer<'de>
                    {
                        let ctor = &(self.0).0.get(&self.1)
                            .ok_or(errconv(ErrorKind::UnknownConstructorId("Box<TLObject>", self.1)))?.0;

                        ctor(&mut ErasedDeserializer::erase(deserializer)).map_err(|e| D::Error::custom(e))
                    }
                }

                let type_id = seq.next_element()?
                    .ok_or(errconv(ErrorKind::NotEnoughFields("Box<TLObject>", 0)))?;
                let object = seq.next_element_seed(BoxTLObjectSeed(self.0, type_id))?
                    .ok_or(errconv(ErrorKind::NotEnoughFields("Box<TLObject>", 1)))?;

                Ok(object)
            }
        }

        deserializer.deserialize_tuple(2, BoxTLObjectVisitor(self))
    }
}
