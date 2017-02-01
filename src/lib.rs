#![feature(associated_consts)]

extern crate byteorder;
extern crate chrono;
extern crate crc;
#[macro_use] extern crate error_chain;
extern crate openssl;
#[macro_use] extern crate tl_macros;

pub mod error {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Utf8(::std::str::Utf8Error);
            FromUtf8(::std::string::FromUtf8Error);
            Openssl(::openssl::error::ErrorStack);
        }

        errors {
            InvalidData {}
            InvalidType {}
            UnknownType {}
            PrimitiveAsPolymorphic {}
            BoxedAsBare {}
            ReceivedSendType {}
            UnsupportedLayer {}
            WrongAuthKey {}
            InvalidLength {}
            Unknown {}
        }
    }
}

pub mod tl;
pub mod rpc;

#[derive(TLDynamic)]
#[tl_register_all]
pub struct AllDynamicTypes;
