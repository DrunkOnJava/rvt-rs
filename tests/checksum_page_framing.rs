//! Synthetic checksum-page framing fixtures (issue #151 / Discussion #112).
//!
//! Wave 2 support-lane regression set. These fixtures must:
//!
//! 1. Fail under bare [`inflate_at`] / [`inflate_all_chunks`] on injected
//!    pages (drift, hard error, or wrong member yield).
//! 2. Pass under gated strip ([`prepare_stream_for_inflate`] /
//!    [`inflate_stream_at`] / strip-then-inflate).
//!
//! Cases covered: exact page boundary, multi-page, final partial page,
//! empty, truncated mid-page stored bytes, and malformed trailer contents.
//!
//! Credit: page dimensions reported by [@STE1200](https://github.com/STE1200);
//! constants match ahzs645/reviter `stripRevitPageChecksums`. Judge verdict:
//! **narrow** (Wave 1 independent review).

use rvt::compression::{
    REVIT_PAGE_CHECKSUM_BYTES, REVIT_PAGE_PAYLOAD_BYTES, REVIT_STORED_PAGE_BYTES,
    inflate_all_chunks, inflate_all_chunks_for_stream, inflate_at, inflate_stream_at,
    is_checksum_paged_stream, is_revit_paged_loader_candidate, prepare_stream_for_inflate,
    strip_revit_page_checksums, truncated_gzip_encode, truncated_gzip_encode_with_prefix8,
};

/// Insert a synthetic 353-byte trailer after every full 64_896-byte payload
/// slice — the inverse of [`strip_revit_page_checksums`] for test fixtures.
fn inject_page_checksums(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        payload.len() + (payload.len() / REVIT_PAGE_PAYLOAD_BYTES + 1) * REVIT_PAGE_CHECKSUM_BYTES,
    );
    let mut offset = 0;
    let mut page = 0u8;
    while offset + REVIT_PAGE_PAYLOAD_BYTES <= payload.len() {
        out.extend_from_slice(&payload[offset..offset + REVIT_PAGE_PAYLOAD_BYTES]);
        // Distinct non-deflate-looking trailer per page (high entropy, starts
        // with 0x00 like observed real-file tails on redistributable corpus).
        let mut tail = vec![0u8; REVIT_PAGE_CHECKSUM_BYTES];
        tail[0] = 0x00;
        for (i, b) in tail.iter_mut().enumerate().skip(1) {
            *b = page
                .wrapping_add((i as u8).wrapping_mul(17))
                .wrapping_add(0xA5);
        }
        out.extend_from_slice(&tail);
        offset += REVIT_PAGE_PAYLOAD_BYTES;
        page = page.wrapping_add(1);
    }
    out.extend_from_slice(&payload[offset..]);
    out
}

fn high_entropy_payload(len: usize) -> Vec<u8> {
    let mut state = 0xC0FFEE_u32;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 16) as u8
        })
        .collect()
}

fn compressible_payload(min_gzip_len: usize) -> (Vec<u8>, Vec<u8>) {
    // Prefer a real continuous truncated-gzip that is itself ≥ one page of
    // payload bytes, so strip recovers a valid bitstream (not padded junk).
    let mut payload = high_entropy_payload(180_000);
    let mut gzip = truncated_gzip_encode(&payload).expect("encode");
    while gzip.len() < min_gzip_len {
        let start = payload.len();
        payload.extend(high_entropy_payload(30_000));
        // Keep growth deterministic relative to prior length.
        let _ = start;
        gzip = truncated_gzip_encode(&payload).expect("encode");
    }
    (payload, gzip)
}

fn assert_bare_does_not_round_trip(paged: &[u8], payload: &[u8]) {
    match inflate_at(paged, 0) {
        Ok(naive) => assert_ne!(
            naive, payload,
            "without stripping, a successful inflate must not match the original payload"
        ),
        Err(_) => {}
    }
}

#[test]
fn constants_match_reported_65249_layout() {
    assert_eq!(REVIT_STORED_PAGE_BYTES, 65_249);
    assert_eq!(REVIT_PAGE_PAYLOAD_BYTES, 64_896);
    assert_eq!(REVIT_PAGE_CHECKSUM_BYTES, 353);
    assert_eq!(
        REVIT_PAGE_PAYLOAD_BYTES + REVIT_PAGE_CHECKSUM_BYTES,
        REVIT_STORED_PAGE_BYTES
    );
}

#[test]
fn inject_then_strip_is_identity_on_payload_bytes() {
    let clean: Vec<u8> = (0..REVIT_PAGE_PAYLOAD_BYTES + 500)
        .map(|i| (i % 251) as u8)
        .collect();
    let paged = inject_page_checksums(&clean);
    assert!(paged.len() >= REVIT_STORED_PAGE_BYTES);
    assert_eq!(strip_revit_page_checksums(&paged), clean);
}

#[test]
fn empty_stored_bytes_strip_and_prepare_are_identity() {
    assert!(strip_revit_page_checksums(&[]).is_empty());
    let prepared = prepare_stream_for_inflate("Formats/Latest", &[]);
    assert!(prepared.as_ref().is_empty());
    assert!(inflate_all_chunks_for_stream("Partitions/1", &[]).is_empty());
}

#[test]
fn exact_one_page_boundary_bare_fails_gated_strip_passes() {
    // Take exactly one clean payload page of a real truncated-gzip blob and
    // inject a trailer so stored length is exactly 65_249 (no final partial).
    let (_payload, gzip) = compressible_payload(REVIT_PAGE_PAYLOAD_BYTES);
    let clean_page = &gzip[..REVIT_PAGE_PAYLOAD_BYTES];
    let exact = inject_page_checksums(clean_page);
    assert_eq!(exact.len(), REVIT_STORED_PAGE_BYTES);
    assert_eq!(exact.len() % REVIT_STORED_PAGE_BYTES, 0);

    let stripped = strip_revit_page_checksums(&exact);
    assert_eq!(stripped, clean_page);

    // Bare inflate on payload+trailer must not match clean-prefix inflate.
    match (inflate_at(&exact, 0), inflate_at(clean_page, 0)) {
        (Ok(naive), Ok(clean_out)) => assert_ne!(naive, clean_out),
        (Err(_), _) => {}
        (Ok(_), Err(_)) => {}
    }

    // Gated Partitions strip recovers the same outcome as inflating the clean page.
    assert_eq!(
        inflate_stream_at("Partitions/46", &exact, 0).ok(),
        inflate_at(clean_page, 0).ok()
    );
    // Formats stays ungated — same outcome as bare inflate on the paged buffer.
    assert_eq!(
        inflate_stream_at("Formats/Latest", &exact, 0).ok(),
        inflate_at(&exact, 0).ok()
    );
}

#[test]
fn final_partial_page_retained_verbatim_after_strip() {
    let mut clean = vec![0x11u8; REVIT_PAGE_PAYLOAD_BYTES];
    clean.extend(vec![0x22u8; 1_234]); // short final page (< 65_249)
    let paged = inject_page_checksums(&clean);
    assert_eq!(
        paged.len(),
        REVIT_STORED_PAGE_BYTES + 1_234,
        "one full page + partial"
    );
    let stripped = strip_revit_page_checksums(&paged);
    assert_eq!(stripped, clean);
    assert_eq!(&stripped[REVIT_PAGE_PAYLOAD_BYTES..], &vec![0x22u8; 1_234]);
}

#[test]
fn multi_page_bare_inflate_does_not_round_trip_gated_does() {
    let (payload, gzip) = compressible_payload(REVIT_PAGE_PAYLOAD_BYTES + 2_000);
    let paged = inject_page_checksums(&gzip);
    assert!(
        paged.len() >= REVIT_STORED_PAGE_BYTES * 2 || paged.len() > REVIT_STORED_PAGE_BYTES,
        "fixture must cross at least one stored-page boundary (got {})",
        paged.len()
    );

    assert_bare_does_not_round_trip(&paged, &payload);

    let cleaned = strip_revit_page_checksums(&paged);
    let recovered = inflate_at(&cleaned, 0).expect("strip+inflate");
    assert_eq!(recovered, payload);

    let gated = inflate_stream_at("Partitions/46", &paged, 0).expect("gated");
    assert_eq!(gated, payload);

    // Formats/Latest deliberately ungated — must not round-trip via strip.
    assert_eq!(
        inflate_stream_at("Formats/Latest", &paged, 0)
            .err()
            .map(|e| e.to_string()),
        inflate_at(&paged, 0).err().map(|e| e.to_string())
    );
}

#[test]
fn truncated_mid_page_keeps_available_bytes_no_panic() {
    // A buffer that starts a second page but is cut before a full trailer
    // can be formed — strip must keep the short final page verbatim.
    let mut stored = vec![0xABu8; REVIT_PAGE_PAYLOAD_BYTES];
    stored.extend(vec![0xCDu8; REVIT_PAGE_CHECKSUM_BYTES]);
    stored.extend(vec![0xEFu8; 40]); // truncated final page (≪ 65_249)
    assert!(stored.len() < REVIT_STORED_PAGE_BYTES * 2);

    let clean = strip_revit_page_checksums(&stored);
    assert_eq!(clean.len(), REVIT_PAGE_PAYLOAD_BYTES + 40);
    assert!(
        clean
            .iter()
            .take(REVIT_PAGE_PAYLOAD_BYTES)
            .all(|&b| b == 0xAB)
    );
    assert!(
        clean
            .iter()
            .skip(REVIT_PAGE_PAYLOAD_BYTES)
            .all(|&b| b == 0xEF)
    );
    assert!(!clean.contains(&0xCD));
}

#[test]
fn malformed_trailer_bytes_still_stripped_on_full_pages() {
    // Wave 2 does not validate ECC; any 353-byte tail on a full page is cut.
    let mut stored = vec![0x10u8; REVIT_PAGE_PAYLOAD_BYTES];
    stored.extend(std::iter::repeat_n(0xFF, REVIT_PAGE_CHECKSUM_BYTES));
    // Trailer that looks like gzip magic — must still be removed.
    stored[REVIT_PAGE_PAYLOAD_BYTES] = 0x1f;
    stored[REVIT_PAGE_PAYLOAD_BYTES + 1] = 0x8b;
    stored[REVIT_PAGE_PAYLOAD_BYTES + 2] = 0x08;
    let clean = strip_revit_page_checksums(&stored);
    assert_eq!(clean, vec![0x10u8; REVIT_PAGE_PAYLOAD_BYTES]);
}

#[test]
fn prepare_stream_for_inflate_gates_on_path() {
    let (payload, gzip) = compressible_payload(REVIT_PAGE_PAYLOAD_BYTES + 2_000);
    let paged = inject_page_checksums(&gzip);

    // Wave 2 narrowed gate: Partitions strip; Formats/Latest does not.
    let prepared_part = prepare_stream_for_inflate("Partitions/46", &paged);
    assert_eq!(inflate_at(prepared_part.as_ref(), 0).unwrap(), payload);

    let prepared_fmt = prepare_stream_for_inflate("Formats/Latest", &paged);
    assert_eq!(
        prepared_fmt.as_ref(),
        paged.as_slice(),
        "Formats/Latest must remain ungated by default"
    );
    assert!(!is_checksum_paged_stream("Formats/Latest"));
    assert!(is_revit_paged_loader_candidate("Formats/Latest"));

    // Non-paged paths must leave trailers in place (writer/metadata safety).
    let untouched = prepare_stream_for_inflate("BasicFileInfo", &paged);
    assert_eq!(untouched.as_ref(), paged.as_slice());
    assert!(is_checksum_paged_stream("Partitions/46"));
    assert!(is_checksum_paged_stream("Global/Latest"));
    assert!(!is_checksum_paged_stream("ProjectInformation"));
}

#[test]
fn synthetic_prefix8_global_stream_round_trips_after_strip() {
    let mut body = high_entropy_payload(90_000);
    let mut stored = truncated_gzip_encode_with_prefix8(&body).unwrap();
    while inject_page_checksums(&stored).len() < REVIT_STORED_PAGE_BYTES {
        body.extend(high_entropy_payload(20_000));
        stored = truncated_gzip_encode_with_prefix8(&body).unwrap();
    }
    let paged = inject_page_checksums(&stored);
    // Bare auto-inflate on paged prefix8 must not equal body.
    if let Ok((_, naive)) = rvt::compression::inflate_at_auto(&paged) {
        assert_ne!(naive, body);
    }
    let gated = rvt::compression::inflate_stream_auto("Global/Latest", &paged).unwrap();
    assert_eq!(gated.0, 8);
    assert_eq!(gated.1, body);
}

#[test]
fn synthetic_multi_member_partition_chunk_count_recovers_with_strip() {
    let a = truncated_gzip_encode(&vec![0xAAu8; 40_000]).unwrap();
    let b = truncated_gzip_encode(&vec![0xBBu8; 40_000]).unwrap();
    let mut clean = a;
    clean.extend_from_slice(&b);
    while clean.len() < REVIT_PAGE_PAYLOAD_BYTES + 1_000 {
        let extra = truncated_gzip_encode(&vec![0xCCu8; 20_000]).unwrap();
        clean.extend_from_slice(&extra);
    }
    let paged = inject_page_checksums(&clean);
    assert!(paged.len() >= REVIT_STORED_PAGE_BYTES);

    let control = inflate_all_chunks(&paged);
    let experiment = inflate_all_chunks_for_stream("Partitions/46", &paged);
    let control_total: usize = control.iter().map(|c| c.len()).sum();
    let experiment_total: usize = experiment.iter().map(|c| c.len()).sum();

    assert!(
        experiment.len() >= control.len(),
        "strip should recover ≥ as many members (control={}, experiment={})",
        control.len(),
        experiment.len()
    );
    let expected_members = inflate_all_chunks(&clean);
    assert_eq!(experiment.len(), expected_members.len());
    assert_eq!(
        experiment_total,
        expected_members.iter().map(|c| c.len()).sum::<usize>()
    );
    assert_ne!(control_total, experiment_total);

    // Formats path must behave like bare inflate (ungated).
    let formats_chunks = inflate_all_chunks_for_stream("Formats/Latest", &paged);
    assert_eq!(formats_chunks.len(), control.len());
}
