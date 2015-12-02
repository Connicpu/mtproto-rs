use std::io::{Read, Write};
use tl;
use tl::parsing::ConstructorId;
use tl::complex_types::auth;
use rpc::RpcContext;

pub const CHECK_PHONE: ConstructorId = ConstructorId(0x6fe51dfb);
pub const SEND_CODE: ConstructorId = ConstructorId(0x768d5f4d);
pub const SEND_CALL: ConstructorId = ConstructorId(0x3c51564);
pub const SIGN_UP: ConstructorId = ConstructorId(0x1b067634);
pub const SIGN_IN: ConstructorId = ConstructorId(0xbcd51581);
pub const LOG_OUT: ConstructorId = ConstructorId(0x5717da40);
pub const RESET_AUTHORIZATIONS: ConstructorId = ConstructorId(0x9fab0d1a);
pub const SEND_INVITES: ConstructorId = ConstructorId(0x771c1d97);
pub const EXPORT_AUTHORIZATION: ConstructorId = ConstructorId(0xe5bfffcd);
pub const IMPORT_AUTHORIZATION: ConstructorId = ConstructorId(0xe3ef9613);
pub const BIND_TEMP_AUTH_KEY: ConstructorId = ConstructorId(0xcdd42a05);
pub const SEND_SMS: ConstructorId = ConstructorId(0xda9f3e8);

pub fn check_phone<'a, R: Read, W: Write>(context: &'a mut RpcContext<'a, R, W>, phone_number: &str) -> tl::Result<auth::CheckedPhone> {
    let phone_number = tl::SendStr(phone_number);
    context.command(CHECK_PHONE).invoke((&phone_number,))
}

