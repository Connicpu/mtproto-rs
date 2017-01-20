use std::io;

use byteorder::{LittleEndian, ByteOrder};
use openssl::{bn, hash, rsa};

pub type Result<T> = ::std::result::Result<T, ::openssl::error::ErrorStack>;

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

pub fn find_first_key(of_fingerprints: &[u64]) -> Result<Option<(RsaPublicKey, u64)>> {
    let iter = KNOWN_KEYS.iter()
        .map(|k| {
            let key = k.read()?;
            let fingerprint = key.fingerprint()?;
            if of_fingerprints.contains(&fingerprint) {
                Ok(Some((key, fingerprint)))
            } else { Ok(None) }
        });
    for item in iter {
        if let Some(x) = item? {
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
            let mut writer = ::tl::parsing::WriteContext::new(&mut buf);
            writer.write_bare(&self.0.n().unwrap().to_vec()).unwrap();
            writer.write_bare(&self.0.e().unwrap().to_vec()).unwrap();
        }
        let mut hasher = hash::Hasher::new(hash::MessageDigest::sha1())?;
        hasher.update(&buf.into_inner())?;
        hasher.finish()
    }

    pub fn fingerprint(&self) -> Result<u64> {
        Ok(LittleEndian::read_u64(&self.sha1_fingerprint()?[12..20]))
    }

    pub fn encrypt(&self, input: &[u8]) -> Result<Vec<u8>> {
        if input.len() != 255 {
            panic!("bad input length: {}", input.len());
        }
        let mut real_input = vec![0; 256];
        (&mut real_input[1..]).copy_from_slice(input);
        let mut output = vec![0; 256];
        self.0.public_encrypt(&real_input, &mut output, rsa::NO_PADDING)?;
        Ok(output)
    }
}

pub fn calculate_auth_key(g: u32, dh_prime: &[u8], g_a: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
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
    Ok((g_b.to_vec(), auth_key.to_vec()))
}
