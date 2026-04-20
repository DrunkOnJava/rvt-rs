// Fuzz target for `rvt::compression::inflate_at_with_limits`.
//
// The function under test is the single chokepoint between
// untrusted DEFLATE-compressed byte spans (Revit truncated-gzip
// streams) and a `Vec<u8>` allocation in the host process. It is
// also the only surface we ship with explicit audit language
// about decompression-bomb resistance (see `InflateLimits` and
// AUDIT-2026-04-19.md P0 item 4), so it gets its own libFuzzer
// target rather than being covered only at the container level.
//
// SEC-06 already pins the "compressed-bomb rejected" case with a
// unit test using a hand-built pathological DEFLATE block. This
// fuzz target complements that test by exercising the function
// with arbitrary byte slices, arbitrary offsets into those bytes,
// and a modest output ceiling. We care about two properties:
//
//   1. The function never panics / aborts / OOMs on any input.
//      Every error path must return a structured `Err(...)`.
//   2. On `Ok(out)`, the returned buffer honours the cap — i.e.
//      `out.len() <= limits.max_output_bytes`.
//
// We deliberately clamp `max_output_bytes` to 1 MiB (well under
// the 256 MiB default) so the fuzzer can explore many inputs per
// second without libFuzzer's RSS limit kicking in.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rvt::compression::{InflateLimits, inflate_at_with_limits};

/// Fuzz input shape.
///
/// `offset` is narrowed to `u16` so the fuzzer spends its mutation
/// budget exercising the inflate path rather than the "offset is
/// way past end" trivial rejection branch. `data` is the raw byte
/// span the parser sees.
#[derive(Arbitrary, Debug)]
struct Input<'a> {
    offset: u16,
    data: &'a [u8],
}

fuzz_target!(|input: Input<'_>| {
    let limits = InflateLimits {
        max_output_bytes: 1 * 1024 * 1024,
    };
    match inflate_at_with_limits(input.data, input.offset as usize, limits) {
        Ok(out) => {
            // The cap is an absolute upper bound. If this ever
            // fails we have a real bug: the reader returned more
            // bytes than the caller asked to allow.
            assert!(
                out.len() <= limits.max_output_bytes,
                "inflate_at_with_limits exceeded cap: {} > {}",
                out.len(),
                limits.max_output_bytes,
            );
        }
        Err(_) => {
            // All error paths are acceptable — the point of this
            // fuzzer is to verify we never panic / OOM / abort on
            // adversarial input.
        }
    }
});
