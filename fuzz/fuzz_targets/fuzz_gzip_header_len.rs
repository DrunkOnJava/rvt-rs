#![no_main]

//! Fuzz target for `rvt::compression::gzip_header_len`.
//!
//! SEC-16. The parser reads a gzip header starting at `offset` and
//! returns `Some(header_len)` or `None`. On arbitrary bytes it must
//! not panic, must not overflow arithmetic, and must not index
//! out of bounds. The function parses variable-length optional
//! fields (FEXTRA / FNAME / FCOMMENT / FHCRC) gated by flag bits,
//! each of which is an attacker-controlled cursor advance — those
//! are the shapes worth exploring.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Common case: parse at offset 0.
    let _ = rvt::compression::gzip_header_len(data, 0);

    // Also probe a data-derived in-bounds offset so the fuzzer
    // exercises offset arithmetic paths without relying on
    // constant inputs. `usize::MAX` and oversize offsets are
    // valid to pass — the function returns `None` early when the
    // magic isn't present at that offset.
    if !data.is_empty() {
        let off = (data[0] as usize) % data.len();
        let _ = rvt::compression::gzip_header_len(data, off);
    }
});
