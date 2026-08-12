//! Property-based tests mirroring the OCaml port's fuzz suite
//! (`fuzz/fuzz_http_date.ml`, https://github.com/bikallem/http-date/tree/main/fuzz).
//!
//! The properties here match the Alcobar suite there:
//!
//! 1. `decode` never panics on arbitrary input (`test_decode_no_crash`);
//! 2. if `decode` succeeds, re-encoding and re-decoding must yield the same
//!    value (`test_decode_encode_stable`);
//! 3. `encode` → `decode` round-trips for each of the three formats;
//! 4. a corpus of known-good examples round-trips (`test_corpus`);
//! 5. malformed inputs are rejected (`test_malformed`).

use http_date::{Date, DateTime, DayName, HttpDate, Time, decode, encode};
use proptest::prelude::*;

/// Strategy for a day of the week, matching the OCaml `dayname` generator.
fn dayname() -> impl Strategy<Value = DayName> {
    prop_oneof![
        Just(DayName::Mon),
        Just(DayName::Tue),
        Just(DayName::Wed),
        Just(DayName::Thu),
        Just(DayName::Fri),
        Just(DayName::Sat),
        Just(DayName::Sun),
    ]
}

/// Strategy for a [`DateTime`], matching the OCaml generators' constraints.
///
/// - `day` is capped at 28: the parser doesn't validate day-of-month vs month,
///   but 28 is safe for all months so round-trips never depend on calendar
///   logic.
/// - when `four_digit_year` is false (RFC 850), the year is 0–99 because the
///   encoder emits exactly two digits and the parser reads two.
fn datetime(four_digit_year: bool) -> impl Strategy<Value = DateTime> {
    let year = if four_digit_year {
        0..10_000i32
    } else {
        0..100i32
    };
    (
        dayname(),
        year,
        1..13i32, // month
        1..29i32, // day
        0..24i32, // hour
        0..60i32, // minute
        0..60i32, // second
    )
        .prop_map(
            |(dayname, year, month, day, hour, minute, second)| DateTime {
                dayname,
                date: Date { year, month, day },
                time: Time {
                    hour,
                    minute,
                    second,
                },
            },
        )
}

proptest! {
    /// Property 1: `decode` must return a value or an error — never panic
    /// (parity: `test_decode_no_crash`).
    #[test]
    fn decode_never_panics(input in any::<String>()) {
        let _ = decode(&input);
    }

    /// Property 2: if `decode` succeeds, re-encoding and re-decoding must
    /// yield the same value. Invalid inputs are discarded, as in the OCaml
    /// suite's `test_decode_encode_stable`.
    #[test]
    fn decode_encode_decode_is_stable(input in any::<String>()) {
        if let Ok(date) = decode(&input) {
            let s = encode(&date);
            prop_assert_eq!(
                decode(&s),
                Ok(date),
                "encode({:?}) produced {:?}, which did not round-trip",
                input,
                s
            );
        }
    }

    /// IMF-fixdate round-trip (parity: `IMF round-trip`).
    #[test]
    fn imf_fixdate_round_trips(dt in datetime(true)) {
        let date = HttpDate::ImfFixdate(dt);
        prop_assert_eq!(decode(&encode(&date)), Ok(date));
    }

    /// RFC 850 round-trip (parity: `RFC850 round-trip`).
    #[test]
    fn rfc850_round_trips(dt in datetime(false)) {
        let date = HttpDate::Rfc850(dt);
        prop_assert_eq!(decode(&encode(&date)), Ok(date));
    }

    /// asctime round-trip (parity: `ASCTIME round-trip`).
    #[test]
    fn asctime_round_trips(dt in datetime(true)) {
        let date = HttpDate::Asctime(dt);
        prop_assert_eq!(decode(&encode(&date)), Ok(date));
    }
}

/// Corpus of known-good examples that must decode to the expected value and
/// round-trip (parity: `test_corpus` and `test_asctime_day_width`).
#[test]
fn corpus_decodes_and_round_trips() {
    let cases: &[(&str, HttpDate)] = &[
        (
            "Sun, 06 Nov 1994 08:49:37 GMT",
            HttpDate::ImfFixdate(DateTime {
                dayname: DayName::Sun,
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
            }),
        ),
        (
            "Sunday, 06-Nov-94 08:49:37 GMT",
            HttpDate::Rfc850(DateTime {
                dayname: DayName::Sun,
                date: Date {
                    year: 94,
                    month: 11,
                    day: 6,
                },
                time: Time {
                    hour: 8,
                    minute: 49,
                    second: 37,
                },
            }),
        ),
        (
            "Sun Nov  6 08:49:37 1994",
            HttpDate::Asctime(DateTime {
                dayname: DayName::Sun,
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
            }),
        ),
        (
            "Sun Nov 16 08:49:37 1994",
            HttpDate::Asctime(DateTime {
                dayname: DayName::Sun,
                date: Date {
                    year: 1994,
                    month: 11,
                    day: 16,
                },
                time: Time {
                    hour: 8,
                    minute: 49,
                    second: 37,
                },
            }),
        ),
        (
            "Mon, 01 Jan 2000 00:00:00 GMT",
            HttpDate::ImfFixdate(DateTime {
                dayname: DayName::Mon,
                date: Date {
                    year: 2000,
                    month: 1,
                    day: 1,
                },
                time: Time {
                    hour: 0,
                    minute: 0,
                    second: 0,
                },
            }),
        ),
        (
            "Saturday, 01-Jan-00 00:00:00 GMT",
            HttpDate::Rfc850(DateTime {
                dayname: DayName::Sat,
                date: Date {
                    year: 0,
                    month: 1,
                    day: 1,
                },
                time: Time {
                    hour: 0,
                    minute: 0,
                    second: 0,
                },
            }),
        ),
        (
            "Fri Dec 31 23:59:59 9999",
            HttpDate::Asctime(DateTime {
                dayname: DayName::Fri,
                date: Date {
                    year: 9999,
                    month: 12,
                    day: 31,
                },
                time: Time {
                    hour: 23,
                    minute: 59,
                    second: 59,
                },
            }),
        ),
        (
            "Tue, 15 Mar 2022 12:30:00 GMT",
            HttpDate::ImfFixdate(DateTime {
                dayname: DayName::Tue,
                date: Date {
                    year: 2022,
                    month: 3,
                    day: 15,
                },
                time: Time {
                    hour: 12,
                    minute: 30,
                    second: 0,
                },
            }),
        ),
        (
            "Wednesday, 25-Dec-99 18:00:00 GMT",
            HttpDate::Rfc850(DateTime {
                dayname: DayName::Wed,
                date: Date {
                    year: 99,
                    month: 12,
                    day: 25,
                },
                time: Time {
                    hour: 18,
                    minute: 0,
                    second: 0,
                },
            }),
        ),
        (
            "Thu Jan 10 06:15:45 2008",
            HttpDate::Asctime(DateTime {
                dayname: DayName::Thu,
                date: Date {
                    year: 2008,
                    month: 1,
                    day: 10,
                },
                time: Time {
                    hour: 6,
                    minute: 15,
                    second: 45,
                },
            }),
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(&decode(input).unwrap(), expected, "decode({input:?})");
        let s = encode(expected);
        assert_eq!(&decode(&s).unwrap(), expected, "round-trip of {input:?}");
    }
}

/// Single-digit asctime days are space-padded to width 2 (parity:
/// `test_asctime_day_width`).
#[test]
fn asctime_single_digit_day_padding() {
    let cases = [
        "Sun Nov  1 00:00:00 2000",
        "Sun Nov  9 00:00:00 2000",
        "Sun Nov 10 00:00:00 2000",
        "Sun Nov 28 00:00:00 2000",
    ];
    for input in cases {
        let date = decode(input).unwrap();
        assert_eq!(
            decode(&encode(&date)).unwrap(),
            date,
            "round-trip of {input:?}"
        );
    }
}

/// Malformed inputs must be rejected (parity: `test_malformed`).
#[test]
fn malformed_inputs_are_rejected() {
    let cases = [
        "",
        "XXX, 06 Nov 1994 08:49:37 GMT",
        "Sun, 06 Xxx 1994 08:49:37 GMT",
        "Sun, 06 Nov 1994 08:49:37 PST",
        "Sun, 6 Nov 1994 08:49:37 GMT",
        "Sunday, 06-Nov-1994 08:49:37 GMT",
        "Sun, 06 Nov 1994",
        "not a date",
        "Sun Nov  6 08:49:37",
        "Sunday, 06-Nov-94",
        "Sun, 32 Nov 1994 08:49:37 GMT",
        "Sun, 06 Nov 1994 24:00:00 GMT",
        "Sun, 06 Nov 1994 08:60:00 GMT",
    ];
    for input in cases {
        assert!(decode(input).is_err(), "expected {input:?} to be rejected");
    }
}
