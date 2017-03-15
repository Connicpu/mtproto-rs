#![feature(associated_consts, specialization)]
#![recursion_limit = "128"]

extern crate byteorder;
extern crate chrono;
extern crate crc;
#[macro_use] extern crate error_chain;
extern crate openssl;

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
            InvalidType(expected: Vec<::tl::parsing::ConstructorId>, received: Option<::tl::parsing::ConstructorId>) {}
            BoxedAsBare {}
            ReceivedSendType {}
            UnsupportedLayer {}
            WrongAuthKey {}
            InvalidLength {}
            Unknown {}
            FactorizationFailure {}
            AuthenticationFailure {}
        }
    }
}

pub mod tl;
pub mod rpc;
pub mod schema;
mod manual_types;
