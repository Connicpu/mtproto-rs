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

#[derive(Debug, TLType)]
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

pub mod authz {
    use tl::Vector;
    pub type Nonce = (u64, u64);

    #[derive(TLType)]
    #[tl_id(_60469778)]
    pub struct ReqPQ {
        pub nonce: Nonce,
    }

    #[derive(Debug, TLType)]
    #[tl_id(_05162463)]
    pub struct ResPQ {
        pub nonce: Nonce,
        pub server_nonce: Nonce,
        pub pq: Vec<u8>,
        pub server_public_key_fingerprints: Vector<u64>,
    }

    #[derive(Debug, TLType)]
    #[tl_id(_83c95aec)]
    pub struct PQInnerData {
        pub pq: Vec<u8>,
        pub p: Vec<u8>,
        pub q: Vec<u8>,
        pub nonce: Nonce,
        pub server_nonce: Nonce,
        pub new_nonce: (Nonce, Nonce),
    }

    #[derive(Debug, TLType)]
    #[tl_id(_d712e4be)]
    pub struct ReqDHParams {
        pub nonce: Nonce,
        pub server_nonce: Nonce,
        pub p: Vec<u8>,
        pub q: Vec<u8>,
        pub public_key_fingerprint: u64,
        pub encrypted_data: Vec<u8>,
    }

    #[derive(Debug, TLType)]
    pub enum ServerDHParams {
        #[tl_id(_79cb045d)] Fail {
            nonce: Nonce,
            server_nonce: Nonce,
            new_nonce_hash: Nonce,
        },
        #[tl_id(_d0e8075c)] Ok {
            nonce: Nonce,
            server_nonce: Nonce,
            encrypted_answer: Vec<u8>,
        },
    }

    #[derive(Debug, TLType)]
    #[tl_id(_b5890dba)]
    pub struct ServerDHInnerData {
        pub nonce: Nonce,
        pub server_nonce: Nonce,
        pub g: u32,
        pub dh_prime: Vec<u8>,
        pub g_a: Vec<u8>,
        pub server_time: u32,
    }

    #[derive(Debug, TLType)]
    #[tl_id(_6643b654)]
    pub struct ClientDHInnerData {
        pub nonce: Nonce,
        pub server_nonce: Nonce,
        pub retry_id: u64,
        pub g_b: Vec<u8>,
    }

    #[derive(Debug, TLType)]
    #[tl_id(_f5045f1f)]
    pub struct SetClientDHParams {
        pub nonce: Nonce,
        pub server_nonce: Nonce,
        pub encrypted_data: Vec<u8>,
    }

    #[derive(Debug, TLType)]
    pub enum SetClientDHParamsAnswer {
        #[tl_id(_3bcbf734)] Ok {
            nonce: Nonce,
            server_nonce: Nonce,
            new_nonce_hash: Nonce,
        },
        #[tl_id(_46dc1fb9)] Retry {
            nonce: Nonce,
            server_nonce: Nonce,
            new_nonce_hash: Nonce,
        },
        #[tl_id(_a69dae02)] Fail {
            nonce: Nonce,
            server_nonce: Nonce,
            new_nonce_hash: Nonce,
        },
    }
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

#[derive(Debug, TLType)]
#[tl_id(_b921bd04)]
pub struct GetFutureSalts {
    pub num: i32,
}
