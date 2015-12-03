use super::*;

pub enum ForeignLink {
    Unknown,
    Requested {
        has_phone: bool,
    },
    Mutual,
}