//! Generic partition-stream record scanner (M3-03 / M3-04).
//!
//! Scans decompressed `Partitions/*` buffers for schema class-tag hits
//! that pass the RE-14.2 record-prefix filter (`filter_pad == 0` at
//! `+0x02`), yielding neutral [`PartitionRecordCandidate`] values for
//! downstream typed decoders.
//!
//! # Version guard
//!
//! The filtered-tag layout is corpus-proven on Revit **2023** and
//! **2024** (RE-14.2). Other releases return
//! [`PartitionScanStatus::UnsupportedVersion`] with an empty candidate
//! list so exporters never apply an unproven pattern.
//!
//! # Confidence model
//!
//! | Score | Meaning |
//! | --- | --- |
//! | `0.55` | Schema tag + `filter_pad == 0` (RE-14.2 baseline) |
//! | `0.85` | Baseline plus ArcWall-standard envelope gates (2023) |
//! | `0.95` | ArcWall envelope plus validated trailer ElementId |
//!
//! Scores are clamped to `[0.0, 1.0]`. Callers may raise
//! [`ScanOptions::min_confidence`] to drop low-signal hits.
//!
//! # ElemTable linkage (M3-04)
//!
//! When a candidate recovers an ElementId (today: ArcWall singleton
//! trailer), [`element_id_partition_index`] builds
//! `ElementId → PartitionRecordRef`. Join against
//! [`crate::elem_table::index_by_element_id`] via
//! [`link_elem_table_to_partitions`].

use crate::arc_wall_record::{
    ARC_WALL_TAG, ARC_WALL_VARIANT_STANDARD, ArcWallRecord, SCHEMA_FAMILY_MARKER,
    STANDARD_RECORD_MIN_SIZE,
};
use crate::compression;
use crate::control::{Stage, WalkerControl};
use crate::elem_table::{self, ElemRecord};
use crate::formats::{self, SchemaTable};
use crate::streams::FORMATS_LATEST;
use crate::{Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

/// Revit releases where the generic filtered-tag scan is known-safe.
pub const PARTITION_SCANNER_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2023, 2024];

/// Minimum bytes to read envelope fields past the tag (`+0x00..+0x12`).
pub const ENVELOPE_MIN_SIZE: usize = 0x12;

/// Default raw excerpt length hashed into [`PartitionRecordCandidate::excerpt_hash`].
pub const DEFAULT_EXCERPT_LEN: usize = 64;

/// Whether the generic scanner may run for a Revit release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionScanStatus {
    /// Filtered-tag scan is corpus-proven for this release.
    Supported { revit_version: u32 },
    /// Layout not proven — candidates are suppressed.
    UnsupportedVersion { revit_version: u32 },
}

impl PartitionScanStatus {
    pub fn is_supported(self) -> bool {
        matches!(self, PartitionScanStatus::Supported { .. })
    }

    pub fn diagnostic_message(self) -> Option<String> {
        match self {
            PartitionScanStatus::Supported { .. } => None,
            PartitionScanStatus::UnsupportedVersion { revit_version } => Some(format!(
                "partition scanner skipped: Revit {revit_version} is outside supported versions \
                 {PARTITION_SCANNER_SUPPORTED_REVIT_VERSIONS:?}"
            )),
        }
    }
}

/// Stable reference to a partition record (ElemTable join value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionRecordRef {
    /// Stream name, e.g. `"Partitions/5"`.
    pub partition: String,
    /// Byte offset inside the concatenated decompressed partition buffer.
    pub offset: usize,
    /// Schema class tag at that offset.
    pub class_tag: u16,
}

/// Envelope fields observed immediately after a filtered tag hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionEnvelope {
    /// Bytes at `+0x02..+0x04`. Accepted candidates always have `0`.
    pub filter_pad: u16,
    /// `u32` at `+0x04` when the buffer covers it.
    pub fixed_header_0: Option<u32>,
    /// `u32` at `+0x08`.
    pub count_version: Option<u32>,
    /// `u32` at `+0x0c`.
    pub type_code: Option<u32>,
    /// `u16` at `+0x10` (ArcWall variant marker on 2023 standard walls).
    pub variant_marker: Option<u16>,
}

/// Neutral partition record candidate for downstream typed decoders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionRecordCandidate {
    /// Stream name, e.g. `"Partitions/5"`.
    pub stream: String,
    /// Gzip chunk index inside the stream, when mappable from the
    /// concatenated offset.
    pub chunk_index: Option<usize>,
    /// Byte offset of the class tag in the concatenated buffer.
    pub offset: usize,
    /// Schema class tag (`u16` LE at `offset`).
    pub class_tag: u16,
    /// Class name from `Formats/Latest` when the tag is known.
    pub class_name: Option<String>,
    /// Envelope fields past the tag.
    pub envelope: PartitionEnvelope,
    /// Confidence in `[0.0, 1.0]` — see module confidence model.
    pub confidence: f32,
    /// Inclusive-exclusive byte range tentatively attributed to this hit.
    pub consumed_start: usize,
    pub consumed_end: usize,
    /// Deterministic FNV-1a-64 hex digest of the raw excerpt.
    pub excerpt_hash: String,
    /// ElementId recovered from a known trailer layout, when present.
    pub element_id: Option<u32>,
}

impl PartitionRecordCandidate {
    pub fn consumed_range(&self) -> Range<usize> {
        self.consumed_start..self.consumed_end
    }

    pub fn partition_ref(&self) -> PartitionRecordRef {
        PartitionRecordRef {
            partition: self.stream.clone(),
            offset: self.offset,
            class_tag: self.class_tag,
        }
    }
}

/// Full-file scan report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionScan {
    pub status: PartitionScanStatus,
    pub candidates: Vec<PartitionRecordCandidate>,
}

/// Tunables for [`scan_partitions`] / [`scan_partition_buffer`].
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// When `Some`, only these tags are considered. `None` uses every
    /// tagged class from the supplied schema.
    pub tag_allowlist: Option<BTreeSet<u16>>,
    /// Drop candidates below this confidence (inclusive lower bound).
    pub min_confidence: f32,
    /// Hard cap on emitted candidates (stable scan order).
    pub max_candidates: Option<usize>,
    /// Raw excerpt length hashed into `excerpt_hash`.
    pub excerpt_len: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            tag_allowlist: None,
            min_confidence: 0.55,
            max_candidates: None,
            excerpt_len: DEFAULT_EXCERPT_LEN,
        }
    }
}

impl ScanOptions {
    /// Restrict the scan to ArcWall's 2023 tag (`0x0191`).
    pub fn arcwall_2023_only() -> Self {
        Self {
            tag_allowlist: Some(BTreeSet::from([ARC_WALL_TAG])),
            min_confidence: 0.55,
            max_candidates: None,
            excerpt_len: DEFAULT_EXCERPT_LEN,
        }
    }
}

/// Version-scope status for the generic scanner.
pub fn scanner_status(revit_version: u32) -> PartitionScanStatus {
    if PARTITION_SCANNER_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        PartitionScanStatus::Supported { revit_version }
    } else {
        PartitionScanStatus::UnsupportedVersion { revit_version }
    }
}

/// True when the generic scanner covers `revit_version`.
pub fn supports_revit_version(revit_version: u32) -> bool {
    scanner_status(revit_version).is_supported()
}

/// Scan one concatenated partition buffer.
///
/// `chunk_ends` is the exclusive end offset of each inflated chunk in
/// the concatenated buffer (used to recover `chunk_index`). Pass an
/// empty slice when chunk mapping is unavailable.
pub fn scan_partition_buffer(
    stream: &str,
    buf: &[u8],
    chunk_ends: &[usize],
    revit_version: u32,
    tag_to_name: &BTreeMap<u16, String>,
    options: &ScanOptions,
) -> Vec<PartitionRecordCandidate> {
    if !supports_revit_version(revit_version) || buf.len() < 4 {
        return Vec::new();
    }

    let tags: BTreeSet<u16> = if let Some(allow) = &options.tag_allowlist {
        allow.clone()
    } else {
        tag_to_name.keys().copied().collect()
    };
    if tags.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let last = buf.len().saturating_sub(3);
    for offset in 0..=last {
        let tag = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        if !tags.contains(&tag) {
            continue;
        }
        // RE-14.2 record-prefix filter: bytes at +2/+3 must be zero.
        if buf[offset + 2] != 0 || buf[offset + 3] != 0 {
            continue;
        }

        let envelope = read_envelope(buf, offset);
        let class_name = tag_to_name.get(&tag).cloned();
        let (confidence, element_id, consumed_end) =
            score_candidate(buf, offset, tag, revit_version, &envelope);

        if confidence < options.min_confidence {
            continue;
        }

        let excerpt_end = (offset + options.excerpt_len).min(buf.len());
        let excerpt_hash = fnv1a64_hex(&buf[offset..excerpt_end]);
        let chunk_index = chunk_index_for_offset(offset, chunk_ends);

        out.push(PartitionRecordCandidate {
            stream: stream.to_string(),
            chunk_index,
            offset,
            class_tag: tag,
            class_name,
            envelope,
            confidence,
            consumed_start: offset,
            consumed_end,
            excerpt_hash,
            element_id,
        });

        if let Some(max) = options.max_candidates {
            if out.len() >= max {
                break;
            }
        }
    }
    out
}

/// Scan every `Partitions/*` stream in a Revit file.
pub fn scan_partitions(
    rf: &mut RevitFile,
    revit_version: u32,
    options: &ScanOptions,
) -> Result<PartitionScan> {
    scan_partitions_with_control(rf, revit_version, options, &WalkerControl::default())
}

/// Same as [`scan_partitions`], checking the [`WalkerControl`] cancellation
/// token before every partition stream and reporting per-stream progress.
pub fn scan_partitions_with_control(
    rf: &mut RevitFile,
    revit_version: u32,
    options: &ScanOptions,
    control: &WalkerControl,
) -> Result<PartitionScan> {
    let status = scanner_status(revit_version);
    if !status.is_supported() {
        return Ok(PartitionScan {
            status,
            candidates: Vec::new(),
        });
    }

    let tag_to_name = load_schema_tag_map(rf).unwrap_or_default();
    let partition_streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    let mut candidates = Vec::new();
    let stream_count = partition_streams.len() as u64;
    for (index, stream) in partition_streams.into_iter().enumerate() {
        control.check()?;
        control.report(Stage::PartitionScan, index as u64, Some(stream_count));
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(&stream, &raw);
        let mut chunk_ends = Vec::with_capacity(chunks.len());
        let mut concat = Vec::new();
        for chunk in &chunks {
            concat.extend_from_slice(chunk);
            chunk_ends.push(concat.len());
        }
        let mut found = scan_partition_buffer(
            &stream,
            &concat,
            &chunk_ends,
            revit_version,
            &tag_to_name,
            options,
        );
        candidates.append(&mut found);
        if let Some(max) = options.max_candidates {
            if candidates.len() >= max {
                candidates.truncate(max);
                break;
            }
        }
    }

    control.check()?;
    control.report(Stage::PartitionScan, stream_count, Some(stream_count));
    Ok(PartitionScan { status, candidates })
}

/// Convenience: read BasicFileInfo version, then scan with defaults.
pub fn iter_partition_candidates(rf: &mut RevitFile) -> Result<PartitionScan> {
    let version = rf.basic_file_info()?.version;
    scan_partitions(rf, version, &ScanOptions::default())
}

/// Offsets of ArcWall-standard candidates — expressible as the existing
/// [`ArcWallRecord::find_all`] path when confidence ≥ 0.85.
pub fn arcwall_standard_offsets(candidates: &[PartitionRecordCandidate]) -> Vec<usize> {
    candidates
        .iter()
        .filter(|c| {
            c.class_tag == ARC_WALL_TAG
                && c.envelope.variant_marker == Some(ARC_WALL_VARIANT_STANDARD)
                && c.confidence >= 0.85
        })
        .map(|c| c.offset)
        .collect()
}

/// Build `ElementId → PartitionRecordRef` from candidates that recovered
/// an ElementId. Duplicate ids keep the first occurrence (stable order).
pub fn element_id_partition_index(
    candidates: &[PartitionRecordCandidate],
) -> BTreeMap<u32, PartitionRecordRef> {
    let mut map = BTreeMap::new();
    for candidate in candidates {
        if let Some(id) = candidate.element_id {
            map.entry(id).or_insert_with(|| candidate.partition_ref());
        }
    }
    map
}

/// One ElementId present in both ElemTable and a partition candidate.
#[derive(Debug, Clone)]
pub struct LinkedPartitionElement<'a> {
    pub element_id: u32,
    pub elem_record: &'a ElemRecord,
    pub partition_ref: PartitionRecordRef,
}

/// Join partition ElementIds to ElemTable rows. Returns only ids present
/// in **both** maps.
pub fn link_elem_table_to_partitions<'a>(
    elem_by_id: &BTreeMap<u32, &'a ElemRecord>,
    partition_by_id: &BTreeMap<u32, PartitionRecordRef>,
) -> Vec<LinkedPartitionElement<'a>> {
    let mut out = Vec::new();
    for (id, partition_ref) in partition_by_id {
        if let Some(&elem) = elem_by_id.get(id) {
            out.push(LinkedPartitionElement {
                element_id: *id,
                elem_record: elem,
                partition_ref: partition_ref.clone(),
            });
        }
    }
    out
}

/// Declared ElemTable ids that have no matching partition candidate
/// ElementId. Useful for CLI "declared but unlocated" reports.
pub fn declared_but_unlocated_ids(
    declared: &[u32],
    partition_by_id: &BTreeMap<u32, PartitionRecordRef>,
) -> Vec<u32> {
    declared
        .iter()
        .copied()
        .filter(|id| !partition_by_id.contains_key(id))
        .collect()
}

/// Coverage of `partition_by_id` over `declared` ids in `0.0..1.0`.
pub fn linkage_coverage(
    declared: &[u32],
    partition_by_id: &BTreeMap<u32, PartitionRecordRef>,
) -> f64 {
    if declared.is_empty() {
        return 0.0;
    }
    let hit = declared
        .iter()
        .filter(|id| partition_by_id.contains_key(id))
        .count();
    hit as f64 / declared.len() as f64
}

/// Load schema tag → class-name map from `Formats/Latest`.
pub fn load_schema_tag_map(rf: &mut RevitFile) -> Result<BTreeMap<u16, String>> {
    let raw = rf.read_stream(FORMATS_LATEST)?;
    let decompressed = compression::inflate_stream_at(FORMATS_LATEST, &raw, 0)
        .or_else(|_| compression::inflate_stream_auto(FORMATS_LATEST, &raw).map(|(_, d)| d))?;
    let schema = formats::parse_schema(&decompressed)?;
    Ok(tag_map_from_schema(&schema))
}

fn tag_map_from_schema(schema: &SchemaTable) -> BTreeMap<u16, String> {
    schema
        .classes
        .iter()
        .filter_map(|c| c.tag.map(|t| (t, c.name.clone())))
        .collect()
}

fn read_envelope(buf: &[u8], offset: usize) -> PartitionEnvelope {
    let filter_pad = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);
    let mut envelope = PartitionEnvelope {
        filter_pad,
        fixed_header_0: None,
        count_version: None,
        type_code: None,
        variant_marker: None,
    };
    if offset + 8 <= buf.len() {
        envelope.fixed_header_0 = Some(u32::from_le_bytes([
            buf[offset + 4],
            buf[offset + 5],
            buf[offset + 6],
            buf[offset + 7],
        ]));
    }
    if offset + 12 <= buf.len() {
        envelope.count_version = Some(u32::from_le_bytes([
            buf[offset + 8],
            buf[offset + 9],
            buf[offset + 10],
            buf[offset + 11],
        ]));
    }
    if offset + 16 <= buf.len() {
        envelope.type_code = Some(u32::from_le_bytes([
            buf[offset + 12],
            buf[offset + 13],
            buf[offset + 14],
            buf[offset + 15],
        ]));
    }
    if offset + ENVELOPE_MIN_SIZE <= buf.len() {
        envelope.variant_marker =
            Some(u16::from_le_bytes([buf[offset + 0x10], buf[offset + 0x11]]));
    }
    envelope
}

fn score_candidate(
    buf: &[u8],
    offset: usize,
    tag: u16,
    revit_version: u32,
    envelope: &PartitionEnvelope,
) -> (f32, Option<u32>, usize) {
    // Baseline: schema tag + filter_pad == 0.
    let mut confidence = 0.55_f32;
    let mut element_id = None;
    let mut consumed_end = (offset + ENVELOPE_MIN_SIZE).min(buf.len());

    let is_2023_arcwall = revit_version == 2023
        && tag == ARC_WALL_TAG
        && envelope.variant_marker == Some(ARC_WALL_VARIANT_STANDARD)
        && envelope.fixed_header_0 == Some(SCHEMA_FAMILY_MARKER);

    if is_2023_arcwall
        && offset + STANDARD_RECORD_MIN_SIZE <= buf.len()
        && ArcWallRecord::decode_standard(buf, offset).is_ok()
    {
        confidence = 0.85;
        consumed_end = offset + STANDARD_RECORD_MIN_SIZE;
        if let Some(trailer) = ArcWallRecord::decode_trailer(buf, offset) {
            if let Some(id) = trailer.element_id {
                element_id = Some(id);
                confidence = 0.95;
                consumed_end = offset + crate::arc_wall_record::STANDARD_TRAILER_DECODE_END;
            }
        }
    }

    (confidence.clamp(0.0, 1.0), element_id, consumed_end)
}

fn chunk_index_for_offset(offset: usize, chunk_ends: &[usize]) -> Option<usize> {
    chunk_ends.iter().position(|&end| offset < end)
}

/// Deterministic FNV-1a 64-bit hex digest (no extra crate dependency).
fn fnv1a64_hex(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

// Re-export ElemTable helpers so M3-04 callers have one import path.
pub use elem_table::{declared_element_ids, index_by_element_id};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arc_wall_record::{
        ARC_WALL_VARIANT_STANDARD, RECORD_TRAILER, TRAILER_ELEMENT_ID_DUP_OFFSET,
        TRAILER_ELEMENT_ID_OFFSET,
    };

    fn synth_arcwall_with_id(element_id: u32) -> Vec<u8> {
        let mut buf = vec![0u8; crate::arc_wall_record::STANDARD_TRAILER_DECODE_END];
        buf[0..2].copy_from_slice(&ARC_WALL_TAG.to_le_bytes());
        // filter_pad already 0
        buf[4..8].copy_from_slice(&SCHEMA_FAMILY_MARKER.to_le_bytes());
        buf[8..12].copy_from_slice(&1u32.to_le_bytes());
        buf[12..16].copy_from_slice(&3u32.to_le_bytes());
        buf[0x10..0x12].copy_from_slice(&ARC_WALL_VARIANT_STANDARD.to_le_bytes());
        // Finite coords
        for i in 0..12 {
            let p = 0x12 + i * 8;
            buf[p..p + 8].copy_from_slice(&1.0f64.to_le_bytes());
        }
        buf[0x72] = RECORD_TRAILER;
        buf[TRAILER_ELEMENT_ID_OFFSET..TRAILER_ELEMENT_ID_OFFSET + 4]
            .copy_from_slice(&element_id.to_le_bytes());
        buf[TRAILER_ELEMENT_ID_DUP_OFFSET..TRAILER_ELEMENT_ID_DUP_OFFSET + 4]
            .copy_from_slice(&element_id.to_le_bytes());
        buf
    }

    #[test]
    fn version_guard_supports_2023_and_2024_only() {
        assert!(supports_revit_version(2023));
        assert!(supports_revit_version(2024));
        assert!(!supports_revit_version(2022));
        assert!(!supports_revit_version(2025));
        assert_eq!(
            scanner_status(2025),
            PartitionScanStatus::UnsupportedVersion {
                revit_version: 2025
            }
        );
    }

    #[test]
    fn unsupported_version_emits_no_candidates() {
        let buf = synth_arcwall_with_id(42);
        let tags = BTreeMap::from([(ARC_WALL_TAG, "ArcWall".into())]);
        let found = scan_partition_buffer(
            "Partitions/5",
            &buf,
            &[buf.len()],
            2025,
            &tags,
            &ScanOptions::arcwall_2023_only(),
        );
        assert!(found.is_empty());
    }

    #[test]
    fn filter_pad_rejects_nonzero() {
        let mut buf = synth_arcwall_with_id(7);
        buf[2] = 0x01;
        let tags = BTreeMap::from([(ARC_WALL_TAG, "ArcWall".into())]);
        let found = scan_partition_buffer(
            "Partitions/5",
            &buf,
            &[buf.len()],
            2023,
            &tags,
            &ScanOptions::arcwall_2023_only(),
        );
        assert!(found.is_empty());
    }

    #[test]
    fn arcwall_path_expressible_through_scanner() {
        let mut buf = vec![0u8; 16];
        buf.extend_from_slice(&synth_arcwall_with_id(99));
        let tags = BTreeMap::from([(ARC_WALL_TAG, "ArcWall".into())]);
        let found = scan_partition_buffer(
            "Partitions/5",
            &buf,
            &[buf.len()],
            2023,
            &tags,
            &ScanOptions::arcwall_2023_only(),
        );
        let scanner_offsets = arcwall_standard_offsets(&found);
        let direct = ArcWallRecord::find_all(&buf);
        assert_eq!(scanner_offsets, direct);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].offset, 16);
        assert_eq!(found[0].element_id, Some(99));
        assert!((found[0].confidence - 0.95).abs() < f32::EPSILON);
        assert_eq!(found[0].class_name.as_deref(), Some("ArcWall"));
        assert_eq!(found[0].chunk_index, Some(0));
    }

    #[test]
    fn chunk_index_maps_concat_offset() {
        let chunk0 = vec![0u8; 10];
        let mut chunk1 = synth_arcwall_with_id(3);
        let mut concat = chunk0;
        let end0 = concat.len();
        concat.append(&mut chunk1);
        let ends = [end0, concat.len()];
        let tags = BTreeMap::from([(ARC_WALL_TAG, "ArcWall".into())]);
        let found = scan_partition_buffer(
            "Partitions/5",
            &concat,
            &ends,
            2023,
            &tags,
            &ScanOptions::arcwall_2023_only(),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].chunk_index, Some(1));
        assert_eq!(found[0].offset, end0);
    }

    #[test]
    fn elem_table_linkage_joins_ids() {
        let buf = synth_arcwall_with_id(42);
        let tags = BTreeMap::from([(ARC_WALL_TAG, "ArcWall".into())]);
        let found = scan_partition_buffer(
            "Partitions/5",
            &buf,
            &[buf.len()],
            2023,
            &tags,
            &ScanOptions::arcwall_2023_only(),
        );
        let partition_index = element_id_partition_index(&found);
        assert_eq!(partition_index.len(), 1);

        let records = vec![
            ElemRecord {
                offset: 0,
                id_primary: 42,
                id_secondary: 42,
                raw: vec![],
            },
            ElemRecord {
                offset: 28,
                id_primary: 99,
                id_secondary: 99,
                raw: vec![],
            },
        ];
        let elem_index = index_by_element_id(&records);
        let linked = link_elem_table_to_partitions(&elem_index, &partition_index);
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].element_id, 42);
        assert_eq!(linked[0].partition_ref.partition, "Partitions/5");

        let declared = vec![42u32, 99];
        let missing = declared_but_unlocated_ids(&declared, &partition_index);
        assert_eq!(missing, vec![99]);
        assert!((linkage_coverage(&declared, &partition_index) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn excerpt_hash_is_stable() {
        let a = fnv1a64_hex(b"hello");
        let b = fnv1a64_hex(b"hello");
        let c = fnv1a64_hex(b"world");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn candidate_serializes_to_json() {
        let buf = synth_arcwall_with_id(1);
        let tags = BTreeMap::from([(ARC_WALL_TAG, "ArcWall".into())]);
        let found = scan_partition_buffer(
            "Partitions/5",
            &buf,
            &[buf.len()],
            2023,
            &tags,
            &ScanOptions::arcwall_2023_only(),
        );
        let json = serde_json::to_string(&found[0]).expect("serialize");
        assert!(json.contains("\"class_tag\":401")); // 0x0191
        assert!(json.contains("excerpt_hash"));
    }
}
