//! `Global/ElemTable` — Revit's element-id index.
//!
//! This stream lists every ElementId in the file along with metadata.
//! The record layout varies by file variant (see
//! `docs/elem-table-record-layout-2026-04-21.md` for the hex-dump
//! reverse-engineering notes):
//!
//! | Variant               | Record start | Marker per record      | Record size |
//! | ---                   | ---          | ---                    | ---         |
//! | Family (.rfa, 2016-2026) | `0x30`     | none (implicit)        | 12 B        |
//! | Project 2023 (.rvt)   | `0x1E`       | `FF FF FF FF` (4 B)    | 28 B        |
//! | Project 2024 (.rvt)   | `0x22`       | `FF`×8 (8 B)           | 40 B        |
//!
//! Header (bytes 0..0x10) is common across all variants:
//!
//! ```text
//! [u16 LE element_count]
//! [u16 LE record_count]
//! [12 bytes zero-padding]
//! ```
//!
//! The `header_flag = 0x0011` at `0x22` is present only on family files.
//! Project files have either zeros or the record-0 marker at that offset,
//! so `parse_header` returns 0 for the flag on those variants — not a
//! parser bug, the flag genuinely isn't there.

use crate::{Error, Result, RevitFile, compression, streams::GLOBAL_ELEM_TABLE};
use serde::{Deserialize, Serialize};

/// Header extracted from the first 32 bytes of decompressed Global/ElemTable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElemTableHeader {
    /// Declared number of distinct ElementIds in this file.
    pub element_count: u16,
    /// Declared number of records (may differ if some elements have multiple
    /// records, e.g. versioned entries).
    pub record_count: u16,
    /// Invariant magic word that appears at byte offset 0x1e on family files
    /// across every Revit release we've inspected. 0 on project files.
    pub header_flag: u16,
    /// Decompressed stream size, for diagnostics.
    pub decompressed_bytes: usize,
}

/// How records are framed in this ElemTable stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordFraming {
    /// Family files: 12-byte homogeneous records, no per-record marker.
    Implicit,
    /// Project files: each record begins with N FF bytes (4 on 2023, 8 on 2024).
    Explicit { marker_len: usize },
}

/// Detected record layout — where records start, how big they are, how they're framed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElemTableLayout {
    pub start: usize,
    pub stride: usize,
    pub framing: RecordFraming,
}

/// A fully-parsed record from ElemTable.
///
/// On family files, `id_primary`/`id_secondary` are the first two `u32`s of
/// the 12-byte record (semantics still exploratory — on observed samples both
/// are 0x0000003F). On project files, these are the monotonic element-id pair
/// that starts each record past the marker; observations show `id_secondary`
/// matches `id_primary` on most rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElemRecord {
    /// Offset in the decompressed stream where this record begins.
    pub offset: usize,
    /// First u32 after the marker (element id on project files).
    pub id_primary: u32,
    /// Second u32 (secondary id / version on project files).
    pub id_secondary: u32,
    /// Raw record bytes (including the marker on project files).
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

/// Detect the record layout by finding the first two per-record markers and
/// taking their stride. Falls back to the family-file implicit layout
/// (12 B from `0x30`) when no markers are present.
pub fn detect_layout(d: &[u8]) -> ElemTableLayout {
    let scan_start = 0x10usize;
    let scan_end = d.len().min(512);

    let mut markers: Vec<(usize, usize)> = Vec::with_capacity(3);
    let mut i = scan_start;
    while i + 4 <= scan_end && markers.len() < 3 {
        if d[i] == 0xFF && d[i + 1] == 0xFF && d[i + 2] == 0xFF && d[i + 3] == 0xFF {
            let eight_ff = i + 8 <= d.len()
                && d[i + 4] == 0xFF
                && d[i + 5] == 0xFF
                && d[i + 6] == 0xFF
                && d[i + 7] == 0xFF;
            let len = if eight_ff { 8 } else { 4 };
            markers.push((i, len));
            i += len;
        } else {
            i += 1;
        }
    }

    if markers.len() >= 2 {
        let (m0, marker_len) = markers[0];
        let (m1, _) = markers[1];
        let stride = m1 - m0;
        ElemTableLayout {
            start: m0,
            stride,
            framing: RecordFraming::Explicit { marker_len },
        }
    } else {
        ElemTableLayout {
            start: 0x30,
            stride: 12,
            framing: RecordFraming::Implicit,
        }
    }
}

/// Parse only the header portion of Global/ElemTable. Sufficient for counts
/// + invariants; full record decode is in `parse_records`.
pub fn parse_header(rf: &mut RevitFile) -> Result<ElemTableHeader> {
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    let d = compression::inflate_at(&raw, 8).or_else(|_| compression::inflate_at(&raw, 0))?;
    parse_header_bytes(&d)
}

/// Parse records from an already-decompressed ElemTable byte slice.
/// Splits the pure-byte-slice path out from `parse_records` (which takes
/// a `RevitFile` and handles the stream-read + inflate). Useful for fuzz
/// targets and unit tests that want to feed synthetic inputs directly.
///
/// `limit` is the maximum number of records to return — typically
/// `header.record_count` from `parse_header_bytes`. Returns fewer
/// records if the stream runs out of bytes before `limit` is reached.
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
        if record_end > d.len() {
            break;
        }
        let (id_primary, id_secondary) = match layout.framing {
            RecordFraming::Implicit => {
                let a = u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
                let b = u32::from_le_bytes([d[i + 4], d[i + 5], d[i + 6], d[i + 7]]);
                (a, b)
            }
            RecordFraming::Explicit { marker_len } => {
                let Some(body) = i.checked_add(marker_len) else {
                    break;
                };
                if body > record_end {
                    break;
                }
                if layout.stride == 28 {
                    let Some(body_end) = body.checked_add(8) else {
                        break;
                    };
                    if body_end > record_end {
                        break;
                    }
                    let a = u32::from_le_bytes([d[body], d[body + 1], d[body + 2], d[body + 3]]);
                    let b =
                        u32::from_le_bytes([d[body + 4], d[body + 5], d[body + 6], d[body + 7]]);
                    (a, b)
                } else if layout.stride == 40 {
                    let Some(body_end) = body.checked_add(28) else {
                        break;
                    };
                    if body_end > record_end {
                        break;
                    }
                    // 40-byte layout (observed on Revit 2024 projects):
                    //   [8 B marker][4 B zero][u32 id_primary][16 B zero/payload][u32 id_secondary][8 B payload]
                    // id_primary is at body+4, id_secondary at body+24
                    // (record offsets +12 and +32 respectively).
                    let a =
                        u32::from_le_bytes([d[body + 4], d[body + 5], d[body + 6], d[body + 7]]);
                    let b = u32::from_le_bytes([
                        d[body + 24],
                        d[body + 25],
                        d[body + 26],
                        d[body + 27],
                    ]);
                    (a, b)
                } else {
                    let Some(body_end) = body.checked_add(4) else {
                        break;
                    };
                    if body_end > record_end {
                        break;
                    }
                    let a = u32::from_le_bytes([d[body], d[body + 1], d[body + 2], d[body + 3]]);
                    (a, a)
                }
            }
        };
        let raw = d[i..record_end].to_vec();
        records.push(ElemRecord {
            offset: i,
            id_primary,
            id_secondary,
            raw,
        });
        i = record_end;
    }
    records
}

/// Parse all records from Global/ElemTable, bounded by the header's
/// `record_count`. Uses `detect_layout` to pick the correct stride/start for
/// each file variant, so works on both family and project files.
pub fn parse_records(rf: &mut RevitFile) -> Result<Vec<ElemRecord>> {
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    let d = compression::inflate_at(&raw, 8).or_else(|_| compression::inflate_at(&raw, 0))?;
    let header = parse_header_bytes(&d)?;
    let layout = detect_layout(&d);
    let limit = header.record_count as usize;
    Ok(parse_records_from_bytes(&d, layout, limit))
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
    let mut ids: Vec<u32> = records.iter().map(|r| r.id_primary).collect();
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Attempt to enumerate records after the header. Conservative: stops at
/// `max_records` or stream end. Prefer `parse_records` for new work — this
/// wrapper is kept for backward-compat with pre-corpus-probe callers.
pub fn parse_records_rough(rf: &mut RevitFile, max_records: usize) -> Result<Vec<ElemRecordRough>> {
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    let d = compression::inflate_at(&raw, 8).or_else(|_| compression::inflate_at(&raw, 0))?;
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
    fn header_has_element_count() {
        // Synthesize the first 48 bytes we've observed across 11 releases.
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&[0x96, 0x04]); // element_count = 1174 (2016)
        buf.extend_from_slice(&[0x3c, 0x06]); // record_count = 1596
        buf.resize(0x1e, 0);
        buf.extend_from_slice(&[0x11, 0x00]); // header_flag
        buf.resize(0x30, 0);
        assert_eq!(u16::from_le_bytes([buf[0], buf[1]]), 1174);
        assert_eq!(u16::from_le_bytes([buf[2], buf[3]]), 1596);
        assert_eq!(u16::from_le_bytes([buf[0x1e], buf[0x1f]]), 0x0011);
    }

    #[test]
    fn detect_family_layout_falls_back_to_implicit_12b() {
        // Family-file header: no FF markers in first 512 bytes.
        let mut buf = vec![0u8; 0x80];
        buf[0] = 0x83;
        buf[1] = 0x05;
        buf[2] = 0xb7;
        buf[3] = 0x07;
        buf[0x22] = 0x11;
        buf[0x23] = 0x00;
        let layout = detect_layout(&buf);
        assert_eq!(layout.framing, RecordFraming::Implicit);
        assert_eq!(layout.start, 0x30);
        assert_eq!(layout.stride, 12);
    }

    #[test]
    fn detect_project_2023_layout_28b_4byte_marker() {
        // Synthesize header + two 28B records with 4-byte FF FF FF FF markers.
        let mut buf = vec![0u8; 0x80];
        buf[0x1e] = 0xff;
        buf[0x1f] = 0xff;
        buf[0x20] = 0xff;
        buf[0x21] = 0xff;
        buf[0x22] = 0x01;
        buf[0x3a] = 0xff;
        buf[0x3b] = 0xff;
        buf[0x3c] = 0xff;
        buf[0x3d] = 0xff;
        buf[0x3e] = 0x02;
        let layout = detect_layout(&buf);
        assert_eq!(layout.framing, RecordFraming::Explicit { marker_len: 4 });
        assert_eq!(layout.start, 0x1e);
        assert_eq!(layout.stride, 28);
    }

    #[test]
    fn detect_project_2024_layout_40b_8byte_marker() {
        // Synthesize header + two 40B records with 8-byte FF markers.
        let mut buf = vec![0u8; 0x80];
        buf[0x22..0x2a].fill(0xff);
        buf[0x2e] = 0x01;
        buf[0x4a..0x52].fill(0xff);
        buf[0x56] = 0x02;
        let layout = detect_layout(&buf);
        assert_eq!(layout.framing, RecordFraming::Explicit { marker_len: 8 });
        assert_eq!(layout.start, 0x22);
        assert_eq!(layout.stride, 40);
    }

    #[test]
    fn parse_records_honors_header_record_count_on_project_2023_layout() {
        let mut buf = vec![0u8; 0x200];
        buf[2] = 0x02; // record_count = 2
        buf[3] = 0x00;
        buf[0x1e] = 0xff;
        buf[0x1f] = 0xff;
        buf[0x20] = 0xff;
        buf[0x21] = 0xff;
        buf[0x22] = 0x01; // id_primary = 1
        buf[0x26] = 0x01; // id_secondary = 1
        buf[0x3a] = 0xff;
        buf[0x3b] = 0xff;
        buf[0x3c] = 0xff;
        buf[0x3d] = 0xff;
        buf[0x3e] = 0x02; // id_primary = 2
        buf[0x42] = 0x02; // id_secondary = 2
        buf[0x56] = 0xff; // third marker — we should stop before it
        buf[0x57] = 0xff;
        buf[0x58] = 0xff;
        buf[0x59] = 0xff;
        let layout = detect_layout(&buf);
        let records = parse_records_from_bytes(&buf, layout, 2);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id_primary, 1);
        assert_eq!(records[0].id_secondary, 1);
        assert_eq!(records[1].id_primary, 2);
        assert_eq!(records[1].id_secondary, 2);
        assert_eq!(records[0].offset, 0x1e);
        assert_eq!(records[1].offset, 0x3a);
    }

    #[test]
    fn parse_records_project_2024_layout_reads_id_at_offset_plus_12_and_32() {
        // 40-byte record, record_start=0x22. body=record_start+8=0x2a.
        // id_primary at body+4 = 0x2e (record_start+12).
        // id_secondary at body+24 = 0x42 (record_start+32).
        let mut buf = vec![0u8; 0x200];
        buf[2] = 0x02;
        buf[3] = 0x00;
        buf[0x22..0x2a].fill(0xff);
        buf[0x2e] = 0x01;
        buf[0x42] = 0x01;
        buf[0x4a..0x52].fill(0xff);
        // next record_start=0x4a. body=0x52. body+4=0x56, body+24=0x6a.
        buf[0x56] = 0x02;
        buf[0x6a] = 0x02;
        let layout = detect_layout(&buf);
        assert_eq!(layout.stride, 40);
        let records = parse_records_from_bytes(&buf, layout, 2);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id_primary, 1);
        assert_eq!(records[0].id_secondary, 1);
        assert_eq!(records[1].id_primary, 2);
        assert_eq!(records[1].id_secondary, 2);
    }
}
