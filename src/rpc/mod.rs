use std::io::{Read, Write};
use tl::{self, Type};
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

pub struct RpcContext<'a, R: Read + 'a, W: Write + 'a> {
	reader: &'a mut ReadContext<'a, R>,
	writer: &'a mut WriteContext<'a, W>,
}

pub struct RpcCommand<'a, R: Read + 'a, W: Write + 'a> {
	context: &'a mut RpcContext<'a, R, W>,
	command: ConstructorId,
}

impl<'a, R: Read + 'a, W: Write + 'a> RpcContext<'a, R, W> {
	pub fn create(read: &'a mut ReadContext<'a, R>, write: &'a mut WriteContext<'a, W>) -> Self {
		RpcContext {
			reader: read,
			writer: write,
		}
	}
	
	pub fn command(&'a mut self, method: ConstructorId) -> RpcCommand<'a, R, W> {
		RpcCommand {
			context: self,
			command: method,
		}
	}
}

impl<'a, R: Read + 'a, W: Write + 'a> RpcCommand<'a, R, W> {
	pub fn invoke<Ret: Type, Args: RpcArgs>(&mut self, args: Args) -> tl::Result<Ret> {
		// Write command
		try!(self.context.writer.write_bare(&self.command));
		try!(args.write_args(self.context.writer));
		try!(self.context.writer.flush());
		
		// Read result
		Ok(try!(self.context.reader.read_generic()))
	}
}

pub trait RpcArgs {
	fn write_args<W: Write>(self, writer: &mut WriteContext<W>) -> tl::Result<()>;
}

impl RpcArgs for () {
	fn write_args<W: Write>(self, _: &mut WriteContext<W>) -> tl::Result<()> {
		Ok(())
	}
}

impl<'a, T1: Type> RpcArgs for (&'a T1,) {
	fn write_args<W: Write>(self, writer: &mut WriteContext<W>) -> tl::Result<()> {
		try!(writer.write_generic(self.0));
		Ok(())
	}
}

impl<'a, T1: Type, T2: Type> RpcArgs for (&'a T1, &'a T2) {
	fn write_args<W: Write>(self, writer: &mut WriteContext<W>) -> tl::Result<()> {
		try!(writer.write_generic(self.0));
		try!(writer.write_generic(self.1));
		Ok(())
	}
}

impl<'a, T1: Type, T2: Type, T3: Type> RpcArgs for (&'a T1, &'a T2, &'a T3) {
	fn write_args<W: Write>(self, writer: &mut WriteContext<W>) -> tl::Result<()> {
		try!(writer.write_generic(self.0));
		try!(writer.write_generic(self.1));
		try!(writer.write_generic(self.2));
		Ok(())
	}
}

impl<'a, T1: Type, T2: Type, T3: Type, T4: Type> RpcArgs for (&'a T1, &'a T2, &'a T3, &'a T4) {
	fn write_args<W: Write>(self, writer: &mut WriteContext<W>) -> tl::Result<()> {
		try!(writer.write_generic(self.0));
		try!(writer.write_generic(self.1));
		try!(writer.write_generic(self.2));
		try!(writer.write_generic(self.3));
		Ok(())
	}
}

impl<'a, T1: Type, T2: Type, T3: Type, T4: Type, T5: Type> RpcArgs for (&'a T1, &'a T2, &'a T3, &'a T4, &'a T5) {
	fn write_args<W: Write>(self, writer: &mut WriteContext<W>) -> tl::Result<()> {
		try!(writer.write_generic(self.0));
		try!(writer.write_generic(self.1));
		try!(writer.write_generic(self.2));
		try!(writer.write_generic(self.3));
		try!(writer.write_generic(self.4));
		Ok(())
	}
}

