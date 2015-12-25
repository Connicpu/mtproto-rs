use std::mem;
use crypto::{buffer, aes, blockmodes};
use crypto::aes::KeySize::*;
use crypto::symmetriccipher::{Encryptor, Decryptor, BlockEncryptor, SymmetricCipherError};
use crypto::buffer::{ReadBuffer, WriteBuffer, BufferResult, RefReadBuffer, RefWriteBuffer};

const BLOCK_SIZE: usize = 16;

#[derive(Copy, Clone)]
struct AesBlock {
    data: [u32; 4],
}

impl AesBlock {
    fn from_bytes(data: &[u8]) -> AesBlock {
        *AesBlock::from_ref(data)
    }
    
    fn from_ref(data: &[u8]) -> &AesBlock {
        unsafe { mem::transmute(&data[0]) }
    }
    
    fn from_ref_mut(data: &mut [u8]) -> &mut AesBlock {
        unsafe { mem::transmute(&mut data[0]) }
    }
    
    fn copy_to(&self, data: &mut [u8]) {
        let temp: &[u8; BLOCK_SIZE] = unsafe { mem::transmute(&self.data) };
        data.clone_from_slice(temp);
    }
    
    fn as_bytes(&self) -> &[u8] {
        let temp: &[u8; BLOCK_SIZE] = unsafe { mem::transmute(&self.data) };
        temp
    }
}

#[derive(Copy, Clone)]
struct IvBlock {
    iv1: AesBlock,
    iv2: AesBlock,
}

pub struct AesIge<T: BlockEncryptor> {
    aes: T,
    iv: IvBlock,
}

fn ige_enc_before(input: &[u8], output: &mut [u8], iv: &IvBlock) {
    let inp = AesBlock::from_ref(input);
    let outp = AesBlock::from_ref_mut(output);
    
    for (iv, (i, o)) in iv.iv1.data.iter().zip(inp.data.iter().zip(outp.data.iter_mut())) {
        *o = *i ^ *iv;
    }
}

fn ige_enc_after(input: &[u8], output: &mut [u8], iv: &mut IvBlock) {
    let inp = AesBlock::from_ref(input);
    let outp = AesBlock::from_ref_mut(output);
    
    for (iv, o) in iv.iv2.data.iter().zip(outp.data.iter_mut()) {
        *o ^= *iv;
    }
    
    iv.iv1 = *outp;
    iv.iv2 = *inp;
}

impl<T: BlockEncryptor> AesIge<T> {
    pub fn new(aes: T, iv: &[u8]) -> Self {
        assert!(aes.block_size() == BLOCK_SIZE);
        let mut ige = AesIge {
            aes: aes,
            iv: unsafe { mem::uninitialized() },
        };
        unsafe { mem::transmute::<_, &mut [u8; 32]>(&mut ige.iv) }.clone_from_slice(iv);
        ige
    }
    
    pub fn encrypt_block(&mut self, input: &[u8], output: &mut [u8]) {
        // debug_assert is fine because these buffers should hopefully
        // be stack-alloc'd buffers and this will be caught during
        // debugging if the buffers are too small
        debug_assert!(input.len() == BLOCK_SIZE);
        debug_assert!(output.len() >= BLOCK_SIZE);
        
        ige_enc_before(input, output, &self.iv);
        
        let temp_in = AesBlock::from_bytes(output);
        self.aes.encrypt_block(temp_in.as_bytes(), output);
        
        ige_enc_after(input, output, &mut self.iv);
    }
}

