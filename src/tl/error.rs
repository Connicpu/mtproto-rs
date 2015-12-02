use std::{io, result};
use byteorder;

pub type Result<T> = result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Byte(byteorder::Error),
    InvalidData,
    InvalidType,
    UnknownType,
    PrimitiveAsPolymorphic,
    BoxedAsBare,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<byteorder::Error> for Error {
    fn from(e: byteorder::Error) -> Error {
        Error::Byte(e)
    }
}
