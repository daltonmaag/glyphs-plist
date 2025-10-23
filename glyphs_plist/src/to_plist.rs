use std::collections::HashMap;

pub use glyphs_plist_derive::ToPlist;

use crate::plist::Plist;

// TODO: for macro hygiene, this trait should be moved to glyphs_plist_derive
// and just       re-exported by glyphs_plist
pub trait ToPlist {
    fn to_plist(self) -> Plist;
}

// TODO: this trait could (and should) be a private implementation detail to
// glyphs_plist_derive
pub trait ToPlistOpt {
    fn to_plist(self) -> Option<Plist>;
}

impl ToPlist for Plist {
    fn to_plist(self) -> Plist {
        self
    }
}

impl ToPlist for String {
    fn to_plist(self) -> Plist {
        self.into()
    }
}

impl ToPlist for bool {
    fn to_plist(self) -> Plist {
        (self as i64).into()
    }
}

impl ToPlist for u16 {
    fn to_plist(self) -> Plist {
        Plist::Integer(self.into())
    }
}

impl ToPlist for i64 {
    fn to_plist(self) -> Plist {
        self.into()
    }
}

impl ToPlist for f64 {
    fn to_plist(self) -> Plist {
        // Opportunistically output integers.
        if (self - self.round()).abs() < f64::EPSILON {
            Plist::Integer(self.round() as i64)
        } else {
            self.into()
        }
    }
}

impl ToPlist for HashMap<String, Plist> {
    fn to_plist(self) -> Plist {
        self.into()
    }
}

impl<T: ToPlist> ToPlist for Vec<T> {
    fn to_plist(self) -> Plist {
        let mut result = Vec::new();
        for element in self {
            result.push(ToPlist::to_plist(element));
        }
        result.into()
    }
}

impl<T: ToPlist> ToPlistOpt for T {
    fn to_plist(self) -> Option<Plist> {
        Some(ToPlist::to_plist(self))
    }
}

impl<T: ToPlist> ToPlistOpt for Option<T> {
    fn to_plist(self) -> Option<Plist> {
        self.map(ToPlist::to_plist)
    }
}
