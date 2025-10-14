use std::collections::HashMap;

pub use glyphs_plist_derive::FromPlist;
use thiserror::Error;

use crate::plist::Plist;

impl From<Plist> for String {
    fn from(plist: Plist) -> Self {
        plist.into_string()
    }
}

#[derive(Debug, Error)]
pub enum BoolConversionError {
    #[error("can't convert non-integer plist value to bool; got: {0:#?}")]
    WrongVariant(Plist),
    #[error("integer plist value wasn't 0 or 1: {0}")]
    BadNumber(i64),
}

impl TryFrom<Plist> for bool {
    type Error = BoolConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let convert_number = |n| match n {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(BoolConversionError::BadNumber(n)),
        };

        match &plist {
            Plist::Integer(n) => convert_number(*n),
            Plist::String(s) => match s.parse::<i64>() {
                Ok(n) => convert_number(n),
                Err(_) => Err(BoolConversionError::WrongVariant(plist)),
            },
            _ => Err(BoolConversionError::WrongVariant(plist)),
        }
    }
}

#[derive(Debug, Error)]
#[error("expected {0}, got {1:#?}")]
pub struct VariantError(pub(crate) &'static str, Plist);

impl TryFrom<Plist> for i64 {
    type Error = VariantError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match &plist {
            Plist::Integer(n) => Ok(*n),
            Plist::String(s) => {
                s.parse::<i64>().map_err(|_| VariantError("integer", plist))
            },
            _ => Err(VariantError("integer", plist)),
        }
    }
}

#[derive(Debug, Error)]
pub enum DownsizeToU16Error {
    #[error("can't convert non-integer plist value to u16; got: {0:#?}")]
    WrongVariant(Plist),
    #[error("{0} is out-of-bounds for a u16")]
    OutOfBounds(i64),
}

impl TryFrom<Plist> for u16 {
    type Error = DownsizeToU16Error;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let convert_number = |int: i64| {
            int.try_into()
                .map_err(|_| DownsizeToU16Error::OutOfBounds(int))
        };

        match &plist {
            Plist::Integer(n) => convert_number(*n),
            Plist::String(s) => {
                let n: i64 = s
                    .parse::<i64>()
                    .map_err(|_| DownsizeToU16Error::WrongVariant(plist))?;
                convert_number(n)
            },
            _ => Err(DownsizeToU16Error::WrongVariant(plist)),
        }
    }
}

impl TryFrom<Plist> for f64 {
    type Error = VariantError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match &plist {
            Plist::Integer(i) => Ok(*i as f64),
            Plist::Float(f) => Ok(*f),
            Plist::String(s) => {
                s.parse::<f64>().map_err(|_| VariantError("float", plist))
            },
            _ => Err(VariantError("float", plist)),
        }
    }
}

impl From<Plist> for HashMap<String, Plist> {
    fn from(plist: Plist) -> Self {
        plist.into_hashmap()
    }
}

#[derive(Debug, Error)]
pub enum ArrayConversionError<E: std::error::Error> {
    #[error("expected array")]
    WrongVariant,
    #[error(transparent)]
    Element(#[from] E),
}

impl<T> TryFrom<Plist> for Vec<T>
where
    T: TryFrom<Plist>,
    T::Error: std::error::Error,
{
    type Error = ArrayConversionError<T::Error>;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::Array(array) => array
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<_, _>>()
                .map_err(ArrayConversionError::Element),
            _ => Err(ArrayConversionError::WrongVariant),
        }
    }
}
