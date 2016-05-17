use tl::{Type, Vector};

#[derive(TLType)]
#[tl_id(_cb9f372d)]
pub struct InvokeAfterMsg<T: Type> {
    msg_id: i64,
    query: T,
}

#[derive(TLType)]
#[tl_id(_3dc4b4f0)]
pub struct InvokeAfterMsgs<T: Type> {
    msg_ids: Vector<i64>,
    query: T,
}

#[derive(TLType)]
#[tl_id(_da9b0d0d)]
pub struct InvokeWithLayer<T: Type> {
    layer: i32,
    query: T,
}

