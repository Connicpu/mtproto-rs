use std::fmt;

use byteorder::{LittleEndian, ByteOrder};
use num_traits::cast::cast;
use num_traits::int::PrimInt;
use num_traits::sign::Unsigned;
use openssl::{bn, hash, rsa};
use serde_bytes::ByteBuf;
use serde_mtproto;

use error::{self, ErrorKind};

use super::symm::AuthKey;
use super::utils::{Padding, sha1_and_or_pad};


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
        let mut padded_input = sha1_and_or_pad(input, true, Padding::Total255Random)?;
        padded_input.insert(0, 0);    // OpenSSL requires exactly 256 bytes
        println!("*** Padded input: {:?}", &padded_input);

        let mut output = [0; 256];
        self.0.public_encrypt(&padded_input, &mut output, rsa::NO_PADDING)?;

        Ok(output)
    }

    // Other implementation of RSA encryption, just to verify that we are on the right track with
    // `encrypt()`. Also can be served as a drop-in replacement in case if we abandon OpenSSL
    // dependency after rewriting this method to use `num_bigint::BigUInt`.
    pub fn encrypt2(&self, input: &[u8]) -> error::Result<Vec<u8>> {
        let padded_input = sha1_and_or_pad(input, true, Padding::Total255Random)?;
        println!("!!! Padded input: {:?}", &padded_input);

        let n = self.0.n().ok_or(error::Error::from(ErrorKind::NoModulus))?;
        let e = self.0.e().ok_or(error::Error::from(ErrorKind::NoExponent))?;

        let bn_padded_input = bn::BigNum::from_slice(&padded_input)?;
        let mut output = bn::BigNum::new()?;
        let mut context = bn::BigNumContext::new()?;
        output.mod_exp(&bn_padded_input, e, n, &mut context)?;

        Ok(output.to_vec())
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
        // .num_bytes() returns i32 and AUTH_KEY_SIZE is usize, so use u64 since it embraces
        // both i32 and usize (until 128-bit machines are in the wild)
        if g_b.num_bytes() as u64 != super::AUTH_KEY_SIZE as u64 || g_b >= dh_prime {
            continue;
        }
        let mut auth_key = bn::BigNum::new()?;
        auth_key.mod_exp(&g_a, &b, &dh_prime, &mut ctx)?;
        // Same here
        if auth_key.num_bytes() as u64 != super::AUTH_KEY_SIZE as u64 {
            continue;
        }
        let auth_key = AuthKey::new(&auth_key.to_vec())?;
        return Ok((auth_key, g_b.to_vec()));
    }
}


fn ceil_isqrt(x: u64) -> u64 {
    let mut ret = (x as f64).sqrt().trunc() as u64;
    while ret * ret > x { ret -= 1; }
    while ret * ret < x { ret += 1; }
    ret
}

pub fn decompose_pq(pq: u64) -> error::Result<(u32, u32)> {
    let mut pq_sqrt = ceil_isqrt(pq);

    loop {
        let y_sqr = pq_sqrt * pq_sqrt - pq;
        if y_sqr == 0 { bail!(ErrorKind::FactorizationFailureSquarePq(pq)) }
        let y = ceil_isqrt(y_sqr);
        if y + pq_sqrt >= pq { bail!(ErrorKind::FactorizationFailureOther(pq)) }
        if y * y != y_sqr {
            pq_sqrt += 1;
            continue;
        }
        let p = safe_uint_cast(pq_sqrt + y)?;
        let q = safe_uint_cast(if pq_sqrt > y { pq_sqrt - y } else { y - pq_sqrt })?;
        return Ok(if p > q {(q, p)} else {(p, q)});
    }
}

fn safe_uint_cast<T: PrimInt + Unsigned + Copy, U: PrimInt + Unsigned>(n: T) -> error::Result<U> {
    cast(n).ok_or_else(|| {
        let upcasted = cast::<T, u64>(n).unwrap();    // Shouldn't panic
        ErrorKind::IntegerCast(upcasted).into()
    })
}
