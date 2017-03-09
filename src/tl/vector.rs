use std;
use tl::{self, Type};
use tl::parsing::{ConstructorId, Reader, Writer};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug)]
pub struct Vector<T: Type> {
    pub elements: Vec<T>,
}

#[derive(Debug, Clone)]
pub struct Bare<T>(pub T);

pub struct SendSlice<'a, T: Type + 'a> {
    pub elements: &'a [T],
}

const TYPE_ID: ConstructorId = ConstructorId(0x1cb5c415);

impl<T: Type> Vector<T> {
    pub fn new() -> Vector<T> {
        Vector {
            elements: vec![],
        }
    }

    pub fn from_elements(vec: Vec<T>) -> Vector<T> {
        Vector {
            elements: vec,
        }
    }
}

impl<T: Type> Type for Vector<T> {
    fn bare_type() -> bool {
        false
    }

    fn type_id(&self) -> Option<ConstructorId> {
        Some(TYPE_ID)
    }

    fn serialize<W: Writer>(&self, writer: &mut W) -> tl::Result<()> {
        SendSlice::from_elements(&self.elements).serialize(writer)
    }

    fn deserialize<R: Reader>(reader: &mut R) -> tl::Result<Self> {
        let mut vec = Vector { elements: vec![] };
        let count = try!(reader.read_u32::<LittleEndian>()) as usize;
        for _ in 0..count {
            vec.elements.push(try!(reader.read_generic()));
        }
        Ok(vec)
    }

    fn deserialize_boxed<R: Reader>(id: ConstructorId, reader: &mut R) -> tl::Result<Self> {
        if id != TYPE_ID {
            return Err(::error::ErrorKind::InvalidData.into());
        }

        Vector::deserialize(reader)
    }
}

impl<T: Type> From<Vec<T>> for Vector<T> {
    fn from(v: Vec<T>) -> Self {
        Vector::from_elements(v)
    }
}

impl<T: Type> Bare<Vec<T>> {
    pub fn new() -> Self {
        Bare(vec![])
    }
}

impl<T: Type> Type for Bare<Vec<T>> {
    fn bare_type() -> bool {
        true
    }

    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Writer>(&self, _: &mut W) -> tl::Result<()> {
        unimplemented!()
    }

    fn deserialize<R: Reader>(reader: &mut R) -> tl::Result<Self> {
        let count: u32 = reader.read_bare()?;
        let vec = (0..count).into_iter()
            .map(|_| reader.read_bare())
            .collect::<tl::Result<Vec<T>>>()?;
        Ok(Bare(vec))
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> tl::Result<Self> {
        Err(::error::ErrorKind::PrimitiveAsPolymorphic.into())
    }
}

impl<'a, T: Type + 'a> SendSlice<'a, T> {
    pub fn from_elements(elements: &'a [T]) -> SendSlice<T> {
        SendSlice {
            elements: elements,
        }
    }
}

impl<'a, T: Type + 'a> Type for SendSlice<'a, T> {
    fn bare_type() -> bool {
        false
    }

    fn type_id(&self) -> Option<ConstructorId> {
        Some(TYPE_ID)
    }

    fn serialize<W: Writer>(&self, writer: &mut W) -> tl::Result<()> {
        assert!(self.elements.len() <= std::u32::MAX as usize);
        try!(writer.write_u32::<LittleEndian>(self.elements.len() as u32));
        for item in self.elements {
            try!(writer.write_generic(item));
        }
        Ok(())
    }

    fn deserialize<R: Reader>(_: &mut R) -> tl::Result<Self> {
        Err(::error::ErrorKind::ReceivedSendType.into())
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> tl::Result<Self> {
        Err(::error::ErrorKind::ReceivedSendType.into())
    }
}
