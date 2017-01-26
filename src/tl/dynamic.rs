use super::{Result, Type};
use std::io::Write;
use std::any::Any;
use std::collections::HashMap;
use tl::parsing::{ConstructorId, Reader, Writer};

pub struct TLCtor<R: Reader>(pub fn(ConstructorId, &mut R) -> Result<Box<TLObject>>);
pub struct TLCtorMap<R: Reader>(pub HashMap<ConstructorId, TLCtor<R>>);

pub trait TLObject: Any {
    fn tl_id(&self) -> ConstructorId;
    fn as_any(&self) -> &Any;
}

impl<T: Type + Any> TLObject for T {
    fn tl_id(&self) -> ConstructorId {
        self.type_id().unwrap()
    }

    fn as_any(&self) -> &Any { self }
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

pub trait TLDynamic: TLObject {
    fn deserialize<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Box<TLObject>>;
}

impl<T: TLObject + Type> TLDynamic for T {
    fn deserialize<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Box<TLObject>> {
        Ok(Box::new(<T as Type>::deserialize_boxed(id, reader)?))
    }
}
