//! Fuzz target for `http_date::decode`: it must never panic on arbitrary
//! input (parity with the OCaml port's `test_decode_no_crash`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // `decode` operates on UTF-8 text; non-UTF-8 input is out of scope.
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = http_date::decode(input);
    }
});
