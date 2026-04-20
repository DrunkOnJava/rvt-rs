//! Layer 5a walker — schema-directed instance reader for ADocument.
//!
//! Reads the root `ADocument` class's instance data from
//! `Global/Latest` using the 100 %-classified schema produced by
//! `formats::parse_schema`. Current status: **partial**. Validated
//! cross-version on Revit 2024–2026, where the walker reads all 13
//! declared fields with 8 of 13 producing clean values and the last
//! three ElementIds (`m_ownerFamilyId`, `m_ownerFamilyContainingGroupId`,
//! `m_devBranchInfo`) matching byte-for-byte across those three
//! releases — strong cross-version validation of both the decoder
//! and the entry-point detector.
//!
//! Fields 2–5 and the 2016–2023 stream layout still need work. See
//! `docs/rvt-moat-break-reconnaissance.md` §Q6.5 for the full decode
//! state + open questions.
//!
//! Wire encoding (observed as of v0.1.2):
//!
//! | FieldType | Wire shape |
//! |---|---|
//! | `Pointer { .. }` | 8 bytes `[u32 slot_a][u32 slot_b]` — `00 00 00 00 00 00 00 00` means NULL; `0xff…f` also NULL-like |
//! | `ElementId` / `ElementIdRef` | 8 bytes `[u32 tag_or_zero][u32 id]` — `id` is the runtime element identifier |
//! | `Container { kind: 0x0e, .. }` | 2-column — `[u32 count][count × 6-byte [u16 id][u32 mask]][u32 count2][count2 × 6-byte records]` |
//! | Other `FieldType` variants | not yet exercised in ADocument — TBD |

use crate::{Error, Result, RevitFile, compression, formats, streams};

/// One field's value as read by the walker. Best-effort — only the
/// variants that `ADocument` exercises today are meaningfully
/// populated; others return `Bytes` with raw wire bytes for downstream
/// tools to reanalyse.
#[derive(Debug, Clone)]
pub enum InstanceField {
    /// `[u32 a][u32 b]` pointer slot. Both-zero means NULL; both-ones
    /// also sometimes seen as a NULL sentinel.
    Pointer { raw: [u32; 2] },
    /// Typed element reference. `tag` is 0 for references that don't
    /// carry a class-tag on the wire; `id` is the runtime ElementId.
    ElementId { tag: u32, id: u32 },
    /// Reference container, 2-column layout: `col_a` is the primary
    /// id list, `col_b` is typically masks or a parallel id stream.
    RefContainer { col_a: Vec<u16>, col_b: Vec<u16> },
    /// Unused / unexercised paths return the raw bytes consumed.
    Bytes(Vec<u8>),
}

/// ADocument's instance, as extracted by the v0.1.2 walker. Field
/// names mirror the schema exactly.
#[derive(Debug, Clone)]
pub struct ADocumentInstance {
    /// Byte offset in `Global/Latest`'s decompressed stream where the
    /// ADocument record was found.
    pub entry_offset: usize,
    /// Revit release (from `BasicFileInfo`), useful for downstream
    /// consumers that want to know which layout was parsed.
    pub version: u32,
    /// Parsed fields, one entry per declared schema field, in order.
    pub fields: Vec<(String, InstanceField)>,
}

/// Read ADocument from a `RevitFile`. Returns `None` if the
/// entry-point detector can't confidently land on the record —
/// currently reliable on Revit 2024+ releases; older releases return
/// `None`.
pub fn read_adocument(rf: &mut RevitFile) -> Result<Option<ADocumentInstance>> {
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema
        .classes
        .iter()
        .find(|c| c.name == "ADocument")
        .ok_or_else(|| Error::BasicFileInfo("ADocument not in schema".into()))?;

    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;
    let Some(entry) = find_adocument_start_with_schema(&d, Some(adoc)) else {
        return Ok(None);
    };

    let mut cursor = entry;
    let mut fields = Vec::with_capacity(adoc.fields.len());
    for field in &adoc.fields {
        let Some(ft) = &field.field_type else {
            break;
        };
        let Some((consumed, value)) = read_field(ft, &d[cursor..]) else {
            break;
        };
        fields.push((field.name.clone(), value));
        cursor = cursor.saturating_add(consumed);
        if cursor > d.len() {
            break;
        }
    }

    let version = rf.basic_file_info().ok().map(|b| b.version).unwrap_or(0);
    Ok(Some(ADocumentInstance {
        entry_offset: entry,
        version,
        fields,
    }))
}

#[cfg(test)]
fn find_adocument_start(d: &[u8]) -> Option<usize> {
    find_adocument_start_with_schema(d, None)
}

/// Locate ADocument's entry point. When an ADocument class schema is
/// supplied, also runs a scoring-based brute-force scan that picks
/// the offset whose trial walk produces the most-sensible values for
/// the last three fields (small distinct `ElementId` ids with tag=0).
/// That's strong enough to find the entry point on Revit 2021–2026,
/// where the heuristic-only path misses 2021–2023.
fn find_adocument_start_with_schema(
    d: &[u8],
    adoc_schema: Option<&formats::ClassEntry>,
) -> Option<usize> {
    // Strategy 1: sequential-id-table end + 8-zero signature scan.
    if let Some(h) = heuristic_find(d) {
        if let Some(cls) = adoc_schema {
            if trial_walk(cls, &d[h..]).is_some_and(|w| walk_score(&w) >= 90) {
                return Some(h);
            }
        } else {
            return Some(h);
        }
    }
    // Strategy 2: score-based byte-aligned brute-force scan. Only
    // runs when a schema is supplied.
    if let Some(cls) = adoc_schema {
        let mut best: Option<(i64, usize)> = None;
        let end = d.len().saturating_sub(256);
        for offset in 0x100..end {
            if let Some(walk) = trial_walk(cls, &d[offset..]) {
                let sc = walk_score(&walk);
                if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                    best = Some((sc, offset));
                }
            }
        }
        if let Some((sc, off)) = best {
            if sc >= 80 {
                return Some(off);
            }
        }
    }
    None
}

fn heuristic_find(d: &[u8]) -> Option<usize> {
    let mut last_table_end = 0usize;
    let mut i = 0;
    while i + 4 < d.len() {
        if d[i..i + 4] == [1, 0, 0, 0] {
            let mut cursor = i + 4;
            let mut expect: u32 = 2;
            let mut end = i + 4;
            while cursor + 4 <= d.len() {
                let marker = expect.to_le_bytes();
                let window_end = (cursor + 64).min(d.len());
                if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
                    end = cursor + p + 4;
                    cursor = end;
                    expect += 1;
                } else {
                    break;
                }
            }
            if expect >= 6 {
                last_table_end = end + 32;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    let min_start = last_table_end.max(0x200);
    let mut k = min_start;
    while k + 16 <= d.len() {
        if d[k..k + 8].iter().all(|&b| b == 0) {
            let next_u32 = u32::from_le_bytes([d[k + 8], d[k + 9], d[k + 10], d[k + 11]]);
            let next_next = u32::from_le_bytes([d[k + 12], d[k + 13], d[k + 14], d[k + 15]]);
            if (1..=100).contains(&next_u32) && (next_next == 0xffffffff || next_next <= 0x10000) {
                return Some(k);
            }
        }
        k += 1;
    }
    None
}

fn trial_walk(adoc: &formats::ClassEntry, bytes: &[u8]) -> Option<Vec<(u32, u32)>> {
    let mut cursor = 0;
    let mut out = Vec::new();
    for field in &adoc.fields {
        let ft = field.field_type.as_ref()?;
        let (consumed, tag_id) = match ft {
            formats::FieldType::Pointer { .. } => {
                if cursor + 8 > bytes.len() {
                    return None;
                }
                (8, (u32::MAX, u32::MAX))
            }
            formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. } => {
                if cursor + 8 > bytes.len() {
                    return None;
                }
                let tag = u32::from_le_bytes([
                    bytes[cursor],
                    bytes[cursor + 1],
                    bytes[cursor + 2],
                    bytes[cursor + 3],
                ]);
                let id = u32::from_le_bytes([
                    bytes[cursor + 4],
                    bytes[cursor + 5],
                    bytes[cursor + 6],
                    bytes[cursor + 7],
                ]);
                (8, (tag, id))
            }
            formats::FieldType::Container { kind: 0x0e, .. } => {
                if cursor + 4 > bytes.len() {
                    return None;
                }
                let count = u32::from_le_bytes([
                    bytes[cursor],
                    bytes[cursor + 1],
                    bytes[cursor + 2],
                    bytes[cursor + 3],
                ]) as usize;
                if count > 1000 {
                    return None;
                }
                let col_bytes = 4 + count * 6;
                if cursor + col_bytes + 4 > bytes.len() {
                    return None;
                }
                let col2_count = u32::from_le_bytes([
                    bytes[cursor + col_bytes],
                    bytes[cursor + col_bytes + 1],
                    bytes[cursor + col_bytes + 2],
                    bytes[cursor + col_bytes + 3],
                ]) as usize;
                if col2_count != count {
                    return None;
                }
                (2 * col_bytes, (u32::MAX, u32::MAX))
            }
            _ => return None,
        };
        out.push(tag_id);
        cursor += consumed;
    }
    Some(out)
}

fn walk_score(walk: &[(u32, u32)]) -> i64 {
    if walk.len() < 3 {
        return i64::MIN;
    }
    let last3 = &walk[walk.len() - 3..];
    let real_ids: Vec<u32> = last3
        .iter()
        .filter(|(t, _)| *t != u32::MAX)
        .map(|(_, i)| *i)
        .collect();
    if real_ids.is_empty() || real_ids.iter().all(|i| *i == 0) {
        return i64::MIN;
    }
    let mut s: i64 = 0;
    for (t, i) in last3 {
        if *t == u32::MAX {
            continue;
        }
        if *t == 0 {
            s += 10;
        }
        if (1..=10000).contains(i) {
            s += 20;
        } else if (1..=0xffff).contains(i) {
            s += 5;
        } else {
            s -= 10;
        }
    }
    if real_ids.len() >= 2 {
        let max = *real_ids.iter().max().unwrap();
        let min = *real_ids.iter().min().unwrap();
        if max > 0 && max - min <= 50 {
            s += 25;
        }
    }
    s
}

fn read_field(ft: &formats::FieldType, bytes: &[u8]) -> Option<(usize, InstanceField)> {
    match ft {
        formats::FieldType::Pointer { .. } => {
            if bytes.len() < 8 {
                return None;
            }
            let a = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let b = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            Some((8, InstanceField::Pointer { raw: [a, b] }))
        }
        formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. } => {
            if bytes.len() < 8 {
                return None;
            }
            let tag = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let id = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            Some((8, InstanceField::ElementId { tag, id }))
        }
        formats::FieldType::Container { kind: 0x0e, .. } => {
            if bytes.len() < 4 {
                return None;
            }
            let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            if count > 1000 {
                return None;
            }
            let elem_size = 6;
            let col_bytes = 4 + count * elem_size;
            let total = 2 * col_bytes;
            if bytes.len() < total {
                return None;
            }
            let col2_count = u32::from_le_bytes([
                bytes[col_bytes],
                bytes[col_bytes + 1],
                bytes[col_bytes + 2],
                bytes[col_bytes + 3],
            ]) as usize;
            if col2_count != count {
                // Fallback to 1-column shape on mismatch.
                let mut col_a = Vec::with_capacity(count);
                for k in 0..count {
                    let base = 4 + k * elem_size;
                    col_a.push(u16::from_le_bytes([bytes[base], bytes[base + 1]]));
                }
                return Some((
                    col_bytes,
                    InstanceField::RefContainer {
                        col_a,
                        col_b: Vec::new(),
                    },
                ));
            }
            let mut col_a = Vec::with_capacity(count);
            let mut col_b = Vec::with_capacity(count);
            for k in 0..count {
                let base_a = 4 + k * elem_size;
                let base_b = col_bytes + 4 + k * elem_size;
                col_a.push(u16::from_le_bytes([bytes[base_a], bytes[base_a + 1]]));
                col_b.push(u16::from_le_bytes([bytes[base_b], bytes[base_b + 1]]));
            }
            Some((total, InstanceField::RefContainer { col_a, col_b }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_adocument_start_returns_none_on_empty_stream() {
        assert_eq!(find_adocument_start(&[]), None);
        assert_eq!(find_adocument_start(&[0u8; 0x200]), None);
    }

    #[test]
    fn read_field_pointer_reads_8_bytes() {
        let ft = formats::FieldType::Pointer { kind: 2 };
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00];
        let (n, v) = read_field(&ft, &bytes).unwrap();
        assert_eq!(n, 8);
        match v {
            InstanceField::Pointer { raw } => assert_eq!(raw, [1, 2]),
            _ => panic!("expected Pointer"),
        }
    }

    #[test]
    fn read_field_element_id_reads_8_bytes() {
        let ft = formats::FieldType::ElementId;
        let bytes = [0x00u8, 0x00, 0x00, 0x00, 0x1b, 0x00, 0x00, 0x00];
        let (n, v) = read_field(&ft, &bytes).unwrap();
        assert_eq!(n, 8);
        match v {
            InstanceField::ElementId { tag, id } => {
                assert_eq!(tag, 0);
                assert_eq!(id, 27);
            }
            _ => panic!("expected ElementId"),
        }
    }

    #[test]
    fn read_field_ref_container_2column() {
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        // count=2, col1=[0xaaaa, 0xbbbb], col2=[0xcccc, 0xdddd]
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[2, 0, 0, 0]); // count1
        bytes.extend_from_slice(&[0xaa, 0xaa, 0xff, 0xff, 0xff, 0xff]); // row1a
        bytes.extend_from_slice(&[0xbb, 0xbb, 0xff, 0xff, 0xff, 0xff]); // row2a
        bytes.extend_from_slice(&[2, 0, 0, 0]); // count2
        bytes.extend_from_slice(&[0xcc, 0xcc, 0xff, 0xff, 0xff, 0xff]); // row1b
        bytes.extend_from_slice(&[0xdd, 0xdd, 0xff, 0xff, 0xff, 0xff]); // row2b
        let (n, v) = read_field(&ft, &bytes).unwrap();
        assert_eq!(n, 32); // 2 * (4 + 2*6)
        match v {
            InstanceField::RefContainer { col_a, col_b } => {
                assert_eq!(col_a, vec![0xaaaa, 0xbbbb]);
                assert_eq!(col_b, vec![0xcccc, 0xdddd]);
            }
            _ => panic!("expected RefContainer"),
        }
    }
}
