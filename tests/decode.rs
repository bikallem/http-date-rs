//! Tests of `decode`, `Display` and the constructors, through the public API.

use expect_test::expect;
use http_date::{Date, DateTime, DayName, HttpDate, Time, decode};
use std::fmt::Write;

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
        "Sun, 6 Nov 1994 08:49:37 GMT",
        "Sunday, 06-Nov-1994 08:49:37 GMT",
        "Sun, 06 Nov 1994 08:49:37 PST",
        "Sunday, 06-Nov-94",
        "Sun Nov  6 08:49:37",
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
        "Sun, 6 Nov 1994 08:49:37 GMT" => expected digit at position 6
        "Sunday, 06-Nov-1994 08:49:37 GMT" => unexpected character at position 17
        "Sun, 06 Nov 1994 08:49:37 PST" => unexpected character at position 26
        "Sunday, 06-Nov-94" => unexpected end of input at position 17
        "Sun Nov  6 08:49:37" => unexpected end of input at position 19
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
