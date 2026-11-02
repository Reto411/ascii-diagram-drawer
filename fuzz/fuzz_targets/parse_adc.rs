//! Fuzz target (S-043, ADR 018): arbitrary bytes → the `.adc` custom-shape
//! import parser.
//!
//! Mirrors the real import boundary (`core-services` collections loader):
//! non-UTF-8 input is rejected before parsing; a parse error is a *legal*
//! outcome (the file is skipped with a warning), so the only invariant is:
//! never panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shape_registry::parse_collection;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return; // read_to_string would have rejected the file before parsing
    };
    let _ = parse_collection("fuzz", text); // errors are legal; panics are not
});
