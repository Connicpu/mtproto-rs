#[macro_use]
extern crate log;
extern crate mtproto;
#[macro_use]
extern crate pretty_assertions;
extern crate serde_mtproto;
extern crate test_logger;


use std::thread::sleep;
use std::time::Duration;

use mtproto::rpc::{AppInfo, Message, Session};
use mtproto::rpc::encryption::AuthKey;
use mtproto::schema::FutureSalt;
use serde_mtproto::MtProtoSized;
use test_logger::ensure_env_logger_initialized;


#[test]
fn test_plain_text() {
    ensure_env_logger_initialized();

    let app_info = AppInfo::new(9000, "random text".to_owned());
    let session = Session::new(892103, app_info);

    let message = session.create_plain_text_message(23).unwrap();
    debug!("{:#?}", message);
    let bytes = serde_mtproto::to_bytes(&message).unwrap();
    debug!("{:?}", bytes);
    assert_eq!(bytes.len(), message.size_hint().unwrap());

    // Since the message is plain-text, we don't need the second parameter
    let msg: Message<i32> = session.process_message(&bytes, None).unwrap();
    debug!("{:#?}", msg);
    assert_eq!(message, msg);
}

#[test]
fn test_encrypted() {
    ensure_env_logger_initialized();

    let app_info = AppInfo::new(9000, "random text".to_owned());
    let mut session = Session::new(892103, app_info);
    session.adopt_key(AuthKey::new(&[0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87]).unwrap());

    let future_salt = FutureSalt {
        valid_since: 0x0100_0000,
        valid_until: 0x0fff_ffff,
        salt: 0x1234_5678_90ab_cdef,
    };
    session.add_server_salts(vec![future_salt]);

    let message = session.create_encrypted_message_no_acks(23).unwrap().unwrap();
    debug!("{:?}", message);
    let bytes = serde_mtproto::to_bytes(&message).unwrap();
    debug!("{:?}", bytes);
    assert_eq!(bytes.len(), message.size_hint().unwrap());

    // Pass number of bytes of encrypted data as second parameter
    let msg: Message<i32> = session.process_message(&bytes, Some(48)).unwrap();
    debug!("{:?}", msg);
    assert_eq!(message, msg);
}

#[test]
fn test_next_message_id_monotonicity() {
    let session = Session::new(0, AppInfo::new(100, "foo hash".to_owned()));

    let mut prev_id = 0;
    for _ in 0..10 {
        for _ in 0..32_000 {
            let message = session.create_plain_text_message(false).unwrap();

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
