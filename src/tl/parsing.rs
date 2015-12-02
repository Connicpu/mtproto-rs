use std::io::{self, Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use tl;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstructorId(pub u32);

pub struct ReadContext<'a, R: Read + 'a> {
    stream: &'a mut R,
    position: u64,
}

pub struct WriteContext<'a, W: Write + 'a> {
    stream: &'a mut W,
    position: u64,
}

impl<'a, R: Read> ReadContext<'a, R> {
    pub fn read_boxed<T: tl::Type>(&mut self) -> tl::Result<T> {
        let con_id = ConstructorId(try!(self.read_u32::<LittleEndian>()));
        T::deserialize_boxed(con_id, self)
    }
    
    pub fn read_bare<T: tl::Type>(&mut self) -> tl::Result<T> {
        assert!(T::bare_type());
        T::deserialize(self)
    }
    
    pub fn read_generic<T: tl::Type>(&mut self) -> tl::Result<T> {
        if T::bare_type() {
            self.read_bare()
        } else {
            self.read_boxed()
        }
    }
}

impl<'a, R: Read> Read for ReadContext<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.stream.read(buf);
        if let Ok(len) = result {
            self.position += len as u64;
        }
        result
    }
}

impl<'a, R: Read> tl::ReadHelpers for ReadContext<'a, R> {
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

impl<'a, W: Write> WriteContext<'a, W> {
    pub fn write_boxed<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        let con_id = value.type_id().unwrap();
        try!(self.write_u32::<LittleEndian>(con_id.0));
        value.serialize(self)
    }
    
    pub fn write_bare<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        assert!(T::bare_type());
        value.serialize(self)
    }
    
    pub fn write_generic<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        if T::bare_type() {
            self.write_bare(value)
        } else {
            self.write_boxed(value)
        }
    }
}

impl<'a, W: Write> Write for WriteContext<'a, W> {
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

impl<'a, W: Write> tl::WriteHelpers for WriteContext<'a, W> {
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
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> tl::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        try!(writer.write_u32::<LittleEndian>(self.0));
        Ok(())
    }
    
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> tl::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(ConstructorId(try!(reader.read_u32::<LittleEndian>())))
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> tl::Result<Self> {
        Err(tl::Error::PrimitiveAsPolymorphic)
    }
}
