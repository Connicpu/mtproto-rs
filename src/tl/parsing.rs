use std::fmt;
use std::cmp::min;
use std::io::{self, Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use tl::dynamic::{TLCtorMap, TLObject, UnreadableBag};
use tl;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorId(pub u32);

impl fmt::Debug for ConstructorId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:08x}", self.0)
    }
}

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

    fn aligned<'a>(&'a mut self, alignment: usize) -> AlignedReader<'a, Self> {
        AlignedReader {
            reader: self,
            alignment: alignment,
            position: 0,
        }
    }

    fn take<'a>(&'a mut self, limit: usize) -> TakeReader<'a, Self> {
        TakeReader {
            reader: self,
            limit: limit,
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

    fn aligned<'a>(&'a mut self, alignment: usize) -> AlignedWriter<'a, Self> {
        AlignedWriter {
            writer: self,
            alignment: alignment,
            position: 0,
        }
    }
}

pub struct ReadContext<R: Read> {
    stream: R,
    ctors: Option<TLCtorMap<ReadContext<R>>>,
}

pub struct AlignedReader<'a, R: 'a + ?Sized + Reader> {
    reader: &'a mut R,
    alignment: usize,
    position: usize,
}

pub struct TakeReader<'a, R: 'a + ?Sized + Reader> {
    reader: &'a mut R,
    limit: usize,
}

pub struct WriteContext<W: Write> {
    stream: W,
}

pub struct AlignedWriter<'a, W: 'a + ?Sized + Writer> {
    writer: &'a mut W,
    alignment: usize,
    position: usize,
}

impl<R: Read> ReadContext<R> {
    pub fn new(reader: R) -> Self {
        ReadContext {
            stream: reader,
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
        match self.ctors.as_ref().and_then(|m| m.0.get(&id)).map(|c| c.0) {
            Some(ctor) => ctor(id, self),
            None => {
                let mut remainder = vec![];
                self.read_to_end(&mut remainder)?;
                Ok(Box::new(UnreadableBag {
                    tl_id: id,
                    bytes: remainder,
                }))
            },
        }
    }
}

impl<R: Read> Read for ReadContext<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl<'a, R: ?Sized + Reader> Reader for AlignedReader<'a, R> {
    fn read_boxed<T: tl::Type>(&mut self) -> tl::Result<T> {
        self.reader.read_boxed()
    }

    fn read_bare<T: tl::Type>(&mut self) -> tl::Result<T> {
        self.reader.read_bare()
    }

    fn read_polymorphic(&mut self) -> tl::Result<Box<TLObject>> {
        self.reader.read_polymorphic()
    }
}


impl<'a, R: ?Sized + Reader> Read for AlignedReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.reader.read(buf)?;
        self.position += read;
        Ok(read)
    }
}

impl<'a, R: ?Sized + Reader> Drop for AlignedReader<'a, R> {
    fn drop(&mut self) {
        let remainder = self.position % self.alignment;
        if remainder != 0 {
            let mut buf = [0u8; 256];
            let pad = self.alignment - remainder;
            self.read_exact(&mut buf[..pad]).expect("couldn't pad")
        }
    }
}

impl<'a, R: ?Sized + Reader> Reader for TakeReader<'a, R> {
    fn read_boxed<T: tl::Type>(&mut self) -> tl::Result<T> {
        self.reader.read_boxed()
    }

    fn read_bare<T: tl::Type>(&mut self) -> tl::Result<T> {
        self.reader.read_bare()
    }

    fn read_polymorphic(&mut self) -> tl::Result<Box<TLObject>> {
        self.reader.read_polymorphic()
    }
}


impl<'a, R: ?Sized + Reader> Read for TakeReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        let max = min(buf.len(), self.limit);
        let read = self.reader.read(&mut buf[..max])?;
        self.limit -= read;
        Ok(read)
    }
}

impl<W: Write> WriteContext<W> {
    pub fn new(writer: W) -> Self {
        WriteContext {
            stream: writer,
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
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<'a, W: ?Sized + Writer> Write for AlignedWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.writer.write(buf)?;
        self.position += written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<'a, W: ?Sized + Writer> Drop for AlignedWriter<'a, W> {
    fn drop(&mut self) {
        let remainder = self.position % self.alignment;
        if remainder != 0 {
            let buf = [0u8; 256];
            let pad = self.alignment - remainder;
            self.write_all(&buf[..pad]).expect("couldn't pad");
        }
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
