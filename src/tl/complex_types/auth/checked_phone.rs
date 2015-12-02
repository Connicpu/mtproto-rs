use std::io::{Read, Write};
use tl::{self, Type};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

pub struct CheckedPhone {
	pub phone_registered: bool,
	pub phone_invited: bool,
}

const TYPE_ID: ConstructorId = ConstructorId(0xe300cc3b);

impl Type for CheckedPhone {
    fn bare_type() -> bool {
		false
	}
	
    fn type_id(&self) -> Option<ConstructorId> {
		Some(TYPE_ID)
	}
	
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> tl::Result<()> {
		try!(writer.write_generic(&tl::Bool(self.phone_registered)));
		try!(writer.write_generic(&tl::Bool(self.phone_invited)));
		Ok(())
	}
	
    fn deserialize<R: Read>(reader: &mut ReadContext<R>) -> tl::Result<Self> {
		let registered: tl::Bool = try!(reader.read_generic());
		let invited: tl::Bool = try!(reader.read_generic());
		
		Ok(CheckedPhone {
			phone_registered: registered.0,
			phone_invited: invited.0,
		})
	}
	
    fn deserialize_boxed<R: Read>(id: ConstructorId, reader: &mut ReadContext<R>) -> tl::Result<Self> {
		if id != TYPE_ID {
			return Err(tl::Error::InvalidType);
		}
		
		Self::deserialize(reader)
	}
}
