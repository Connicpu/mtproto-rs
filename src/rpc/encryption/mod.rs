pub mod asymm;
pub mod symm;
mod utils;

pub use self::asymm::{RsaPublicKey,
                      calculate_auth_key, decompose_pq, find_first_key, find_first_key_fail_safe};
pub use self::symm::{AesParams, AuthKey};


const AUTH_KEY_SIZE: usize = 256;
