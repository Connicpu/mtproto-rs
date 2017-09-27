use std::fmt;

use byteorder::{LittleEndian, ByteOrder};
use openssl::{bn, hash, rsa};
use serde_bytes::ByteBuf;
use serde_mtproto;

use error::{self, ErrorKind};
use super::{AuthKey, Padding, sha1_and_or_pad};


#[derive(Debug)]
pub struct RsaRawPublicKeyRef<'a>(&'a [u8]);

pub const KNOWN_RAW_KEYS: &'static [RsaRawPublicKeyRef<'static>] = &[
    RsaRawPublicKeyRef(b"\
-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwVACPi9w23mF3tBkdZz+\n\
zwrzKOaaQdr01vAbU4E1pvkfj4sqDsm6lyDONS789sVoD/xCS9Y0hkkC3gtL1tSf\n\
TlgCMOOul9lcixlEKzwKENj1Yz/s7daSan9tqw3bfUV/nqgbhGX81v/+7RFAEd+R\n\
wFnK7a+XYl9sluzHRyVVaTTveB2GazTwEfzk2DWgkBluml8OREmvfraX3bkHZJTK\n\
X4EQSjBbbdJ2ZXIsRrYOXfaA+xayEGB+8hdlLmAjbCVfaigxX0CDqWeR1yFL9kwd\n\
9P0NsZRPsmoqVwMbMu7mStFai6aIhc3nSlv8kg9qv1m6XHVQY3PnEw+QQtqSIXkl\n\
HwIDAQAB\n\
-----END PUBLIC KEY-----"),
];

impl<'a> RsaRawPublicKeyRef<'a> {
    pub fn read(&self) -> error::Result<RsaPublicKey> {
        let key = rsa::Rsa::public_key_from_pem(&self.0)?;
        Ok(RsaPublicKey(key))
    }
}


pub struct RsaPublicKey(rsa::Rsa);

impl fmt::Debug for RsaPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct RsaRepr<'a> {
            n: Option<&'a bn::BigNumRef>,
            e: Option<&'a bn::BigNumRef>,
        }

        impl<'a> fmt::Debug for RsaRepr<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let debug_option_big_num = |opt_big_num: Option<&bn::BigNumRef>| {
                    match opt_big_num {
                        Some(big_num) => match big_num.to_hex_str() {
                            Ok(hex_str) => hex_str.to_lowercase(),
                            Err(_) => big_num.to_vec().iter()
                                .map(|byte| format!("{:02x}", byte)).collect::<String>(),
                        },
                        None => "(None)".to_owned(),
                    }
                };

                f.debug_struct("RsaRepr")
                    .field("n", &DisplayStr(&debug_option_big_num(self.n)))
                    .field("e", &DisplayStr(&debug_option_big_num(self.e)))
                    .finish()
            }
        }

        struct DisplayStr<'a>(&'a str);

        impl<'a> fmt::Debug for DisplayStr<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(self.0, f)
            }
        }

        let rsa_repr = RsaRepr {
            n: self.0.n(),
            e: self.0.e(),
        };

        f.debug_tuple("RsaPublicKey")
            .field(&rsa_repr)
            .finish()
    }
}


impl RsaPublicKey {
    pub fn sha1_fingerprint(&self) -> error::Result<Vec<u8>> {
        let mut buf = Vec::new();

        let n_bytes = self.0.n().ok_or(error::Error::from(ErrorKind::NoModulus))?.to_vec();
        let e_bytes = self.0.e().ok_or(error::Error::from(ErrorKind::NoExponent))?.to_vec();

        // Need to allocate new space, so use `&mut buf` instead of `buf.as_mut_slice()`
        serde_mtproto::to_writer(&mut buf, &ByteBuf::from(n_bytes))?;
        serde_mtproto::to_writer(&mut buf, &ByteBuf::from(e_bytes))?;

        let mut hasher = hash::Hasher::new(hash::MessageDigest::sha1())?;
        hasher.update(&buf)?;

        Ok(hasher.finish2().map(|b| b.to_vec())?)
    }

    pub fn fingerprint(&self) -> error::Result<i64> {
        let sha1_fingerprint = self.sha1_fingerprint()?;

        Ok(LittleEndian::read_i64(&sha1_fingerprint[12..20]))
    }

    pub fn encrypt(&self, input: &[u8]) -> error::Result<[u8; 256]> {
        let mut padded_input = sha1_and_or_pad(input, true, Padding::Total255)?;
        padded_input.insert(0, 0);    // OpenSSL requires exactly 256 bytes

        let mut output = [0; 256];
        self.0.public_encrypt(&padded_input, &mut output, rsa::NO_PADDING)?;

        Ok(output)
    }
}

pub fn find_first_key_fail_safe(of_fingerprints: &[i64]) -> error::Result<(RsaPublicKey, i64)> {
    find_first_key(of_fingerprints)?
        .ok_or(ErrorKind::NoRsaPublicKeyForFingerprints(of_fingerprints.to_vec()).into())
}

pub fn find_first_key(of_fingerprints: &[i64]) -> error::Result<Option<(RsaPublicKey, i64)>> {
    for raw_key in KNOWN_RAW_KEYS {
        let key = raw_key.read()?;
        let fingerprint = key.fingerprint()?;

        if of_fingerprints.contains(&fingerprint) {
            return Ok(Some((key, fingerprint)));
        }
    }

    Ok(None)
}

pub fn calculate_auth_key(g: u32, dh_prime: &[u8], g_a: &[u8]) -> error::Result<(AuthKey, Vec<u8>)> {
    let mut ctx = bn::BigNumContext::new()?;
    let g = bn::BigNum::from_u32(g)?;
    let dh_prime = bn::BigNum::from_slice(dh_prime)?;
    let g_a = bn::BigNum::from_slice(g_a)?;

    loop {
        let mut b = bn::BigNum::new()?;
        b.rand(2048, bn::MSB_MAYBE_ZERO, false)?;
        let mut g_b = bn::BigNum::new()?;
        g_b.mod_exp(&g, &b, &dh_prime, &mut ctx)?;
        if g_b.num_bytes() as usize != super::AUTH_KEY_SIZE || g_b >= dh_prime {
            continue;
        }
        let mut auth_key = bn::BigNum::new()?;
        auth_key.mod_exp(&g_a, &b, &dh_prime, &mut ctx)?;
        if auth_key.num_bytes() as usize != super::AUTH_KEY_SIZE {
            continue;
        }
        let auth_key = AuthKey::new(&auth_key.to_vec())?;
        return Ok((auth_key, g_b.to_vec()));
    }
}


fn isqrt(x: u64) -> u64 {
    let mut ret = (x as f64).sqrt().trunc() as u64;
    while ret * ret > x { ret -= 1; }
    while ret * ret < x { ret += 1; }
    ret
}

pub fn decompose_pq(pq: u64) -> error::Result<(u32, u32)> {
    let mut pq_sqrt = isqrt(pq);

    loop {
        let y_sqr = pq_sqrt * pq_sqrt - pq;
        if y_sqr == 0 { bail!(ErrorKind::FactorizationFailure) }
        let y = isqrt(y_sqr);
        if y + pq_sqrt >= pq { bail!(ErrorKind::FactorizationFailure) }
        if y * y != y_sqr {
            pq_sqrt += 1;
            continue;
        }
        let p = (pq_sqrt + y) as u32;
        let q = (if pq_sqrt > y { pq_sqrt - y } else { y - pq_sqrt }) as u32;
        return Ok(if p > q {(q, p)} else {(p, q)});
    }
}
