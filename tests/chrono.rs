//! Tests of the chrono conversions, through the public API.

#![cfg(feature = "chrono")]

use chrono::{DateTime, Datelike, TimeZone, Utc};
use http_date::{HttpDate, decode, encode};

fn to_chrono(input: &str) -> DateTime<Utc> {
    DateTime::<Utc>::try_from(decode(input).unwrap()).unwrap()
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
    assert!(DateTime::<Utc>::try_from(date).is_err());
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
        let c = DateTime::<Utc>::try_from(original).unwrap();
        let back = HttpDate::try_from(c).unwrap();
        // RFC 850's 94 maps to 1994, so all three inputs produce the same
        // IMF-fixdate value.
        assert_eq!(back, expected, "round-trip of {input:?}");
    }
}
