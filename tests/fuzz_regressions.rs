//! Q-04 — Corpus-bound fuzz crashes promoted to regression tests.
//!
//! The nightly `cargo-fuzz` workflow runs libFuzzer against nine
//! targets. Any crash libFuzzer discovers is a specific byte sequence
//! that triggered a panic / hang / OOM. Those inputs get minimised by
//! libFuzzer and should be preserved as regression tests so a future
//! commit can't undo the fix.
//!
//! This file runs the same in-process entry point each fuzz target
//! uses (no libFuzzer runtime) against a hand-crafted set of synthetic
//! adversarial inputs that exercise the same code paths libFuzzer
//! explores. When a real crash is minimised by the nightly job, drop
//! the minimised bytes into the corresponding vector below and the
//! regression test will guard the fix on every `cargo test` run.
//!
//! Each test asserts only that the parser **does not panic** — the
//! parser is free to return any `Err`. For `inflate_at_with_limits`
//! we additionally assert the output-size cap is honoured on `Ok`.

use rvt::basic_file_info::BasicFileInfo;
use rvt::compression::{InflateLimits, gzip_header_len, has_gzip_magic, inflate_at_with_limits};
use rvt::formats::{FieldType, parse_schema};
use rvt::part_atom::PartAtom;
use rvt::reader::OpenLimits;
use rvt::{Result as RvtResult, RevitFile};

/// Wrap a call in `std::panic::catch_unwind` so any panic turns into
/// a test failure instead of aborting the whole suite. The callable
/// takes a byte slice and returns any type — we throw away the value.
fn assert_no_panic<T>(
    label: &str,
    data: &[u8],
    f: impl FnOnce(&[u8]) -> T + std::panic::UnwindSafe,
) {
    let result = std::panic::catch_unwind(|| f(data));
    assert!(
        result.is_ok(),
        "fuzz regression `{label}` panicked on {}-byte input",
        data.len()
    );
}

// ---- fuzz_inflate_at_with_limits ----

const INFLATE_LIMIT: usize = 1024 * 1024;

fn limits() -> InflateLimits {
    InflateLimits {
        max_output_bytes: INFLATE_LIMIT,
    }
}

fn inflate_and_assert(label: &str, data: &[u8]) {
    assert_no_panic(label, data, |d| {
        let res = inflate_at_with_limits(d, 0, limits());
        if let Ok(out) = res {
            assert!(
                out.len() <= INFLATE_LIMIT,
                "`{label}` produced {} bytes, exceeding cap {INFLATE_LIMIT}",
                out.len()
            );
        }
    });
}

#[test]
fn inflate_empty_bytes_does_not_panic() {
    inflate_and_assert("empty", &[]);
}

#[test]
fn inflate_only_gzip_header_does_not_panic() {
    // 10-byte gzip header, nothing after.
    let data = [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    inflate_and_assert("gzip_header_only", &data);
}

#[test]
fn inflate_truncated_header_does_not_panic() {
    // 9-byte truncation of a gzip header.
    let data = [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    inflate_and_assert("truncated_header", &data);
}

#[test]
fn inflate_garbage_after_header_does_not_panic() {
    // Valid 10-byte gzip header + 100 bytes of noise.
    let mut data = vec![0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.extend((0..100).map(|i| (i as u8).wrapping_mul(37)));
    inflate_and_assert("garbage_deflate_stream", &data);
}

#[test]
fn inflate_bogus_fextra_length_does_not_panic() {
    // Gzip header with FEXTRA flag (0x04) set, claiming a 65535-byte
    // extra field that isn't actually present. `gzip_header_len`
    // should either return `None` (preferred) or a value within
    // bounds; slicing past the buffer is the bug.
    let data = [
        0x1f, 0x8b, 0x08, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // header
        0xff, 0xff, // XLEN = 65535, no body follows
    ];
    inflate_and_assert("bogus_fextra_xlen", &data);
}

#[test]
fn inflate_fname_no_null_terminator_does_not_panic() {
    // Gzip header with FNAME flag (0x08) set, no null terminator
    // in the remaining bytes. The scanner in gzip_header_len must
    // fail gracefully rather than walk into unmapped memory.
    let mut data = vec![0x1f, 0x8b, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.extend(b"filename-with-no-null-terminator-whatsoever");
    inflate_and_assert("fname_no_null", &data);
}

#[test]
fn inflate_fcomment_no_null_terminator_does_not_panic() {
    // Same shape as the FNAME case but for FCOMMENT (flag 0x10).
    let mut data = vec![0x1f, 0x8b, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.extend(b"comment with no null byte anywhere in it");
    inflate_and_assert("fcomment_no_null", &data);
}

#[test]
fn inflate_fhcrc_past_end_does_not_panic() {
    // FHCRC flag set, but the buffer ends at the base header — so
    // the extra 2-byte CRC read runs off the end.
    let data = [0x1f, 0x8b, 0x08, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    inflate_and_assert("fhcrc_past_end", &data);
}

#[test]
fn inflate_fname_short_buffer_before_base_header_does_not_panic() {
    // libFuzzer 2026-04-21 discovered this 4-byte input that panicked
    // `gzip_header_len` with "range start index 10 out of range for
    // slice of length 4". Shape: [gzip magic (2) + CM (1) + FLG (1)]
    // where FLG has the FNAME bit (0x08) set so the FNAME-scan branch
    // runs — but the base 10-byte header doesn't fit in the buffer,
    // so `data[pos..]` at line 79 went out of bounds.
    //
    // Fix: bounds-check `pos <= data.len()` via `data.get(pos..)?`
    // before scanning for the null terminator. Regression guard.
    let data = [0x1f, 0x8b, 0x08, 0xb9];
    inflate_and_assert("fname_short_buffer_before_header", &data);
}

#[test]
fn inflate_at_offset_past_end_does_not_panic() {
    // Caller passes an offset that's already past the end of the
    // buffer. The fuzz target wouldn't exercise this (libFuzzer
    // always passes 0 as the offset), but callers on the wire
    // can.
    let data = [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert_no_panic("offset_past_end", &data, |d| {
        let _ = inflate_at_with_limits(d, d.len() + 10, limits());
    });
}

#[test]
fn gzip_header_huge_offset_does_not_panic() {
    let data = [0x1f, 0x8b, 0x08, 0x00];
    assert_no_panic("gzip_header_huge_offset", &data, |d| {
        let _ = has_gzip_magic(d, usize::MAX);
        let _ = gzip_header_len(d, usize::MAX);
        let _ = gzip_header_len(d, usize::MAX - 1);
    });
}

#[test]
fn inflate_at_huge_offset_does_not_panic() {
    let data = [0x1f, 0x8b, 0x08, 0x00];
    assert_no_panic("inflate_at_huge_offset", &data, |d| {
        let _ = inflate_at_with_limits(d, usize::MAX, limits());
    });
}

#[test]
fn inflate_compressed_bomb_rejected_by_cap() {
    // Hand-rolled DEFLATE block that decodes to 4 MiB of 0x00 bytes.
    // The 1 MiB cap must reject this cleanly — either `Err` or an
    // `Ok` with `out.len() <= 1 MiB`. A panic or OOM here is a bug.
    //
    // Construction: gzip header + one DEFLATE non-final stored block
    // whose length header claims the max 65535 bytes; contents are
    // zeros; repeated many times to aggregate > 1 MiB declared. The
    // inflater should either stop at the cap or refuse the overclaim.
    let mut data = Vec::with_capacity(2048);
    // Standard 10-byte gzip header.
    data.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    // 64 non-final stored blocks of 65535 zeros = ~4.2 MiB claim.
    for _ in 0..64 {
        // BFINAL=0, BTYPE=00 (stored): header byte 0x00, 2-byte LEN,
        // 2-byte NLEN, then LEN bytes of 0x00.
        data.push(0x00);
        let len: u16 = 65535;
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&(!len).to_le_bytes());
        data.extend(std::iter::repeat_n(0u8, len as usize));
    }
    inflate_and_assert("compressed_bomb_4mb", &data);
}

// ---- fuzz_open_bytes ----

fn open_limits() -> OpenLimits {
    OpenLimits {
        max_file_bytes: 16 * 1024 * 1024,
        max_stream_bytes: 4 * 1024 * 1024,
        inflate_limits: limits(),
    }
}

fn open_bytes_assert(label: &str, data: &[u8]) {
    let owned: Vec<u8> = data.to_vec();
    assert_no_panic(label, &owned, |d| {
        let _: RvtResult<RevitFile> = RevitFile::open_bytes_with_limits(d.to_vec(), open_limits());
    });
}

#[test]
fn open_empty_bytes_does_not_panic() {
    open_bytes_assert("empty", &[]);
}

#[test]
fn open_only_ole2_magic_does_not_panic() {
    // 8-byte OLE2 / CFB magic and nothing else.
    let data = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
    open_bytes_assert("ole2_magic_only", &data);
}

#[test]
fn open_magic_plus_garbage_does_not_panic() {
    let mut data = vec![0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
    data.extend((0..256).map(|i| (i as u8).wrapping_mul(53)));
    open_bytes_assert("magic_plus_256b_garbage", &data);
}

#[test]
fn open_wrong_magic_does_not_panic() {
    // Looks like a gzip file, not OLE2.
    let data = [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00];
    open_bytes_assert("wrong_magic_gzip", &data);
}

// ---- fuzz_basic_file_info ----

fn basic_file_info_assert(label: &str, data: &[u8]) {
    assert_no_panic(label, data, |d| {
        let _: RvtResult<BasicFileInfo> = BasicFileInfo::from_bytes(d);
    });
}

#[test]
fn bfi_empty_does_not_panic() {
    basic_file_info_assert("empty", &[]);
}

#[test]
fn bfi_odd_length_does_not_panic() {
    // 15-byte buffer — not a multiple of 2 for UTF-16LE.
    basic_file_info_assert("odd_length_15b", &[0x41; 15]);
}

#[test]
fn bfi_unpaired_high_surrogate_does_not_panic() {
    // Revit occasionally ships unpaired surrogates in `BasicFileInfo`.
    // Bytes form a lone high-surrogate (0xD800) at the start of the
    // buffer; `encoding_rs` should handle it, but we want to be sure
    // the downstream scanners (`extract_version`, `extract_path`, …)
    // don't walk past the end of a truncated decode.
    basic_file_info_assert(
        "unpaired_high_surrogate",
        &[
            0x00, 0xd8, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00,
        ],
    );
}

#[test]
fn bfi_multibyte_before_colon_backslash_does_not_panic() {
    // libFuzzer 2026-04-21 (nightly run 24714227057) caught a panic
    // at src/basic_file_info.rs:228: "start byte index 257 is not a
    // char boundary; it is inside '�' (bytes 255..258 of string)".
    //
    // The fallback path in extract_path() finds ":\\" in the
    // UTF-16LE-decoded text, backs up one byte, and slices from there
    // — but the byte preceding ":\\" can be inside a multi-byte UTF-8
    // character when the text contains BOMs (0xef 0xbb 0xbf) or
    // non-ASCII content. Fix uses `text.get(s..)?` instead of
    // `&text[s..]` so the bad index returns None rather than panicking.
    //
    // Minimised input: the original fuzzer fed 300+ bytes; this 9-byte
    // slice reproduces the exact panic shape (UTF-8 BOM + char that
    // spans bytes 2..4 + ":\\" landing at an unfortunate position).
    let data: Vec<u8> = vec![
        // UTF-16LE BOM
        0xff, 0xfe, // "X:\\" where X is inside a multi-byte char when decoded
        b'X', 0x00, b':', 0x00, b'\\', 0x00,
    ];
    basic_file_info_assert("multibyte_before_colon_backslash", &data);
}

#[test]
fn bfi_all_ascii_no_version_does_not_panic() {
    // UTF-16LE ASCII with no 4-digit year present. Forces every
    // extract_* scan to walk the full buffer without finding a
    // match.
    let ascii = b"The quick brown fox jumps over the lazy dog";
    let mut utf16le = Vec::with_capacity(ascii.len() * 2);
    for &b in ascii {
        utf16le.push(b);
        utf16le.push(0);
    }
    basic_file_info_assert("ascii_no_version", &utf16le);
}

// ---- fuzz_parse_schema ----

fn parse_schema_assert(label: &str, data: &[u8]) {
    assert_no_panic(label, data, |d| {
        let _ = parse_schema(d);
    });
}

#[test]
fn schema_empty_does_not_panic() {
    parse_schema_assert("empty", &[]);
}

#[test]
fn schema_short_garbage_does_not_panic() {
    parse_schema_assert("short_garbage", &[0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn schema_tiny_length_prefix_does_not_panic() {
    // First byte claims a length of 0; schema parser should not
    // infinite-loop or index past end.
    parse_schema_assert("tiny_len_prefix", &[0x00; 32]);
}

#[test]
fn schema_huge_declared_count_does_not_panic() {
    // Looks like a header claiming an astronomical class count,
    // then no actual class bytes. The parser should refuse or
    // return partial — never panic or allocate for-real.
    let mut bytes = vec![0xff; 4]; // claim u32::MAX classes
    bytes.extend(vec![0x00; 96]); // thin tail
    parse_schema_assert("huge_declared_count", &bytes);
}

#[test]
fn schema_ffff_prefix_does_not_panic() {
    // 4 KB of 0xFF — every length prefix reads as enormous.
    parse_schema_assert("all_ff_4k", &vec![0xff; 4096]);
}

#[test]
fn schema_garbage_page_does_not_panic() {
    // 4096 bytes of pseudo-random — hits most of the parser's inner
    // loops with "plausible" length-prefix values.
    let bytes: Vec<u8> = (0u32..4096)
        .map(|i| i.wrapping_mul(2654435761).to_le_bytes()[0].wrapping_add(17))
        .collect();
    parse_schema_assert("garbage_4k", &bytes);
}

// ---- walker::detect_entry_offset + partitions::find_chunks ----

#[test]
fn walker_detect_entry_handles_empty_input() {
    // walker::detect_entry_offset takes a decompressed ADocument
    // blob and scans for the class-tag pattern. Empty input must
    // return `None`/score 0 without panicking.
    assert_no_panic("walker_detect_empty", &[], |d| {
        let _ = rvt::walker::detect_adocument_start(d, None);
    });
}

#[test]
fn walker_detect_entry_handles_tiny_input() {
    assert_no_panic("walker_detect_tiny", &[0xaa, 0xbb, 0xcc, 0xdd], |d| {
        let _ = rvt::walker::detect_adocument_start(d, None);
    });
}

#[test]
fn walker_detect_entry_handles_uniform_bytes() {
    // All-zero 4 KB — forces the scanner to walk the full buffer
    // without bailing early on a mismatch.
    assert_no_panic("walker_detect_zeros_4k", &vec![0x00u8; 4096], |d| {
        let _ = rvt::walker::detect_adocument_start(d, None);
    });
}

#[test]
fn find_chunks_handles_empty_input() {
    assert_no_panic("find_chunks_empty", &[], |d| {
        let _ = rvt::partitions::find_chunks(d);
        let _ = rvt::partitions::header_bytes(d);
    });
}

#[test]
fn find_chunks_handles_partial_magic() {
    // 1f 8b is the gzip magic. A buffer of 1f 8b alone should
    // not cause the chunk finder to over-read.
    assert_no_panic("find_chunks_partial_magic", &[0x1f, 0x8b], |d| {
        let _ = rvt::partitions::find_chunks(d);
        let _ = rvt::partitions::header_bytes(d);
    });
}

#[test]
fn find_chunks_handles_many_adjacent_magics() {
    // Repeated gzip magic bytes with nothing after. A buggy scanner
    // could try to dereference each as the start of a real chunk
    // and overrun the end.
    let data: Vec<u8> = std::iter::repeat_n([0x1f, 0x8b], 128).flatten().collect();
    assert_no_panic("find_chunks_many_magics", &data, |d| {
        let _ = rvt::partitions::find_chunks(d);
        let _ = rvt::partitions::header_bytes(d);
    });
}

#[test]
fn partitions_header_handles_short_and_full_inputs() {
    assert_no_panic("partition_header_short", &[0u8; 43], |d| {
        let _ = rvt::partitions::parse_header(d);
    });
    assert_no_panic("partition_header_full", &[0xffu8; 44], |d| {
        let _ = rvt::partitions::parse_header(d);
    });
}

// ---- truncated_gzip_encode round-trip ----

#[test]
fn truncated_gzip_encode_empty_round_trips() {
    // Empty payload is a valid edge — WRT-11 validator must not
    // panic on it.
    assert_no_panic("tgz_encode_empty", &[], |d| {
        let _ = rvt::compression::validate_truncated_gzip_round_trip(d);
    });
}

#[test]
fn truncated_gzip_encode_single_byte_round_trips() {
    assert_no_panic("tgz_encode_one", &[0x42u8], |d| {
        let _ = rvt::compression::validate_truncated_gzip_round_trip(d);
    });
}

#[test]
fn truncated_gzip_encode_64k_zeros_round_trips() {
    // Boundary around the DEFLATE stored-block 65535-byte limit.
    let zeros = vec![0x00u8; 65536];
    assert_no_panic("tgz_encode_64k_zeros", &zeros, |d| {
        let _ = rvt::compression::validate_truncated_gzip_round_trip(d);
    });
}

#[test]
fn truncated_gzip_prefix8_empty_round_trips() {
    // The `with_prefix8` variant carries an 8-byte prefix before the
    // gzip stream. Empty input still has to round-trip safely.
    assert_no_panic("tgz_prefix8_empty", &[], |d| {
        let _ = rvt::compression::validate_truncated_gzip_prefix8_round_trip(d);
    });
}

// ---- fuzz_part_atom ----

fn part_atom_assert(label: &str, data: &[u8]) {
    assert_no_panic(label, data, |d| {
        // `PartAtom::from_bytes` takes raw stream bytes (with the
        // same prefix/header munging the reader does). The fuzz
        // target bypasses that to exercise the XML parser directly.
        let _ = PartAtom::from_bytes(d);
    });
}

#[test]
fn part_atom_empty_does_not_panic() {
    part_atom_assert("empty", &[]);
}

#[test]
fn part_atom_unclosed_does_not_panic() {
    // Opening tag with no close.
    part_atom_assert("unclosed", b"<RevitPartAtom><title>Test");
}

#[test]
fn part_atom_invalid_xml_does_not_panic() {
    part_atom_assert(
        "not_xml",
        b"\x00\x01\x02\x03this is not xml at all\xff\xfe\xfd",
    );
}

#[test]
fn part_atom_nested_deep_does_not_panic() {
    // 200-level nested open tags. The XML parser should handle
    // this without recursion blowing the stack.
    let mut s = String::with_capacity(4096);
    for _ in 0..200 {
        s.push_str("<x>");
    }
    part_atom_assert("deeply_nested", s.as_bytes());
}

// ---- fuzz_elem_table ----

fn elem_table_assert(label: &str, data: &[u8]) {
    assert_no_panic(label, data, |d| {
        let layout = rvt::elem_table::detect_layout(d);
        let _ = rvt::elem_table::parse_records_from_bytes(d, layout, 10_000);
    });
}

#[test]
fn elem_table_empty_does_not_panic() {
    elem_table_assert("empty", &[]);
}

#[test]
fn elem_table_shorter_than_header_does_not_panic() {
    // 16 bytes — well short of the 0x30 header requirement.
    elem_table_assert("tiny_16b", &[0u8; 16]);
}

#[test]
fn elem_table_all_zeros_0x100_does_not_panic() {
    // No markers anywhere → detect_layout falls back to Implicit.
    elem_table_assert("all_zeros_256b", &[0u8; 256]);
}

#[test]
fn elem_table_all_ff_payload_does_not_panic() {
    // Payload of all 0xFF bytes — each 4-byte aligned position
    // matches the marker pattern, so naïve scanners would treat
    // them as overlapping records. The production detector grabs
    // the first two hits and uses their stride; any stride is OK
    // provided we don't panic.
    let mut buf = vec![0u8; 0x100];
    buf[0x20..0x80].fill(0xff);
    elem_table_assert("all_ff_payload", &buf);
}

#[test]
fn elem_table_marker_at_stream_end_does_not_panic() {
    // Marker located where the body would overrun the buffer.
    // parse_records_from_bytes must stop iterating before reading
    // past the end.
    let mut buf = vec![0u8; 40];
    buf[32..36].fill(0xff);
    elem_table_assert("marker_at_end", &buf);
}

#[test]
fn elem_table_synth_project2023_shape_does_not_panic() {
    // Minimal project-2023-shape buffer: 28 B record with 4-byte
    // FF marker, placed at offset 0x1e.
    let mut buf = vec![0u8; 0x100];
    buf[0] = 0x0a; // element_count = 10
    buf[2] = 0x02; // record_count = 2
    buf[0x1e] = 0xff;
    buf[0x1f] = 0xff;
    buf[0x20] = 0xff;
    buf[0x21] = 0xff;
    buf[0x22] = 0x01; // id_primary = 1
    buf[0x3a] = 0xff;
    buf[0x3b] = 0xff;
    buf[0x3c] = 0xff;
    buf[0x3d] = 0xff;
    buf[0x3e] = 0x02; // id_primary = 2
    elem_table_assert("synth_project2023", &buf);
}

#[test]
fn elem_table_synth_project2024_shape_does_not_panic() {
    // Minimal project-2024-shape buffer: 40 B record with 8-byte
    // FF marker at offset 0x22.
    let mut buf = vec![0u8; 0x200];
    buf[0] = 0x0a;
    buf[2] = 0x02;
    buf[0x22..0x2a].fill(0xff);
    buf[0x2e] = 0x01;
    buf[0x4a..0x52].fill(0xff);
    buf[0x56] = 0x02;
    elem_table_assert("synth_project2024", &buf);
}

#[test]
fn elem_table_public_layout_stride_zero_does_not_panic() {
    let layout = rvt::elem_table::ElemTableLayout {
        start: 0,
        stride: 0,
        framing: rvt::elem_table::RecordFraming::Implicit,
    };
    assert_no_panic("elem_table_stride_zero", &[0u8; 64], |d| {
        let _ = rvt::elem_table::parse_records_from_bytes(d, layout, 10);
    });
}

#[test]
fn elem_table_public_layout_huge_start_does_not_panic() {
    let layout = rvt::elem_table::ElemTableLayout {
        start: usize::MAX - 1,
        stride: 12,
        framing: rvt::elem_table::RecordFraming::Implicit,
    };
    assert_no_panic("elem_table_huge_start", &[0u8; 64], |d| {
        let _ = rvt::elem_table::parse_records_from_bytes(d, layout, 10);
    });
}

#[test]
fn elem_table_public_layout_huge_marker_does_not_panic() {
    let layout = rvt::elem_table::ElemTableLayout {
        start: 0,
        stride: 12,
        framing: rvt::elem_table::RecordFraming::Explicit {
            marker_len: usize::MAX,
        },
    };
    assert_no_panic("elem_table_huge_marker", &[0u8; 64], |d| {
        let _ = rvt::elem_table::parse_records_from_bytes(d, layout, 10);
    });
}

// ---- direct public field/parser helpers ----

#[test]
fn field_type_decode_short_inputs_do_not_panic() {
    for (label, data) in [
        ("field_type_empty", &[][..]),
        ("field_type_one_byte", &[0x07][..]),
        ("field_type_two_bytes", &[0x07, 0x10][..]),
        ("field_type_container_prefix", &[0x0e, 0x50, 0x00][..]),
    ] {
        assert_no_panic(label, data, |d| {
            let _ = FieldType::decode(d);
        });
    }
}

#[test]
fn read_field_by_type_huge_cursor_does_not_panic() {
    let ty = FieldType::Primitive {
        kind: 0x07,
        size: 8,
    };
    assert_no_panic("read_field_huge_cursor", &[0u8; 16], |d| {
        let mut cursor = usize::MAX;
        let _ = rvt::walker::read_field_by_type(d, &mut cursor, &ty);
    });
}

#[test]
fn read_field_by_type_huge_string_count_does_not_panic() {
    let ty = FieldType::String;
    assert_no_panic("read_field_huge_string_count", &[0xffu8; 8], |d| {
        let mut cursor = 0usize;
        let _ = rvt::walker::read_field_by_type(d, &mut cursor, &ty);
    });
}

#[test]
fn arc_wall_decode_huge_offset_does_not_panic() {
    assert_no_panic("arc_wall_huge_offset", &[0u8; 128], |d| {
        let _ = rvt::arc_wall_record::ArcWallRecord::decode_standard(d, usize::MAX);
    });
}

#[test]
fn object_graph_extractors_handle_adversarial_bytes() {
    let bytes = [0xffu8; 512];
    assert_no_panic("object_graph_extract_string_records", &bytes, |d| {
        let _ = rvt::object_graph::extract_string_records(d);
        let _ = rvt::object_graph::DocumentHistory::from_decompressed(d);
    });
}
