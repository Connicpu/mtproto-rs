use std::fs::File;
use std::io::Read;
use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use erased_serde::Serialize as ErasedSerialize;
use serde::Deserialize;
use toml;

use error;
use schema::FutureSalt;
use tl::dynamic::TLObject;


pub mod encryption;
pub mod message;
pub mod session;
mod utils;

pub use self::message::{Message, MessageType};
pub use self::session::Session;


pub trait RpcFunction: ErasedSerialize {
    type Reply: TLObject + 'static;
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppInfo {
    api_id: i32,
    // FIXME: use &'a str or Cow<'a, str> here
    api_hash: String,
}

impl AppInfo {
    pub fn new(api_id: i32, api_hash: String) -> AppInfo {
        AppInfo {
            api_id: api_id,
            api_hash: api_hash,
        }
    }

    pub fn load_from_toml_value(value: toml::Value) -> error::Result<AppInfo> {
        AppInfo::deserialize(value).map_err(Into::into)
    }

    pub fn load_from_toml_str(s: &str) -> error::Result<AppInfo> {
        toml::from_str(s).map_err(Into::into)
    }

    pub fn load_from_toml_file<P: AsRef<Path>>(path: P) -> error::Result<AppInfo> {
        let mut buf = String::new();
        let mut file = File::open(path)?;

        file.read_to_string(&mut buf)?;
        let app_info = toml::from_str(&buf)?;

        Ok(app_info)
    }
}


#[derive(Debug, Clone)]
pub struct Salt {
    valid_since: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    salt: i64,
}

impl From<FutureSalt> for Salt {
    fn from(fs: FutureSalt) -> Self {
        Salt {
            valid_since: Utc.timestamp(fs.valid_since as i64, 0), // from i32
            valid_until: Utc.timestamp(fs.valid_until as i64, 0), // same here
            salt: fs.salt,
        }
    }
}
