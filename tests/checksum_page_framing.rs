//! Synthetic multi-page checksum framing regression (issue #151 / Discussion #112).
//!
//! **Probe / contract tests only** — these do not change production inflate
//! callers. They pin:
//!
//! 1. Injecting 353-byte trailers every 64_896 payload bytes makes bare
//!    [`inflate_at`] fail to round-trip (silent drift / wrong payload).
//! 2. [`strip_revit_page_checksums`] before inflate restores the payload.
//! 3. Path gating via [`prepare_stream_for_inflate`] (strip only on checksum-
//!    paged stream names). Production call sites use `inflate_stream_*`
//!    wrappers; bare [`inflate_at`] stays a raw codec.
//!
//! Credit: page dimensions reported by @STE1200; constants match
//! ahzs645/reviter `stripRevitPageChecksums`.

use rvt::compression::{
    REVIT_PAGE_CHECKSUM_BYTES, REVIT_PAGE_PAYLOAD_BYTES, REVIT_STORED_PAGE_BYTES,
    inflate_all_chunks, inflate_at, is_checksum_paged_stream, prepare_stream_for_inflate,
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

fn compressible_payload(min_gzip_len: usize) -> (Vec<u8>, Vec<u8>) {
    // Prefer a real continuous truncated-gzip that is itself ≥ one page of
    // payload bytes, so strip recovers a valid bitstream (not padded junk).
    let mut payload = vec![0u8; 200_000];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = ((i * 131) % 251) as u8;
    }
    let mut gzip = truncated_gzip_encode(&payload).expect("encode");
    // If still short (highly compressible), grow with mid-entropy bytes.
    while gzip.len() < min_gzip_len {
        let start = payload.len();
        payload.extend((0..30_000).map(|i| ((start + i) % 251) as u8));
        gzip = truncated_gzip_encode(&payload).expect("encode");
    }
    (payload, gzip)
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
fn synthetic_multipage_bare_inflate_does_not_round_trip() {
    let (payload, gzip) = compressible_payload(REVIT_PAGE_PAYLOAD_BYTES + 2_000);
    let paged = inject_page_checksums(&gzip);
    assert!(
        paged.len() >= REVIT_STORED_PAGE_BYTES,
        "fixture must cross a stored-page boundary (got {})",
        paged.len()
    );

    // Control: production-style bare inflate on checksum-paged stored bytes.
    // flate2 may Err (hard fail) or Ok with drifted bytes (silent corruption).
    // Either outcome falsifies a correct decode without strip.
    if let Ok(naive) = inflate_at(&paged, 0) {
        assert_ne!(
            naive, payload,
            "without stripping, a successful inflate must not match the original payload"
        );
    }

    // Experiment: strip then inflate recovers the payload exactly.
    let cleaned = strip_revit_page_checksums(&paged);
    let recovered = inflate_at(&cleaned, 0).expect("strip+inflate");
    assert_eq!(recovered, payload);
}

#[test]
fn prepare_stream_for_inflate_gates_on_path() {
    let (payload, gzip) = compressible_payload(REVIT_PAGE_PAYLOAD_BYTES + 2_000);
    let paged = inject_page_checksums(&gzip);

    let prepared = prepare_stream_for_inflate("Formats/Latest", &paged);
    assert_eq!(inflate_at(prepared.as_ref(), 0).unwrap(), payload);

    // Non-paged paths must leave trailers in place (writer/metadata safety).
    let untouched = prepare_stream_for_inflate("BasicFileInfo", &paged);
    assert_eq!(untouched.as_ref(), paged.as_slice());
    assert!(is_checksum_paged_stream("Partitions/46"));
    assert!(!is_checksum_paged_stream("ProjectInformation"));
}

#[test]
fn synthetic_prefix8_global_stream_round_trips_after_strip() {
    let mut body = vec![0u8; 90_000];
    for (i, b) in body.iter_mut().enumerate() {
        *b = ((i * 41) % 253) as u8;
    }
    let mut stored = truncated_gzip_encode_with_prefix8(&body).unwrap();
    while inject_page_checksums(&stored).len() < REVIT_STORED_PAGE_BYTES {
        let start = body.len();
        body.extend((0..20_000).map(|i| ((start + i) % 253) as u8));
        stored = truncated_gzip_encode_with_prefix8(&body).unwrap();
    }
    let paged = inject_page_checksums(&stored);
    let cleaned = strip_revit_page_checksums(&paged);
    let (off, out) = rvt::compression::inflate_at_auto(&cleaned).unwrap();
    assert_eq!(off, 8);
    assert_eq!(out, body);
}

#[test]
fn synthetic_multi_member_partition_chunk_count_recovers_with_strip() {
    // Two truncated-gzip members concatenated, then checksum-paged.
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
    let experiment = inflate_all_chunks(&strip_revit_page_checksums(&paged));
    let control_total: usize = control.iter().map(|c| c.len()).sum();
    let experiment_total: usize = experiment.iter().map(|c| c.len()).sum();

    // Stripping must not reduce successful member recovery on this fixture.
    assert!(
        experiment.len() >= control.len(),
        "strip should recover ≥ as many members (control={}, experiment={})",
        control.len(),
        experiment.len()
    );
    // Exact round-trip of concatenated payloads after strip.
    let expected_members = inflate_all_chunks(&clean);
    assert_eq!(experiment.len(), expected_members.len());
    assert_eq!(
        experiment_total,
        expected_members.iter().map(|c| c.len()).sum::<usize>()
    );
    // Control on injected pages should diverge from the clean baseline.
    assert_ne!(control_total, experiment_total);
}
