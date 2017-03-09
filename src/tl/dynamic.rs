use super::{Result, Type};
use std::fmt;
use std::any::Any;
use std::collections::HashMap;
use tl::parsing::{ConstructorId, Reader, Writer};

pub struct TLCtor<R: Reader>(pub fn(ConstructorId, &mut R) -> Result<Box<TLObject>>);
pub struct TLCtorMap<R: Reader>(pub HashMap<ConstructorId, TLCtor<R>>);

impl<R: Reader> Clone for TLCtor<R> {
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
}

pub trait TLObject: Any {
    fn tl_id(&self) -> Option<ConstructorId>;
    fn as_any(&self) -> &Any;
    fn as_boxed_any(self: Box<Self>) -> Box<Any>;
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

impl TLDynamic for UnreadableBag {
    fn deserialize<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Box<TLObject>> {
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
}

impl<T: TLObject + Type> TLDynamic for T {
    fn deserialize<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Box<TLObject>> {
        Ok(Box::new(<T as Type>::deserialize_boxed(id, reader)?))
    }

    default fn downcast_from(b: Box<TLObject>) -> ::std::result::Result<Self, Box<TLObject>>
        where Self: Sized,
    {
        downcast::<Self>(b)
    }
}

impl<T: TLObject + Type> TLDynamic for Vec<T> {
    fn downcast_from(b: Box<TLObject>) -> ::std::result::Result<Self, Box<TLObject>>
        where Self: Sized,
    {
        downcast::<Vec<Box<TLObject>>>(b)?
            .into_iter()
            .map(downcast::<T>)
            .collect()
    }
}
