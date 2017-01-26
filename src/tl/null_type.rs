use std::io::{Read, Write};
use tl::{self, Type};
use tl::parsing::{ConstructorId, Reader, Writer};
use tl::dynamic::{TLCtorMap, TLDynamic};

#[derive(Copy, Clone, Debug)]
pub struct Null;

impl Null {
    pub const SIGNATURE: ConstructorId = ConstructorId(0x56730bcc);
}

impl Type for Null {
    fn bare_type() -> bool {
        false
    }

    fn type_id(&self) -> Option<ConstructorId> {
        Some(Null::SIGNATURE)
    }

    fn serialize<W: Writer>(&self, _: &mut W) -> tl::Result<()> {
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> tl::Result<Self> {
        Err(tl::Error::BoxedAsBare)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, _: &mut R) -> tl::Result<Self> {
        match id {
            Null::SIGNATURE => Ok(Null),
            _ => Err(tl::Error::InvalidData),
        }
    }
}
