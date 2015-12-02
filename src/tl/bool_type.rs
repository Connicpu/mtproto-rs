use tl::{self, Polymorphic, Type};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bool(pub bool);

impl Bool {
    const TRUE: ConstructorId = ConstructorId(0xbc799737);
    const FALSE: ConstructorId = ConstructorId(0x997275b5);
}

impl Polymorphic for Bool {
    fn type_id(&self) -> Option<ConstructorId> {
        if self.0 {
            Some(Bool::TRUE)
        } else {
            Some(Bool::FALSE)
        }
    }
    
    fn serialize(&self, _: &mut WriteContext) -> tl::Result<()> {
        Ok(())
    }
}

impl Type for Bool {
    fn bare_type() -> bool {
        false
    }
    
    fn deserialize(reader: &mut ReadContext) -> tl::Result<Self> {
        Err(tl::Error::BoxedAsBare)
    }
    
    fn deserialize_boxed(id: ConstructorId, _: &mut ReadContext) -> tl::Result<Self> {
        match id {
            Bool::TRUE => Ok(Bool(true)),
            Bool::FALSE => Ok(Bool(false)),
            _ => Err(tl::Error::InvalidData),
        }
    }
}

