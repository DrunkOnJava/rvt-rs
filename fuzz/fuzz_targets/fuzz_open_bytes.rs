#![no_main]

//! Fuzz target for the byte-oriented Revit opener.
//!
//! Exercises `RevitFile::open_bytes_with_limits` — the in-memory entry point
//! that runs the full stack: CFB magic check, `cfb` crate parsing, stream
//! enumeration, and the basic-file-info probe. Tight limits keep libFuzzer
//! from OOM-ing on the compressed bomb / huge-allocation classes of inputs.
//!
//! Limits picked to be permissive enough that legitimate fixture bytes get
//! through the opener, but low enough that the fuzzer worker stays under
//! ~64 MiB RSS even on adversarial inputs.

use libfuzzer_sys::fuzz_target;
use rvt::{RevitFile, compression::InflateLimits, reader::OpenLimits};

fuzz_target!(|data: &[u8]| {
    let limits = OpenLimits {
        max_file_bytes: 16 * 1024 * 1024,
        max_stream_bytes: 4 * 1024 * 1024,
        inflate_limits: InflateLimits {
            max_output_bytes: 4 * 1024 * 1024,
        },
    };
    // open_bytes_with_limits takes Vec<u8>, so copy the fuzzer slice.
    // This is the only byte-oriented opener on the public surface; the
    // path-based open_with_limits would require a temp file per input.
    let _ = RevitFile::open_bytes_with_limits(data.to_vec(), limits);
});
