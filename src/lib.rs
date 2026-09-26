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
//! assert!(date.is_imf_fixdate());
//! ```
//!
//! Values can also be constructed directly from components:
//!
//! ```
//! use http_date::{Date, DateTime, DayName, HttpDate, Time, encode};
//!
//! fn datetime(year: u16) -> DateTime {
//!     DateTime {
//!         dayname: DayName::Sun,
//!         date: Date::new(year, 11, 6).expect("valid date"),
//!         time: Time::new(8, 49, 37).expect("valid time"),
//!     }
//! }
//!
//! let imf = HttpDate::imf_fixdate(datetime(1994));
//! assert!(imf.is_imf_fixdate());
//!
//! // RFC 850 uses a two-digit year; construction rejects anything larger.
//! let rfc850 = HttpDate::rfc850(datetime(94)).expect("year is 0-99");
//! assert!(rfc850.is_rfc850());
//!
//! let asctime = HttpDate::asctime(datetime(1994));
//! assert!(asctime.is_asctime());
//!
//! assert_eq!(encode(&imf), "Sun, 06 Nov 1994 08:49:37 GMT");
//! assert_eq!(encode(&rfc850), "Sunday, 06-Nov-94 08:49:37 GMT");
//! assert_eq!(encode(&asctime), "Sun Nov  6 08:49:37 1994");
//! ```
//!
//! The decoded components are available on the [`DateTime`] inside the
//! returned [`HttpDate`].

use std::fmt;
#[cfg(feature = "chrono")]
pub mod chrono;

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

impl DayName {
    /// The three-letter abbreviation used by IMF-fixdate and asctime, e.g. `Mon`.
    fn short(self) -> &'static str {
        DAYS[self as usize].1
    }

    /// The full name used by RFC 850 dates, e.g. `Monday`.
    fn long(self) -> &'static str {
        DAYS[self as usize].2
    }
}

/// Each day with its short and long name, in `DayName` declaration order.
const DAYS: [(DayName, &str, &str); 7] = [
    (DayName::Mon, "Mon", "Monday"),
    (DayName::Tue, "Tue", "Tuesday"),
    (DayName::Wed, "Wed", "Wednesday"),
    (DayName::Thu, "Thu", "Thursday"),
    (DayName::Fri, "Fri", "Friday"),
    (DayName::Sat, "Sat", "Saturday"),
    (DayName::Sun, "Sun", "Sunday"),
];

/// Three-letter month names, January first.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The calendar date portion of an HTTP date.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Date {
    /// The year, e.g. `1994`.
    year: u16,
    /// The month, 1–12, where 1 is January.
    month: u8,
    /// The day of the month, 1–31.
    day: u8,
}

impl Date {
    /// Constructs a date.
    ///
    /// # Errors
    ///
    /// Returns a [`Error`] if a component is out of range: year 0–9999,
    /// month 1–12, day 1–31.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, Error> {
        if year > 9999 || month == 0 || month > 12 || day == 0 || day > 31 {
            return Err(Error::new("date out of range"));
        }
        Ok(Self { year, month, day })
    }

    #[must_use]
    pub fn year(&self) -> u16 {
        self.year
    }

    #[must_use]
    pub fn month(&self) -> u8 {
        self.month
    }

    #[must_use]
    pub fn day(&self) -> u8 {
        self.day
    }
}

/// The time-of-day portion of an HTTP date.
///
/// Constructed with [`Time::new`], which enforces the valid ranges, so a
/// `Time` value is always well-formed (hour 0–23, minute/second 0–59).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Time {
    /// The hour, 0–23.
    hour: u8,
    /// The minute, 0–59.
    minute: u8,
    /// The second, 0–59.
    second: u8,
}

impl Time {
    /// Constructs a time.
    ///
    /// # Errors
    ///
    /// Returns a [`Error`] if a component is out of range: hour 0–23,
    /// minute 0–59, second 0–59.
    pub fn new(hour: u8, minute: u8, second: u8) -> Result<Self, Error> {
        if hour > 23 || minute > 59 || second > 59 {
            return Err(Error::new("time out of range"));
        }
        Ok(Self {
            hour,
            minute,
            second,
        })
    }

    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    #[must_use]
    pub fn minute(&self) -> u8 {
        self.minute
    }

    #[must_use]
    pub fn second(&self) -> u8 {
        self.second
    }
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
/// Values are created with [`HttpDate::imf_fixdate`], [`HttpDate::rfc850`],
/// and [`HttpDate::asctime`], or parsed with [`decode`]. Construction
/// guarantees the components are always in range, so [`encode`] never fails.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct HttpDate {
    format: Format,
    dt: DateTime,
}

/// One piece of a date format. `decode` and `Display` read the same tokens,
/// so the layout of each format is written once.
#[derive(Clone, Copy)]
enum Tok {
    /// Literal text, e.g. `", "`.
    Lit(&'static str),
    /// Short day name, e.g. `Sun`.
    DayShort,
    /// Long day name, e.g. `Sunday`.
    DayLong,
    /// Day of the month as two digits, e.g. `06`.
    Day2,
    /// Day of the month padded with a space, e.g. ` 6` or `16`.
    DaySp,
    /// Month name, e.g. `Nov`.
    Month,
    /// Year with this many digits, e.g. `1994` or `94`.
    Year(usize),
    /// Time as `HH:MM:SS`, e.g. `08:49:37`.
    Hms,
}

use Tok::{Day2, DayLong, DayShort, DaySp, Hms, Lit, Month, Year};

/// The textual format of an [`HttpDate`].
///
/// Private: `HttpDate` is constructor-only, so the RFC 850 two-digit-year
/// invariant can be enforced at construction.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Format {
    ImfFixdate,
    Rfc850,
    Asctime,
}

impl Format {
    #[rustfmt::skip]
    fn toks(self) -> &'static [Tok] {
        match self {
            Format::ImfFixdate => &[DayShort, Lit(", "), Day2, Lit(" "), Month, Lit(" "), Year(4), Lit(" "), Hms, Lit(" GMT")],
            Format::Rfc850 => &[DayLong, Lit(", "), Day2, Lit("-"), Month, Lit("-"), Year(2), Lit(" "), Hms, Lit(" GMT")],
            Format::Asctime => &[DayShort, Lit(" "), Month, Lit(" "), DaySp, Lit(" "), Hms, Lit(" "), Year(4)],
        }
    }
}

impl HttpDate {
    /// Constructs an IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`.
    #[must_use]
    pub fn imf_fixdate(dt: DateTime) -> Self {
        Self {
            format: Format::ImfFixdate,
            dt,
        }
    }

    /// Constructs an RFC 850 date, e.g. `Sunday, 06-Nov-94 08:49:37 GMT`.
    ///
    /// The year must be in the range 0–99.
    ///
    /// # Errors
    ///
    /// Returns a [`Error`] if the year is outside 0–99, which cannot be
    /// represented in RFC 850's two-digit-year format.
    pub fn rfc850(dt: DateTime) -> Result<Self, Error> {
        if dt.date.year > 99 {
            return Err(Error::new("RFC 850 year out of range (0-99)"));
        }
        Ok(Self {
            format: Format::Rfc850,
            dt,
        })
    }

    /// Constructs an asctime date, e.g. `Sun Nov  6 08:49:37 1994`.
    #[must_use]
    pub fn asctime(dt: DateTime) -> Self {
        Self {
            format: Format::Asctime,
            dt,
        }
    }

    /// Returns the [`DateTime`] components of this HTTP date.
    #[must_use]
    pub fn datetime(&self) -> &DateTime {
        &self.dt
    }

    #[must_use]
    pub fn is_imf_fixdate(&self) -> bool {
        self.format == Format::ImfFixdate
    }

    #[must_use]
    pub fn is_rfc850(&self) -> bool {
        self.format == Format::Rfc850
    }

    #[must_use]
    pub fn is_asctime(&self) -> bool {
        self.format == Format::Asctime
    }
}

/* ------------------- decoder ------------------- */
/// An error returned when an HTTP date cannot be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    msg: &'static str,
    pos: Option<usize>,
}

impl Error {
    /// Constructs a new decode error with the given message.
    #[must_use]
    const fn new(msg: &'static str) -> Self {
        Self { msg, pos: None }
    }

    /// Sets the position in the input where decoding failed.
    const fn at(self, pos: usize) -> Self {
        Self {
            pos: Some(pos),
            ..self
        }
    }

    #[must_use]
    pub const fn position(&self) -> Option<usize> {
        self.pos
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.msg)?;
        if let Some(pos) = self.pos {
            write!(f, " at position {pos}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

/// A streaming parser for HTTP date values, used internally by [`decode`].
struct Decoder<'a> {
    buf: &'a str,
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(buf: &'a str) -> Self {
        Decoder { buf, pos: 0 }
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    #[inline]
    fn byte_at(&self, idx: usize) -> Result<u8, Error> {
        self.buf
            .as_bytes()
            .get(idx)
            .copied()
            .ok_or_else(|| Error::new("unexpected end of input").at(idx))
    }

    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        let actual = self.byte_at(self.pos)?;
        if actual == expected {
            self.advance(1);
            Ok(())
        } else {
            Err(Error::new("unexpected character").at(self.pos))
        }
    }

    fn month(&mut self) -> Result<u8, Error> {
        let m = self
            .buf
            .as_bytes()
            .get(self.pos..self.pos + 3)
            .ok_or_else(|| Error::new("unexpected end of input").at(self.pos))?;
        let n = (1..)
            .zip(MONTHS)
            .find_map(|(n, name)| (name.as_bytes() == m).then_some(n))
            .ok_or_else(|| Error::new("invalid month value").at(self.pos))?;
        self.advance(3);
        Ok(n)
    }

    fn digits(&mut self, n: usize) -> Result<u16, Error> {
        let buf = self.buf.as_bytes();
        let mut value: u16 = 0;
        for i in 0..n {
            let pos = self.pos + i;
            match buf.get(pos) {
                Some(&c @ b'0'..=b'9') => {
                    value = value * 10 + u16::from(c - b'0');
                }
                Some(_) => return Err(Error::new("expected digit").at(pos)),
                None => return Err(Error::new("unexpected end of input").at(pos)),
            }
        }
        self.advance(n);
        Ok(value)
    }

    /// Reads a one- or two-digit field, which always fits in a `u8`.
    fn digits_u8(&mut self, n: usize) -> Result<u8, Error> {
        let value = self.digits(n)?;
        Ok(u8::try_from(value).expect("at most two digits"))
    }

    /// Consumes a run of ASCII letters and returns it as a slice of the input.
    fn string(&mut self) -> &'a str {
        let start = self.pos;
        while self.pos < self.buf.len() && self.buf.as_bytes()[self.pos].is_ascii_alphabetic() {
            self.advance(1);
        }
        // Only ASCII bytes are ever consumed, so both ends are char boundaries.
        &self.buf[start..self.pos]
    }

    /// Reads a short (`Sun`) or long (`Sunday`) day name.
    fn dayname(&mut self, long: bool) -> Result<DayName, Error> {
        let start = self.pos;
        let s = self.string();
        DAYS.iter()
            .find(|&&(_, short, full)| s == if long { full } else { short })
            .map(|&(d, ..)| d)
            .ok_or_else(|| Error::new("invalid day name").at(start))
    }

    /// Reads the asctime day of the month: a space and one digit, or two digits.
    fn day_sp(&mut self) -> Result<u8, Error> {
        if self.byte_at(self.pos)? == b' ' {
            self.advance(1);
            self.digits_u8(1)
        } else {
            self.digits_u8(2)
        }
    }

    /// Parses the whole input as `format`, one token at a time.
    fn parse(mut self, format: Format) -> Result<HttpDate, Error> {
        let mut dayname = DayName::Mon;
        let (mut year, mut month, mut day) = (0, 0, 0);
        let mut time = Time {
            hour: 0,
            minute: 0,
            second: 0,
        };
        // Where the date starts, so that a range error points at it.
        let mut date_start = None;
        for &tok in format.toks() {
            let start = self.pos;
            match tok {
                Lit(s) => s.bytes().try_for_each(|b| self.expect(b))?,
                DayShort => dayname = self.dayname(false)?,
                DayLong => dayname = self.dayname(true)?,
                Day2 => day = self.digits_u8(2)?,
                DaySp => day = self.day_sp()?,
                Month => month = self.month()?,
                Year(n) => year = self.digits(n)?,
                Hms => time = self.time()?,
            }
            if matches!(tok, Day2 | DaySp | Month | Year(_)) {
                date_start.get_or_insert(start);
            }
        }
        let date = Date::new(year, month, day).map_err(|e| e.at(date_start.unwrap_or_default()))?;
        // The RFC 9110 `HTTP-date` grammar spans the whole field value, so any
        // leftover input means `buf` is not a well-formed HTTP date.
        if self.pos != self.buf.len() {
            return Err(Error::new("trailing data after HTTP date").at(self.pos));
        }
        Ok(HttpDate {
            format,
            dt: DateTime {
                dayname,
                date,
                time,
            },
        })
    }

    fn time(&mut self) -> Result<Time, Error> {
        let start = self.pos;
        let hour = self.digits_u8(2)?;
        self.expect(b':')?;
        let minute = self.digits_u8(2)?;
        self.expect(b':')?;
        let second = self.digits_u8(2)?;
        Time::new(hour, minute, second).map_err(|e| e.at(start))
    }
}

/// Returns the error that got further into the input.
fn furthest(a: Error, b: Error) -> Error {
    if b.pos > a.pos { b } else { a }
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
/// Returns a [`Error`] if `buf` is not a well-formed HTTP date, for
/// example when the day name or month is unknown, a required separator is
/// missing, the input is truncated, there is trailing data after the date,
/// or a component is out of range (day 1–31, hour 0–23, minute/second 0–59).
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
/// assert!(date.is_imf_fixdate());
/// ```
///
/// An RFC 850 date:
///
/// ```
/// use http_date::{decode, HttpDate};
///
/// let date = decode("Sunday, 06-Nov-94 08:49:37 GMT")
///     .expect("valid RFC 850 date");
/// assert!(date.is_rfc850());
/// ```
///
/// An asctime date:
///
/// ```
/// use http_date::{decode, HttpDate};
///
/// let date = decode("Sun Nov  6 08:49:37 1994")
///     .expect("valid asctime date");
/// assert!(date.is_asctime());
/// ```
///
/// Malformed input is rejected:
///
/// ```
/// use http_date::decode;
///
/// assert!(decode("not a date").is_err());
/// ```
pub fn decode(buf: &str) -> Result<HttpDate, Error> {
    // Like Prolog trying each `http_date//2` clause in turn. If every format
    // fails, report the one that got furthest.
    let parse = |format| Decoder::new(buf).parse(format);
    parse(Format::ImfFixdate)
        .or_else(|a| parse(Format::Rfc850).map_err(|b| furthest(a, b)))
        .or_else(|a| parse(Format::Asctime).map_err(|b| furthest(a, b)))
}

/* ------------------- encoder ------------------- */

/// The three-letter month abbreviation, e.g. `Jan`.
fn month_name(month: u8) -> &'static str {
    MONTHS[usize::from(month) - 1]
}

/// Formats an HTTP date as a string in its original textual format.
///
/// The output is guaranteed to parse back with [`decode`] to the same value.
/// `HttpDate` values are always well-formed — constructed via
/// [`HttpDate::imf_fixdate`], [`HttpDate::rfc850`], or [`HttpDate::asctime`],
/// or parsed with [`decode`] — so formatting cannot fail.
///
/// # Examples
///
/// ```
/// use http_date::{decode, encode};
///
/// let date = decode("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
/// assert_eq!(encode(&date), "Sun, 06 Nov 1994 08:49:37 GMT");
/// ```
#[must_use]
pub fn encode(date: &HttpDate) -> String {
    date.to_string()
}

impl fmt::Display for HttpDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DateTime {
            dayname,
            date,
            time,
        } = self.dt;
        for &tok in self.format.toks() {
            match tok {
                Lit(s) => f.write_str(s),
                DayShort => f.write_str(dayname.short()),
                DayLong => f.write_str(dayname.long()),
                Day2 => write!(f, "{:02}", date.day),
                DaySp => write!(f, "{:2}", date.day),
                Month => f.write_str(month_name(date.month)),
                // `Format` is private, so `Format::Rfc850` implies year <= 99
                // by construction (`HttpDate::rfc850`) and fits `Year(2)`.
                Year(n) => write!(f, "{:0n$}", date.year),
                Hms => write!(f, "{:02}:{:02}:{:02}", time.hour, time.minute, time.second),
            }?;
        }
        Ok(())
    }
}

/* ------------------- tests ------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use std::fmt::Write;

    // Test cases for decode

    fn expect_imf_fixdate(decoded: &HttpDate) -> DateTime {
        assert!(decoded.is_imf_fixdate(), "expected ImfFixdate");
        *decoded.datetime()
    }

    fn expect_rfc850(decoded: &HttpDate) -> DateTime {
        assert!(decoded.is_rfc850(), "expected Rfc850");
        *decoded.datetime()
    }

    fn expect_asctime(decoded: &HttpDate) -> DateTime {
        assert!(decoded.is_asctime(), "expected Asctime");
        *decoded.datetime()
    }

    fn decode_all(inputs: &[&str]) -> String {
        let mut out = String::new();
        for input in inputs {
            match decode(input) {
                Ok(date) => writeln!(out, "{input:?} => {date}").unwrap(),
                Err(err) => writeln!(out, "{input:?} => {err}").unwrap(),
            }
        }
        out
    }

    #[test]
    fn decode_out_of_range_and_boundaries() {
        let actual = decode_all(&[
            "Sun, 00 Nov 1994 08:49:37 GMT",
            "Sun, 32 Nov 1994 08:49:37 GMT",
            "Sunday, 32-Nov-94 08:49:37 GMT",
            "Sun Nov 32 08:49:37 1994",
            "Sun Nov  0 08:49:37 1994",
            "Sun, 06 Nov 1994 24:49:37 GMT",
            "Sun, 06 Nov 1994 08:60:37 GMT",
            "Sun, 06 Nov 1994 08:49:60 GMT",
            "Sun, 31 Nov 1994 08:49:37 GMT",
            "Sun, 06 Nov 1994 23:59:59 GMT",
        ]);
        expect![[r#"
            "Sun, 00 Nov 1994 08:49:37 GMT" => date out of range at position 5
            "Sun, 32 Nov 1994 08:49:37 GMT" => date out of range at position 5
            "Sunday, 32-Nov-94 08:49:37 GMT" => date out of range at position 8
            "Sun Nov 32 08:49:37 1994" => date out of range at position 4
            "Sun Nov  0 08:49:37 1994" => date out of range at position 4
            "Sun, 06 Nov 1994 24:49:37 GMT" => time out of range at position 17
            "Sun, 06 Nov 1994 08:60:37 GMT" => time out of range at position 17
            "Sun, 06 Nov 1994 08:49:60 GMT" => time out of range at position 17
            "Sun, 31 Nov 1994 08:49:37 GMT" => Sun, 31 Nov 1994 08:49:37 GMT
            "Sun, 06 Nov 1994 23:59:59 GMT" => Sun, 06 Nov 1994 23:59:59 GMT
        "#]]
        .assert_eq(&actual);
    }

    #[test]
    fn decode_parses_imf_fixdate() {
        expect![[r"
            Ok(
                HttpDate {
                    format: ImfFixdate,
                    dt: DateTime {
                        dayname: Sun,
                        date: Date {
                            year: 1994,
                            month: 11,
                            day: 6,
                        },
                        time: Time {
                            hour: 8,
                            minute: 49,
                            second: 37,
                        },
                    },
                },
            )
        "]]
        .assert_debug_eq(&decode("Sun, 06 Nov 1994 08:49:37 GMT"));
    }

    #[test]
    fn decode_parses_rfc850_date() {
        let dt = expect_rfc850(&decode("Sunday, 06-Nov-94 08:49:37 GMT").unwrap());
        assert_eq!(dt.dayname, DayName::Sun);
        assert_eq!(dt.date, Date::new(94, 11, 6).unwrap());
        assert_eq!(dt.time, Time::new(8, 49, 37).unwrap());
    }

    #[test]
    fn decode_parses_asctime() {
        let dt = expect_asctime(&decode("Sun Nov  6 08:49:37 1994").unwrap());
        assert_eq!(dt.dayname, DayName::Sun);
        assert_eq!(dt.date, Date::new(1994, 11, 6).unwrap());
        assert_eq!(dt.time, Time::new(8, 49, 37).unwrap());
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
    fn decode_rejects_malformed_input() {
        let actual = decode_all(&[
            "",
            "Funday, 06 Nov 1994 08:49:37 GMT",
            "mon, 06 Nov 1994 08:49:37 GMT",
            "Wednes, 06-Nov-94 08:49:37 GMT",
            ", 06 Nov 1994 08:49:37 GMT",
            "Sun",
            "Sun;06 Nov 1994 08:49:37 GMT",
            "Sun,06 Nov 1994 08:49:37 GMT",
            "Sun 06 Nov 1994",
            "Sun, 0x Nov 1994 08:49:37 GMT",
            "Sun, 06Nov 1994 08:49:37 GMT",
            "Sun, 06 Xyz 1994 08:49:37 GMT",
            "Sun, 06 Nov",
            "Sun, 06 Nov 19",
            "Sun, 06 Nov 1994 0849:37 GMT",
            "Sun, 06 Nov 1994 08:x9:37 GMT",
            "Sun, 06 Nov 1994 08:49:3",
            "Sun, 06 Nov 1994 08:49:37 ",
            "Sunday 06-Nov-94 08:49:37 GMT",
            "Sunday, 06 Nov 94 08:49:37 GMT",
        ]);
        expect![[r#"
            "" => invalid day name at position 0
            "Funday, 06 Nov 1994 08:49:37 GMT" => invalid day name at position 0
            "mon, 06 Nov 1994 08:49:37 GMT" => invalid day name at position 0
            "Wednes, 06-Nov-94 08:49:37 GMT" => invalid day name at position 0
            ", 06 Nov 1994 08:49:37 GMT" => invalid day name at position 0
            "Sun" => unexpected end of input at position 3
            "Sun;06 Nov 1994 08:49:37 GMT" => unexpected character at position 3
            "Sun,06 Nov 1994 08:49:37 GMT" => unexpected character at position 4
            "Sun 06 Nov 1994" => invalid month value at position 4
            "Sun, 0x Nov 1994 08:49:37 GMT" => expected digit at position 6
            "Sun, 06Nov 1994 08:49:37 GMT" => unexpected character at position 7
            "Sun, 06 Xyz 1994 08:49:37 GMT" => invalid month value at position 8
            "Sun, 06 Nov" => unexpected end of input at position 11
            "Sun, 06 Nov 19" => unexpected end of input at position 14
            "Sun, 06 Nov 1994 0849:37 GMT" => unexpected character at position 19
            "Sun, 06 Nov 1994 08:x9:37 GMT" => expected digit at position 20
            "Sun, 06 Nov 1994 08:49:3" => unexpected end of input at position 24
            "Sun, 06 Nov 1994 08:49:37 " => unexpected end of input at position 26
            "Sunday 06-Nov-94 08:49:37 GMT" => unexpected character at position 6
            "Sunday, 06 Nov 94 08:49:37 GMT" => unexpected character at position 10
        "#]]
        .assert_eq(&actual);
    }

    #[test]
    fn decode_rejects_trailing_data() {
        let actual = decode_all(&[
            "Sun, 06 Nov 1994 08:49:37 GMT ",
            "Sun, 06 Nov 1994 08:49:37 GMT 123",
            "Sun, 06 Nov 1994 08:49:37 GMT.",
            "Sunday, 06-Nov-94 08:49:37 GMT extra",
            "Sun Nov  6 08:49:37 1994!",
        ]);
        expect![[r#"
            "Sun, 06 Nov 1994 08:49:37 GMT " => trailing data after HTTP date at position 29
            "Sun, 06 Nov 1994 08:49:37 GMT 123" => trailing data after HTTP date at position 29
            "Sun, 06 Nov 1994 08:49:37 GMT." => trailing data after HTTP date at position 29
            "Sunday, 06-Nov-94 08:49:37 GMT extra" => trailing data after HTTP date at position 30
            "Sun Nov  6 08:49:37 1994!" => trailing data after HTTP date at position 24
        "#]]
        .assert_eq(&actual);
    }

    // A fixed valid `DateTime` used by the constructor tests below.
    fn sun_nov_6(year: u16) -> DateTime {
        DateTime {
            dayname: DayName::Sun,
            date: Date::new(year, 11, 6).unwrap(),
            time: Time::new(8, 49, 37).unwrap(),
        }
    }

    #[test]
    fn date_new_rejects_out_of_range() {
        assert!(Date::new(1994, 11, 6).is_ok());
        assert!(Date::new(10_000, 11, 6).is_err());
        assert!(Date::new(1994, 0, 6).is_err());
        assert!(Date::new(1994, 13, 6).is_err());
        assert!(Date::new(1994, 11, 0).is_err());
        assert!(Date::new(1994, 11, 32).is_err());
    }

    #[test]
    fn time_new_rejects_out_of_range() {
        assert!(Time::new(8, 49, 37).is_ok());
        assert!(Time::new(24, 0, 0).is_err());
        assert!(Time::new(0, 60, 0).is_err());
        assert!(Time::new(0, 0, 60).is_err());
    }

    #[test]
    fn rfc850_constructor_rejects_year_above_99() {
        assert!(HttpDate::rfc850(sun_nov_6(99)).is_ok());
        assert!(HttpDate::rfc850(sun_nov_6(100)).is_err());
        assert!(HttpDate::rfc850(sun_nov_6(1994)).is_err());
    }

    #[test]
    fn encode_round_trips_constructed_values() {
        let cases = [
            HttpDate::imf_fixdate(sun_nov_6(1994)),
            HttpDate::rfc850(sun_nov_6(94)).unwrap(),
            HttpDate::asctime(sun_nov_6(1994)),
        ];
        for date in cases {
            let s = encode(&date);
            assert_eq!(decode(&s).unwrap(), date, "encode produced {s:?}");
        }
    }
}
