//! Fuzz target (S-043, ADR 018): arbitrary bytes → the `.ad` parser.
//!
//! Mirrors the real import boundary (`core-services` `FsStore::load`): non-UTF-8
//! input is rejected *before* parsing — `std::fs::read_to_string` does exactly
//! that, so `from_utf8` failure is a legal end of the path.
//!
//! Invariants: never panic; and anything that parses must re-serialize
//! losslessly (QS7): `parse_document(serialize_document(doc)) == doc`.

#![no_main]

use ad_format::{parse_document, serialize_document};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return; // read_to_string would have rejected the file before parsing
    };
    if let Ok(doc) = parse_document(text) {
        let reparsed = parse_document(&serialize_document(&doc))
            .expect("canonical serialization of a parsed document must parse");
        assert_eq!(reparsed, doc, "round-trip must be lossless (QS7)");
    }
});
