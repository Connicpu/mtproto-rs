use std::string;
use std::io::{Read, Write};
use tl::{self, Type, WriteHelpers, ReadHelpers};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

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
		let len = self.0.len();
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
        try!(writer.write_all(self.0.as_bytes()));
        try!(writer.pad(4));
		
        Ok(())
    }
    
    fn deserialize<R: Read>(_: &mut ReadContext<R>) -> tl::Result<Self> {
        Err(tl::Error::ReceivedSendType)
    }
    
    fn deserialize_boxed<R: Read>(_: ConstructorId, _: &mut ReadContext<R>) -> tl::Result<Self> {
        Err(tl::Error::ReceivedSendType)
    }
}
