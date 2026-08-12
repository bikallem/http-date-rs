# http-date

Parsing and formatting of HTTP date values as defined in
[RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7).

The three HTTP date formats are supported, and the format a value was parsed
from is preserved so that re-encoding reproduces the original representation:

| Format      | Example                          | Notes                                       |
|-------------|----------------------------------|---------------------------------------------|
| IMF-fixdate | `Sun, 06 Nov 1994 08:49:37 GMT`  | Preferred; the only format senders must generate |
| RFC 850     | `Sunday, 06-Nov-94 08:49:37 GMT` | Obsolete; two-digit year (0–99)             |
| asctime     | `Sun Nov  6 08:49:37 1994`       | Obsolete; space-padded day                  |

`decode` enforces the grammar strictly (case-sensitive day/month names, exact
fixed-width fields, no trailing data) and validates component ranges.
`HttpDate` is constructor-only, so every value is guaranteed well-formed and
`encode` cannot fail.

## Usage

### Parsing

```rust
use http_date::decode;

let date = decode("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
assert!(date.is_imf_fixdate());
```

All three formats are accepted:

```rust
use http_date::decode;

assert!(decode("Sunday, 06-Nov-94 08:49:37 GMT").unwrap().is_rfc850());
assert!(decode("Sun Nov  6 08:49:37 1994").unwrap().is_asctime());
```

### Formatting

```rust
use http_date::{decode, encode};

let date = decode("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
assert_eq!(encode(&date), "Sun, 06 Nov 1994 08:49:37 GMT");
```

### Building from components

```rust
use http_date::{Date, DateTime, DayName, HttpDate, Time};

let date = HttpDate::imf_fixdate(DateTime {
    dayname: DayName::Sun,
    date: Date::new(1994, 11, 6).expect("valid date"),
    time: Time::new(8, 49, 37).expect("valid time"),
});
assert_eq!(date.to_string(), "Sun, 06 Nov 1994 08:49:37 GMT");
```

`HttpDate::rfc850` returns a `Result` because RFC 850 can only represent the
years 0–99; `HttpDate::imf_fixdate` and `HttpDate::asctime` are infallible.

### Chrono integration

Enabled with the `chrono` feature:

```toml
[dependencies]
http-date = { version = "0.1", features = ["chrono"] }
```

Convert `HttpDate` to a UTC datetime and back:

```rust
use chrono::{DateTime, Utc};
use http_date::decode;

let date = decode("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();

// HttpDate -> chrono::DateTime<Utc>
let chrono: DateTime<Utc> = DateTime::try_from(date).unwrap();
assert_eq!(chrono.to_rfc3339(), "1994-11-06T08:49:37+00:00");

// chrono::DateTime<Utc> -> HttpDate (always IMF-fixdate)
let back = http_date::HttpDate::try_from(chrono).unwrap();
assert!(back.is_imf_fixdate());
```

Notes:

- RFC 850's two-digit year is interpreted with the POSIX century window
  (`69–99` → 1900s, `0–68` → 2000s).
- Converting to chrono fails on impossible calendar dates (e.g. 31 February),
  which `decode` accepts but chrono does not.
- Converting from chrono fails for years outside 0–9999, which cannot be
  represented in a four-digit HTTP date year.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `chrono` | off | Conversions between `HttpDate` and `chrono::DateTime<Utc>` |

## Development

The project uses a [Nix flake](flake.nix) with two dev shells, managed via
direnv:

```sh
nix develop             # stable toolchain, just, cargo-nextest, rust-analyzer
nix develop .#fuzz      # nightly toolchain, cargo-fuzz, clang (for fuzzing)
```

Common tasks are wrapped in a [`justfile`](justfile):

```sh
just test        # unit + proptest + doctests (--all-features)
just lint        # clippy with -D warnings
just fmt         # rustfmt
just doc         # build API docs
just ci          # fmt-check + lint + test
```

Fuzzing uses [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) with three
targets — `decode` (never panics), `roundtrip` (encode/decode stability), and
`chrono_roundtrip` (chrono conversion fixed point):

```sh
just fuzz roundtrip 2000      # run a target with a run cap
just fuzz chrono_roundtrip    # run until a crash is found
just fuzz-list
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE-APACHE)
(<http://www.apache.org/licenses/LICENSE-2.0>).
