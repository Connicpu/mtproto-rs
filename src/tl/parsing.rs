use std::io::{self, Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use tl::dynamic::{TLCtorMap, TLObject};
use tl;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstructorId(pub u32);

pub trait Reader: Read {
    fn read_boxed<T: tl::Type>(&mut self) -> tl::Result<T>;
    fn read_bare<T: tl::Type>(&mut self) -> tl::Result<T>;
    fn read_polymorphic(&mut self) -> tl::Result<Box<TLObject>>;

    fn read_generic<T: tl::Type>(&mut self) -> tl::Result<T> {
        if T::bare_type() {
            self.read_bare()
        } else {
            self.read_boxed()
        }
    }
}

pub trait Writer: Write {
    fn write_boxed<T: tl::Type>(&mut self, value: &T) -> tl::Result<()>;
    fn write_bare<T: tl::Type>(&mut self, value: &T) -> tl::Result<()>;

    fn write_generic<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        if T::bare_type() {
            self.write_bare(value)
        } else {
            self.write_boxed(value)
        }
    }
}

pub struct ReadContext<R: Read> {
    stream: R,
    position: u64,
    ctors: Option<TLCtorMap<ReadContext<R>>>,
}

pub struct WriteContext<W: Write> {
    stream: W,
    position: u64,
}

impl<R: Read> ReadContext<R> {
    pub fn new(reader: R) -> Self {
        ReadContext {
            stream: reader,
            position: 0,
            ctors: None,
        }
    }

    pub fn set_ctors(&mut self, ctors: TLCtorMap<ReadContext<R>>) {
        self.ctors = Some(ctors);
    }
}

impl<R: Read> Reader for ReadContext<R> {
    fn read_boxed<T: tl::Type>(&mut self) -> tl::Result<T> {
        let con_id = ConstructorId(try!(self.read_u32::<LittleEndian>()));
        T::deserialize_boxed(con_id, self)
    }

    fn read_bare<T: tl::Type>(&mut self) -> tl::Result<T> {
        assert!(T::bare_type());
        T::deserialize(self)
    }

    fn read_polymorphic(&mut self) -> tl::Result<Box<TLObject>> {
        let id: ConstructorId = self.read_bare()?;
        let ctor = match self.ctors.as_ref().and_then(|m| m.0.get(&id)) {
            Some(ctor) => ctor.0,
            None => return Err(super::Error::UnknownType),
        };
        ctor(id, self)
    }
}

impl<R: Read> Read for ReadContext<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.stream.read(buf);
        if let Ok(len) = result {
            self.position += len as u64;
        }
        result
    }
}

impl<R: Read> tl::ReadHelpers for ReadContext<R> {
    fn align(&mut self, alignment: u8) -> tl::Result<()> {
        let stub = (self.position % alignment as u64) as usize;
        if stub != 0 {
            let mut buf: [u8; 256] = [0; 256];
            let remaining = alignment as usize - stub;
            try!(self.read_exact(&mut buf[0..remaining]));
        }
        Ok(())
    }
}

impl<W: Write> WriteContext<W> {
    pub fn new(writer: W) -> Self {
        WriteContext {
            stream: writer,
            position: 0,
        }
    }
}

impl<W: Write> Writer for WriteContext<W> {
    fn write_boxed<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        let con_id = value.type_id().unwrap();
        try!(self.write_u32::<LittleEndian>(con_id.0));
        value.serialize(self)
    }

    fn write_bare<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        assert!(T::bare_type());
        value.serialize(self)
    }
}

impl<W: Write> Write for WriteContext<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.stream.write(buf);
        if let Ok(len) = result {
            self.position += len as u64;
        }
        result
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<W: Write> tl::WriteHelpers for WriteContext<W> {
    fn pad(&mut self, alignment: u8) -> tl::Result<()> {
        let stub = (self.position % alignment as u64) as usize;
        if stub != 0 {
            let buf: [u8; 256] = [0; 256];
            let remaining = alignment as usize - stub;
            try!(self.write_all(&buf[0..remaining]));
        }
        Ok(())
    }
}

impl tl::Type for ConstructorId {
    fn bare_type() -> bool {
        true
    }

    fn type_id(&self) -> Option<ConstructorId> {
        None
    }

    fn serialize<W: Writer>(&self, writer: &mut W) -> tl::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        try!(writer.write_u32::<LittleEndian>(self.0));
        Ok(())
    }

    fn deserialize<R: Reader>(reader: &mut R) -> tl::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(ConstructorId(try!(reader.read_u32::<LittleEndian>())))
    }

    fn deserialize_boxed<R: Reader>(_: ConstructorId, _: &mut R) -> tl::Result<Self> {
        Err(tl::Error::PrimitiveAsPolymorphic)
    }
}
