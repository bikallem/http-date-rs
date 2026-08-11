//! Parsing and formatting of HTTP date values as defined in
//! [RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7).
//!
//! An HTTP date is an IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`.
//! The public API will be added here.

pub struct DecodeError(String);

impl DecodeError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DecodeError: {}", self.0)
    }
}

impl std::error::Error for DecodeError {}

/* ------------------- decoder ------------------- */

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DaynNameTok {
    Short(DaynName),
    Long(DaynName),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DaynName {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PunctuationTok {
    Comma,
    Space,
}

struct Decoder<'a> {
    buf: &'a str,
    pos: usize,
}

impl Decoder<'_> {
    #[inline(always)]
    fn advance(&mut self, n: usize) {
        self.pos += n;
    }
}
