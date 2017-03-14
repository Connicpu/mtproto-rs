use tl::dynamic::{LengthAndObject, TLCtorMap, TLObject};
use tl::{Bare, VEC_TYPE_ID};

#[derive(Debug, TLType)]
#[tl_id(_f35c6d01)]
pub struct RpcResult {
    pub req_msg_id: i64,
    pub result: Box<TLObject>,
}

#[derive(Debug, TLType)]
pub struct Message {
    pub msg_id: i64,
    pub seqno: i32,
    pub body: LengthAndObject,
}

#[derive(Debug, TLType)]
#[tl_id(_73f1f8dc)]
pub struct MessageContainer(pub Bare<Vec<Message>>);

#[derive(Debug, TLType)]
#[tl_id(_3072cfa1)]
pub struct GzipPacked(pub Vec<u8>);

pub type BareVec<T> = Bare<Vec<T>>;
pub type Int128 = (i64, i64);
pub type Int256 = (Int128, Int128);

#[doc(hidden)]
pub fn register_manual_ctors<R: ::tl::parsing::Reader>(cstore: &mut TLCtorMap<R>) {
    cstore.add::<GzipPacked>(GzipPacked::TYPE_ID);
    cstore.add::<MessageContainer>(MessageContainer::TYPE_ID);
    cstore.add::<RpcResult>(RpcResult::TYPE_ID);
    cstore.add::<Vec<Box<TLObject>>>(VEC_TYPE_ID);
}
