use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use thiserror::Error;

/// An enum representing a property list.
#[derive(Clone, Debug, PartialEq)]
pub enum Plist {
    Dictionary(HashMap<String, Plist>),
    Array(Vec<Plist>),
    String(String),
    Integer(i64),
    Float(f64),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected character {0}")]
    UnexpectedChar(char),
    #[error("unclosed string")]
    UnclosedString,
    #[error("unknown escape")]
    UnknownEscape,
    #[error("expected string")]
    NotAString,
    #[error("expected `=`")]
    ExpectedEquals,
    #[error("expected `,`")]
    ExpectedComma,
    #[error("expected `;`")]
    ExpectedSemicolon,
    #[error("in the event of this error, use hammer to break glass and escape")]
    SomethingWentWrong,
}

enum Token<'a> {
    Eof,
    OpenBrace,
    OpenParen,
    String(Cow<'a, str>),
    Atom(&'a str),
}

fn is_numeric(b: u8) -> bool {
    b.is_ascii_digit() || b == b'.' || b == b'-'
}

fn is_alnum(b: u8) -> bool {
    // https://github.com/opensource-apple/CF/blob/3cc41a76b1491f50813e28a4ec09954ffa359e6f/CFOldStylePList.c#L79
    is_numeric(b)
        || b.is_ascii_uppercase()
        || b.is_ascii_lowercase()
        || b == b'_'
        || b == b'$'
        || b == b'/'
        || b == b':'
        || b == b'.'
        || b == b'-'
}

// Used for serialization; make sure UUID's get quoted
fn is_alnum_strict(b: u8) -> bool {
    is_alnum(b) && b != b'-'
}

fn is_hex_upper(b: u8) -> bool {
    b.is_ascii_digit() || (b'A'..=b'F').contains(&b)
}

fn is_ascii_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\r' || b == b'\n'
}

fn numeric_ok(s: &str) -> bool {
    let s = s.as_bytes();
    if s.is_empty() {
        return false;
    }
    if s.iter().all(|&b| is_hex_upper(b)) && !s.iter().all(|&b| b.is_ascii_digit()) {
        return false;
    }
    if s.len() > 1 && s[0] == b'0' {
        return !s.iter().all(|&b| b.is_ascii_digit());
    }
    true
}

fn skip_ws(s: &str, mut ix: usize) -> usize {
    while ix < s.len() && is_ascii_whitespace(s.as_bytes()[ix]) {
        ix += 1;
    }
    ix
}

fn escape_string(buf: &mut String, s: &str) {
    if !s.is_empty() && s.as_bytes().iter().all(|&b| is_alnum_strict(b)) {
        // Strings can drop quotation marks if they're alphanumeric, but not if
        // they look like numbers.
        match s.parse::<f64>() {
            Ok(_) => {
                buf.push('"');
                buf.push_str(s);
                buf.push('"');
            }
            Err(_) => buf.push_str(s),
        }
    } else {
        buf.push('"');
        let mut start = 0;
        let mut ix = start;
        while ix < s.len() {
            let b = s.as_bytes()[ix];
            match b {
                b'"' | b'\\' => {
                    buf.push_str(&s[start..ix]);
                    buf.push('\\');
                    start = ix;
                }
                _ => (),
            }
            ix += 1;
        }
        buf.push_str(&s[start..]);
        buf.push('"');
    }
}

impl std::fmt::Display for Plist {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut s = String::new();
        self.push_to_string(&mut s);
        write!(f, "{s}")
    }
}

impl Plist {
    pub fn parse(s: &str) -> Result<Plist, Error> {
        let (plist, _ix) = Plist::parse_rec(s, 0)?;
        // TODO: check that we're actually at eof
        Ok(plist)
    }

    #[allow(unused)]
    pub fn as_dict(&self) -> Option<&HashMap<String, Plist>> {
        match self {
            Plist::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    #[allow(unused)]
    pub fn get(&self, key: &str) -> Option<&Plist> {
        match self {
            Plist::Dictionary(d) => d.get(key),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Plist]> {
        match self {
            Plist::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Plist::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Plist::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Plist::Integer(i) => Some(*i as f64),
            Plist::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn into_string(self) -> String {
        match self {
            Plist::String(s) => s,
            _ => panic!("expected string"),
        }
    }

    pub fn into_vec(self) -> Vec<Plist> {
        match self {
            Plist::Array(a) => a,
            _ => panic!("expected array"),
        }
    }

    pub fn into_hashmap(self) -> HashMap<String, Plist> {
        match self {
            Plist::Dictionary(d) => d,
            _ => panic!("expected dictionary"),
        }
    }

    fn parse_rec(s: &str, ix: usize) -> Result<(Plist, usize), Error> {
        let (tok, mut ix) = Token::lex(s, ix)?;
        match tok {
            Token::Atom(s) => Ok((Plist::parse_atom(s), ix)),
            Token::String(s) => Ok((Plist::String(s.into()), ix)),
            Token::OpenBrace => {
                let mut dict = HashMap::new();
                loop {
                    if let Some(ix) = Token::expect(s, ix, b'}') {
                        return Ok((Plist::Dictionary(dict), ix));
                    }
                    let (key, next) = Token::lex(s, ix)?;
                    let key_str = Token::try_into_string(key)?;
                    let next = Token::expect(s, next, b'=');
                    if next.is_none() {
                        return Err(Error::ExpectedEquals);
                    }
                    let (val, next) = Self::parse_rec(s, next.unwrap())?;
                    dict.insert(key_str, val);
                    if let Some(next) = Token::expect(s, next, b';') {
                        ix = next;
                    } else {
                        return Err(Error::ExpectedSemicolon);
                    }
                }
            }
            Token::OpenParen => {
                let mut list = Vec::new();
                if let Some(ix) = Token::expect(s, ix, b')') {
                    return Ok((Plist::Array(list), ix));
                }
                loop {
                    let (val, next) = Self::parse_rec(s, ix)?;
                    list.push(val);
                    if let Some(ix) = Token::expect(s, next, b')') {
                        return Ok((Plist::Array(list), ix));
                    }
                    if let Some(next) = Token::expect(s, next, b',') {
                        ix = next;
                    } else {
                        return Err(Error::ExpectedComma);
                    }
                }
            }
            _ => Err(Error::SomethingWentWrong),
        }
    }

    fn parse_atom(s: &str) -> Plist {
        if numeric_ok(s) {
            if let Ok(num) = s.parse() {
                return Plist::Integer(num);
            }
            if let Ok(num) = s.parse() {
                return Plist::Float(num);
            }
        }
        Plist::String(s.into())
    }

    fn push_to_string(&self, s: &mut String) {
        match self {
            Plist::Array(a) => {
                s.push('(');
                let mut delim = "\n";
                for el in a {
                    s.push_str(delim);
                    el.push_to_string(s);
                    delim = ",\n";
                }
                s.push_str("\n)");
            }
            Plist::Dictionary(a) => {
                s.push_str("{\n");
                let mut keys: Vec<_> = a.keys().collect();
                keys.sort();
                for k in keys {
                    let el = &a[k];
                    // TODO: quote if needed?
                    escape_string(s, k);
                    s.push_str(" = ");
                    el.push_to_string(s);
                    s.push_str(";\n");
                }
                s.push('}');
            }
            Plist::String(st) => escape_string(s, st),
            Plist::Integer(i) => write!(s, "{i}").unwrap(),
            Plist::Float(f) => write!(s, "{f}").unwrap(),
        }
    }
}

impl<'a> Token<'a> {
    fn lex(s: &'a str, ix: usize) -> Result<(Token<'a>, usize), Error> {
        let start = skip_ws(s, ix);
        if start == s.len() {
            return Ok((Token::Eof, start));
        }
        let b = s.as_bytes()[start];
        match b {
            b'{' => Ok((Token::OpenBrace, start + 1)),
            b'(' => Ok((Token::OpenParen, start + 1)),
            b'"' => {
                let mut ix = start + 1;
                let mut cow_start = ix;
                let mut buf = String::new();
                while ix < s.len() {
                    let b = s.as_bytes()[ix];
                    match b {
                        b'"' => {
                            // End of string
                            let string = if buf.is_empty() {
                                s[cow_start..ix].into()
                            } else {
                                buf.push_str(&s[cow_start..ix]);
                                buf.into()
                            };
                            return Ok((Token::String(string), ix + 1));
                        }
                        b'\\' => {
                            buf.push_str(&s[cow_start..ix]);
                            ix += 1;
                            if ix == s.len() {
                                return Err(Error::UnclosedString);
                            }
                            let b = s.as_bytes()[ix];
                            match b {
                                b'"' | b'\\' => cow_start = ix,
                                b'n' => {
                                    buf.push('\n');
                                    cow_start = ix + 1;
                                }
                                b'r' => {
                                    buf.push('\r');
                                    cow_start = ix + 1;
                                }
                                _ => {
                                    if (b'0'..=b'3').contains(&b) && ix + 2 < s.len() {
                                        // octal escape
                                        let b1 = s.as_bytes()[ix + 1];
                                        let b2 = s.as_bytes()[ix + 2];
                                        if (b'0'..=b'7').contains(&b1)
                                            && (b'0'..=b'7').contains(&b2)
                                        {
                                            let oct =
                                                (b - b'0') * 64 + (b1 - b'0') * 8 + (b2 - b'0');
                                            buf.push(oct as char);
                                            ix += 2;
                                            cow_start = ix + 1;
                                        } else {
                                            return Err(Error::UnknownEscape);
                                        }
                                    } else {
                                        return Err(Error::UnknownEscape);
                                    }
                                }
                            }
                            ix += 1;
                        }
                        _ => ix += 1,
                    }
                }
                Err(Error::UnclosedString)
            }
            _ => {
                if is_alnum(b) {
                    let mut ix = start + 1;
                    while ix < s.len() {
                        if !is_alnum(s.as_bytes()[ix]) {
                            break;
                        }
                        ix += 1;
                    }
                    Ok((Token::Atom(&s[start..ix]), ix))
                } else {
                    Err(Error::UnexpectedChar(s[start..].chars().next().unwrap()))
                }
            }
        }
    }

    fn try_into_string(self) -> Result<String, Error> {
        match self {
            Token::Atom(s) => Ok(s.into()),
            Token::String(s) => Ok(s.into()),
            _ => Err(Error::NotAString),
        }
    }

    fn expect(s: &str, ix: usize, delim: u8) -> Option<usize> {
        let ix = skip_ws(s, ix);
        if ix < s.len() {
            let b = s.as_bytes()[ix];
            if b == delim {
                return Some(ix + 1);
            }
        }
        None
    }
}

impl From<String> for Plist {
    fn from(x: String) -> Plist {
        Plist::String(x)
    }
}
impl From<u8> for Plist {
    fn from(x: u8) -> Plist {
        Plist::Integer(x as i64)
    }
}

impl From<i32> for Plist {
    fn from(x: i32) -> Plist {
        Plist::Integer(x as i64)
    }
}

impl From<i64> for Plist {
    fn from(x: i64) -> Plist {
        Plist::Integer(x)
    }
}

impl From<f64> for Plist {
    fn from(x: f64) -> Plist {
        Plist::Float(x)
    }
}

impl From<Vec<Plist>> for Plist {
    fn from(x: Vec<Plist>) -> Plist {
        Plist::Array(x)
    }
}

impl From<HashMap<String, Plist>> for Plist {
    fn from(x: HashMap<String, Plist>) -> Plist {
        Plist::Dictionary(x)
    }
}

// Macros from: https://github.com/ebarnard/rust-plist/blob/a7430c8a30521c7db7857d1619beb29b8595841d/src/macros.rs
// Adapted for this crate

/// Create a [`Dictionary`](crate::Dictionary) from a list of key-value pairs
///
/// ## Example
///
/// ```
/// # use glyphs_plist::{plist_dict, Plist};
/// let map = plist_dict! {
///     "a" => 1,
///     "b" => 2,
/// };
/// let Plist::Dictionary(map) = &map else {
///     unreachable!();
/// };
/// assert_eq!(map["a"], Plist::from(1));
/// assert_eq!(map["b"], Plist::from(2));
/// assert_eq!(map.get("c"), None);
/// ```
#[macro_export]
macro_rules! plist_dict {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$($crate::plist_dict!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { $crate::plist_dict!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let item_count = $crate::plist_dict!(@count $($key),*);
            let mut _dict = std::collections::HashMap::with_capacity(item_count);
            $(
                let _ = _dict.insert(::std::string::String::from($key), $crate::Plist::from($value));
            )*
            $crate::Plist::Dictionary(_dict)
        }
    };
}

/// Create a [`Plist::Array`] from a list of values
///
/// ## Example
///
/// ```
/// # use glyphs_plist::{plist_array, Plist};
/// let array = plist_array![1, 2];
/// assert_eq!(array, Plist::Array(vec![Plist::from(1), Plist::from(2)]));
///
/// let other_array = plist_array![String::from("hi"); 2];
/// assert_eq!(
///     other_array,
///     Plist::Array(vec![
///         Plist::from(String::from("hi")),
///         Plist::from(String::from("hi"))
///     ]),
/// );
/// ```
#[macro_export]
macro_rules! plist_array {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$($crate::plist_array!(@single $rest)),*]));

    ($($value:expr,)+) => { $crate::plist_array!($($value),+) };
    ($($value:expr),*) => {
        {
            let item_count = $crate::plist_array!(@count $($value),*);
            let mut _array = ::std::vec::Vec::with_capacity(item_count);
            $(
                _array.push($crate::Plist::from($value));
            )*
            $crate::Plist::Array(_array)
        }
    };

    ($value:expr; $n:expr) => ($crate::Plist::Array(::std::vec![$crate::Plist::from($value); $n]));
}

#[cfg(test)]
mod macro_tests {
    use crate::Plist;

    #[test]
    fn test_plist_dict() {
        let digits = plist_dict! {
            "one" => 1,
            "two" => 2,
        };
        let Plist::Dictionary(digits) = &digits else {
            panic!("wrong Plist variant, expected Plist::Dictionary, got {digits:?}");
        };
        assert_eq!(digits.len(), 2);
        assert_eq!(digits["one"], 1.into());
        assert_eq!(digits["two"], 2.into());

        let empty = plist_dict! {};
        let Plist::Dictionary(empty) = &empty else {
            panic!("wrong Plist variant, expected Plist::Dictionary, got {digits:?}");
        };
        assert!(empty.is_empty());

        let _nested_compiles = plist_dict! {
            "inner" => plist_dict! {
                "one" => 1,
                "two" => 2,
            },
        };
    }

    #[test]
    fn test_plist_array() {
        let digits = plist_array![1, 2, 3];
        let Plist::Array(digits) = &digits else {
            panic!("wrong Plist variant, expected Plist::Array, got {digits:?}");
        };
        assert_eq!(
            digits,
            &vec![Plist::from(1), Plist::from(2), Plist::from(3)],
        );

        let repeated = plist_array![1; 5];
        let Plist::Array(repeated) = &repeated else {
            panic!("wrong Plist variant, expected Plist::Array, got {repeated:?}");
        };
        assert_eq!(repeated, &vec![Plist::from(1); 5]);

        let empty = plist_array![];
        let Plist::Array(empty) = &empty else {
            panic!("wrong Plist variant, expected Plist::Array, got {empty:?}");
        };
        assert!(empty.is_empty());

        let _nested_compiles = plist_array![plist_array![1, 2, 3]];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Plist;

    use maplit::hashmap;
    use proptest::prelude::*;

    #[test]
    fn quoting() {
        let contents = r#"
        {
            name = "UFO Filename";
            value1 = ../../build/instance_ufos/Testing_Rg.ufo;
            value2 = _;
            value3 = $;
            value4 = /;
            value5 = :;
            value6 = .;
            value7 = -;
        }
        "#;

        let plist = Plist::parse(contents).unwrap();
        let plist_expected = Plist::Dictionary(hashmap! {
            "name".into() => String::from("UFO Filename").into(),
            "value1".into() => String::from("../../build/instance_ufos/Testing_Rg.ufo").into(),
            "value2".into() => String::from("_").into(),
            "value3".into() => String::from("$").into(),
            "value4".into() => String::from("/").into(),
            "value5".into() => String::from(":").into(),
            "value6".into() => String::from(".").into(),
            "value7".into() => String::from("-").into(),
        });
        assert_eq!(plist, plist_expected);
    }

    proptest! {
        #[test]
        fn escape_strings_float(num in proptest::num::f64::ANY) {
            let mut buf = String::new();
            let num_str = format!("{num}");
            escape_string(&mut buf, &num_str);

            assert_eq!(buf, format!("\"{num_str}\""));
        }
    }

    proptest! {
        #[test]
        fn escape_strings_int(num in proptest::num::i64::ANY) {
            let mut buf = String::new();
            let num_str = format!("{num}");
            escape_string(&mut buf, &num_str);

            assert_eq!(buf, format!("\"{num_str}\""));
        }
    }

    #[test]
    fn escape_strings_inf() {
        let mut buf = String::new();
        escape_string(&mut buf, "inf");
        assert_eq!(buf, "\"inf\"");

        buf.clear();
        escape_string(&mut buf, "-inf");
        assert_eq!(buf, "\"-inf\"");

        buf.clear();
        escape_string(&mut buf, "infinity");
        assert_eq!(buf, "\"infinity\"");

        buf.clear();
        escape_string(&mut buf, "-infinity");
        assert_eq!(buf, "\"-infinity\"");
    }
}
