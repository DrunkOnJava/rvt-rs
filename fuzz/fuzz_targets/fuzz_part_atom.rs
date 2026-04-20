#![no_main]

// SEC-21 — Fuzz the PartAtom XML-like parser.
//
// Target: rvt::part_atom::PartAtom::from_bytes(&[u8]) -> Result<PartAtom>
//
// The parser accepts arbitrary bytes, validates UTF-8, then runs a
// quick-xml state machine over the input. We want to ensure:
//   - no panics on malformed UTF-8 (should return Err cleanly)
//   - no panics on malformed XML (should return Err or Ok with
//     whatever partial state the state machine recovered)
//   - no unbounded allocation or stack overflow on nested / deep input
//
// The result is intentionally discarded: libFuzzer only needs the
// process to not crash.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = rvt::part_atom::PartAtom::from_bytes(data);
});
