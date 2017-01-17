use std::cmp::min;
use std::io::{self, Read, Write};
use crypto::symmetriccipher::{BlockEncryptor, BlockDecryptor};

pub const BLOCK_SIZE: usize = 16;

pub trait IgeOperator {
    fn process(&mut self, input: &[u8], output: &mut [u8]);
}

pub struct IgeStream<S, I: IgeOperator> {
    stream: S,
    ige: I,
    buffer_pos: u8,
    buffer_filled: bool,
    buffer: [u8; BLOCK_SIZE],
}

impl<S, I: IgeOperator> IgeStream<S, I> {
    pub fn new(stream: S, ige: I) -> Self {
        IgeStream {
            stream: stream,
            ige: ige,
            buffer_pos: 0,
            buffer_filled: false,
            buffer: [0; BLOCK_SIZE],
        }
    }
}

impl<W: Write, I: IgeOperator> Write for IgeStream<W, I> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut start = 0;
        let mut ige_buf = [0; BLOCK_SIZE];
        let mut written = 0;

        if self.buffer_pos != 0 {
            let extra = BLOCK_SIZE - self.buffer_pos as usize;
            if buf.len() < extra {
                let buf_range = (self.buffer_pos as usize)..(self.buffer_pos as usize) + buf.len();
                self.buffer[buf_range].clone_from_slice(buf);
                self.buffer_pos += buf.len() as u8;
                return Ok(buf.len());
            } else {
                let buf_range = (self.buffer_pos as usize)..BLOCK_SIZE;
                self.buffer[buf_range].clone_from_slice(&buf[0..extra]);
                start = extra;
            }

            self.ige.process(&self.buffer, &mut ige_buf);
            try!(self.stream.write_all(&ige_buf));
            written += extra;

            self.buffer_pos = 0;
        }

        for chunk in buf[start..].chunks(BLOCK_SIZE) {
            if chunk.len() == BLOCK_SIZE {
                self.ige.process(chunk, &mut ige_buf);
                try!(self.stream.write_all(&ige_buf));
                written += BLOCK_SIZE;
            } else {
                self.buffer[0..chunk.len()].clone_from_slice(chunk);
                self.buffer_pos = chunk.len() as u8;
                written += chunk.len();
            }
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<R: Read, I: IgeOperator> Read for IgeStream<R, I> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let total = buf.len();
        let mut cursor = io::Cursor::new(buf);
        while (cursor.position() as usize) < total {
            let available = if self.buffer_filled { BLOCK_SIZE - (self.buffer_pos as usize) } else { 0 };
            if available != 0 {
                // Buffer contains decrypted data
                let amount = min(total - cursor.position() as usize, available);
                try!(cursor.write_all(&self.buffer[self.buffer_pos as usize..][0..amount]));
                self.buffer_pos += amount as u8;
            } else {
                // Buffer is empty, fill it with decrypted data
                let mut temp = [0; BLOCK_SIZE];
                self.buffer_filled = false;
                self.buffer_pos = 0;

                try!(self.stream.read_exact(&mut temp));
                self.ige.process(&temp, &mut self.buffer);
                self.buffer_filled = true;
            }
        }

        Ok(total)
    }
}

#[derive(Copy, Clone)]
pub struct IgeEncryptor<T: BlockEncryptor> {
    aes: T,
    iv: IvBlock,
}

impl<T: BlockEncryptor> IgeEncryptor<T> {
    pub fn new(aes: T, iv: &[u8]) -> Self {
        assert!(aes.block_size() == BLOCK_SIZE);
        IgeEncryptor {
            aes: aes,
            iv: IvBlock {
                iv1: AesBlock::from_bytes(&iv[0..BLOCK_SIZE]),
                iv2: AesBlock::from_bytes(&iv[BLOCK_SIZE..BLOCK_SIZE*2]),
            },
        }
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

impl<T: BlockEncryptor> IgeOperator for IgeEncryptor<T> {
    fn process(&mut self, input: &[u8], output: &mut [u8]) {
        self.encrypt_block(input, output)
    }
}

#[derive(Copy, Clone)]
pub struct IgeDecryptor<T: BlockDecryptor> {
    aes: T,
    iv: IvBlock,
}

impl<T: BlockDecryptor> IgeDecryptor<T> {
    pub fn new(aes: T, iv: &[u8]) -> Self {
        assert!(aes.block_size() == BLOCK_SIZE);
        IgeDecryptor {
            aes: aes,
            iv: IvBlock {
                iv1: AesBlock::from_bytes(&iv[0..BLOCK_SIZE]),
                iv2: AesBlock::from_bytes(&iv[BLOCK_SIZE..BLOCK_SIZE*2]),
            },
        }
    }

    pub fn decrypt_block(&mut self, input: &[u8], output: &mut [u8]) {
        debug_assert!(input.len() == BLOCK_SIZE);
        debug_assert!(output.len() >= BLOCK_SIZE);

        let mut temp = AesBlock::zeroed();
        ige_dec_before(input, &mut temp, &self.iv);

        self.aes.decrypt_block(temp.as_bytes(), output);

        ige_dec_after(input, output, &mut self.iv);
    }
}

impl<T: BlockDecryptor> IgeOperator for IgeDecryptor<T> {
    fn process(&mut self, input: &[u8], output: &mut [u8]) {
        self.decrypt_block(input, output)
    }
}

#[derive(Copy, Clone)]
struct AesBlock([u8; BLOCK_SIZE]);

impl AesBlock {
    fn zeroed() -> AesBlock {
        AesBlock([0; BLOCK_SIZE])
    }

    fn from_bytes(data: &[u8]) -> AesBlock {
        let mut ret = AesBlock::zeroed();
        ret.copy_from(data);
        ret
    }

    fn copy_from(&mut self, data: &[u8]) {
        self.0.copy_from_slice(data)
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Copy, Clone)]
struct IvBlock {
    iv1: AesBlock,
    iv2: AesBlock,
}

fn ige_enc_before(input: &[u8], output: &mut [u8], iv: &IvBlock) {
    for (iv, (i, o)) in iv.iv1.0.iter().zip(input.iter().zip(output.iter_mut())) {
        *o = *i ^ *iv;
    }
}

fn ige_enc_after(input: &[u8], output: &mut [u8], iv: &mut IvBlock) {
    for (iv, o) in iv.iv2.0.iter().zip(output.iter_mut()) {
        *o ^= *iv;
    }

    iv.iv1.copy_from(output);
    iv.iv2.copy_from(input);
}

fn ige_dec_before(input: &[u8], temp: &mut AesBlock, iv: &IvBlock) {
    for (iv, (i, t)) in iv.iv2.0.iter().zip(input.iter().zip(temp.0.iter_mut())) {
        *t = *i ^ *iv;
    }
}

fn ige_dec_after(input: &[u8], output: &mut [u8], iv: &mut IvBlock) {
    for (iv, o) in iv.iv1.0.iter().zip(output.iter_mut()) {
        *o ^= *iv;
    }

    iv.iv1.copy_from(input);
    iv.iv2.copy_from(output);
}
