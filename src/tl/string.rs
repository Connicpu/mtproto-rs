use std::string;
use std::io::{Read, Write};
use tl::{self, Type};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

pub struct String(pub string::String);
pub struct SendStr<'a>(pub &'a str);

impl From<string::String> for String {
    fn from(string: string::String) -> String {
        String(string)
    }
}

impl<'a> From<&'a str> for SendStr<'a> {
    fn from(string: &'a str) -> SendStr<'a> {
        SendStr(string)
    }
}

impl<'a> Type for String {
    fn bare_type() -> bool {
        true
    }
    
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> tl::Result<()> {
        SendStr(&self.0).serialize(writer)
    }
    
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> tl::Result<Self> {
        let bytes = try!(reader.read_bare());
        Ok(String(try!(string::String::from_utf8(bytes))))
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> tl::Result<Self> {
        Err(tl::Error::PrimitiveAsPolymorphic)
    }
}

impl<'a> Type for SendStr<'a> {
    fn bare_type() -> bool {
        true
    }
    
    fn type_id(&self) -> Option<ConstructorId> {
        None
    }
    
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> tl::Result<()> {
        try!(writer.write_bare(&self.0.as_bytes()));
        Ok(())
    }
    
    fn deserialize<R: Read>(_: &mut ReadContext<R>) -> tl::Result<Self> {
        Err(tl::Error::ReceivedSendType)
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> tl::Result<Self> {
        Err(tl::Error::ReceivedSendType)
    }
}
