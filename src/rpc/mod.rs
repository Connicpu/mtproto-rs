use chrono::{UTC, Timelike};

pub struct Session {
    session_id: u64,
    server_salt: u64,
    message_id_seq: u16,
    message_id_last_nano: u16,
}

impl Session {
    pub fn next_message_id(&mut self) -> u64 {
        let time = UTC::now();
        let timestamp = time.timestamp();
        let nano = time.nanosecond();
        
        let nano_bits = (nano >> 14) as u16 & 0xFFFC;
        let id = if nano_bits == self.message_id_last_nano {
            let id = self.message_id_seq;
            self.message_id_seq += 1;
            id
        } else {
            self.message_id_seq = 0;
            0
        };
        
        // [ lower 32-bits of unix time | bits 13..30 of the nanosecond (14 total) | id | 0b00 ]
        ((timestamp as u64) << 32) |
        ((nano_bits as u64) << 16)
        ((id as u64) << 2)
    }
}

