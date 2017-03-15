use std;
use std::io::{self, Read, Write};
use tl::parsing::{ConstructorId, Reader, Writer};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use schema::Bool;

pub use super::error::{Error, ErrorKind, Result};

pub mod parsing;
pub mod dynamic;

/// The API version we've implemented against
pub const MTPROTO_LAYER: i32 = 62;

pub trait IdentifiableType {
    fn type_id(&self) -> Option<ConstructorId> { None }
}

pub trait WriteType: IdentifiableType {
    fn serialize(&self, writer: &mut Writer) -> Result<()>;
}

pub trait ReadType: Sized {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self>;

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Self::deserialize_bare(Some(reader.read_type_id()?), reader)
    }
}

fn ensure_type_id(expected: Option<ConstructorId>, actual: Option<ConstructorId>) -> Result<()> {
    match (expected, actual) {
        (_, None) => Ok(()),
        (Some(ref a), Some(ref b)) if a == b => Ok(()),
        _ => Err(ErrorKind::InvalidType(expected.into_iter().collect(), actual).into()),
    }
}

macro_rules! impl_tl_primitive {
    ($ptype:ident, $read:ident, $write:ident) => {
        impl IdentifiableType for $ptype {}
        impl WriteType for $ptype {
            fn serialize(&self, writer: &mut Writer) -> Result<()> {
                use byteorder::{LittleEndian, WriteBytesExt};
                writer.$write::<LittleEndian>(*self)?;
                Ok(())
            }
        }

        impl ReadType for $ptype {
            fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
                ensure_type_id(None, id)?;
                use byteorder::{LittleEndian, ReadBytesExt};
                Ok(reader.$read::<LittleEndian>()?)
            }

            fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
                Self::deserialize_bare(None, reader)
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
        impl< $( $t : IdentifiableType ),* > IdentifiableType for ($($t),*) {}
        impl< $( $t : WriteType ),* > WriteType for ($($t),*) {
            fn serialize(&self, writer: &mut Writer) -> Result<()> {
                let &($(ref $i),*) = self;
                $( writer.write_tl($i)?; )*
                Ok(())
            }
        }

        impl< $( $t : ReadType ),* > ReadType for ($($t),*) {
            fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
                ensure_type_id(None, id)?;
                Ok(( $( reader.read_tl::<$t>()?, )* ))
            }

            fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
                Self::deserialize_bare(None, reader)
            }
        }
    };
}

impl_tl_tuple! { a: A, b: B }

pub const VEC_TYPE_ID: ConstructorId = ConstructorId(0x1cb5c415);

impl<'a, T: IdentifiableType> IdentifiableType for &'a [T] {
    fn type_id(&self) -> Option<ConstructorId> {
        Some(VEC_TYPE_ID)
    }
}

impl<'a, T: WriteType> WriteType for &'a [T] {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        assert!(self.len() <= std::u32::MAX as usize);
        writer.write_u32::<LittleEndian>(self.len() as u32)?;
        for item in *self {
            writer.write_tl(item)?;
        }
        Ok(())
    }
}

impl<T: IdentifiableType> IdentifiableType for Vec<T> {
    fn type_id(&self) -> Option<ConstructorId> {
        Some(VEC_TYPE_ID)
    }
}

impl<T: WriteType> WriteType for Vec<T> {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        (&self[..]).serialize(writer)
    }
}

impl<T: ReadType> ReadType for Vec<T> {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
        ensure_type_id(Some(VEC_TYPE_ID), id)?;
        let mut vec = vec![];
        let count = reader.read_u32::<LittleEndian>()? as usize;
        vec.reserve(count);
        for _ in 0..count {
            vec.push(reader.read_tl()?);
        }
        Ok(vec)
    }
}

impl<'a> IdentifiableType for &'a [u8] {}

impl<'a> WriteType for &'a [u8] {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        let mut writer = writer.aligned(4);
        let len = self.len();
        assert!(len & 0xFF000000 == 0); // len fits in a 24-bit integer

        // Writing string length is WAT
        if len <= 253 {
            writer.write_u8(len as u8)?;
        } else {
            writer.write_u32::<LittleEndian>((len as u32) << 8 | 254)?;
        }

        // Write the actual string and padding
        writer.write_all(*self)?;

        Ok(())
    }
}

impl IdentifiableType for Vec<u8> {}

impl WriteType for Vec<u8> {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        (&self[..]).serialize(writer)
    }
}

impl ReadType for Vec<u8> {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader_: &mut R) -> Result<Self> {
        ensure_type_id(None, id)?;
        let mut reader = reader_.aligned(4);
        let low_len = reader.read_u8()?;
        let len = if low_len != 254 {
            low_len as usize
        } else {
            let mut buf = [0; 4];
            reader.read_exact(&mut buf[0..3])?;
            LittleEndian::read_u32(&buf) as usize
        };

        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes)?;

        Ok(bytes)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Self::deserialize_bare(None, reader)
    }
}

impl IdentifiableType for String {}

impl WriteType for String {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        self.as_bytes().serialize(writer)
    }
}

impl ReadType for String {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
        ensure_type_id(None, id)?;
        let bytes = reader.read_tl()?;
        Ok(String::from_utf8(bytes)?)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Self::deserialize_bare(None, reader)
    }
}

impl<T: IdentifiableType> IdentifiableType for Box<T> {
    fn type_id(&self) -> Option<ConstructorId> {
        T::type_id(&*self)
    }
}

impl<T: WriteType> WriteType for Box<T> {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        T::serialize(&*self, writer)
    }
}

impl<T: ReadType> ReadType for Box<T> {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
        T::deserialize_bare(id, reader).map(Box::new)
    }
}

impl IdentifiableType for () {}

impl WriteType for () {
    fn serialize(&self, _: &mut Writer) -> Result<()> {
        Ok(())
    }
}

impl ReadType for () {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, _: &mut R) -> Result<Self> {
        ensure_type_id(None, id)?;
        Ok(())
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Self::deserialize_bare(None, reader)
    }
}

impl From<bool> for Bool {
    fn from(b: bool) -> Bool {
        if b { Bool::boolTrue } else { Bool::boolFalse }
    }
}

impl Into<bool> for Bool {
    fn into(self) -> bool {
        match self {
            Bool::boolTrue => true,
            Bool::boolFalse => false,
        }
    }
}

impl IdentifiableType for bool {
    fn type_id(&self) -> Option<ConstructorId> {
        Into::<Bool>::into(*self).type_id()
    }
}

impl WriteType for bool {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        Into::<Bool>::into(*self).serialize(writer)
    }
}

impl ReadType for bool {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
        Bool::deserialize_bare(id, reader).map(Into::into)
    }
}

#[derive(Debug, Clone)]
pub struct Bare<T>(pub T);

impl<T> IdentifiableType for Bare<Vec<T>> {}

impl<T: WriteType> WriteType for Bare<Vec<T>> {
    fn serialize(&self, writer: &mut Writer) -> Result<()> {
        writer.write_tl(&(self.0.len() as u32))?;
        for item in &self.0 {
            item.serialize(writer)?;
        }
        Ok(())
    }
}

impl<T: ReadType> ReadType for Bare<Vec<T>> {
    fn deserialize_bare<R: Reader>(id: Option<ConstructorId>, reader: &mut R) -> Result<Self> {
        ensure_type_id(None, id)?;
        let count: u32 = reader.read_tl()?;
        let vec = (0..count).into_iter()
            .map(|_| T::deserialize_bare(None, reader))
            .collect::<Result<Vec<T>>>()?;
        Ok(Bare(vec))
    }

    fn deserialize<R: Reader>(reader: &mut R) -> Result<Self> {
        Self::deserialize_bare(None, reader)
    }
}

pub fn serialize_message<M>(msg: M) -> Result<Vec<u8>>
    where M: WriteType,
{
    let mut buf = io::Cursor::new(Vec::<u8>::new());
    parsing::WriteContext::new(&mut buf).write_tl(&msg)?;
    Ok(buf.into_inner())
}
