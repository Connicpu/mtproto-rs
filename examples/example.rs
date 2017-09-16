extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate log;
extern crate mtproto;
extern crate serde_mtproto;

use mtproto::rpc::{AppId, Session};
use mtproto::rpc::message::{Message, MessageType};
use mtproto::rpc::encryption::AuthKey;


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


fn run() -> error::Result<()> {
    env_logger::init()?;

    let app_id = AppId::new(9000, "random text".to_owned());
    let mut session = Session::new(100500, app_id);
    session.adopt_key(AuthKey::default());

    let message = session.create_message(23, MessageType::PlainText)?;
    println!("{:?}", message);
    let bytes = serde_mtproto::to_bytes(&message)?;
    println!("{:?}", bytes);
    let msg: Message<i32> = session.process_message(&bytes)?;
    println!("{:?}", msg);

    assert_eq!(message, msg);

    Ok(())
}

quick_main!(run);
