use std::io::{Read, Write};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

pub use self::error::{Error, Result};
#[doc(inline)]
pub use self::bool_type::Bool;
#[doc(inline)]
pub use self::true_type::True;
#[doc(inline)]
pub use self::vector::{Vector, SendSlice};

pub mod error;
pub mod parsing;
pub mod complex_types;

mod bool_type;
mod true_type;
mod vector;

/// The API version we've implemented against
pub const MTPROTO_LAYER: u32 = 23;

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

impl<'a> Type for &'a [u8] {
    fn bare_type() -> bool {
        true
    }
    
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        let len = self.len();
        assert!(len & 0xFF000000 == 0); // len fits in a 24-bit integer
        
        // Writing string length is WAT
        if len <= 253 {
            try!(writer.write_u8(len as u8));
        } else {
            let mut buf = [254; 5];
            LittleEndian::write_u32(&mut buf[1..], len as u32);
            try!(writer.write_all(&buf[0..4]));
        }
        
        // Write the actual string and padding
        try!(writer.write_all(*self));
        try!(writer.pad(4));
        
        Ok(())
    }
    
    fn deserialize<R: Read>(_: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }
}

impl<'a> Type for Vec<u8> {
    fn bare_type() -> bool {
        true
    }
    
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        (&self[..]).serialize(writer)
    }
    
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> Result<Self> {
        let low_len = try!(reader.read_u8());
        let len = if low_len != 254 {
            low_len as usize
        } else {
            let mut buf = [0; 4];
            try!(reader.read_exact(&mut buf[0..3]));
            LittleEndian::read_u32(&buf) as usize
        };
        
        let mut bytes = vec![0; len];
        try!(reader.read_exact(&mut bytes));
        try!(reader.align(4));
        
        Ok(bytes)
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::PrimitiveAsPolymorphic)
    }
}

impl<'a> Type for String {
    fn bare_type() -> bool {
        true
    }
    
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        (&self[..]).serialize(writer)
    }
    
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> Result<Self> {
        let bytes = try!(reader.read_bare());
        Ok(try!(String::from_utf8(bytes)))
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::PrimitiveAsPolymorphic)
    }
}

impl<'a> Type for &'a str {
    fn bare_type() -> bool {
        true
    }
    
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        try!(writer.write_bare(&self.as_bytes()));
        Ok(())
    }
    
    fn deserialize<R: Read>(_: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }
}

impl Type for bool {
    fn bare_type() -> bool { Bool::bare_type() }
    fn type_id(&self) -> Option<ConstructorId> { Bool(*self).type_id() }
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        Bool(*self).serialize(writer)
    }
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> Result<Self> {
        Ok(try!(Bool::deserialize(reader)).0)
    }
    fn deserialize_boxed<R: Read>(id: ConstructorId, reader: &mut ReadContext<R>) -> Result<Self> {
        Ok(try!(Bool::deserialize_boxed(id, reader)).0)
    }
}
