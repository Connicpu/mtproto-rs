use std::fmt;
use std::any::Any;

use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde_mtproto::Identifiable;

//use tl::{self, Result};
//use tl::parsing::{ConstructorId, Reader, Writer};

//pub struct TLCtor<R: Reader>(pub fn(Option<ConstructorId>, &mut R) -> Result<Box<TLObject>>);
//pub struct TLCtorMap<R: Reader>(pub HashMap<ConstructorId, TLCtor<R>>);

/*impl<R: Reader> Clone for TLCtor<R> {
    fn clone(&self) -> Self {
        TLCtor(self.0)
    }
}

impl<R: Reader> Copy for TLCtor<R> {}

impl<R: Reader> TLCtorMap<R> {
    pub fn add<T: TLDynamic>(&mut self, id: ConstructorId) {
        self.0.insert(id, TLCtor(T::deserialize));
    }

    pub fn get(&self, id: &ConstructorId) -> TLCtor<R> {
        self.0.get(id)
            .cloned()
            .unwrap_or_else(|| TLCtor(UnreadableBag::deserialize))
    }
}

impl<R: Reader> Default for TLCtorMap<R> {
    fn default() -> TLCtorMap<R> {
        TLCtorMap(Default::default())
    }
}*/

/*pub trait TLObject: Any + tl::WriteType {
    fn as_any(&self) -> &Any;
    fn as_boxed_any(self: Box<Self>) -> Box<Any>;
}*/

pub trait TLObject: Any + Serialize {
    fn as_any(&self) -> &Any;
    fn as_boxed_any(self: Box<Self>) -> Box<Any>;
}

/*impl<T: Any + tl::WriteType> TLObject for T {
    /*default*/ fn as_any(&self) -> &Any { self }
    /*default*/ fn as_boxed_any(self: Box<Self>) -> Box<Any> { self }
}*/

impl<T: Any + Serialize> TLObject for T {
    fn as_any(&self) -> &Any { self }
    fn as_boxed_any(self: Box<Self>) -> Box<Any> { self }
}

/*#[derive(Debug, Clone)]
pub struct UnreadableBag {
    pub tl_id: Option<ConstructorId>,
    pub bytes: Vec<u8>,
}

impl tl::IdentifiableType for UnreadableBag {
    fn type_id(&self) -> Option<ConstructorId> {
        self.tl_id
    }
}

impl tl::WriteType for UnreadableBag {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        writer.write_all(&self.bytes)?;
        Ok(())
    }
}

impl TLObject for UnreadableBag {
    fn as_any(&self) -> &Any { self }
    fn as_boxed_any(self: Box<Self>) -> Box<Any> { self }
}*/

impl Clone for Box<TLObject> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

/*impl tl::IdentifiableType for Box<TLObject> {
    fn type_id(&self) -> Option<ConstructorId> {
        tl::IdentifiableType::type_id(&**self)
    }
}*/

impl Identifiable for Box<TLObject> {
    fn type_id(&self) -> Option<i32> {
        Identifiable::type_id(&**self)
    }
}

/*impl tl::WriteType for Box<TLObject> {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        tl::WriteType::serialize(&**self, writer)
    }
}*/

impl Serialize for Box<TLObject> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        Serialize::serialize(&**self, serializer)
    }
}

/*impl<'a> tl::IdentifiableType for &'a TLObject {
    fn type_id(&self) -> Option<ConstructorId> {
        (*self).type_id()
    }
}*/

impl<'a> Identifiable for &'a TLObject {
    fn type_id(&self) -> Option<i32> {
        (*self).type_id()
    }
}

/*impl<'a> tl::WriteType for &'a TLObject {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        (*self).serialize(writer)
    }
}*/

impl Serialize for Box<TLObject> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        (*self).serialize(serializer)
    }
}

/*impl tl::ReadType for Box<TLObject> {
    fn deserialize_bare<R: Reader>(_: Option<ConstructorId>, _: &mut R) -> Result<Self> {
        unimplemented!()
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        reader.read_polymorphic()
    }
}*/

impl<'de> Deserialize<'de> for Box<TLObject> {
    fn deserialize<D>(deserializer: D) -> Result<Box<TLObject>, D::Error>
        where D: Deserializer<'de>
    {
        unimplemented!()
    }
}

/*impl TLObject for Box<TLObject> {
    fn as_any(&self) -> &Any {
        (&**self).as_any()
    }

    fn as_boxed_any(self: Box<Self>) -> Box<Any> {
        (*self).as_boxed_any()
    }
}*/

impl fmt::Debug for TLObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(boxed TLObject tl_id:{:?})", self.type_id())
    }
}

/*#[derive(Debug, Clone)]
pub struct LengthAndObject(pub Box<TLObject>);

/*impl tl::IdentifiableType for LengthAndObject {
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
}*/

impl Identifiable for LengthAndObject {
    fn type_id(&self) -> Option<i32> {
        None
    }
}

impl tl::WriteType for LengthAndObject {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        let bytes = tl::serialize_message(&*self.0)?;
        writer.write_tl(&(bytes.len() as u32))?;
        writer.write_all(&bytes)?;
        Ok(())
    }
}

impl tl::ReadType for LengthAndObject {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
        super::ensure_type_id(None, id)?;
        let len: u32 = reader.read_tl()?;
        let ret = reader.take(len as usize).read_polymorphic()?;
        Ok(LengthAndObject(ret))
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Self::deserialize_bare(None, reader)
    }
}

impl From<Box<TLObject>> for LengthAndObject {
    fn from(b: Box<TLObject>) -> LengthAndObject {
        LengthAndObject(b)
    }
}

// impl<'a> From<&'a [u8]> for LengthAndObject {
//     fn from(bytes: &'a [u8]) -> LengthAndObject {
//         LengthAndObject(Box::new(UnreadableBag {
//             tl_id: None,
//             bytes: bytes.into(),
//         }))
//     }
// }
*/

/*pub trait TLDynamic: TLObject {
    fn deserialize<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Box<TLObject>>;
    fn downcast_from(b: Box<TLObject>) -> ::std::result::Result<Self, Box<TLObject>>
        where Self: Sized;
}

fn downcast<T: TLObject>(b: Box<TLObject>) -> ::std::result::Result<T, Box<TLObject>> {
    if b.as_any().is::<T>() {
        Ok(*b.as_boxed_any().downcast::<T>().unwrap())
    } else {
        Err(b)
    }
}

/*impl TLDynamic for UnreadableBag {
    fn deserialize<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Box<TLObject>> {
        let mut remainder = vec![];
        reader.read_to_end(&mut remainder)?;
        Ok(Box::new(UnreadableBag {
            tl_id: id,
            bytes: remainder,
        }))
    }

    fn downcast_from(b: Box<TLObject>) -> ::std::result::Result<Self, Box<TLObject>>
        where Self: Sized,
    {
        downcast::<Self>(b)
    }
}*/

impl<T: ?Sized + TLObject + tl::ReadType> TLDynamic for T {
    fn deserialize<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Box<TLObject>> {
        Ok(Box::new(<T as tl::ReadType>::deserialize_bare(id, reader)?))
    }

    default fn downcast_from(b: Box<TLObject>) -> ::std::result::Result<Self, Box<TLObject>>
        where Self: Sized,
    {
        downcast::<Self>(b)
    }
}

impl<T: TLObject + tl::ReadType> TLDynamic for Vec<T> {
    fn downcast_from(b: Box<TLObject>) -> ::std::result::Result<Self, Box<TLObject>>
        where Self: Sized,
    {
        downcast::<Vec<Box<TLObject>>>(b)?
            .into_iter()
            .map(downcast::<T>)
            .collect()
    }
}*/
