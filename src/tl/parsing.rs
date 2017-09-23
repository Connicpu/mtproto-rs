use std::fmt;
//use std::cmp::min;
//use std::io::{self, Read, Write};
//use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

//use error;
//use tl::dynamic::{TLCtorMap, TLObject};
//use tl;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorId(pub u32);

impl fmt::Debug for ConstructorId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:08x}", self.0)
    }
}

/*pub trait Reader: Read {
    fn read_tl<T: tl::ReadType>(&mut self) -> error::Result<T>;
    fn read_polymorphic(&mut self) -> error::Result<Box<TLObject>>;

    fn read_type_id(&mut self) -> error::Result<ConstructorId> {
        Ok(ConstructorId(self.read_u32::<LittleEndian>()?))
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
    fn write_tl(&mut self, value: &tl::WriteType) -> error::Result<()>;

    fn aligned(self, alignment: usize) -> AlignedWriter<Self>
        where Self: Sized
    {
        AlignedWriter {
            writer: self,
            alignment: alignment,
            position: 0,
        }
    }
}

/*pub struct ReadContext<R: Read> {
    stream: R,
    ctors: Option<TLCtorMap<ReadContext<R>>>,
}*/

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

pub struct AlignedWriter<W: Writer>{
    writer: W,
    alignment: usize,
    position: usize,
}

/*impl<R: Read> ReadContext<R> {
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
    fn read_tl<T: tl::ReadType>(&mut self) -> error::Result<T> {
        T::deserialize(self)
    }

    fn read_polymorphic(&mut self) -> error::Result<Box<TLObject>> {
        let id = self.read_type_id()?;
        let ctor = self.ctors.as_ref().unwrap().get(&id).0;
        ctor(Some(id), self)
    }
}

impl<R: Read> Read for ReadContext<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}*/

impl<'a, R: ?Sized + Reader> Reader for AlignedReader<'a, R> {
    fn read_tl<T: tl::ReadType>(&mut self) -> error::Result<T> {
        self.reader.read_tl()
    }

    fn read_polymorphic(&mut self) -> error::Result<Box<TLObject>> {
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
    fn read_tl<T: tl::ReadType>(&mut self) -> error::Result<T> {
        self.reader.read_tl()
    }

    fn read_polymorphic(&mut self) -> error::Result<Box<TLObject>> {
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
    fn write_tl(&mut self, value: &tl::WriteType) -> error::Result<()> {
        if let Some(con_id) = value.type_id() {
            self.write_u32::<LittleEndian>(con_id.0)?;
        }
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

impl<'a> Writer for &'a mut Writer {
    fn write_tl(&mut self, value: &tl::WriteType) -> error::Result<()> {
        (*self).write_tl(value)
    }
}

impl<W: Writer> Write for AlignedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.writer.write(buf)?;
        self.position += written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Writer> Drop for AlignedWriter<W> {
    fn drop(&mut self) {
        let remainder = self.position % self.alignment;
        if remainder != 0 {
            let buf = [0u8; 256];
            let pad = self.alignment - remainder;
            self.write_all(&buf[..pad]).expect("couldn't pad");
        }
    }
}*/
