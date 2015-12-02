use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::Arc;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use tl;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstructorId(pub u32);
pub type CtorResult = tl::Result<Box<tl::Polymorphic>>;
type Constructor = fn(ConstructorId, &mut ReadContext) -> CtorResult;

pub struct Schema {
    constructors: HashMap<ConstructorId, Constructor>,
}

pub struct ReadContext {
    schema: Arc<Schema>,
    stream: Box<Read>,
    position: u64,
}

pub struct WriteContext {
    stream: Box<Write>,
    position: u64,
}

fn deserialize_boxed_impl<T: tl::Type>(id: ConstructorId, reader: &mut ReadContext) -> CtorResult {
    use tl::Type;
    match T::deserialize_boxed(id, reader) {
        Ok(value) => Ok(Box::new(value)),
        Err(e) => Err(e),
    }
}

impl Schema {
    pub fn new() -> Schema {
        Schema {
            constructors: HashMap::new(),
        }
    }
    
    pub fn add_constructor<T: tl::Type>(&mut self, sample_value: T) {
        let id = sample_value.type_id().unwrap();
        let ctor = deserialize_boxed_impl::<T>;
        self.constructors.insert(id, ctor);
    }
}

impl ReadContext {
    pub fn read_polymorphic(&mut self) -> tl::Result<Box<tl::Polymorphic>> {
        let con_id = ConstructorId(try!(self.read_u32::<LittleEndian>()));
        let schema = self.schema.clone();
        if let Some(ctor) = schema.constructors.get(&con_id) {
            ctor(con_id, self)
        } else {
            return Err(tl::Error::UnknownType)
        }
    }
    
    pub fn read_boxed<T: tl::Type>(&mut self) -> tl::Result<T> {
        use tl::Type;
        let con_id = ConstructorId(try!(self.read_u32::<LittleEndian>()));
        T::deserialize_boxed(con_id, self)
    }
    
    pub fn read_bare<T: tl::Type>(&mut self) -> tl::Result<T> {
        assert!(T::bare_type());
        T::deserialize(self)
    }
}

impl Read for ReadContext {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.stream.read(buf);
        if let Ok(len) = result {
            self.position += len as u64;
        }
        result
    }
}

impl tl::ReadHelpers for ReadContext {
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

impl WriteContext {
    pub fn write_polymorphic(&mut self, value: &tl::Polymorphic) -> tl::Result<()> {
        let con_id = value.type_id().unwrap();
        try!(self.write_u32::<LittleEndian>(con_id.0));
        value.serialize(self)
    }
    
    pub fn write_boxed<T: tl::Polymorphic>(&mut self, value: &T) -> tl::Result<()> {
        let con_id = value.type_id().unwrap();
        try!(self.write_u32::<LittleEndian>(con_id.0));
        value.serialize(self)
    }
    
    pub fn write_bare<T: tl::Type>(&mut self, value: &T) -> tl::Result<()> {
        assert!(T::bare_type());
        value.serialize(self)
    }
}

impl Write for WriteContext {
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

impl tl::WriteHelpers for WriteContext {
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
