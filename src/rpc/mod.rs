//! RPC essentials.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use envy;
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


/// Telegram application information required for authorization.
///
/// A single specific instance of `AppInfo` is typically tied to a
/// single phone number. You can obtain it here:
/// https://core.telegram.org/api/obtaining_api_id.
///
/// After registration you will be given `api_id` and `api_hash` values
/// which are used here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppInfo {
    /// First field under "App configuration" section at
    /// https://my.telegram.org/apps.
    pub api_id: i32,
    // FIXME: use &'a str or Cow<'a, str> here
    /// Second field under "App configuration" section at
    /// https://my.telegram.org/apps.
    pub api_hash: String,
}

impl AppInfo {
    /// Construct an `AppInfo` instance from API id and API hash.
    pub fn new(api_id: i32, api_hash: String) -> AppInfo {
        AppInfo {
            api_id: api_id,
            api_hash: api_hash,
        }
    }

    /// Obtain an `AppInfo` from environment variables.
    ///
    /// This method works with `MTPROTO_API_ID` and `MTPROTO_API_HASH`
    /// variables.
    pub fn from_env() -> error::Result<AppInfo> {
        envy::prefixed("MTPROTO_")
            .from_env::<AppInfo>()
            .map_err(Into::into)
    }

    /// Read an `AppInfo` from a TOML value.
    pub fn read_from_toml_value(value: toml::Value) -> error::Result<AppInfo> {
        AppInfo::deserialize(value).map_err(Into::into)
    }

    /// Read an `AppInfo` from a TOML string.
    pub fn read_from_toml_str(s: &str) -> error::Result<AppInfo> {
        toml::from_str(s).map_err(Into::into)
    }

    /// Read an `AppInfo` from a TOML file.
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> error::Result<AppInfo> {
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
