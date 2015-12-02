use std::io::{Read, Write};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

pub use self::error::{Error, Result};
#[doc(inline)]
pub use self::bool_type::Bool;
#[doc(inline)]
pub use self::true_type::True;
#[doc(inline)]
pub use self::vector::{Vector, SendSlice};
#[doc(inline)]
pub use self::string::{String, SendStr};

pub mod error;
pub mod parsing;
pub mod complex_types;

mod bool_type;
mod true_type;
mod vector;
mod string;

pub trait Type: Sized {
    fn bare_type() -> bool;
    fn type_id(&self) -> Option<ConstructorId>;
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()>;
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> Result<Self>;
    fn deserialize_boxed<R: Read>(id: ConstructorId, reader: &mut ReadContext<R>) -> Result<Self>;
}

trait ReadHelpers {
    fn align(&mut self, alignment: u8) -> Result<()>;
}

trait WriteHelpers {
    fn pad(&mut self, alignment: u8) -> Result<()>;
}

macro_rules! impl_tl_primitive {
    ($ptype:ident, $read:ident, $write:ident) => {
        impl Type for $ptype {
            fn bare_type() -> bool {
                true
            }
            
            fn type_id(&self) -> Option<ConstructorId> {
                None
            }
            
            fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
                use byteorder::{LittleEndian, WriteBytesExt};
                try!(writer.$write::<LittleEndian>(*self));
                Ok(())
            }
            
            fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> Result<Self> {
                use byteorder::{LittleEndian, ReadBytesExt};
                Ok(try!(reader.$read::<LittleEndian>()))
            }
            
            fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
                Err(Error::PrimitiveAsPolymorphic)
            }
        }
    }
}

impl_tl_primitive! { i32, read_i32, write_i32 }
impl_tl_primitive! { i64, read_i64, write_i64 }
impl_tl_primitive! { f64, read_f64, write_f64 }

