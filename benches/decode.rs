//! Times `decode` and `encode` for each date format. Run with `just bench`.
//!
//! Uses only std, so it needs no extra dependencies. Each case runs `ROUNDS`
//! times and reports the fastest round, which is the least affected by noise.

use std::hint::black_box;
use std::time::Instant;

const CASES: [(&str, &str); 4] = [
    ("imf", "Sun, 06 Nov 1994 08:49:37 GMT"),
    ("rfc850", "Sunday, 06-Nov-94 08:49:37 GMT"),
    ("asctime", "Sun Nov  6 08:49:37 1994"),
    ("invalid", "Sun, 06 Nov 1994 08:49:37 PST"),
];

const ROUNDS: u32 = 15;

/// Returns the fastest time per call of `f`, in nanoseconds.
fn best_ns(iters: u32, mut f: impl FnMut()) -> f64 {
    (0..ROUNDS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..iters {
                f();
            }
            start.elapsed().as_secs_f64() * 1e9 / f64::from(iters)
        })
        .fold(f64::INFINITY, f64::min)
}

fn main() {
    for (name, input) in CASES {
        let decode = best_ns(2_000_000, || {
            let _ = black_box(http_date::decode(black_box(input)));
        });
        print!("{name:8} decode {decode:6.1} ns");
        if let Ok(date) = http_date::decode(input) {
            let encode = best_ns(500_000, || {
                black_box(http_date::encode(black_box(&date)));
            });
            print!("   encode {encode:6.1} ns");
        }
        println!();
    }
}
