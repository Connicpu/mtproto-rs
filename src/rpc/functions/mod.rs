use tl::Type;

#[derive(TLType)]
#[tl_id(_da9b0d0d)]
pub struct InvokeWithLayer<T: Type> {
    layer: i32,
    query: T,
}
