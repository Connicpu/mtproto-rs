use std::io::{Read, Write};
use tl::{self, Type};
use tl::parsing::{ConstructorId, Reader, Writer};

#[derive(Copy, Clone, Debug)]
pub struct True;

impl True {
    pub const SIGNATURE: ConstructorId = ConstructorId(0x3fedd339);
}

impl Type for True {
    fn bare_type() -> bool {
        false
    }

    fn type_id(&self) -> Option<ConstructorId> {
        Some(True::SIGNATURE)
    }

    fn serialize<W: Writer>(&self, _: &mut W) -> tl::Result<()> {
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> tl::Result<Self> {
        Err(tl::Error::BoxedAsBare)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, _: &mut R) -> tl::Result<Self> {
        match id {
            True::SIGNATURE => Ok(True),
            _ => Err(tl::Error::InvalidData),
        }
    }
}
