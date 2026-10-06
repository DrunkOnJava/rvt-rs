//! `Global/ElemTable` — Revit's element-id index.
//!
//! This stream lists every ElementId in the file, in ascending order, with
//! its owner (#152). After a `u16` (the tag of the class `ElemTable` in the
//! file's own schema) and a `u32` record count at `0x02`, the records start
//! at `0x06` and close with their owner:
//!
//! | Releases            | Record size | Layout                                          |
//! | ---                 | ---         | ---                                             |
//! | Revit 2008 to 2023  | 28 B        | `[u32 id][u32 id][16 B][u32 owner]`             |
//! | Revit 2024 and later| 40 B        | `[u64 id][12 B][u64 id][u64 owner][u32]`        |
//!
//! That is STE1200's frame, measured by puzzbobb on 30 files from 2008 to
//! 2027: read so, the stated number of records fits before a 19-byte
//! (28 B) or 24-byte (40 B) tail on project files, the ids rise strictly,
//! every set owner is an id of the table, and no owner chain loops. Family
//! files use the same records and leave an unset owner at `0` where project
//! files write all `0xFF`; their records are followed by a longer trailer.
//!
//! The first record is whatever element has the lowest surviving id, which
//! depends on the file's history (the template it started from, deleted
//! elements): `AllProjectPhases` at 0 on Einhoven and Core Interior, at 1 on
//! Autodesk's sample projects, `DimensionStyle` at 0 on the families. Id 0 is
//! an ordinary id. Its owner, at `0x1E` (28 B) or `0x22` (40 B), is element
//! 17 on the 2016 to 2026 families; [`ElemTableHeader::header_flag`] still
//! reports it under its old name.
//!
//! Earlier versions of this parser read 24 bytes into each record (from
//! `0x1E`), so they missed the first record and read one more from the
//! tail (#152, #553). A 12-byte implicit layout from `0x30` remains only as
//! the fallback for a table neither record size fits.

use crate::{Error, Result, RevitFile, compression, streams::GLOBAL_ELEM_TABLE};
use serde::{Deserialize, Serialize};

/// Header extracted from the first bytes of decompressed Global/ElemTable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElemTableHeader {
    /// Named for what it was assumed to be, but not a count of elements:
    /// the serialization tag of the class `ElemTable` in the file's own
    /// schema (RE-80), so the same on every file of a release (1174 on
    /// 2016, 1411 on 2024, 1481 on 2026). For the declared ElementIds use
    /// [`declared_element_ids`].
    pub element_count: u16,
    /// The number of records, the low 16 bits of the `u32` count at `0x02`
    /// (every file measured has fewer than 65,536 records; [`parse_records`]
    /// reads the whole `u32`).
    pub record_count: u16,
    /// The `0x0011` found at `0x1E` / `0x22` on family files, 0 elsewhere.
    /// It is the owner field of the table's first record (ElementId 17 on
    /// family files), not a header field (#152).
    pub header_flag: u16,
    /// Decompressed stream size, for diagnostics.
    pub decompressed_bytes: usize,
}

/// How records are framed in this ElemTable stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordFraming {
    /// 12-byte records from `0x30`, no owner field: the fallback for a
    /// table neither record size fits. No corpus file uses it.
    Implicit,
    /// The 28- and 40-byte records. `marker_len` is the width of the owner
    /// field (4 or 8 bytes), whose unset value is all `0xFF` on project
    /// files and `0` on family files.
    Explicit { marker_len: usize },
}

/// Detected record layout — where records start, how big they are, how they're framed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElemTableLayout {
    /// Offset of record 0's first byte in the decompressed stream: `0x06`
    /// on the explicit layouts.
    pub start: usize,
    pub stride: usize,
    /// Offset of the owner field within a record: `24` on the 28-byte
    /// layout, `28` on the 40-byte one. Unused for
    /// [`RecordFraming::Implicit`].
    pub marker_offset: usize,
    pub framing: RecordFraming,
}

/// A fully-parsed record from ElemTable.
///
/// On the implicit fallback layout, `id_primary`/`id_secondary` are the first
/// two `u32`s of the 12-byte record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElemRecord {
    /// Offset in the decompressed stream where this record begins.
    pub offset: usize,
    /// The record's first id, at `+0` (the low half of a `u64` on the
    /// 40-byte layout).
    pub id_primary: u32,
    /// The record's second id (`+4` on 28 bytes, `+20` on 40). Equal to
    /// `id_primary` on nearly every record. Where the two differ on the
    /// 40-byte layout, this one is the element's ElementId as its partition
    /// record and Revit's own IFC export carry it (RE-41, [`declared_ids`]).
    pub id_secondary: u32,
    /// The element this record's element belongs to (RE-31), the field
    /// that closes the record (`u32` at `+24` on 28 bytes, `u64` at `+28`
    /// on 40); `None` when it holds its unset value, all `0xFF` on project
    /// files and `0` on family files. Read so, no record owns itself and no
    /// owner chain loops (#152). Always `None` on the implicit fallback
    /// layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<u32>,
    /// Always `None`. It held the table's first record's ids while the
    /// parser read from `0x1E` and missed that record (RE-140); the first
    /// record is now `records[0]` itself. Kept so code that names the field
    /// still compiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_ids: Option<(u32, u32)>,
    /// Raw record bytes.
    pub raw: Vec<u8>,
}

/// Backward-compat record type from the pre-corpus-probe parser. Kept so
/// existing callers and tests keep compiling; prefer `ElemRecord` for new work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElemRecordRough {
    /// Offset in the decompressed stream where this record begins.
    pub offset: usize,
    /// Presumptive u32 fields (12 bytes' worth). Not yet bound to semantics.
    pub presumptive_u32_triple: [u32; 3],
}

fn parse_header_bytes(d: &[u8]) -> Result<ElemTableHeader> {
    if d.len() < 0x30 {
        return Err(Error::BasicFileInfo(
            "Global/ElemTable stream too short for header".into(),
        ));
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
    Ok(ElemTableHeader {
        element_count,
        record_count,
        header_flag,
        decompressed_bytes: d.len(),
    })
}

/// Where the records start: after the `u16` tag and the `u32` count.
pub const RECORDS_ORIGIN: usize = 0x06;

/// The record count the table states, the `u32` at `0x02`.
fn stated_count(d: &[u8]) -> Option<usize> {
    let bytes = d.get(2..6)?;
    usize::try_from(u32::from_le_bytes(bytes.try_into().ok()?)).ok()
}

/// `(second id, owner, owner width)` offsets of a record of `stride` bytes.
fn record_fields(stride: usize) -> Option<(usize, usize, usize)> {
    match stride {
        28 => Some((4, 24, 4)),
        40 => Some((20, 28, 8)),
        _ => None,
    }
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    let bytes = d.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

/// Records out of order that a table may hold, in hundredths of its stated
/// count. A few ids sit out of order in some large tables: 3 of 50,529
/// records on Snowdon Towers Architectural (Revit 2025), 163 of 44,157 on a
/// Japanese Autodesk sample (RE-174). At the wrong record size the ids rise
/// on about a third of the steps, so a share this small separates the sizes
/// as the exact rule did. Below 100 records it allows none.
pub const MAX_OUT_OF_ORDER_PERCENT: usize = 1;

/// Detect the record layout: the record size (28 or 40 bytes) at which the
/// stated number of records fits from `0x06` with ids rising over the whole
/// table, but for at most [`MAX_OUT_OF_ORDER_PERCENT`] of its records. When
/// both fit equally, the one that leaves the shorter tail wins (a 40-byte
/// table also fits at 28). Falls back to the 12-byte implicit layout from
/// `0x30` when neither fits.
pub fn detect_layout(d: &[u8]) -> ElemTableLayout {
    let fallback = ElemTableLayout {
        start: 0x30,
        stride: 12,
        marker_offset: 0,
        framing: RecordFraming::Implicit,
    };
    let Some(count) = stated_count(d).filter(|&count| count > 0) else {
        return fallback;
    };
    let rising = |stride: usize| -> Option<(usize, usize)> {
        let end = count.checked_mul(stride)?.checked_add(RECORDS_ORIGIN)?;
        if end > d.len() {
            return None;
        }
        let mut previous = None;
        let mut rises = 0;
        for k in 0..count {
            let id = u32_at(d, RECORDS_ORIGIN + k * stride)?;
            if previous.is_some_and(|p| id > p) {
                rises += 1;
            }
            previous = Some(id);
        }
        Some((rises, d.len() - end))
    };
    let best = [40usize, 28]
        .into_iter()
        .filter_map(|stride| rising(stride).map(|(rises, tail)| (stride, rises, tail)))
        .filter(|&(_, rises, _)| (count - 1 - rises) * 100 <= count * MAX_OUT_OF_ORDER_PERCENT)
        .min_by_key(|&(_, _, tail)| tail);
    let Some((stride, _, _)) = best else {
        return fallback;
    };
    let (_, owner_at, owner_width) = record_fields(stride).expect("28 or 40");
    ElemTableLayout {
        start: RECORDS_ORIGIN,
        stride,
        marker_offset: owner_at,
        framing: RecordFraming::Explicit {
            marker_len: owner_width,
        },
    }
}

/// Parse only the header portion of Global/ElemTable. Sufficient for counts
/// + invariants; full record decode is in `parse_records`.
pub fn parse_header(rf: &mut RevitFile) -> Result<ElemTableHeader> {
    let d = inflate(rf)?;
    parse_header_bytes(&d)
}

/// Parse records from an already-decompressed ElemTable byte slice.
/// Splits the pure-byte-slice path out from `parse_records` (which takes
/// a `RevitFile` and handles the stream-read + inflate). Useful for fuzz
/// targets that want to feed synthetic inputs directly.
///
/// `limit` is the maximum number of records to return — typically the
/// table's stated count. Returns fewer records if the stream runs out of
/// bytes before `limit` is reached.
pub fn parse_records_from_bytes(
    d: &[u8],
    layout: ElemTableLayout,
    limit: usize,
) -> Vec<ElemRecord> {
    let mut records = Vec::new();
    if layout.stride == 0 {
        return records;
    }
    let mut i = layout.start;
    while records.len() < limit {
        let Some(record_end) = i.checked_add(layout.stride) else {
            break;
        };
        let Some(record) = d.get(i..record_end) else {
            break;
        };
        let (id_primary, id_secondary, owner_id) = match layout.framing {
            RecordFraming::Implicit => {
                let (Some(a), Some(b)) = (u32_at(record, 0), u32_at(record, 4)) else {
                    break;
                };
                (a, b, None)
            }
            RecordFraming::Explicit { marker_len } => {
                let second_at = record_fields(layout.stride).map_or(4, |(second, _, _)| second);
                let (Some(a), Some(b)) = (u32_at(record, 0), u32_at(record, second_at)) else {
                    break;
                };
                (a, b, owner_field(record, layout.marker_offset, marker_len))
            }
        };
        records.push(ElemRecord {
            offset: i,
            id_primary,
            id_secondary,
            owner_id,
            previous_ids: None,
            raw: record.to_vec(),
        });
        i = record_end;
    }
    records
}

/// The owner ElementId held in the field at `at` (`width` 4 or 8 bytes);
/// `None` for its all-`0xFF` unset value or a value outside the `u32`
/// ElementId range.
fn owner_field(record: &[u8], at: usize, width: usize) -> Option<u32> {
    let field = record.get(at..at.checked_add(width)?)?;
    let value = match width {
        4 => u64::from(u32::from_le_bytes(field.try_into().ok()?)),
        8 => u64::from_le_bytes(field.try_into().ok()?),
        _ => return None,
    };
    if field.iter().all(|&b| b == 0xFF) || value == 0 {
        return None;
    }
    u32::try_from(value).ok()
}

/// Inflate `Global/ElemTable` and detect its record layout.
pub fn read_layout(rf: &mut RevitFile) -> Result<ElemTableLayout> {
    Ok(detect_layout(&inflate(rf)?))
}

fn inflate(rf: &mut RevitFile) -> Result<Vec<u8>> {
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    compression::inflate_stream_at(GLOBAL_ELEM_TABLE, &raw, 8)
        .or_else(|_| compression::inflate_stream_at(GLOBAL_ELEM_TABLE, &raw, 0))
}

/// Parse all records from Global/ElemTable: the `u32` count at `0x02`
/// records from `0x06`, at the record size [`detect_layout`] finds. On the
/// implicit fallback layout the count is the header's `record_count`.
pub fn parse_records(rf: &mut RevitFile) -> Result<Vec<ElemRecord>> {
    let d = inflate(rf)?;
    let header = parse_header_bytes(&d)?;
    let layout = detect_layout(&d);
    let limit = match layout.framing {
        RecordFraming::Explicit { .. } => stated_count(&d).unwrap_or(0),
        RecordFraming::Implicit => usize::from(header.record_count),
    };
    Ok(parse_records_from_bytes(&d, layout, limit))
}

/// Index ElemTable records by `id_primary` for ElementId lookups.
///
/// Duplicate primary ids keep the first occurrence. Used by ArcWall
/// trailer linkage (RE-15) to confirm recovered ElementIds exist in
/// the declared table before exposing partition refs.
pub fn index_by_element_id(records: &[ElemRecord]) -> std::collections::BTreeMap<u32, &ElemRecord> {
    let mut map = std::collections::BTreeMap::new();
    for record in records {
        map.entry(record.id_primary).or_insert(record);
    }
    map
}

/// One ElementId that appears in both ElemTable and a decoded
/// partition ArcWall trailer.
#[derive(Debug, Clone)]
pub struct LinkedArcWallElement<'a> {
    pub element_id: u32,
    pub elem_record: &'a ElemRecord,
    pub partition_ref: crate::partition_arc_walls::PartitionArcWallRef,
}

/// Join ArcWall trailer ElementIds to ElemTable rows and partition
/// record refs. Returns only ids present in **both** maps.
pub fn link_arcwall_element_ids<'a>(
    elem_by_id: &std::collections::BTreeMap<u32, &'a ElemRecord>,
    partition_by_id: &std::collections::BTreeMap<
        u32,
        crate::partition_arc_walls::PartitionArcWallRef,
    >,
) -> Vec<LinkedArcWallElement<'a>> {
    let mut out = Vec::new();
    for (id, partition_ref) in partition_by_id {
        if let Some(&elem) = elem_by_id.get(id) {
            out.push(LinkedArcWallElement {
                element_id: *id,
                elem_record: elem,
                partition_ref: partition_ref.clone(),
            });
        }
    }
    out
}

/// Join generic partition-scanner ElementIds to ElemTable rows.
///
/// Thin wrapper around
/// [`crate::partition_scanner::link_elem_table_to_partitions`] so
/// ElemTable callers share one import path with the ArcWall-specific
/// linker above.
pub fn link_partition_element_ids<'a>(
    elem_by_id: &std::collections::BTreeMap<u32, &'a ElemRecord>,
    partition_by_id: &std::collections::BTreeMap<u32, crate::partition_scanner::PartitionRecordRef>,
) -> Vec<crate::partition_scanner::LinkedPartitionElement<'a>> {
    crate::partition_scanner::link_elem_table_to_partitions(elem_by_id, partition_by_id)
}

/// Authoritative set of declared ElementIds from Global/ElemTable.
///
/// Useful for walker coverage validation: after scanning `Global/Latest`
/// and building a `HandleIndex`, compare its key set against this to
/// quantify which elements the schema-directed walker found vs which the
/// file claims to contain.
///
/// Note per `docs/elem-table-record-layout-2026-04-21.md`: record payload
/// bytes do NOT encode a byte offset into `Global/Latest` — this function
/// returns IDs only, not offsets.
pub fn declared_element_ids(rf: &mut RevitFile) -> Result<Vec<u32>> {
    let records = parse_records(rf)?;
    Ok(declared_ids(&records).into_iter().collect())
}

/// Length of a record on the 40-byte layout (Revit 2024 and later).
pub const RECORD_LEN_40: usize = 40;

/// The ElementIds `records` declare (RE-41): every record's `id_primary`,
/// and on the 40-byte layout also its `id_secondary` when that is not `0`.
///
/// The two agree on every record of the 2016-2023 projects, the family
/// files and the 2024 and 2025 projects measured, except Autodesk's Snowdon
/// Towers samples. There they differ on 84 records (Architectural) and 27
/// (Structural), and `id_secondary` is the id the element's partition
/// record carries (RE-35) and the `Tag` Revit's own IFC export writes;
/// `id_primary` is neither. Both are kept, so no id declared before is
/// dropped. A `0` in `id_secondary` is not added.
pub fn declared_ids(records: &[ElemRecord]) -> std::collections::BTreeSet<u32> {
    let mut ids = std::collections::BTreeSet::new();
    for record in records {
        ids.insert(record.id_primary);
        if record.raw.len() == RECORD_LEN_40 && record.id_secondary != 0 {
            ids.insert(record.id_secondary);
        }
    }
    ids
}

/// Attempt to enumerate records after the header. Conservative: stops at
/// `max_records` or stream end. Prefer `parse_records` for new work — this
/// wrapper is kept for backward-compat with pre-corpus-probe callers.
pub fn parse_records_rough(rf: &mut RevitFile, max_records: usize) -> Result<Vec<ElemRecordRough>> {
    let d = inflate(rf)?;
    let layout = detect_layout(&d);
    let mut records = Vec::new();
    let mut i = layout.start;
    while i + 12 <= d.len() && records.len() < max_records {
        let a = u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
        let b = u32::from_le_bytes([d[i + 4], d[i + 5], d[i + 6], d[i + 7]]);
        let c = u32::from_le_bytes([d[i + 8], d[i + 9], d[i + 10], d[i + 11]]);
        records.push(ElemRecordRough {
            offset: i,
            presumptive_u32_triple: [a, b, c],
        });
        i += layout.stride.max(12);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_by_element_id_keeps_first_duplicate() {
        let records = vec![
            ElemRecord {
                offset: 0,
                id_primary: 7,
                id_secondary: 7,
                owner_id: None,
                previous_ids: None,
                raw: vec![1],
            },
            ElemRecord {
                offset: 28,
                id_primary: 7,
                id_secondary: 8,
                owner_id: None,
                previous_ids: None,
                raw: vec![2],
            },
            ElemRecord {
                offset: 56,
                id_primary: 9,
                id_secondary: 9,
                owner_id: None,
                previous_ids: None,
                raw: vec![3],
            },
        ];
        let index = index_by_element_id(&records);
        assert_eq!(index.len(), 2);
        assert_eq!(index.get(&7).map(|r| r.offset), Some(0));
        assert_eq!(index.get(&9).map(|r| r.id_secondary), Some(9));
    }
}
