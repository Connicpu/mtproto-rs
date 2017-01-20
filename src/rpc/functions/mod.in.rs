use tl::{Type, Vector};

#[derive(TLType)]
#[tl_id(_cb9f372d)]
pub struct InvokeAfterMsg<T: Type> {
    pub msg_id: i64,
    pub query: T,
}

#[derive(TLType)]
#[tl_id(_3dc4b4f0)]
pub struct InvokeAfterMsgs<T: Type> {
    pub msg_ids: Vector<i64>,
    pub query: T,
}

#[derive(TLType)]
#[tl_id(_da9b0d0d)]
pub struct InvokeWithLayer<T: Type> {
    pub layer: i32,
    pub query: T,
}

#[derive(TLType)]
#[tl_id(_69796de9)]
pub struct InitConnection<T: Type> {
    pub api_id: i32,
    pub device_model: String,
    pub system_version: String,
    pub app_version: String,
    pub lang_code: String,
    pub query: T,
}

pub mod auth {
    #[derive(TLType)]
    #[tl_id(_6fe51dfb)]
    pub struct CheckPhone {
        pub phone_number: String,
    }

    #[derive(TLType)]
    #[tl_id(_e300cc3b)]
    pub struct CheckedPhone {
        pub phone_registered: bool,
        pub phone_invited: bool,
    }
}

pub mod help {
    #[derive(TLType)]
    #[tl_id(_c4f9186b)]
    pub struct GetConfig;
}
