#![no_main]

//! Fuzz target for [`rvt::elem_table::detect_layout`] and
//! [`rvt::elem_table::parse_records_from_bytes`].
//!
//! `Global/ElemTable` has three observed record-layout variants
//! (family 12 B, project-2023 28 B, project-2024 40 B) and the
//! layout detector picks among them by finding the first two
//! FF-marker positions. Adversarial inputs we care about:
//!
//!   - buffers shorter than `0x30` (header region)
//!   - buffers with exactly one FF marker (should not divide by zero)
//!   - buffers with consecutive FF markers (stride < marker_len)
//!   - 0xFF-byte payloads stretching far into the record area
//!   - count fields that imply `record_count` > actual buffer capacity
//!
//! Signature: `fn detect_layout(&[u8]) -> ElemTableLayout`. Paired
//! with `parse_records_from_bytes(&[u8], ElemTableLayout, limit)`
//! to exercise the full decode path. Neither may panic, OOM, or
//! overrun regardless of input shape.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let layout = rvt::elem_table::detect_layout(data);
    // Bound the limit on fuzz inputs so we don't allocate gigabytes
    // from an attacker-controlled count field.
    let limit = 10_000;
    let _ = rvt::elem_table::parse_records_from_bytes(data, layout, limit);
});
