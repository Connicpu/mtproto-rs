use std::any::Any;
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

pub use self::error::{Error, Result};
#[doc(inline)]
pub use self::bool_type::Bool;

mod bool_type;
pub mod error;
pub mod parsing;

pub trait Polymorphic: Any {
    fn type_id(&self) -> Option<ConstructorId>;
    fn serialize(&self, writer: &mut WriteContext) -> Result<()>;
}

pub trait Type: Polymorphic + Sized {
    fn bare_type() -> bool;
    fn deserialize(reader: &mut ReadContext) -> Result<Self>;
    fn deserialize_boxed(id: ConstructorId, &mut ReadContext) -> Result<Self>;
}

trait ReadHelpers {
    fn align(&mut self, alignment: u8) -> Result<()>;
}

trait WriteHelpers {
    fn pad(&mut self, alignment: u8) -> Result<()>;
}

macro_rules! impl_tl_primitive {
    ($ptype:ident, $read:ident, $write:ident) => {
        impl Polymorphic for $ptype {
            fn type_id(&self) -> Option<ConstructorId> {
                None
            }
            
            fn serialize(&self, writer: &mut WriteContext) -> Result<()> {
                use byteorder::{LittleEndian, WriteBytesExt};
                try!(writer.$write::<LittleEndian>(*self));
                Ok(())
            }
        }
        
        impl Type for $ptype {
            fn bare_type() -> bool {
                true
            }
            
            fn deserialize(reader: &mut ReadContext) -> Result<Self> {
                use byteorder::{LittleEndian, ReadBytesExt};
                Ok(try!(reader.$read::<LittleEndian>()))
            }
            
            fn deserialize_boxed(_: ConstructorId, _: &mut ReadContext) -> Result<Self> {
                Err(Error::PrimitiveAsPolymorphic)
            }
        }
    }
}

impl_tl_primitive! { i32, read_i32, write_i32 }
impl_tl_primitive! { i64, read_i64, write_i64 }
impl_tl_primitive! { f64, read_f64, write_f64 }

