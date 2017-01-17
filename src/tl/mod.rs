use std;
use std::io::{Read, Write};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

pub use self::error::{Error, Result};
#[doc(inline)]
pub use self::bool_type::Bool;
#[doc(inline)]
pub use self::true_type::True;
#[doc(inline)]
pub use self::null_type::Null;
#[doc(inline)]
pub use self::vector::{Vector, SendSlice};

pub mod error;
pub mod parsing;
pub mod complex_types;
pub mod dynamic;

mod bool_type;
mod true_type;
mod null_type;
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
impl_tl_primitive! { u32, read_u32, write_u32 }
impl_tl_primitive! { i64, read_i64, write_i64 }
impl_tl_primitive! { u64, read_u64, write_u64 }
impl_tl_primitive! { f32, read_f32, write_f32 }
impl_tl_primitive! { f64, read_f64, write_f64 }

const VEC_TYPE_ID: ConstructorId = ConstructorId(0x1cb5c415);

impl<'a, T: Type> Type for &'a [T] {
    #[inline]
    fn bare_type() -> bool {
        false
    }

    #[inline]
    fn type_id(&self) -> Option<ConstructorId> {
        Some(VEC_TYPE_ID)
    }

    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        assert!(self.len() <= std::u32::MAX as usize);
        try!(writer.write_u32::<LittleEndian>(self.len() as u32));
        for item in *self {
            try!(writer.write_generic(item));
        }
        Ok(())
    }

    fn deserialize<R: Read>(_: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }

    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }
}

impl<T: Type> Type for Vec<T> {
    #[inline]
    fn bare_type() -> bool {
        false
    }

    #[inline]
    fn type_id(&self) -> Option<ConstructorId> {
        Some(VEC_TYPE_ID)
    }

    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> Result<Self> {
        let mut vec = vec![];
        let count = try!(reader.read_u32::<LittleEndian>()) as usize;
        vec.reserve(count);
        for _ in 0..count {
            vec.push(try!(reader.read_generic()));
        }
        Ok(vec)
    }

    fn deserialize_boxed<R: Read>(id: ConstructorId, reader: &mut ReadContext<R>) -> Result<Self> {
        if id != VEC_TYPE_ID {
            return Err(Error::InvalidData);
        }

        Vec::deserialize(reader)
    }
}

impl<'a> Type for &'a [u8] {
    #[inline]
    fn bare_type() -> bool {
        true
    }

    #[inline]
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        let len = self.len();
        assert!(len & 0xFF000000 == 0); // len fits in a 24-bit integer

        // Writing string length is WAT
        let mut to_write = *self;
        let first_word = if len <= 253 {
            let mut ret = len as u32;
            for (e, &b) in self.iter().take(3).enumerate() {
                ret |= (b as u32) << ((e + 1) * 8);
            }
            if len > 3 {
                to_write = &to_write[3..];
            } else {
                to_write = b"";
            }
            ret
        } else {
            254 | ((len as u32) << 8)
        };
        writer.write_u32::<::byteorder::BigEndian>(first_word)?;
        println!("fw {:x} {:?}", first_word, to_write);

        // Write the actual string and padding
        if to_write.len() > 4 {
            let write_len = to_write.len() & !3;
            writer.write_all(&to_write[..write_len])?;
            to_write = &to_write[write_len..];
        }
        if !to_write.is_empty() {
            let mut last_word = 0u32;
            for (e, &b) in to_write.into_iter().enumerate() {
                last_word |= (b as u32) << (e * 8);
            }
            writer.write_u32::<::byteorder::BigEndian>(last_word)?;
            println!("lw {:x} {:?}", last_word, to_write);
        }

        Ok(())
    }

    fn deserialize<R: Read>(_: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }

    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> Result<Self> {
        Err(Error::ReceivedSendType)
    }
}

impl Type for Vec<u8> {
    #[inline]
    fn bare_type() -> bool {
        true
    }

    #[inline]
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

impl Type for String {
    #[inline]
    fn bare_type() -> bool {
        true
    }

    #[inline]
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
    #[inline]
    fn bare_type() -> bool {
        true
    }

    #[inline]
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
    #[inline]
    fn bare_type() -> bool {
        Bool::bare_type()
    }

    #[inline]
    fn type_id(&self) -> Option<ConstructorId> {
        Bool(*self).type_id()
    }

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
