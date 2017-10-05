extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate log;
extern crate mtproto;
extern crate serde_mtproto;

use mtproto::rpc::{AppInfo, Session};
use mtproto::rpc::encryption::AuthKey;
use mtproto::rpc::message::{Message, MessageType};
use mtproto::schema::FutureSalt;


mod error {
    error_chain! {
        links {
            MtProto(::mtproto::Error, ::mtproto::ErrorKind);
            SerdeMtProto(::serde_mtproto::Error, ::serde_mtproto::ErrorKind);
        }

        foreign_links {
            SetLogger(::log::SetLoggerError);
        }
    }
}


fn plain_text() -> error::Result<()> {
    let app_info = AppInfo::new(9000, "random text".to_owned());
    let mut session = Session::new(892103, app_info);

    let message = session.create_message(23, MessageType::PlainText)?;
    println!("{:#?}", message);
    let bytes = serde_mtproto::to_bytes(&message)?;
    println!("{:?}", bytes);
    let msg: Message<i32> = session.process_message(&bytes, None)?;
    println!("{:#?}", msg);

    assert_eq!(message, msg);

    Ok(())
}

fn encrypted() -> error::Result<()> {
    let app_info = AppInfo::new(9000, "random text".to_owned());
    let mut session = Session::new(892103, app_info);
    session.adopt_key(AuthKey::new(&[0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87])?);

    let future_salt = FutureSalt {
        valid_since: 0x0100_0000,
        valid_until: 0x0fff_ffff,
        salt: 0x1234_5678_90ab_cdef,
    };
    session.add_server_salts(vec![future_salt]);

    let message = session.create_message(23, MessageType::Encrypted)?;
    println!("{:?}", message);
    let bytes = serde_mtproto::to_bytes(&message)?;
    println!("{:?}", bytes);
    let msg: Message<i32> = session.process_message(&bytes, Some(24))?;
    println!("{:?}", msg);

    assert_eq!(message, msg);

    Ok(())
}

fn run() -> error::Result<()> {
    env_logger::init()?;

    plain_text()?;
    encrypted()?;

    Ok(())
}

quick_main!(run);
