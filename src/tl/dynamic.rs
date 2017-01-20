use super::{Result, Type};
use std::io::{Read, Write};
use std::any::Any;
use std::collections::HashMap;
use tl::parsing::{ConstructorId, ReadContext, WriteContext};

pub type TLCtor = fn(ConstructorId, &mut ReadContext<&mut Read>) -> Result<Box<TLObject>>;

pub trait TLObject: Any {
    fn tl_id(&self) -> ConstructorId;
    fn serialize(&self, writer: &mut WriteContext<&mut Write>) -> Result<()>;
}

pub trait TLObjectExt {
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()>;
}

impl<T: Type + Any> TLObject for T {
    fn tl_id(&self) -> ConstructorId {
        self.type_id().unwrap()
    }

    fn serialize(&self, writer: &mut WriteContext<&mut Write>) -> Result<()> {
        <T as Type>::serialize(self, writer)
    }
}

impl<T: TLObject> TLObjectExt for T {
    fn serialize<W: Write>(&self, writer: &mut WriteContext<W>) -> Result<()> {
        let (result, state) = {
            let mut new_writer = writer.borrow_polymorphic();
            let result = <T as TLObject>::serialize(self, &mut new_writer);
            (result, new_writer.end_polymorphic())
        };
        writer.integrate_polymorphic(state);
        result
    }
}

pub trait TLDynamic: TLObject {
    fn register_ctors(cstore: &mut ClassStore);
}

pub struct ClassStore {
    ctors: HashMap<ConstructorId, TLCtor>,
}

impl ClassStore {
    pub fn make_store() -> ClassStore {
        use tl::complex_types::*;
        use tl::Null;
        let mut store = ClassStore { ctors: HashMap::new() };

        Error::register_ctors(&mut store);
        DecryptedMessage::register_ctors(&mut store);
        Config::register_ctors(&mut store);
        DecryptedMessageLayer::register_ctors(&mut store);
        Message::register_ctors(&mut store);
        Null::register_ctors(&mut store);
        Updates::register_ctors(&mut store);
        Video::register_ctors(&mut store);
        Audio::register_ctors(&mut store);
        Document::register_ctors(&mut store);
        Photo::register_ctors(&mut store);
        PhotoSize::register_ctors(&mut store);

        store
    }

    pub fn add_ctor(&mut self, id: ConstructorId, ctor: TLCtor) {
        self.ctors.insert(id, ctor);
    }

    pub fn deserialize<R: Read>(&self, reader: &mut ReadContext<R>) -> Result<Box<TLObject>> {
        let id = try!(reader.read_bare());
        let ctor = match self.ctors.get(&id) {
            Some(ctor) => ctor,
            None => return Err(super::Error::UnknownType)
        };

        let (result, state) = {
            let mut new_reader = reader.borrow_polymorphic();
            let result = ctor(id, &mut new_reader);
            (result, new_reader.end_polymorphic())
        };
        reader.integrate_polymorphic(state);

        result
    }
}
