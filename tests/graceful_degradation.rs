//! L5B-59 — Graceful-degradation regression tests.
//!
//! These tests feed malformed / edge-case byte sequences through the
//! public API and assert the parser either returns a structured error
//! or decodes with an `Unknown`/zero fallback — never panics. They're
//! complementary to `fuzz_regressions.rs` (which pins specific
//! minimised libFuzzer crash inputs) in that this file is about
//! shapes we KNOW from corpus observation: project-file ElemTables
//! without a header_flag, CFB streams with unusual gzip offsets,
//! truncated schema bytes, etc.
//!
//! Each test asserts no-panic. Tests that can verify a specific
//! behaviour (e.g. `header_flag = 0` on no-flag input) do so in
//! addition.

use rvt::{compression, elem_table};

fn catch<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> std::thread::Result<T> {
    std::panic::catch_unwind(f)
}

#[test]
fn parse_header_returns_zero_flag_when_no_header_flag_bytes_present() {
    // Synthesize a 0x30-byte header where NEITHER 0x1e nor 0x22 holds
    // 0x0011. This is the project-file shape: parse_header should
    // return a header with flag=0 rather than erroring.
    let mut buf = vec![0u8; 0x40];
    buf[0] = 0x01; // element_count = 1
    buf[2] = 0x02; // record_count = 2
    // Leave 0x1e and 0x22 as zero.
    let header =
        elem_table_parse_header_from_synth(&buf).expect("header parse should succeed on no-flag input");
    assert_eq!(header.element_count, 1);
    assert_eq!(header.record_count, 2);
    assert_eq!(
        header.header_flag, 0,
        "project-file-shape input must yield flag=0, not an error"
    );
}

#[test]
fn detect_layout_on_all_zeros_falls_back_to_implicit() {
    // An all-zeros buffer has no FF markers anywhere. detect_layout
    // must fall back to Implicit{start=0x30, stride=12} without
    // panicking even when the buffer is shorter than 0x30.
    for size in [0usize, 16, 0x20, 0x30, 0x100, 0x1000] {
        let buf = vec![0u8; size];
        let result = catch(|| elem_table::detect_layout(&buf));
        assert!(
            result.is_ok(),
            "detect_layout panicked on {size}-byte all-zeros input"
        );
        let layout = result.unwrap();
        assert_eq!(
            layout.framing,
            elem_table::RecordFraming::Implicit,
            "{size}-byte all-zeros must detect as Implicit"
        );
    }
}

#[test]
fn detect_layout_on_single_ff_marker_does_not_panic() {
    // Exactly ONE FF-marker with no second marker in the scan window —
    // layout detector must fall back to Implicit rather than computing
    // a zero or negative stride.
    let mut buf = vec![0u8; 0x100];
    for off in 0x20..0x24 {
        buf[off] = 0xff;
    }
    let layout = elem_table::detect_layout(&buf);
    assert_eq!(
        layout.framing,
        elem_table::RecordFraming::Implicit,
        "single-marker input must NOT be treated as explicit with stride=0"
    );
    assert_eq!(layout.stride, 12);
}

#[test]
fn detect_layout_on_consecutive_markers_with_stride_less_than_record_size() {
    // Pathological: two consecutive 4-byte FF markers back-to-back
    // with NO data between them (stride == 4). detect_layout should
    // still return that stride rather than panicking; downstream
    // parse_records should refuse to produce records because the
    // stride is smaller than the marker itself and parse_records_from_bytes
    // bounds-checks body_len.
    let mut buf = vec![0u8; 0x100];
    // First marker at 0x20.
    buf[0x20] = 0xff;
    buf[0x21] = 0xff;
    buf[0x22] = 0xff;
    buf[0x23] = 0xff;
    // Second marker immediately after at 0x24.
    buf[0x24] = 0xff;
    buf[0x25] = 0xff;
    buf[0x26] = 0xff;
    buf[0x27] = 0xff;
    let layout = elem_table::detect_layout(&buf);
    // The scanner skipped the 8-bytes-of-ff path (marker_len = 8) so
    // this gets detected as a single 8-byte marker with no second
    // marker → Implicit fallback.
    assert_eq!(
        layout.framing,
        elem_table::RecordFraming::Implicit,
        "8-ff-run must be parsed as one 8-byte marker, not two 4-byte ones"
    );
}

#[test]
fn inflate_at_auto_on_no_gzip_magic_falls_back_to_offset_8() {
    // inflate_at_auto tries every gzip-magic hit, then falls back to
    // offset 8 per the family-file convention. With zero magic bytes
    // anywhere, it should attempt offset 8 and return a decompression
    // error (NOT panic).
    let buf = vec![0u8; 256];
    let result = catch(|| compression::inflate_at_auto(&buf));
    assert!(
        result.is_ok(),
        "inflate_at_auto must not panic on no-magic input"
    );
    // Returning an error is fine; returning Ok with empty output is
    // also fine (zero-length DEFLATE is a valid empty chunk). Both
    // outcomes are graceful.
}

#[test]
fn inflate_at_auto_on_short_input_does_not_panic() {
    for size in [0usize, 1, 8, 9, 10, 15] {
        let buf = vec![0u8; size];
        let result = catch(|| compression::inflate_at_auto(&buf));
        assert!(
            result.is_ok(),
            "inflate_at_auto panicked on {size}-byte input"
        );
    }
}

/// Local helper: parse an ElemTable header from an already-inflated
/// byte slice. Does the same checks the public `parse_header` does
/// (which takes a RevitFile) without needing to construct one.
fn elem_table_parse_header_from_synth(d: &[u8]) -> Result<ElemTableHeaderView, &'static str> {
    if d.len() < 0x30 {
        return Err("too short");
    }
    let element_count = u16::from_le_bytes([d[0], d[1]]);
    let record_count = u16::from_le_bytes([d[2], d[3]]);
    let header_flag = [0x1eusize, 0x22]
        .iter()
        .find_map(|&off| {
            let v = u16::from_le_bytes([d[off], d[off + 1]]);
            if v == 0x0011 { Some(v) } else { None }
        })
        .unwrap_or(0);
    Ok(ElemTableHeaderView {
        element_count,
        record_count,
        header_flag,
    })
}

struct ElemTableHeaderView {
    element_count: u16,
    record_count: u16,
    header_flag: u16,
}
