use std::io;

use byteorder::{LittleEndian, ByteOrder};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use modexp::modexp;
use num::BigUint;

#[derive(Debug)]
pub struct RsaPublicKey<'a> {
    pub modulus: &'a [u8],
    pub exponent: &'a [u8],
}

pub const KNOWN_KEYS: &'static [RsaPublicKey<'static>] = &[
    RsaPublicKey {
        modulus: b"\xc1\x50\x02\x3e\x2f\x70\xdb\x79\x85\xde\xd0\x64\x75\x9c\
\xfe\xcf\x0a\xf3\x28\xe6\x9a\x41\xda\xf4\xd6\xf0\x1b\x53\x81\x35\xa6\xf9\x1f\
\x8f\x8b\x2a\x0e\xc9\xba\x97\x20\xce\x35\x2e\xfc\xf6\xc5\x68\x0f\xfc\x42\x4b\
\xd6\x34\x86\x49\x02\xde\x0b\x4b\xd6\xd4\x9f\x4e\x58\x02\x30\xe3\xae\x97\xd9\
\x5c\x8b\x19\x44\x2b\x3c\x0a\x10\xd8\xf5\x63\x3f\xec\xed\xd6\x92\x6a\x7f\x6d\
\xab\x0d\xdb\x7d\x45\x7f\x9e\xa8\x1b\x84\x65\xfc\xd6\xff\xfe\xed\x11\x40\x11\
\xdf\x91\xc0\x59\xca\xed\xaf\x97\x62\x5f\x6c\x96\xec\xc7\x47\x25\x55\x69\x34\
\xef\x78\x1d\x86\x6b\x34\xf0\x11\xfc\xe4\xd8\x35\xa0\x90\x19\x6e\x9a\x5f\x0e\
\x44\x49\xaf\x7e\xb6\x97\xdd\xb9\x07\x64\x94\xca\x5f\x81\x10\x4a\x30\x5b\x6d\
\xd2\x76\x65\x72\x2c\x46\xb6\x0e\x5d\xf6\x80\xfb\x16\xb2\x10\x60\x7e\xf2\x17\
\x65\x2e\x60\x23\x6c\x25\x5f\x6a\x28\x31\x5f\x40\x83\xa9\x67\x91\xd7\x21\x4b\
\xf6\x4c\x1d\xf4\xfd\x0d\xb1\x94\x4f\xb2\x6a\x2a\x57\x03\x1b\x32\xee\xe6\x4a\
\xd1\x5a\x8b\xa6\x88\x85\xcd\xe7\x4a\x5b\xfc\x92\x0f\x6a\xbf\x59\xba\x5c\x75\
\x50\x63\x73\xe7\x13\x0f\x90\x42\xda\x92\x21\x79\x25\x1f",
        exponent: b"\x01\x00\x01",
    },
];

impl<'a> RsaPublicKey<'a> {
    pub fn sha1_fingerprint(&self) -> [u8; 20] {
        let mut buf = io::Cursor::new(Vec::<u8>::new());
        {
            let mut writer = ::tl::parsing::WriteContext::new(&mut buf);
            writer.write_bare(&self.modulus).unwrap();
            writer.write_bare(&self.exponent).unwrap();
        }
        let mut hasher = Sha1::new();
        hasher.input(&buf.into_inner());
        let mut ret = [0u8; 20];
        hasher.result(&mut ret);
        ret
    }

    pub fn fingerprint(&self) -> u64 {
        LittleEndian::read_u64(&self.sha1_fingerprint()[12..20])
    }

    pub fn encrypt(&self, input: &[u8]) -> Vec<u8> {
        if input.len() != 255 {
            panic!("bad input length: {}", input.len());
        }
        let (_, result) = modexp(
            BigUint::from_bytes_be(input),
            BigUint::from_bytes_be(self.exponent),
            BigUint::from_bytes_be(self.modulus)).to_bytes_be();
        result
    }
}

pub fn calculate_auth_key(g: u32, dh_prime: &[u8], g_a: &[u8], b: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let dh_prime = BigUint::from_bytes_be(dh_prime);
    let g_a = BigUint::from_bytes_be(g_a);
    let b = BigUint::from_bytes_be(&b);
    let g_b = modexp(g.into(), b.clone(), dh_prime.clone());
    let auth_key = modexp(g_a, b, dh_prime);
    (g_b.to_bytes_be().1, auth_key.to_bytes_be().1)
}
