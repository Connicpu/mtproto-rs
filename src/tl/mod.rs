use std;
use std::io::{Read, Write};
use tl::parsing::{ConstructorId, Reader, Writer};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, TimeZone, UTC};

pub use super::error::{Error, ErrorKind, Result};
#[doc(inline)]
pub use self::bool_type::Bool;
#[doc(inline)]
pub use self::true_type::True;
#[doc(inline)]
pub use self::null_type::Null;
#[doc(inline)]
pub use self::vector::{Vector, BareVector, SendSlice};

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
    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()>;
    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self>;
    fn deserialize_boxed<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Self>;
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

            fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
                use byteorder::{LittleEndian, WriteBytesExt};
                try!(writer.$write::<LittleEndian>(*self));
                Ok(())
            }

            fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
                use byteorder::{LittleEndian, ReadBytesExt};
                Ok(try!(reader.$read::<LittleEndian>()))
            }

            fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
                Err(ErrorKind::PrimitiveAsPolymorphic.into())
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

macro_rules! impl_tl_tuple {
    ($($i:ident : $t:ident),*) => {
        impl<
            $( $t : Type ),*
            > Type for ($($t),*)
        {
            fn bare_type() -> bool {
                true
            }

            fn type_id(&self) -> Option<ConstructorId> {
                None
            }

            fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
                let &($(ref $i),*) = self;
                $(
                    try!(writer.write_bare($i));
                )*
                Ok(())
            }

            fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
                Ok(( $(
                    try!(reader.read_bare::<$t>()),
                )* ))
            }

            fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
                Err(ErrorKind::PrimitiveAsPolymorphic.into())
            }
        }
    };
}

impl_tl_tuple! { a: A, b: B }

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

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        assert!(self.len() <= std::u32::MAX as usize);
        try!(writer.write_u32::<LittleEndian>(self.len() as u32));
        for item in *self {
            try!(writer.write_generic(item));
        }
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> Result<Self> {
        Err(ErrorKind::ReceivedSendType.into())
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Err(ErrorKind::ReceivedSendType.into())
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

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        let mut vec = vec![];
        let count = try!(reader.read_u32::<LittleEndian>()) as usize;
        vec.reserve(count);
        for _ in 0..count {
            vec.push(try!(reader.read_generic()));
        }
        Ok(vec)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Self> {
        if id != VEC_TYPE_ID {
            return Err(ErrorKind::InvalidData.into());
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

    fn serialize<W: Writer>(&self, writer_: &mut W) -> Result<()> {
        let mut writer = writer_.aligned(4);
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

        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> Result<Self> {
        Err(ErrorKind::ReceivedSendType.into())
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Err(ErrorKind::ReceivedSendType.into())
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

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize<R: Reader>(reader_: &mut R) -> Result<Self> {
        let mut reader = reader_.aligned(4);
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

        Ok(bytes)
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Err(ErrorKind::PrimitiveAsPolymorphic.into())
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

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        let bytes = try!(reader.read_bare());
        Ok(try!(String::from_utf8(bytes)))
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Err(ErrorKind::PrimitiveAsPolymorphic.into())
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

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        try!(writer.write_bare(&self.as_bytes()));
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> Result<Self> {
        Err(ErrorKind::ReceivedSendType.into())
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Err(ErrorKind::ReceivedSendType.into())
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

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        Bool(*self).serialize(writer)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Ok(try!(Bool::deserialize(reader)).0)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Self> {
        Ok(try!(Bool::deserialize_boxed(id, reader)).0)
    }
}

impl Type for () {
    fn bare_type() -> bool {
        true
    }

    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Writer>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> Result<Self> {
        Ok(())
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Ok(())
    }
}

impl<T: Type> Type for Option<T> {
    fn bare_type() -> bool {
        T::bare_type()
    }

    fn type_id(&self) -> Option<ConstructorId> {
        self.as_ref().and_then(T::type_id)
    }

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        match self {
            &Some(ref inner) => T::serialize(inner, writer),
            &None => Ok(()),
        }
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        T::deserialize(reader).map(Some)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Self> {
        T::deserialize_boxed(id, reader).map(Some)
    }
}

impl<T: Type> Type for Box<T> {
    fn bare_type() -> bool {
        T::bare_type()
    }

    fn type_id(&self) -> Option<ConstructorId> {
        T::type_id(&*self)
    }

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        T::serialize(&*self, writer)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        T::deserialize(reader).map(Box::new)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, reader: &mut R) -> Result<Self> {
        T::deserialize_boxed(id, reader).map(Box::new)
    }
}

impl Type for DateTime<UTC> {
    fn bare_type() -> bool {
        true
    }

    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Writer>(&self, writer: &mut W) -> Result<()> {
        (self.timestamp() as u32).serialize(writer)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        let ts = u32::deserialize(reader)?;
        Ok(UTC.timestamp(ts as i64, 0))
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> Result<Self> {
        Err(ErrorKind::PrimitiveAsPolymorphic.into())
    }
}
