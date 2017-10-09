extern crate mtproto;

use std::thread::sleep;
use std::time::Duration;

use mtproto::rpc::{AppInfo, Session};
use mtproto::rpc::message::{Message, MessageType};


#[test]
fn test_next_message_id() {
    let mut session = Session::new(0, AppInfo::new(100, "foo hash".to_owned()));

    let mut prev_id = 0;
    for _ in 0..10 {
        for _ in 0..32_000 {
            let message = session.create_message(100, MessageType::PlainText).unwrap();
            match message {
                Message::PlainText { message_id, .. } => {
                    assert!(message_id > prev_id);
                    prev_id = message_id;
                },
                Message::Decrypted { .. } => unreachable!(),
            }
        }

        sleep(Duration::new(0, 25_000));
    }
}
