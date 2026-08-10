//! Parsing and formatting of HTTP date values as defined in
//! [RFC 9110 § 5.6.7](https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7).
//!
//! An HTTP date is an IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`.
//! The public API will be added here.

#![warn(missing_docs)]
