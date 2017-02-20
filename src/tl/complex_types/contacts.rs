use super::*;
use tl::Vector;

#[derive(Debug, TLType)]
pub enum ForeignLink {
    #[tl_id(_133421f8)] Unknown,
    #[tl_id(_a7801f47)] Requested {
        has_phone: bool,
    },
    #[tl_id(_1bea8ce1)] Mutual,
}

#[derive(Debug, TLType)]
pub enum MyLink {
    #[tl_id(_d22a1c60)] Empty,
    #[tl_id(_6c69efee)] Requested {
        contact: bool,
    },
    #[tl_id(_c240ebd9)] Contact,
}

#[derive(Debug, TLType)]
#[tl_id(_eccea3f5)]
pub struct Link {
    pub my_link: MyLink,
    pub foreign_link: ForeignLink,
    pub user: User,
}

#[derive(Debug, TLType)]
pub enum Contacts {
    #[tl_id(_b74ba9d2)] NotModified,
    #[tl_id(_6f8b8cb2)] Contacts {
        contacts: Vector<Contact>,
        users: Vector<User>,
    }
}

#[derive(Debug, TLType)]
#[tl_id(_ad524315)]
pub struct ImportedContacts {
    pub imported: Vector<ImportedContact>,
    pub retry_contacts: Vector<i64>,
    pub users: Vector<User>,
}

#[derive(Debug, TLType)]
pub enum Blocked {
    #[tl_id(_1c138d15)] Blocked {
        blocked: Vector<ContactBlocked>,
        users: Vector<User>,
    },
    #[tl_id(_900802a1)] Slice {
        count: i32,
        blocked: Vector<ContactBlocked>,
        users: Vector<User>,
    },
}

#[derive(Debug, TLType)]
#[tl_id(_5649dcc5)]
pub struct Suggested {
    pub results: Vector<ContactSuggested>,
    pub users: Vector<User>,
}

#[derive(Debug, TLType)]
#[tl_id(_566000e)]
pub struct Found {
    pub results: Vector<ContactFound>,
    pub users: Vector<User>,
}

// #[tl_id(_)]
