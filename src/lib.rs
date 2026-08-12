//! Parsing and formatting of HTTP date values as defined in
//! [RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7).
//!
//! An HTTP date can appear in one of three formats — IMF-fixdate, RFC 850, or
//! asctime. Use [`decode`] to parse a value into a [`HttpDate`]:
//!
//! ```
//! use http_date::{decode, HttpDate};
//!
//! let date = decode("Sun, 06 Nov 1994 08:49:37 GMT")
//!     .expect("valid IMF-fixdate");
//! assert!(matches!(date, HttpDate::ImfFixdate(_)));
//! ```
//!
//! The decoded components are available on the [`DateTime`] inside the
//! returned [`HttpDate`].

use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DayNameTok {
    Short(DayName),
    Long(DayName),
}

/// The day of the week as used in HTTP date values.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DayName {
    /// Monday.
    Mon,
    /// Tuesday.
    Tue,
    /// Wednesday.
    Wed,
    /// Thursday.
    Thu,
    /// Friday.
    Fri,
    /// Saturday.
    Sat,
    /// Sunday.
    Sun,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PunctuationTok {
    Comma,
    Space,
}

/// The calendar date portion of an HTTP date.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Date {
    /// The year, e.g. `1994`.
    pub year: i32,
    /// The month, 1–12, where 1 is January.
    pub month: i32,
    /// The day of the month, 1–31.
    pub day: i32,
}

/// The time-of-day portion of an HTTP date.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Time {
    /// The hour, 0–23.
    pub hour: i32,
    /// The minute, 0–59.
    pub minute: i32,
    /// The second, 0–59.
    pub second: i32,
}

/// A fully decoded HTTP date: weekday, calendar date, and time of day.
///
/// This is the value carried by the [`HttpDate`] variants.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DateTime {
    /// The day of the week.
    pub dayname: DayName,
    /// The calendar date.
    pub date: Date,
    /// The time of day.
    pub time: Time,
}

/// An HTTP date in one of the three formats defined in
/// [RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7).
///
/// The variant indicates which textual format the value was parsed from;
/// the contained [`DateTime`] holds the decoded components.
pub enum HttpDate {
    /// IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`.
    ImfFixdate(DateTime),
    /// RFC 850 date, e.g. `Sunday, 06-Nov-94 08:49:37 GMT`.
    Rfc850(DateTime),
    /// asctime date, e.g. `Sun Nov  6 08:49:37 1994`.
    Asctime(DateTime),
}

/* ------------------- decoder ------------------- */
/// An error returned when an HTTP date cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(String);

impl DecodeError {
    /// Constructs a new decode error with the given message.
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

/// A streaming parser for HTTP date values, used internally by [`decode`].
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

    fn string(&mut self) -> String {
        let mut out = String::with_capacity(5);
        while self.pos < self.buf.len() {
            let c = self.buf.as_bytes()[self.pos];
            if !c.is_ascii_alphabetic() {
                break;
            }
            out.push(c as char);
            self.advance(1);
        }
        out
    }

    fn dayname_tok(&mut self) -> Result<DayNameTok, DecodeError> {
        let s = self.string();
        let dayname = match s.as_str() {
            "Mon" => DayNameTok::Short(DayName::Mon),
            "Tue" => DayNameTok::Short(DayName::Tue),
            "Wed" => DayNameTok::Short(DayName::Wed),
            "Thu" => DayNameTok::Short(DayName::Thu),
            "Fri" => DayNameTok::Short(DayName::Fri),
            "Sat" => DayNameTok::Short(DayName::Sat),
            "Sun" => DayNameTok::Short(DayName::Sun),
            "Monday" => DayNameTok::Long(DayName::Mon),
            "Tuesday" => DayNameTok::Long(DayName::Tue),
            "Wednesday" => DayNameTok::Long(DayName::Wed),
            "Thursday" => DayNameTok::Long(DayName::Thu),
            "Friday" => DayNameTok::Long(DayName::Fri),
            "Saturday" => DayNameTok::Long(DayName::Sat),
            "Sunday" => DayNameTok::Long(DayName::Sun),
            _ => return Err(DecodeError::new(format!("Invalid day name: {}", s))),
        };
        Ok(dayname)
    }

    fn punctuation_tok(&mut self) -> Result<PunctuationTok, DecodeError> {
        let c = self.byte_at(self.pos)?;
        let tok = match c {
            b',' => PunctuationTok::Comma,
            b' ' => PunctuationTok::Space,
            _ => {
                return Err(DecodeError::new(format!(
                    "Expected punctuation at position {}, but found {}",
                    self.pos, c as char
                )));
            }
        };
        self.advance(1);
        Ok(tok)
    }

    fn date1(&mut self) -> Result<Date, DecodeError> {
        let day = self.day()?;
        self.space()?;
        let month = self.month()?;
        self.space()?;
        let year = self.year()?;
        Ok(Date { year, month, day })
    }

    fn time(&mut self) -> Result<Time, DecodeError> {
        let hour = self.digits(2)?;
        self.colon()?;
        let minute = self.digits(2)?;
        self.colon()?;
        let second = self.digits(2)?;
        Ok(Time {
            hour,
            minute,
            second,
        })
    }

    fn gmt(&mut self) -> Result<(), DecodeError> {
        let s = self.string();
        if s != "GMT" {
            return Err(DecodeError::new(format!(
                "Expected 'GMT', but found '{}'",
                s
            )));
        }
        Ok(())
    }

    // IMF-fixdate: day-name "," SP date1 SP time SP "GMT"
    fn imf_fixdate(&mut self, dayname: DayName) -> Result<HttpDate, DecodeError> {
        self.space()?;
        let date = self.date1()?;
        self.space()?;
        let time = self.time()?;
        self.space()?;
        self.gmt()?;
        Ok(HttpDate::ImfFixdate(DateTime {
            dayname,
            date,
            time,
        }))
    }

    fn date2(&mut self) -> Result<Date, DecodeError> {
        let day = self.day()?;
        self.expect(b'-')?;
        let month = self.month()?;
        self.expect(b'-')?;
        let year = self.digits(2)?;
        Ok(Date { year, month, day })
    }

    // RFC 850 date: day-name "," SP date2 SP time SP "GMT"
    fn rfc850_date(&mut self, dayname: DayName) -> Result<HttpDate, DecodeError> {
        self.comma()?;
        self.space()?;
        let date = self.date2()?;
        self.space()?;
        let time = self.time()?;
        self.space()?;
        self.gmt()?;
        Ok(HttpDate::Rfc850(DateTime {
            dayname,
            date,
            time,
        }))
    }

    fn date3(&mut self) -> Result<(i32, i32), DecodeError> {
        // month
        let m = self.month()?;
        self.space()?;
        // day
        let d = match self.byte_at(self.pos)? {
            b' ' => {
                self.space()?;
                self.digits(1)?
            }
            _ => self.digits(2)?,
        };
        Ok((m, d))
    }

    // asctime date: day-name SP month SP (2DIGIT / (SP 1DIGIT)) SP time SP 4DIGIT
    fn asctime_date(&mut self, dayname: DayName) -> Result<HttpDate, DecodeError> {
        let (month, day) = self.date3()?;
        self.space()?;
        let time = self.time()?;
        self.space()?;
        let year = self.year()?;
        Ok(HttpDate::Asctime(DateTime {
            dayname,
            date: Date { year, month, day },
            time,
        }))
    }
}

/// Parses an HTTP date from its textual representation.
///
/// Accepts any of the three date formats defined in
/// [RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7):
///
/// | Format      | Example                          |
/// |-------------|----------------------------------|
/// | IMF-fixdate | `Sun, 06 Nov 1994 08:49:37 GMT`  |
/// | RFC 850     | `Sunday, 06-Nov-94 08:49:37 GMT` |
/// | asctime     | `Sun Nov  6 08:49:37 1994`       |
///
/// # Errors
///
/// Returns a [`DecodeError`] if `buf` is not a well-formed HTTP date, for
/// example when the day name or month is unknown, a required separator is
/// missing, or the input is truncated.
///
/// # Examples
///
/// An IMF-fixdate (the format HTTP servers must emit):
///
/// ```
/// use http_date::{decode, HttpDate};
///
/// let date = decode("Sun, 06 Nov 1994 08:49:37 GMT")
///     .expect("valid IMF-fixdate");
/// assert!(matches!(date, HttpDate::ImfFixdate(_)));
/// ```
///
/// An RFC 850 date:
///
/// ```
/// use http_date::{decode, HttpDate};
///
/// let date = decode("Sunday, 06-Nov-94 08:49:37 GMT")
///     .expect("valid RFC 850 date");
/// assert!(matches!(date, HttpDate::Rfc850(_)));
/// ```
///
/// An asctime date:
///
/// ```
/// use http_date::{decode, HttpDate};
///
/// let date = decode("Sun Nov  6 08:49:37 1994")
///     .expect("valid asctime date");
/// assert!(matches!(date, HttpDate::Asctime(_)));
/// ```
///
/// Malformed input is rejected:
///
/// ```
/// use http_date::decode;
///
/// assert!(decode("not a date").is_err());
/// ```
pub fn decode(buf: &str) -> Result<HttpDate, DecodeError> {
    let mut decoder = Decoder::new(buf);
    match decoder.dayname_tok()? {
        DayNameTok::Long(dayname) => decoder.rfc850_date(dayname),
        DayNameTok::Short(dayname) => match decoder.punctuation_tok()? {
            PunctuationTok::Comma => decoder.imf_fixdate(dayname),
            PunctuationTok::Space => decoder.asctime_date(dayname),
        },
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

    #[test]
    fn string_reads_alphabetic_run_and_advances() {
        let mut d = Decoder::new("Sun, 06");
        assert_eq!(d.string(), "Sun");
        assert_eq!(d.pos, 3); // consumed "Sun"
    }

    #[test]
    fn string_reads_to_end_of_input() {
        let mut d = Decoder::new("GMT");
        assert_eq!(d.string(), "GMT");
        assert_eq!(d.pos, 3); // consumed "GMT"
    }

    #[test]
    fn string_returns_empty_when_current_byte_is_not_alphabetic() {
        let mut d = Decoder::new("123");
        assert_eq!(d.string(), "");
        assert_eq!(d.pos, 0); // position should not advance
    }

    #[test]
    fn string_returns_empty_at_end_of_input() {
        let mut d = Decoder::new("Sun,");
        d.pos = 4; // position at end of input
        assert_eq!(d.string(), "");
        assert_eq!(d.pos, 4); // position should not advance
    }

    // Test cases for dayname_tok

    #[test]
    fn dayname_tok_parses_all_names() {
        let cases = [
            ("Mon", DayNameTok::Short(DayName::Mon)),
            ("Tue", DayNameTok::Short(DayName::Tue)),
            ("Wed", DayNameTok::Short(DayName::Wed)),
            ("Thu", DayNameTok::Short(DayName::Thu)),
            ("Fri", DayNameTok::Short(DayName::Fri)),
            ("Sat", DayNameTok::Short(DayName::Sat)),
            ("Sun", DayNameTok::Short(DayName::Sun)),
            ("Monday", DayNameTok::Long(DayName::Mon)),
            ("Tuesday", DayNameTok::Long(DayName::Tue)),
            ("Wednesday", DayNameTok::Long(DayName::Wed)),
            ("Thursday", DayNameTok::Long(DayName::Thu)),
            ("Friday", DayNameTok::Long(DayName::Fri)),
            ("Saturday", DayNameTok::Long(DayName::Sat)),
            ("Sunday", DayNameTok::Long(DayName::Sun)),
        ];
        for (s, expected) in cases {
            let mut d = Decoder::new(s);
            assert_eq!(d.dayname_tok(), Ok(expected), "day name {s}");
            assert_eq!(d.pos, s.len(), "pos after {s}");
        }
    }

    #[test]
    fn dayname_tok_rejects_invalid_name() {
        let mut d = Decoder::new("Funday");
        let result = d.dayname_tok();
        assert!(result.is_err());
        assert_eq!(d.pos, 6); // consumed the invalid name
    }

    #[test]
    fn dayname_tok_is_case_sensitive() {
        let mut d = Decoder::new("mon");
        let result = d.dayname_tok();
        assert!(result.is_err());
        assert_eq!(d.pos, 3); // consumed the invalid name
    }

    #[test]
    fn dayname_tok_rejects_empty_run() {
        let mut d = Decoder::new(", 06 Nov 1994");
        let result = d.dayname_tok();
        assert!(result.is_err());
        assert_eq!(d.pos, 0); // position should not advance
    }

    #[test]
    fn dayname_tok_rejects_truncated_long_name() {
        let mut d = Decoder::new("Wednes");
        let result = d.dayname_tok();
        assert!(result.is_err());
        assert_eq!(d.pos, 6); // consumed the partial name
    }

    // Test cases for punctuation_tok

    #[test]
    fn punctuation_tok_parses_comma() {
        let mut d = Decoder::new(",06 Nov");
        assert_eq!(d.punctuation_tok(), Ok(PunctuationTok::Comma));
        assert_eq!(d.pos, 1);
    }

    #[test]
    fn punctuation_tok_parses_space() {
        let mut d = Decoder::new(" 06 Nov");
        assert_eq!(d.punctuation_tok(), Ok(PunctuationTok::Space));
        assert_eq!(d.pos, 1);
    }

    #[test]
    fn punctuation_tok_rejects_non_punctuation() {
        let mut d = Decoder::new("06 Nov");
        let err = d.punctuation_tok().unwrap_err();
        assert!(
            err.to_string()
                .contains("Expected punctuation at position 0, but found 0")
        );
        assert_eq!(d.pos, 0); // position unchanged on error
    }

    #[test]
    fn punctuation_tok_at_end_of_input_returns_error() {
        let mut d = Decoder::new("06");
        d.pos = 2; // at end of input
        let err = d.punctuation_tok().unwrap_err();
        assert_eq!(err.to_string(), "DecodeError: Unexpected end of input");
        assert_eq!(d.pos, 2);
    }

    // Test cases for date1

    #[test]
    fn date1_parses_valid_date() {
        let mut d = Decoder::new("06 Nov 1994");
        assert_eq!(
            d.date1(),
            Ok(Date {
                year: 1994,
                month: 11,
                day: 6
            })
        );
        assert_eq!(d.pos, 11); // consumed "06 Nov 1994"
    }

    #[test]
    fn date1_parses_leading_zero_day() {
        let mut d = Decoder::new("01 Jan 2000");
        assert_eq!(
            d.date1(),
            Ok(Date {
                year: 2000,
                month: 1,
                day: 1
            })
        );
        assert_eq!(d.pos, 11);
    }

    #[test]
    fn date1_rejects_non_digit_day() {
        let mut d = Decoder::new("0x Nov 1994");
        let err = d.date1().unwrap_err();
        assert!(
            err.to_string()
                .contains("Expected digit at position 1, but found x")
        );
    }

    #[test]
    fn date1_rejects_missing_space_after_day() {
        let mut d = Decoder::new("06Nov 1994");
        let err = d.date1().unwrap_err();
        assert!(err.to_string().contains("but found N"));
    }

    #[test]
    fn date1_rejects_invalid_month() {
        let mut d = Decoder::new("06 Xyz 1994");
        let err = d.date1().unwrap_err();
        assert!(err.to_string().contains("Invalid month value: Xyz"));
    }

    #[test]
    fn date1_rejects_truncated_year() {
        let mut d = Decoder::new("06 Nov 19");
        let err = d.date1().unwrap_err();
        assert_eq!(err.to_string(), "DecodeError: Unexpected end of input");
    }

    #[test]
    fn time_parses_valid_time() {
        let mut d = Decoder::new("08:49:37 GMT");
        assert_eq!(
            d.time(),
            Ok(Time {
                hour: 8,
                minute: 49,
                second: 37
            })
        );
        assert_eq!(d.pos, 8); // consumed "08:49:37"
    }

    #[test]
    fn time_rejects_missing_colon() {
        let mut d = Decoder::new("0849:37");
        let err = d.time().unwrap_err();
        assert!(err.to_string().contains("but found 4"));
    }

    #[test]
    fn time_rejects_non_digit_minute() {
        let mut d = Decoder::new("08:x9:37");
        let err = d.time().unwrap_err();
        assert!(
            err.to_string()
                .contains("Expected digit at position 3, but found x")
        );
    }

    #[test]
    fn time_rejects_truncated_seconds() {
        let mut d = Decoder::new("08:49:3");
        let err = d.time().unwrap_err();
        assert_eq!(err.to_string(), "DecodeError: Unexpected end of input");
    }

    // Test cases for decode

    fn expect_imf_fixdate(decoded: &HttpDate) -> DateTime {
        match decoded {
            HttpDate::ImfFixdate(dt) => *dt,
            _ => panic!("expected ImfFixdate"),
        }
    }

    fn expect_rfc850(decoded: &HttpDate) -> DateTime {
        match decoded {
            HttpDate::Rfc850(dt) => *dt,
            _ => panic!("expected Rfc850"),
        }
    }

    fn expect_asctime(decoded: &HttpDate) -> DateTime {
        match decoded {
            HttpDate::Asctime(dt) => *dt,
            _ => panic!("expected Asctime"),
        }
    }

    #[test]
    fn decode_parses_imf_fixdate() {
        let dt = expect_imf_fixdate(&decode("Sun, 06 Nov 1994 08:49:37 GMT").unwrap());
        assert_eq!(dt.dayname, DayName::Sun);
        assert_eq!(
            dt.date,
            Date {
                year: 1994,
                month: 11,
                day: 6
            }
        );
        assert_eq!(
            dt.time,
            Time {
                hour: 8,
                minute: 49,
                second: 37
            }
        );
    }

    #[test]
    fn decode_parses_rfc850_date() {
        let dt = expect_rfc850(&decode("Sunday, 06-Nov-94 08:49:37 GMT").unwrap());
        assert_eq!(dt.dayname, DayName::Sun);
        assert_eq!(
            dt.date,
            Date {
                year: 94,
                month: 11,
                day: 6
            }
        );
        assert_eq!(
            dt.time,
            Time {
                hour: 8,
                minute: 49,
                second: 37
            }
        );
    }

    #[test]
    fn decode_parses_asctime() {
        let dt = expect_asctime(&decode("Sun Nov  6 08:49:37 1994").unwrap());
        assert_eq!(dt.dayname, DayName::Sun);
        assert_eq!(
            dt.date,
            Date {
                year: 1994,
                month: 11,
                day: 6
            }
        );
        assert_eq!(
            dt.time,
            Time {
                hour: 8,
                minute: 49,
                second: 37
            }
        );
    }

    #[test]
    fn decode_parses_all_imf_fixdate_weekdays() {
        let cases = [
            ("Mon", DayName::Mon),
            ("Tue", DayName::Tue),
            ("Wed", DayName::Wed),
            ("Thu", DayName::Thu),
            ("Fri", DayName::Fri),
            ("Sat", DayName::Sat),
            ("Sun", DayName::Sun),
        ];
        for (name, dayname) in cases {
            let input = format!("{name}, 06 Nov 1994 08:49:37 GMT");
            let dt = expect_imf_fixdate(&decode(&input).unwrap());
            assert_eq!(dt.dayname, dayname, "weekday {name}");
        }
    }

    #[test]
    fn decode_rejects_invalid_dayname() {
        let err = decode("Funday, 06 Nov 1994 08:49:37 GMT")
            .err()
            .expect("expected decode to fail");
        assert!(err.to_string().contains("Invalid day name: Funday"));
    }

    #[test]
    fn decode_rejects_missing_punctuation() {
        // "Sun 06 Nov..." is neither ", " (IMF) nor " " after a short name
        // followed by a valid asctime; "Sun 06" fails because a month is
        // expected next.
        let err = decode("Sun 06 Nov 1994")
            .err()
            .expect("expected decode to fail");
        assert!(err.to_string().contains("Invalid month value: 06"));
    }

    #[test]
    fn decode_rejects_imf_fixdate_missing_gmt() {
        let err = decode("Sun, 06 Nov 1994 08:49:37 ")
            .err()
            .expect("expected decode to fail");
        assert!(err.to_string().contains("Expected 'GMT', but found ''"));
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let err = decode("Sun, 06 Nov")
            .err()
            .expect("expected decode to fail");
        assert_eq!(err.to_string(), "DecodeError: Unexpected end of input");
    }

    #[test]
    fn decode_rejects_empty_input() {
        let err = decode("").err().expect("expected decode to fail");
        assert_eq!(err.to_string(), "DecodeError: Invalid day name: ");
    }
}
