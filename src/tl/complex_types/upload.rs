use super::*;

#[derive(TLType)]
#[tl_id(_96a18d5)]
pub struct File {
    pub storage_type: storage::FileType,
    pub mtime: i32,
    pub bytes: Vec<u8>,
}

// #[tl_id(_)] 
