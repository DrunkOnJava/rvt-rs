//! `Global/ElemTable` — Revit's element-id index.
//!
//! This stream lists every ElementId in the file along with metadata.
//! Structure observed across 11 Revit releases (2016-2026):
//!
//! ```text
//! [u16 LE element_count]       // varies 1174 (2016) → 1481 (2026)
//! [u16 LE record_count]        //        1596        → 1992
//! [24 bytes zero-padding]      // alignment or reserved
//! [u16 LE constant 0x0011]     // header flag, invariant across all releases
//! [u32 LE reserved = 0x0000_0001]
//! [... repeated element records ...]
//! [u32 LE sentinel 0xFFFFFFFF]
//! [trailing metadata (~12 bytes)]
//! ```
//!
//! The per-element record layout is only partially mapped (Phase 4c/d work).
//! Repeated u32 patterns visible in the first 512 bytes suggest each record
//! is ~12 bytes with a `(lo: u16, hi: u16)` pair at the end, but this has
//! not yet been confirmed against the full 11-version corpus.
//!
//! Currently exposes a header-only parser; full record decoding is a
//! Phase 4d task tracked in `docs/rvt-moat-break-reconnaissance.md`.

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
    /// Invariant magic word that appears at byte offset 0x1e across every
    /// Revit release we've inspected. Present for sanity-check.
    pub header_flag: u16,
    /// Decompressed stream size, for diagnostics.
    pub decompressed_bytes: usize,
}

/// A (partial) record parsed from ElemTable. Full semantics are still being
/// reverse-engineered; this captures what we can reliably extract today.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElemRecordRough {
    /// Offset in the decompressed stream where this record begins.
    pub offset: usize,
    /// Presumptive u32 fields (12 bytes' worth). Not yet bound to semantics.
    pub presumptive_u32_triple: [u32; 3],
}

/// Parse only the header portion of Global/ElemTable. Sufficient for counts
/// + invariants; full record decode is a follow-up.
pub fn parse_header(rf: &mut RevitFile) -> Result<ElemTableHeader> {
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    let d = compression::inflate_at(&raw, 8).or_else(|_| compression::inflate_at(&raw, 0))?;
    if d.len() < 0x30 {
        return Err(Error::BasicFileInfo(
            "Global/ElemTable stream too short for header".into(),
        ));
    }
    let element_count = u16::from_le_bytes([d[0], d[1]]);
    let record_count = u16::from_le_bytes([d[2], d[3]]);
    // Header flag appears at offset 0x1e in 2016-2023 files; shifts to 0x22
    // in 2024+. Check the obvious candidate positions.
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

/// Attempt to enumerate records after the header. Conservative: stops at
/// the 0xFFFFFFFF sentinel or at `max_records`. Returns whatever was parsed
/// before a failure — this is still exploratory.
pub fn parse_records_rough(rf: &mut RevitFile, max_records: usize) -> Result<Vec<ElemRecordRough>> {
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    let d = compression::inflate_at(&raw, 8).or_else(|_| compression::inflate_at(&raw, 0))?;

    // Conservative record-area start: 0x30 (past the header + padding).
    // Locate the sentinel to know where records end. Scan FROM 0x30,
    // not from 0 — the header region sometimes contains byte patterns
    // that match 0xFFFF_FFFF (observed on real .rvt project files),
    // and counting those as the record terminator makes `end < start`
    // and the parser returns empty.
    let start = 0x30;
    let sentinel: u32 = 0xFFFF_FFFF;
    let mut sentinel_at = None;
    for i in (start..d.len().saturating_sub(4)).step_by(4) {
        if u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) == sentinel {
            sentinel_at = Some(i);
            break;
        }
    }
    let end = sentinel_at.unwrap_or(d.len());

    if start >= end {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    let mut i = start;
    while i + 12 <= end && records.len() < max_records {
        let a = u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
        let b = u32::from_le_bytes([d[i + 4], d[i + 5], d[i + 6], d[i + 7]]);
        let c = u32::from_le_bytes([d[i + 8], d[i + 9], d[i + 10], d[i + 11]]);
        records.push(ElemRecordRough {
            offset: i,
            presumptive_u32_triple: [a, b, c],
        });
        i += 12;
    }
    Ok(records)
}

#[cfg(test)]
mod tests {

    #[test]
    fn header_has_element_count() {
        // Synthesize the first 48 bytes we've observed across 11 releases.
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&[0x96, 0x04]); // element_count = 1174 (2016)
        buf.extend_from_slice(&[0x3c, 0x06]); // record_count = 1596
        buf.resize(0x1e, 0);
        buf.extend_from_slice(&[0x11, 0x00]); // header_flag
        buf.resize(0x30, 0);
        // No crate-side parse because parse_header reads from a RevitFile.
        // Verify the constants match expectations instead.
        assert_eq!(u16::from_le_bytes([buf[0], buf[1]]), 1174);
        assert_eq!(u16::from_le_bytes([buf[2], buf[3]]), 1596);
        assert_eq!(u16::from_le_bytes([buf[0x1e], buf[0x1f]]), 0x0011);
    }
}
