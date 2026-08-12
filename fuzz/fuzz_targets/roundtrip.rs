//! Fuzz target for the decode → encode → decode round-trip: whenever
//! `http_date::decode` succeeds, `encode` must produce a string that decodes
//! back to the same value (parity with the OCaml port's
//! `test_decode_encode_stable`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        if let Ok(date) = http_date::decode(input) {
            let encoded = http_date::encode(&date);
            assert_eq!(
                http_date::decode(&encoded),
                Ok(date),
                "encode({input:?}) produced {encoded:?}, which did not round-trip"
            );
        }
    }
});
