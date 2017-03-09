use std::io;

use byteorder::{LittleEndian, ByteOrder};
use openssl::{bn, hash, rsa};

use error::{ErrorKind, Result};
use super::{AuthKey, Padding, sha1_and_or_pad};

#[derive(Debug)]
pub struct RsaPublicKeyRef<'a>(&'a [u8]);

pub const KNOWN_KEYS: &'static [RsaPublicKeyRef<'static>] = &[
    RsaPublicKeyRef(b"\
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

impl<'a> RsaPublicKeyRef<'a> {
    pub fn read(&self) -> Result<RsaPublicKey> {
        let key = rsa::Rsa::public_key_from_pem(&self.0)?;
        Ok(RsaPublicKey(key))
    }
}

pub fn find_first_key(of_fingerprints: &[i64]) -> Result<Option<(RsaPublicKey, i64)>> {
    let iter = KNOWN_KEYS.iter()
        .map(|k| {
            let key = k.read()?;
            let fingerprint = key.fingerprint()?;
            if of_fingerprints.contains(&fingerprint) {
                Ok(Some((key, fingerprint)))
            } else {
                Ok(None)
            }
        });
    for item in iter {
        if let Some(x) = (item as Result<_>)? {
            return Ok(Some(x))
        }
    }
    Ok(None)
}

pub struct RsaPublicKey(rsa::Rsa);

impl RsaPublicKey {
    pub fn sha1_fingerprint(&self) -> Result<Vec<u8>> {
        let mut buf = io::Cursor::new(Vec::<u8>::new());
        {
            use tl::parsing::Writer;
            let mut writer = ::tl::parsing::WriteContext::new(&mut buf);
            writer.write_bare(&self.0.n().unwrap().to_vec()).unwrap();
            writer.write_bare(&self.0.e().unwrap().to_vec()).unwrap();
        }
        let mut hasher = hash::Hasher::new(hash::MessageDigest::sha1())?;
        hasher.update(&buf.into_inner())?;
        Ok(hasher.finish()?)
    }

    pub fn fingerprint(&self) -> Result<i64> {
        Ok(LittleEndian::read_i64(&self.sha1_fingerprint()?[12..20]))
    }

    pub fn encrypt(&self, input: &[u8]) -> Result<Vec<u8>> {
        let mut input = sha1_and_or_pad(input, true, Padding::Total255)?;
        input.insert(0, 0);
        let mut output = vec![0; 256];
        self.0.public_encrypt(&input, &mut output, rsa::NO_PADDING)?;
        Ok(output)
    }
}

pub fn calculate_auth_key(g: u32, dh_prime: &[u8], g_a: &[u8]) -> Result<(AuthKey, Vec<u8>)> {
    let mut ctx = bn::BigNumContext::new()?;
    let g = bn::BigNum::from_u32(g)?;
    let dh_prime = bn::BigNum::from_slice(dh_prime)?;
    let g_a = bn::BigNum::from_slice(g_a)?;
    let mut b = bn::BigNum::new()?;
    b.rand(2048, bn::MSB_MAYBE_ZERO, false)?;
    let mut g_b = bn::BigNum::new()?;
    g_b.mod_exp(&g, &b, &dh_prime, &mut ctx)?;
    let mut auth_key = bn::BigNum::new()?;
    auth_key.mod_exp(&g_a, &b, &dh_prime, &mut ctx)?;
    let auth_key = AuthKey::new(&auth_key.to_vec())?;
    Ok((auth_key, g_b.to_vec()))
}

fn isqrt(x: u64) -> u64 {
    let mut ret = (x as f64).sqrt().trunc() as u64;
    while ret * ret > x { ret -= 1; }
    while ret * ret < x { ret += 1; }
    ret
}

pub fn decompose_pq(pq: u64) -> Result<(u32, u32)> {
    let mut pq_sqrt = isqrt(pq);
    loop {
        let y_sqr = pq_sqrt * pq_sqrt - pq;
        if y_sqr == 0 { return Err(ErrorKind::FactorizationFailure.into()) }
        let y = isqrt(y_sqr);
        if y + pq_sqrt >= pq { return Err(ErrorKind::FactorizationFailure.into()) }
        if y * y != y_sqr {
            pq_sqrt += 1;
            continue;
        }
        let p = (pq_sqrt + y) as u32;
        let q = (if pq_sqrt > y { pq_sqrt - y } else { y - pq_sqrt }) as u32;
        return Ok(if p > q {(q, p)} else {(p, q)});
    }
}
