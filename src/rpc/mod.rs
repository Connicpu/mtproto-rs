use chrono::{UTC, Timelike};

pub struct Session {
    session_id: u64,
    server_salt: u64,
    message_id_seq: u16,
    message_id_last_nano: u16,
}

impl Session {
    pub fn new() -> Session {
        Session {
            session_id: 0,
            server_salt: 0,
            message_id_seq: 0,
            message_id_last_nano: 0,
        }
    }
    
    pub fn next_message_id(&mut self) -> u64 {
        let time = UTC::now();
        let timestamp = time.timestamp();
        let nano = time.nanosecond();
        
        let nano_bits = (nano >> 14) as u16 & 0xFFFC;
        // For the highly unlikely case that the nanosecond is the same
        if nano_bits == self.message_id_last_nano {
            self.message_id_seq += 1;
        } else {
            self.message_id_last_nano = nano_bits;
            self.message_id_seq = 0b0101010101010101; // too many zeroes = ignored, so
        }
        let id = self.message_id_seq;
        
        // [ lower 32-bits of unix time | bits 13..30 of the nanosecond (14 total) | id | 0b00 ]
        ((timestamp as u64) << 32) |
        ((nano_bits as u64) << 16) |
        ((id as u64) << 2)
    }
}

