use super::User;
use tl::Vector;

#[derive(TLType)]
pub enum Photos {
    #[tl_id(_8dca6aa5)] Photos {
        photos: Vector<super::Photo>,
        users: Vector<User>,
    },
    #[tl_id(_15051f54)] Slice {
        count: i32,
        photos: Vector<super::Photo>,
        users: Vector<User>,
    }
}

#[derive(TLType)]
#[tl_id(_20212ca8)]
pub struct Photo {
    pub photo: super::Photo,
    pub users: Vector<User>,
}

// #[tl_id(_)]
