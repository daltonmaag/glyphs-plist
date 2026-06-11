//! Lightweight library for reading and writing Glyphs font files.

mod font;
mod from_plist;
mod norad_interop;
mod plist;
mod to_plist;

pub use font::*;
pub use from_plist::{
    ArrayConversionError, BoolConversionError, DownsizeToU16Error, FromPlist,
    VariantError,
};
pub use plist::Plist;
pub use to_plist::ToPlist;
