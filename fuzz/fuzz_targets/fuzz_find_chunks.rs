#![no_main]

//! Fuzz target for `rvt::partitions::find_chunks`.
//!
//! `find_chunks` scans a raw `Partitions/NN` byte slice for gzip magic
//! triplets `1F 8B 08` and returns `(offset, length)` ranges for each
//! chunk boundary it detects. SEC-06 added bomb-rejection limits upstream
//! of this function; P0-11 fixed an off-by-one in the scan range. Both
//! touched this path, so we fuzz it with arbitrary input to harden the
//! chunk-discovery loop against adversarial or malformed streams.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = rvt::partitions::find_chunks(data);
});
