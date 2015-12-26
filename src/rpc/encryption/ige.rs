use std::mem;
use crypto::symmetriccipher::{BlockEncryptor, BlockDecryptor};

const BLOCK_SIZE: usize = 16;

pub struct IgeEncryptor<T: BlockEncryptor> {
    aes: T,
    iv: IvBlock,
}

impl<T: BlockEncryptor> IgeEncryptor<T> {
    pub fn new(aes: T, iv: &[u8]) -> Self {
        assert!(aes.block_size() == BLOCK_SIZE);
        let mut ige = IgeEncryptor {
            aes: aes,
            iv: unsafe { mem::uninitialized() },
        };
        unsafe { mem::transmute::<_, &mut [u8; 32]>(&mut ige.iv) }.clone_from_slice(iv);
        ige
    }
    
    pub fn encrypt_block(&mut self, input: &[u8], output: &mut [u8]) {
        debug_assert!(input.len() == BLOCK_SIZE);
        debug_assert!(output.len() >= BLOCK_SIZE);
        
        ige_enc_before(input, output, &self.iv);
        
        let temp_in = AesBlock::from_bytes(output);
        self.aes.encrypt_block(temp_in.as_bytes(), output);
        
        ige_enc_after(input, output, &mut self.iv);
    }
}

pub struct IgeDecryptor<T: BlockDecryptor> {
    aes: T,
    iv: IvBlock,
}

impl<T: BlockDecryptor> IgeDecryptor<T> {
    pub fn new(aes: T, iv: &[u8]) -> Self {
        assert!(aes.block_size() == BLOCK_SIZE);
        let mut ige = IgeDecryptor {
            aes: aes,
            iv: unsafe { mem::uninitialized() },
        };
        unsafe { mem::transmute::<_, &mut [u8; 32]>(&mut ige.iv) }.clone_from_slice(iv);
        ige
    }
    
    pub fn decrypt_block(&mut self, input: &[u8], output: &mut [u8]) {
        debug_assert!(input.len() == BLOCK_SIZE);
        debug_assert!(output.len() >= BLOCK_SIZE);
        
        let mut temp = AesBlock::uninitialized();
        ige_dec_before(input, &mut temp, &self.iv);
        
        self.aes.decrypt_block(temp.as_bytes(), output);
        
        ige_dec_after(input, output, &mut self.iv);
    }
}

#[derive(Copy, Clone)]
struct AesBlock {
    data: [u32; 4],
}

impl AesBlock {
    fn uninitialized() -> AesBlock {
        unsafe { mem::uninitialized() }
    }
    
    fn from_bytes(data: &[u8]) -> AesBlock {
        *AesBlock::from_ref(data)
    }
    
    fn from_ref(data: &[u8]) -> &AesBlock {
        unsafe { mem::transmute(&data[0]) }
    }
    
    fn from_ref_mut(data: &mut [u8]) -> &mut AesBlock {
        unsafe { mem::transmute(&mut data[0]) }
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

fn ige_dec_before(input: &[u8], temp: &mut AesBlock, iv: &IvBlock) {
    let inp = AesBlock::from_ref(input);
    
    for (iv, (i, t)) in iv.iv2.data.iter().zip(inp.data.iter().zip(temp.data.iter_mut())) {
        *t = *i ^ *iv;
    }
}

fn ige_dec_after(input: &[u8], output: &mut [u8], iv: &mut IvBlock) {
    let inp = AesBlock::from_ref(input);
    let outp = AesBlock::from_ref_mut(output);
    
    for (iv, o) in iv.iv1.data.iter().zip(outp.data.iter_mut()) {
        *o ^= *iv;
    }
    
    iv.iv1 = *inp;
    iv.iv2 = *outp;
}

