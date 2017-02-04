use super::{Result, Type};
use std::fmt;
use std::any::Any;
use std::collections::HashMap;
use tl::parsing::{ConstructorId, Reader, Writer};

pub struct TLCtor<R: Reader>(pub fn(ConstructorId, &mut R) -> Result<Box<TLObject>>);
pub struct TLCtorMap<R: Reader>(pub HashMap<ConstructorId, TLCtor<R>>);

impl<R: Reader> Default for TLCtorMap<R> {
    fn default() -> TLCtorMap<R> {
        TLCtorMap(Default::default())
    }
}

pub trait TLObject: Any {
    fn tl_id(&self) -> Option<ConstructorId>;
    fn as_any(&self) -> &Any;
    fn as_boxed_any(self: Box<Self>) -> Box<Any>;
}

pub fn downcast<T: TLObject>(b: Box<TLObject>) -> ::std::result::Result<Box<T>, Box<TLObject>> {
    if b.as_any().is::<T>() {
        Ok(b.as_boxed_any().downcast::<T>().unwrap())
    } else {
        Err(b)
    }
}

impl<T: Type + Any> TLObject for T {
    fn tl_id(&self) -> Option<ConstructorId> {
        self.type_id()
    }

    default fn as_any(&self) -> &Any { self }
    default fn as_boxed_any(self: Box<Self>) -> Box<Any> { self }
}

#[derive(Debug)]
pub struct UnreadableBag {
    pub tl_id: ConstructorId,
    pub bytes: Vec<u8>,
}

impl TLObject for UnreadableBag {
    fn tl_id(&self) -> Option<ConstructorId> {
        Some(self.tl_id)
    }

    fn as_any(&self) -> &Any { self }
    fn as_boxed_any(self: Box<Self>) -> Box<Any> { self }
}

impl Type for Box<TLObject> {
    fn bare_type() -> bool {
        true
    }

    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Writer>(&self, _: &mut W) -> Result<()> {
        unimplemented!()
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        reader.read_polymorphic()
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        unimplemented!()
    }
}

impl TLObject for Box<TLObject> {
    fn as_any(&self) -> &Any {
        (&**self).as_any()
    }

    fn as_boxed_any(self: Box<Self>) -> Box<Any> {
        (*self).as_boxed_any()
    }
}

impl fmt::Debug for TLObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(boxed TLObject tl_id:{:?})", self.tl_id())
    }
}

#[derive(Debug)]
pub struct LengthAndObject(pub Box<TLObject>);

impl Type for LengthAndObject {
    fn bare_type() -> bool {
        true
    }

    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Writer>(&self, _: &mut W) -> Result<()> {
        unimplemented!()
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        let len: u32 = reader.read_bare()?;
        let ret = reader.take(len as usize).read_polymorphic()?;
        Ok(LengthAndObject(ret))
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        unimplemented!()
    }
}

pub trait TLDynamic: TLObject {
    fn deserialize<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Box<TLObject>>;
}

impl<T: TLObject + Type> TLDynamic for T {
    fn deserialize<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Box<TLObject>> {
        Ok(Box::new(<T as Type>::deserialize_boxed(id, reader)?))
    }
}
