use std::io::{Read, Write};
use tl::{self, Type};
use tl::parsing::{ConstructorId, Reader, WriteContext};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bool(pub bool);

impl Bool {
    pub const TRUE: ConstructorId = ConstructorId(0x997275b5);
    pub const FALSE: ConstructorId = ConstructorId(0xbc799737);
}

impl Type for Bool {
    fn bare_type() -> bool {
        false
    }

    fn type_id(&self) -> Option<ConstructorId> {
        if self.0 {
            Some(Bool::TRUE)
        } else {
            Some(Bool::FALSE)
        }
    }

    fn serialize<W: Write>(&self, _: &mut WriteContext<W>) -> tl::Result<()> {
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> tl::Result<Self> {
        Err(tl::Error::BoxedAsBare)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, _: &mut R) -> tl::Result<Self> {
        match id {
            Bool::TRUE => Ok(Bool(true)),
            Bool::FALSE => Ok(Bool(false)),
            _ => Err(tl::Error::InvalidData),
        }
    }
}
