use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use erased_serde::{self, Serialize as ErasedSerialize, Deserializer as ErasedDeserializer};
use serde::ser::{Serialize, Serializer};
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer, Error as DeError};
use serde_mtproto::{Identifiable, MtProtoSized};


pub trait TLObjectCloneToBox {
    fn clone_to_box(&self) -> Box<TLObject>;
}

impl<T: 'static + Clone + TLObject> TLObjectCloneToBox for T {
    fn clone_to_box(&self) -> Box<TLObject> {
        Box::new(self.clone())
    }
}

//impl<T: 'static + Serialize + TLObject> TLObjectImpl for T {
//    fn 
//}


pub trait TLObject: Any + ErasedSerialize + Identifiable + MtProtoSized + TLObjectCloneToBox {
    fn as_any(&self) -> &Any;
    fn as_box_any(self: Box<Self>) -> Box<Any>;
}

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

impl<'de> DeserializeSeed<'de> for TLConstructorsMap {
    type Value = Box<TLObject>;

    fn deserialize<D>(self, deserializer: D) -> Result<Box<TLObject>, D::Error>
        where D: Deserializer<'de>
    {
        struct BoxTLObjectVisitor(TLConstructorsMap);

        impl<'de> de::Visitor<'de> for BoxTLObjectVisitor {
            type Value = Box<TLObject>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a boxed dynamically-typed value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Box<TLObject>, A::Error>
                where A: de::SeqAccess<'de>
            {
                struct BoxTLObjectSeed(TLConstructorsMap, i32);

                impl<'de> DeserializeSeed<'de> for BoxTLObjectSeed {
                    type Value = Box<TLObject>;

                    fn deserialize<D>(self, mut deserializer: D) -> Result<Box<TLObject>, D::Error>
                        where D: Deserializer<'de>
                    {
                        let ctor = (self.0).0.get(&self.1).unwrap(); // FIXME
                        ctor(&ErasedDeserializer::erase(deserializer)).map_err(|e| D::Error::custom(e))
                    }
                }

                let type_id = seq.next_element()?.unwrap(); // FIXME
                let object = seq.next_element_seed(BoxTLObjectSeed(self.0, type_id))?.unwrap(); // FIXME

                Ok(object)
            }
        }

        deserializer.deserialize_tuple(2, BoxTLObjectVisitor(self))
    }
}

/*impl Identifiable for TLObject {
    fn type_id(&self) -> i32 {
        (*self).type_id()
    }

    fn enum_variant_id(&self) -> Option<&'static str> {
        (*self).enum_variant_id()
    }
}*/

impl Clone for Box<TLObject> {
    fn clone(&self) -> Box<TLObject> {
        self.clone_to_box()
    }
}

impl Identifiable for Box<TLObject> {
    fn type_id(&self) -> i32 {
        (**self).type_id()
    }

    fn enum_variant_id(&self) -> Option<&'static str> {
        (**self).enum_variant_id()
    }
}

impl<T: Clone + Any + Serialize + Identifiable + MtProtoSized> TLObject for T {
    fn as_any(&self) -> &Any { self }
    fn as_box_any(self: Box<Self>) -> Box<Any> { self }
}


pub struct TLConstructorsMap(pub(crate) HashMap<i32, fn(&ErasedDeserializer) -> Result<Box<TLObject>, erased_serde::Error>>);

impl TLConstructorsMap {
    fn new() -> TLConstructorsMap {
        TLConstructorsMap(HashMap::new())
    }
}
