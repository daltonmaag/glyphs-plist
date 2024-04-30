use std::collections::HashMap;
use thiserror::Error;

pub use glyphs_plist_derive::FromPlist;

use crate::plist::Plist;

// TODO: for macro hygiene, this trait should be moved to glyphs_plist_derive and just
//       re-exported by glyphs_plist
pub trait FromPlist {
    // Consider using result type; just unwrap for now.
    fn from_plist(plist: Plist) -> Self;
}

// TODO: this trait could (and should) be a private implementation detail to glyphs_plist_derive
pub trait FromPlistOpt {
    // Consider using result type; just unwrap for now.
    fn from_plist(plist: Option<Plist>) -> Self;
}

impl FromPlist for Plist {
    fn from_plist(plist: Plist) -> Self {
        plist
    }
}

// TODO: remove, below equivalent
impl FromPlist for String {
    fn from_plist(plist: Plist) -> Self {
        plist.into_string()
    }
}

impl From<Plist> for String {
    fn from(plist: Plist) -> Self {
        plist.into_string()
    }
}

// TODO: remove, below equivalent
impl FromPlist for bool {
    fn from_plist(plist: Plist) -> Self {
        // TODO: maybe error or warn on values other than 0, 1
        plist.as_i64().expect("expected integer") != 0
    }
}

#[derive(Debug, Error)]
pub enum BoolConversionError {
    #[error("can't convert non-integer plist value to bool")]
    WrongVariant,
    #[error("integer plist value wasn't 0 or 1: {0}")]
    BadNumber(i64),
}

impl TryFrom<Plist> for bool {
    type Error = BoolConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        plist
            .as_i64()
            .ok_or(BoolConversionError::WrongVariant)
            .and_then(|n| match n {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(BoolConversionError::BadNumber(n)),
            })
    }
}

// TODO: remove, below equivalent
impl FromPlist for i64 {
    fn from_plist(plist: Plist) -> Self {
        plist.as_i64().expect("expected integer")
    }
}

#[derive(Debug, Error)]
#[error("wrong variant, not {0}")]
pub struct VariantError(pub(crate) &'static str);

impl TryFrom<Plist> for i64 {
    type Error = VariantError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        plist.as_i64().ok_or(VariantError("integer"))
    }
}

// TODO: remove, below equivalent
impl FromPlist for u16 {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::Integer(wider) => wider
                .try_into()
                .expect("Integer '{:?}' is out-of-bounds of u16"),
            _ => panic!("Cannot parse u16 '{:?}'", plist),
        }
    }
}

#[derive(Debug, Error)]
pub enum DownsizeToU16Error {
    #[error("can't convert non-integer plist value to u16")]
    WrongVariant,
    #[error("{0} is out-of-bounds for a u16")]
    OutOfBounds(i64),
}

impl TryFrom<Plist> for u16 {
    type Error = DownsizeToU16Error;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        if let Plist::Integer(int) = plist {
            int.try_into()
                .map_err(|_| DownsizeToU16Error::OutOfBounds(int))
        } else {
            Err(DownsizeToU16Error::WrongVariant)
        }
    }
}

// TODO: remove, below equivalent
impl FromPlist for f64 {
    fn from_plist(plist: Plist) -> Self {
        plist.as_f64().expect("expected float")
    }
}

impl TryFrom<Plist> for f64 {
    type Error = VariantError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        plist.as_f64().ok_or(VariantError("float"))
    }
}

// TODO: remove, below equivalent
impl FromPlist for HashMap<String, Plist> {
    fn from_plist(plist: Plist) -> Self {
        plist.into_hashmap()
    }
}

impl From<Plist> for HashMap<String, Plist> {
    fn from(plist: Plist) -> Self {
        plist.into_hashmap()
    }
}

// TODO: remove, below equivalent
impl<T: FromPlist> FromPlist for Vec<T> {
    fn from_plist(plist: Plist) -> Self {
        let mut result = Vec::new();
        for element in plist.into_vec() {
            result.push(FromPlist::from_plist(element));
        }
        result
    }
}

impl<T: From<Plist>> From<Plist> for Vec<T> {
    fn from(plist: Plist) -> Self {
        plist.into_vec().into_iter().map(T::from).collect()
    }
}

// TODO: redundant by default impl<T, U> TryFrom<U> for T where U: Into<T>
impl<T: FromPlist> FromPlistOpt for T {
    fn from_plist(plist: Option<Plist>) -> Self {
        FromPlist::from_plist(plist.unwrap())
    }
}

impl<T: FromPlist> FromPlistOpt for Option<T> {
    fn from_plist(plist: Option<Plist>) -> Self {
        plist.map(FromPlist::from_plist)
    }
}
