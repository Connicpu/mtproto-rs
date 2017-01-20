extern crate mtproto;
extern crate crypto;
extern crate rand;

use rand::{Rng, SeedableRng};
use crypto::aessafe;
use mtproto::rpc::encryption::ige;

#[test]
fn random_data() {
    let mut rng = rand::StdRng::from_seed(&[1203492034]);

    let mut key = [0; 32];
    let mut iv = [0; 32];
    let mut data = [0; 2048];
    let mut encrypted = [0; 2048];
    let mut decrypted = [0; 2048];

    for _ in 0..1024 {
        rng.fill_bytes(&mut key);
        rng.fill_bytes(&mut iv);
        rng.fill_bytes(&mut data);

        let aes_enc = aessafe::AesSafe256Encryptor::new(&key);
        let mut ige_enc = ige::IgeEncryptor::new(aes_enc, &iv);
        let aes_dec = aessafe::AesSafe256Decryptor::new(&key);
        let mut ige_dec = ige::IgeDecryptor::new(aes_dec, &iv);

        for (data, encrypted) in data.chunks(16).zip(encrypted.chunks_mut(16)) {
            ige_enc.encrypt_block(data, encrypted);
        }

        assert!(encrypted[..] != data[..]);

        for (encrypted, decrypted) in encrypted.chunks(16).zip(decrypted.chunks_mut(16)) {
            ige_dec.decrypt_block(encrypted, decrypted);
        }

        assert_eq!(decrypted[..], data[..]);
    }
}

