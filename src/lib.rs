// `error_chain!` can nest quite deeply
#![recursion_limit = "128"]

extern crate byteorder;
extern crate chrono;
extern crate envy;
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


mod manual_types;
mod utils;

pub mod error;
pub mod rpc;
pub mod schema;
pub mod tl;


pub use error::{Error, ErrorKind, Result, ResultExt};
pub use rpc::{AppInfo, Session};
pub use tl::TLObject;
