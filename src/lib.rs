//#![feature(specialization)]
#![recursion_limit = "128"]
extern crate either;

extern crate byteorder;
extern crate chrono;
extern crate erased_serde;
#[macro_use]
extern crate error_chain;
extern crate extprim;
extern crate num_traits;
extern crate openssl;
extern crate rand;
extern crate serde;
extern crate serde_bytes;
#[macro_use]
extern crate serde_derive;
extern crate serde_mtproto;
#[macro_use]
extern crate serde_mtproto_derive;
extern crate toml;

/*pub mod error {
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
            NoAuthKey {}
            NoSalts {}
            WrongAuthKey {}
            InvalidLength {}
            Unknown {}
            FactorizationFailure {}
            AuthenticationFailure {}
        }
    }
}*/

mod manual_types;
mod utils;

pub mod error;
pub mod rpc;
pub mod schema;
pub mod tl;


pub use error::{Error, ErrorKind, Result, ResultExt};
pub use rpc::{AppInfo, Session};
pub use tl::TLObject;
