use serde::ser::{self, Serialize};
use serde::de::{self, Deserialize};


#[derive(Debug)]
pub enum EitherRef<'a, T: 'a> {
    Ref(&'a T),
    Owned(T),
}

impl<'a, T: 'a> EitherRef<'a, T> {
    pub fn into_ref(self) -> Option<&'a T> {
        match self {
            EitherRef::Ref(r) => Some(r),
            _ => None,
        }
    }

    pub fn into_owned(self) -> Option<T> {
        match self {
            EitherRef::Owned(r) => Some(r),
            _ => None,
        }
    }
}

impl<'a, T: 'a + Serialize> Serialize for EitherRef<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        match *self {
            EitherRef::Ref(ref r) => r.serialize(serializer),
            EitherRef::Owned(ref o) => o.serialize(serializer),
        }
    }
}

impl<'a, 'de, T: 'a + Deserialize<'de>> Deserialize<'de> for EitherRef<'a, T> {
    fn deserialize<D>(deserializer: D) -> Result<EitherRef<'a, T>, D::Error>
        where D: de::Deserializer<'de>
    {
        Ok(EitherRef::Owned(T::deserialize(deserializer)?))
    }
}
