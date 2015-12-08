use chrono::{UTC, Timelike};

pub struct Session {
    session_id: u64,
    server_salt: u64,
    message_id_seq: u16,
}

impl Session {
    pub fn next_message_id(&mut self) -> u64 {
        let time = UTC::now();
        let timestamp = time.timestamp();
        let nano = time.nanosecond();
        let id = self.message_id_seq;
        self.message_id_seq += 1;
        
        ((timestamp as u64) << 32) |
        (((id as u64) << 16) & (0xFFFF0000)) |
        (((nano as u64) >> 14) & (0xFFFF ^ 0b11))
    }
}

