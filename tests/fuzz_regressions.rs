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
use rvt::compression::{InflateLimits, inflate_at_with_limits};
use rvt::formats::parse_schema;
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

const INFLATE_LIMIT: usize = 1 * 1024 * 1024;

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
        data.extend(std::iter::repeat(0u8).take(len as usize));
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
fn schema_garbage_page_does_not_panic() {
    // 4096 bytes of pseudo-random — hits most of the parser's inner
    // loops with "plausible" length-prefix values.
    let bytes: Vec<u8> = (0u32..4096)
        .map(|i| i.wrapping_mul(2654435761).to_le_bytes()[0].wrapping_add(17))
        .collect();
    parse_schema_assert("garbage_4k", &bytes);
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
