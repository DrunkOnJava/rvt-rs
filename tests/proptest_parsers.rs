//! Property-based never-panic and round-trip suites (unified report §22.2).
//!
//! These complement the libFuzzer targets in `fuzz/` (nightly, long-running)
//! with a stable-Rust suite that runs in normal CI: arbitrary bytes into
//! every public byte parser must yield `Ok`/`Err`/empty — never a panic —
//! and the research record types must survive a JSON round trip unchanged.

use proptest::prelude::*;
use rvt::compression;
use rvt::control::CancelToken;
use rvt::elem_table;
use rvt::es_refs::{EsPathSegment, EsReferenceOccurrence, FixtureMutation, FixtureTransition};
use rvt::evidence::EvidenceTier;
use rvt::formats;
use rvt::identity::DocumentIdentity;
use rvt::partition_scanner::{ScanOptions, scan_partition_buffer};
use std::collections::BTreeMap;

fn bytes(max: usize) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..max)
}

/// Bytes that start like a truncated-gzip member so the inflate paths get
/// past their magic check most of the time.
fn gzipish(max: usize) -> impl Strategy<Value = Vec<u8>> {
    bytes(max).prop_map(|mut b| {
        if b.len() >= 10 {
            b[0] = 0x1f;
            b[1] = 0x8b;
            b[2] = 0x08;
        }
        b
    })
}

fn tier() -> impl Strategy<Value = EvidenceTier> {
    prop_oneof![
        Just(EvidenceTier::E0),
        Just(EvidenceTier::E1),
        Just(EvidenceTier::E2),
        Just(EvidenceTier::E3),
        Just(EvidenceTier::E4),
        Just(EvidenceTier::E5),
    ]
}

fn path_segment() -> impl Strategy<Value = EsPathSegment> {
    prop_oneof![
        "[A-Za-z_][A-Za-z0-9_]{0,15}".prop_map(|name| EsPathSegment::Field { name }),
        any::<u64>().prop_map(|index| EsPathSegment::Index { index }),
        "\\PC{0,12}".prop_map(|key| EsPathSegment::MapKey { key }),
        "\\PC{0,12}".prop_map(|label| EsPathSegment::Opaque { label }),
    ]
}

fn document() -> impl Strategy<Value = DocumentIdentity> {
    (
        "[a-z0-9-]{1,24}",
        prop::option::of("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"),
        prop::option::of(2016u32..=2026),
    )
        .prop_map(|(key, guid, year)| DocumentIdentity::from_key(key).with_file_meta(guid, year))
}

fn mutation() -> impl Strategy<Value = FixtureMutation> {
    prop_oneof![
        Just(FixtureMutation::NoOp),
        Just(FixtureMutation::IdentitySave),
        (any::<u32>(), any::<u32>())
            .prop_map(|(from_id, to_id)| FixtureMutation::RemapElementId { from_id, to_id }),
        any::<u32>().prop_map(|element_id| FixtureMutation::NullReference { element_id }),
        "\\PC{0,16}".prop_map(|label| FixtureMutation::CopyEntity { label }),
        "\\PC{0,16}".prop_map(|reason| FixtureMutation::Unsupported { reason }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    // ---- byte parsers never panic -------------------------------------

    #[test]
    fn page_checksum_strip_never_panics_and_never_grows(data in bytes(200_000)) {
        let stripped = compression::strip_revit_page_checksums(&data);
        prop_assert!(stripped.len() <= data.len());
    }

    #[test]
    fn gzip_probes_never_panic(data in gzipish(4096), offset in 0usize..4200) {
        let _ = compression::has_gzip_magic(&data, offset);
        if let Some(len) = compression::gzip_header_len(&data, offset) {
            prop_assert!(offset + len <= data.len());
        }
        for found in compression::find_gzip_offsets(&data) {
            prop_assert!(found < data.len());
        }
        let _ = compression::inflate_all_chunks(&data);
        let _ = compression::inflate_at(&data, offset);
        let _ = compression::inflate_at_auto(&data);
    }

    #[test]
    fn formats_latest_integrity_never_panics(data in gzipish(8192)) {
        let _ = compression::diagnose_formats_latest_integrity(&data);
    }

    #[test]
    fn elem_table_records_never_panic_and_respect_limit(data in bytes(4096), limit in 0usize..512) {
        let layout = elem_table::detect_layout(&data);
        let records = elem_table::parse_records_from_bytes(&data, layout, limit);
        prop_assert!(records.len() <= limit);
    }

    #[test]
    fn parse_schema_never_panics(data in bytes(8192)) {
        let _ = formats::parse_schema(&data);
    }

    #[test]
    fn partition_buffer_scan_never_panics_and_stays_in_bounds(
        data in bytes(4096),
        version in prop_oneof![Just(2023u32), Just(2024u32), Just(2016u32), Just(2026u32)],
    ) {
        let mut tags = BTreeMap::new();
        tags.insert(0x0191u16, "ArcWall".to_string());
        tags.insert(0x019cu16, "ArcWall2024".to_string());
        tags.insert(0x0100u16, "Probe".to_string());
        let options = ScanOptions::default();
        for candidate in scan_partition_buffer("Partitions/7", &data, &[], version, &tags, &options) {
            prop_assert!(candidate.offset < data.len());
            prop_assert!(candidate.consumed_start <= candidate.consumed_end);
            prop_assert!(candidate.consumed_end <= data.len());
        }
    }

    // ---- research record types round-trip through JSON ----------------

    #[test]
    fn es_reference_occurrence_round_trips(
        doc in document(),
        path in prop::collection::vec(path_segment(), 0..6),
        tier in tier(),
        notes in prop::collection::vec("\\PC{0,20}", 0..3),
    ) {
        let mut occurrence = EsReferenceOccurrence::stub(doc);
        occurrence.path = path;
        occurrence.tier = tier;
        occurrence.notes = notes;
        let json = serde_json::to_string(&occurrence).unwrap();
        let back: EsReferenceOccurrence = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, occurrence);
    }

    #[test]
    fn fixture_transition_round_trips(
        id in "[A-Za-z0-9-]{1,12}",
        before in "[A-Za-z0-9-]{1,12}",
        after in "[A-Za-z0-9-]{1,12}",
        mutation in mutation(),
        tier in tier(),
    ) {
        let transition = FixtureTransition {
            transition_id: id,
            before_fixture_id: before,
            after_fixture_id: after,
            mutation,
            tier,
            notes: Vec::new(),
        };
        let json = serde_json::to_string(&transition).unwrap();
        let back: FixtureTransition = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, transition);
    }

    #[test]
    fn evidence_tiers_are_totally_ordered_and_stable(a in tier(), b in tier()) {
        let ja = serde_json::to_string(&a).unwrap();
        let ra: EvidenceTier = serde_json::from_str(&ja).unwrap();
        prop_assert_eq!(ra, a);
        prop_assert_eq!(a <= b, a.as_str() <= b.as_str());
    }

    // ---- cancellation token is monotone -------------------------------

    #[test]
    fn cancel_token_is_monotone(cancels in 0usize..4) {
        let token = CancelToken::new();
        for _ in 0..cancels {
            token.cancel();
        }
        prop_assert_eq!(token.is_cancelled(), cancels > 0);
        prop_assert_eq!(token.check().is_err(), cancels > 0);
    }
}

/// Deterministic companion to `parse_schema_never_panics`: the scan cap is
/// hit on a >64 KiB buffer and the parser still returns without panicking.
#[test]
fn parse_schema_past_the_scan_cap_returns() {
    let mut data = vec![0u8; 96 * 1024];
    for (i, b) in data.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    let _ = formats::parse_schema(&data);
}
