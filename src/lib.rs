//! Parsing and formatting of HTTP date values as defined in
//! [RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7).
//!
//! An HTTP date is an IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`.
//! The public API will be added here.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn new(buf: &str) -> Decoder<'_> {
        Decoder { buf, pos: 0 }
    }

    #[inline(always)]
    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    #[inline]
    fn byte_at(&self, idx: usize) -> Result<u8, DecodeError> {
        self.buf
            .as_bytes()
            .get(idx)
            .copied()
            .ok_or_else(|| DecodeError::new("Unexpected end of input"))
    }

    fn expect(&mut self, expected: u8) -> Result<(), DecodeError> {
        let actual = self.byte_at(self.pos)?;
        if actual == expected {
            self.advance(1);
            Ok(())
        } else {
            Err(DecodeError::new(format!(
                "Expected byte {}, but found {}",
                expected as char, actual as char
            )))
        }
    }

    #[inline(always)]
    fn space(&mut self) -> Result<(), DecodeError> {
        self.expect(b' ')
    }

    #[inline(always)]
    fn comma(&mut self) -> Result<(), DecodeError> {
        self.expect(b',')
    }

    #[inline(always)]
    fn colon(&mut self) -> Result<(), DecodeError> {
        self.expect(b':')
    }

    fn month(&mut self) -> Result<i32, DecodeError> {
        let m = self
            .buf
            .get(self.pos..self.pos + 3)
            .ok_or_else(|| DecodeError::new("Unexpected end of input"))?;
        let n = match m {
            "Jan" => 1,
            "Feb" => 2,
            "Mar" => 3,
            "Apr" => 4,
            "May" => 5,
            "Jun" => 6,
            "Jul" => 7,
            "Aug" => 8,
            "Sep" => 9,
            "Oct" => 10,
            "Nov" => 11,
            "Dec" => 12,
            _ => {
                return Err(DecodeError::new(format!("Invalid month value: {}", m)));
            }
        };
        self.advance(3);
        Ok(n)
    }

    fn digits(&mut self, n: usize) -> Result<i32, DecodeError> {
        let buf = self.buf.as_bytes();
        let mut value: i32 = 0;
        for i in 0..n {
            let pos = self.pos + i;
            match buf.get(pos) {
                Some(&c @ b'0'..=b'9') => {
                    value = value * 10 + (c - b'0') as i32;
                }
                Some(&c) => {
                    return Err(DecodeError::new(format!(
                        "Expected digit at position {}, but found {}",
                        pos, c as char
                    )));
                }
                None => return Err(DecodeError::new("Unexpected end of input")),
            }
        }
        self.advance(n);
        Ok(value)
    }

    fn year(&mut self) -> Result<i32, DecodeError> {
        self.digits(4)
    }

    fn day(&mut self) -> Result<i32, DecodeError> {
        self.digits(2)
    }
}

/* ------------------- tests ------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expect_matching_byte_advances_position() {
        let mut d = Decoder::new("abc");
        assert_eq!(d.expect(b'a'), Ok(()));
        assert_eq!(d.pos, 1);
    }

    #[test]
    fn expect_mismatched_byte_returns_error() {
        let mut d = Decoder::new("abc");
        let result = d.expect(b'b');
        assert!(result.is_err());
        assert_eq!(d.pos, 0); // Position should not advance on error
    }

    #[test]
    fn expect_at_end_of_input_returns_error() {
        let mut d = Decoder::new("ab");
        d.pos = 2;
        let err = d.expect(b'a').unwrap_err();
        assert_eq!(err.to_string(), "DecodeError: Unexpected end of input");
        assert_eq!(d.pos, 2);
    }

    #[test]
    fn parsing_punctuated_sequence_advances_through_input() {
        let mut d = Decoder::new("Sun, 06");
        // weekday(Sun)
        d.expect(b'S').unwrap();
        d.expect(b'u').unwrap();
        d.expect(b'n').unwrap();
        d.comma().unwrap();
        d.space().unwrap();
        // day-of-month(06)
        d.expect(b'0').unwrap();
        d.expect(b'6').unwrap();
        assert_eq!(d.pos, 7); // consumed "Sun, 06"
    }

    #[test]
    fn parsing_date_sequence_with_month_advances_through_input() {
        let mut d = Decoder::new("Sun, 06 Nov 1994");
        // weekday(Sun)
        d.expect(b'S').unwrap();
        d.expect(b'u').unwrap();
        d.expect(b'n').unwrap();
        d.comma().unwrap();
        d.space().unwrap();
        // day-of-month(06)
        assert_eq!(d.day().unwrap(), 6);
        d.space().unwrap();
        // month(Nov)
        let month = d.month().unwrap();
        assert_eq!(month, 11);
        d.space().unwrap();
        // year(1994)
        assert_eq!(d.year().unwrap(), 1994);
        assert_eq!(d.pos, 16); // consumed "Sun, 06 Nov 1994"
    }
}
