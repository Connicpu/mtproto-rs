use std::io::{Cursor, Write};

use error::{self, ErrorKind};
use rand::{self, Rng};
use rpc::utils::sha1_bytes;


pub(super) enum Padding {
    Total255Random,
    Mod16,
}

pub(super) fn sha1_and_or_pad(input: &[u8], prepend_sha1: bool, padding: Padding) -> error::Result<Vec<u8>> {
    let mut result = if prepend_sha1 {
        sha1_bytes(&[input])?
    } else {
        vec![]
    };

    result.extend(input);

    match padding {
        Padding::Total255Random => {
            if result.len() > 255 {
                bail!(ErrorKind::Sha1Total255Longer);
            }

            let old_len = result.len();
            result.resize(255, 0);

            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut result[old_len..]);
        },
        Padding::Mod16 => {
            let old_len = result.len();
            let new_len = old_len + (16 - (old_len % 16)) % 16; // == ceil_div(old_len, 16) * 16
            result.resize(new_len, 0);
        },
    }

    Ok(result)
}

pub(super) fn set_slice_parts(result: &mut [u8], parts: &[&[u8]]) {
    let parts_len = parts.iter().map(|x| x.len()).sum();
    assert_eq!(result.len(), parts_len);

    let mut cursor = Cursor::new(result);
    for part in parts {
        // Can unwrap here safely since we've already checked for length mismatch
        cursor.write(part).unwrap();
    }
}
