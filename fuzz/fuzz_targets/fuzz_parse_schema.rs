#![no_main]

//! Fuzz target for [`rvt::formats::parse_schema`].
//!
//! The `Formats/Latest` schema stream drives every downstream reader
//! (class → field → tag lookups for the Layer 5 walker, IFC emission,
//! cross-version corpus analysis). A panic or unbounded allocation on
//! adversarial bytes here is a denial-of-service on the whole crate.
//!
//! This target feeds raw libFuzzer bytes straight to `parse_schema`.
//! The function signature is `fn parse_schema(&[u8]) -> Result<SchemaTable>`,
//! so we discard the `Result` — we are looking for panics, infinite
//! loops, and allocation blowups, not parse errors.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = rvt::formats::parse_schema(data);
});
