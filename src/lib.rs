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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
enum PunctuationTok {
    Comma,
    Space,
}

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

    #[inline]
    fn space(&mut self) -> Result<(), Error> {
        self.expect(b' ')
    }

    #[inline]
    fn comma(&mut self) -> Result<(), Error> {
        self.expect(b',')
    }

    #[inline]
    fn colon(&mut self) -> Result<(), Error> {
        self.expect(b':')
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

    fn year(&mut self) -> Result<u16, Error> {
        self.digits(4)
    }

    fn day(&mut self) -> Result<u8, Error> {
        self.digits_u8(2)
    }

    fn dayname_tok(&mut self) -> Result<DayNameTok, Error> {
        let start = self.pos;
        let s = self.string();
        for (d, short, long) in DAYS {
            if s == short {
                return Ok(DayNameTok::Short(d));
            }
            if s == long {
                return Ok(DayNameTok::Long(d));
            }
        }
        Err(Error::new("invalid day name").at(start))
    }

    fn punctuation_tok(&mut self) -> Result<PunctuationTok, Error> {
        let c = self.byte_at(self.pos)?;
        let tok = match c {
            b',' => PunctuationTok::Comma,
            b' ' => PunctuationTok::Space,
            _ => return Err(Error::new("expected ',' or ' ' after day name").at(self.pos)),
        };
        self.advance(1);
        Ok(tok)
    }

    fn date1(&mut self) -> Result<Date, Error> {
        let start = self.pos;
        let day = self.day()?;
        self.space()?;
        let month = self.month()?;
        self.space()?;
        let year = self.year()?;
        Date::new(year, month, day).map_err(|e| e.at(start))
    }

    fn time(&mut self) -> Result<Time, Error> {
        let start = self.pos;
        let hour = self.digits_u8(2)?;
        self.colon()?;
        let minute = self.digits_u8(2)?;
        self.colon()?;
        let second = self.digits_u8(2)?;
        Time::new(hour, minute, second).map_err(|e| e.at(start))
    }

    fn gmt(&mut self) -> Result<(), Error> {
        let start = self.pos;
        if self.string() != "GMT" {
            return Err(Error::new("expected 'GMT'").at(start));
        }
        Ok(())
    }

    // IMF-fixdate: day-name "," SP date1 SP time SP "GMT"
    fn imf_fixdate(&mut self, dayname: DayName) -> Result<HttpDate, Error> {
        self.space()?;
        let date = self.date1()?;
        self.space()?;
        let time = self.time()?;
        self.space()?;
        self.gmt()?;
        Ok(HttpDate::imf_fixdate(DateTime {
            dayname,
            date,
            time,
        }))
    }

    fn date2(&mut self) -> Result<Date, Error> {
        let start = self.pos;
        let day = self.day()?;
        self.expect(b'-')?;
        let month = self.month()?;
        self.expect(b'-')?;
        let year = self.digits(2)?;
        Date::new(year, month, day).map_err(|e| e.at(start))
    }

    // RFC 850 date: day-name "," SP date2 SP time SP "GMT"
    fn rfc850_date(&mut self, dayname: DayName) -> Result<HttpDate, Error> {
        self.comma()?;
        self.space()?;
        let date = self.date2()?;
        self.space()?;
        let time = self.time()?;
        self.space()?;
        self.gmt()?;
        let date = DateTime {
            dayname,
            date,
            time,
        };
        HttpDate::rfc850(date)
    }

    fn date3(&mut self) -> Result<(u8, u8), Error> {
        // month
        let m = self.month()?;
        self.space()?;
        // day
        let d = match self.byte_at(self.pos)? {
            b' ' => {
                self.space()?;
                self.digits_u8(1)?
            }
            _ => self.digits_u8(2)?,
        };
        Ok((m, d))
    }

    // asctime date: day-name SP month SP (2DIGIT / (SP 1DIGIT)) SP time SP 4DIGIT
    fn asctime_date(&mut self, dayname: DayName) -> Result<HttpDate, Error> {
        let start = self.pos;
        let (month, day) = self.date3()?;
        self.space()?;
        let time = self.time()?;
        self.space()?;
        let year = self.year()?;
        let date = Date::new(year, month, day).map_err(|e| e.at(start))?;
        Ok(HttpDate::asctime(DateTime {
            dayname,
            date,
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
    let mut decoder = Decoder::new(buf);
    let date = match decoder.dayname_tok()? {
        DayNameTok::Long(dayname) => decoder.rfc850_date(dayname),
        DayNameTok::Short(dayname) => match decoder.punctuation_tok()? {
            PunctuationTok::Comma => decoder.imf_fixdate(dayname),
            PunctuationTok::Space => decoder.asctime_date(dayname),
        },
    }?;
    // The RFC 9110 `HTTP-date` grammar spans the whole field value, so any
    // leftover input means `buf` is not a well-formed HTTP date.
    if decoder.pos != decoder.buf.len() {
        return Err(Error::new("trailing data after HTTP date").at(decoder.pos));
    }
    Ok(date)
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
        let dt = &self.dt;
        let month = month_name(dt.date.month());
        let day = dt.date.day();
        let year = dt.date.year();
        let time = dt.time;

        match self.format {
            Format::ImfFixdate => write!(
                f,
                "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
                dt.dayname.short(),
                day,
                month,
                year,
                time.hour(),
                time.minute(),
                time.second(),
            ),
            // `format` is private, so `Format::Rfc850` implies year <= 99 by
            // construction (`HttpDate::rfc850`).
            Format::Rfc850 => write!(
                f,
                "{}, {:02}-{}-{:02} {:02}:{:02}:{:02} GMT",
                dt.dayname.long(),
                day,
                month,
                year,
                time.hour(),
                time.minute(),
                time.second(),
            ),
            Format::Asctime => write!(
                f,
                "{} {} {:2} {:02}:{:02}:{:02} {:04}",
                dt.dayname.short(),
                month,
                day,
                time.hour(),
                time.minute(),
                time.second(),
                year,
            ),
        }
    }
}
