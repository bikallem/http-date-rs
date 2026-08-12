//! Conversions between [`HttpDate`] and [chrono](https://docs.rs/chrono)
//! values.
//!
//! This module is only available when the `chrono` feature is enabled.
//!
//! HTTP dates are always in GMT (UTC), so conversions use
//! `chrono::DateTime<Utc>`.
//!
//! # Examples
//!
//! ```
//! use http_date::decode;
//! use chrono::{DateTime, Utc};
//!
//! let http = decode("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
//! let chrono: DateTime<Utc> = DateTime::try_from(http).unwrap();
//! assert_eq!(chrono.to_rfc3339(), "1994-11-06T08:49:37+00:00");
//! ```

use ::chrono::{
    DateTime as ChronoDateTime, Datelike, NaiveDate, NaiveTime, Timelike, Utc, Weekday,
};

use crate::{Date, DateTime, DayName, HttpDate, Time};

/// An error returned when an [`HttpDate`] and a chrono value cannot be
/// converted into each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronoError(String);

impl ChronoError {
    /// Constructs a new chrono conversion error with the given message.
    fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for ChronoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChronoError: {}", self.0)
    }
}

impl std::error::Error for ChronoError {}

/// Maps an RFC 850 two-digit year (0–99) to a full year using the POSIX
/// century window: 69–99 → 1900s, 0–68 → 2000s.
fn posix_year(two_digit_year: i32) -> i32 {
    if (69..=99).contains(&two_digit_year) {
        1900 + two_digit_year
    } else {
        2000 + two_digit_year
    }
}

fn dayname_from_weekday(weekday: Weekday) -> DayName {
    match weekday {
        Weekday::Mon => DayName::Mon,
        Weekday::Tue => DayName::Tue,
        Weekday::Wed => DayName::Wed,
        Weekday::Thu => DayName::Thu,
        Weekday::Fri => DayName::Fri,
        Weekday::Sat => DayName::Sat,
        Weekday::Sun => DayName::Sun,
    }
}

/// Converts an [`HttpDate`] to a UTC chrono datetime.
///
/// The RFC 850 two-digit year is interpreted with the POSIX century window
/// (69–99 → 1900s, 0–68 → 2000s).
///
/// This fails if the calendar date does not exist (for example 31 February),
/// which [`decode`](crate::decode) accepts but chrono does not.
#[allow(clippy::cast_sign_loss)] // month/day/time are validated to 1-12/1-31/0-59, so i32 -> u32 casts cannot lose the sign
impl TryFrom<HttpDate> for ChronoDateTime<Utc> {
    type Error = ChronoError;

    fn try_from(date: HttpDate) -> Result<Self, Self::Error> {
        let dt = date.datetime();
        let year = if date.is_rfc850() {
            posix_year(dt.date.year())
        } else {
            dt.date.year()
        };
        let date = NaiveDate::from_ymd_opt(year, dt.date.month() as u32, dt.date.day() as u32)
            .ok_or_else(|| {
                ChronoError::new(format!(
                    "{year:04}-{:02}-{:02} is not a valid calendar date",
                    dt.date.month(),
                    dt.date.day(),
                ))
            })?;
        // HttpDate guarantees hour 0–23, minute/second 0–59, which are exactly
        // chrono's valid ranges, so this cannot fail.
        let time = NaiveTime::from_hms_opt(
            dt.time.hour() as u32,
            dt.time.minute() as u32,
            dt.time.second() as u32,
        )
        .ok_or_else(|| ChronoError::new("time out of range"))?;
        Ok(ChronoDateTime::from_naive_utc_and_offset(
            date.and_time(time),
            Utc,
        ))
    }
}

/// Converts a UTC chrono datetime to an [`HttpDate`], always in IMF-fixdate
/// format (the format HTTP senders must use).
///
/// This fails if the year is outside 0–9999, which cannot be represented in
/// a four-digit HTTP date year.
#[allow(clippy::cast_possible_wrap)] // chrono month/day/time are always <= 9999/59, so u32 -> i32 casts cannot wrap
impl TryFrom<ChronoDateTime<Utc>> for HttpDate {
    type Error = ChronoError;

    fn try_from(dt: ChronoDateTime<Utc>) -> Result<Self, Self::Error> {
        let naive = dt.naive_utc();
        let date = Date::new(naive.year(), naive.month() as i32, naive.day() as i32)
            .map_err(|_| ChronoError::new(format!("year {} is outside 0-9999", naive.year())))?;
        let time = Time::new(
            naive.hour() as i32,
            naive.minute() as i32,
            naive.second() as i32,
        )
        .map_err(|_| ChronoError::new("time out of range"))?;
        Ok(HttpDate::imf_fixdate(DateTime {
            dayname: dayname_from_weekday(dt.weekday()),
            date,
            time,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode, encode};
    use ::chrono::TimeZone;

    fn to_chrono(input: &str) -> ChronoDateTime<Utc> {
        ChronoDateTime::<Utc>::try_from(decode(input).unwrap()).unwrap()
    }

    #[test]
    fn imf_fixdate_to_chrono() {
        let c = to_chrono("Sun, 06 Nov 1994 08:49:37 GMT");
        assert_eq!(c, Utc.with_ymd_and_hms(1994, 11, 6, 8, 49, 37).unwrap());
    }

    #[test]
    fn asctime_to_chrono() {
        let c = to_chrono("Sun Nov  6 08:49:37 1994");
        assert_eq!(c, Utc.with_ymd_and_hms(1994, 11, 6, 8, 49, 37).unwrap());
    }

    #[test]
    fn rfc850_two_digit_year_uses_posix_window() {
        assert_eq!(to_chrono("Sunday, 06-Nov-94 08:49:37 GMT").year(), 1994);
        assert_eq!(to_chrono("Sunday, 06-Nov-69 08:49:37 GMT").year(), 1969);
        assert_eq!(to_chrono("Sunday, 06-Nov-68 08:49:37 GMT").year(), 2068);
        assert_eq!(to_chrono("Sunday, 06-Nov-00 08:49:37 GMT").year(), 2000);
    }

    #[test]
    fn impossible_calendar_date_is_rejected() {
        let date = decode("Sun, 31 Feb 1994 08:49:37 GMT").unwrap();
        assert!(ChronoDateTime::<Utc>::try_from(date).is_err());
    }

    #[test]
    fn chrono_to_http_date_is_imf_fixdate() {
        let c = Utc.with_ymd_and_hms(1994, 11, 6, 8, 49, 37).unwrap();
        let date = HttpDate::try_from(c).unwrap();
        assert!(date.is_imf_fixdate());
        assert_eq!(encode(&date), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[test]
    fn chrono_year_outside_0_9999_is_rejected() {
        let c = Utc.with_ymd_and_hms(10_000, 1, 1, 0, 0, 0).unwrap();
        assert!(HttpDate::try_from(c).is_err());
    }

    #[test]
    fn round_trip_via_chrono() {
        let expected =
            HttpDate::try_from(Utc.with_ymd_and_hms(1994, 11, 6, 8, 49, 37).unwrap()).unwrap();
        for input in [
            "Sun, 06 Nov 1994 08:49:37 GMT",
            "Sunday, 06-Nov-94 08:49:37 GMT",
            "Sun Nov  6 08:49:37 1994",
        ] {
            let original = decode(input).unwrap();
            let c = ChronoDateTime::<Utc>::try_from(original).unwrap();
            let back = HttpDate::try_from(c).unwrap();
            // RFC 850's 94 maps to 1994, so all three inputs produce the same
            // IMF-fixdate value.
            assert_eq!(back, expected, "round-trip of {input:?}");
        }
    }
}
