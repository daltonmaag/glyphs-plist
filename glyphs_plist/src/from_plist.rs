use std::collections::HashMap;

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

impl FromPlist for String {
    fn from_plist(plist: Plist) -> Self {
        plist.into_string()
    }
}

impl FromPlist for bool {
    fn from_plist(plist: Plist) -> Self {
        // TODO: maybe error or warn on values other than 0, 1
        plist.as_i64().expect("expected integer") != 0
    }
}

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

impl FromPlist for i64 {
    fn from_plist(plist: Plist) -> Self {
        plist.as_i64().expect("expected integer")
    }
}

impl FromPlist for f64 {
    fn from_plist(plist: Plist) -> Self {
        plist.as_f64().expect("expected float")
    }
}

impl FromPlist for HashMap<String, Plist> {
    fn from_plist(plist: Plist) -> Self {
        plist.into_hashmap()
    }
}

impl<T: FromPlist> FromPlist for Vec<T> {
    fn from_plist(plist: Plist) -> Self {
        let mut result = Vec::new();
        for element in plist.into_vec() {
            result.push(FromPlist::from_plist(element));
        }
        result
    }
}

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
