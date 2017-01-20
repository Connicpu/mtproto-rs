use std::{self, io, result};
use byteorder;

pub type Result<T> = result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Byte(byteorder::Error),
    Utf8(std::str::Utf8Error),
    InvalidData,
    InvalidType,
    UnknownType,
    PrimitiveAsPolymorphic,
    BoxedAsBare,
    ReceivedSendType,
    UnsupportedLayer,
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

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Error {
        Error::Utf8(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Error {
        Error::Utf8(e.utf8_error())
    }
}
