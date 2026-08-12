#![no_main]

use chrono::{DateTime, Utc};
use http_date::HttpDate;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        if let Ok(date) = http_date::decode(input) {
            // Impossible calendar dates (e.g. 31 Feb) yield Err — that's fine.
            if let Ok(chrono) = DateTime::<Utc>::try_from(date) {
                // Every chrono value derived from decode is in range, so this
                // must succeed; a failure here is a real bug.
                let back =
                    HttpDate::try_from(chrono).expect("decoded date must re-encode to chrono");
                let again =
                    DateTime::<Utc>::try_from(back).expect("HttpDate must convert back to chrono");
                assert_eq!(chrono, again, "chrono conversion is not a fixed point");
            }
        }
    }
});
