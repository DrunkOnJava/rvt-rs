//! Cross-version ADocument entry-point finder. For every 4-byte-aligned
//! offset in `Global/Latest` past the history block, try running the
//! walker from there. Score each candidate on the "sensibility" of its
//! last-three-fields read: for the 2024–2026 samples we know those
//! fields are `ElementIdRef`s with tag=0 and small ids (27, 31, 35).
//! A correct entry-point reproduces that shape across all versions.
//!
//! Output: the best-scoring offset per sample, plus the 3-tuple of
//! ElementId ids the walker reads there. Used to find the true
//! ADocument entry offset in the 2016–2023 samples where the
//! heuristic-based detector fails.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, formats, streams};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: adocument_bruteforce <file>");
    let mut rf = RevitFile::open(&path)?;

    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema
        .classes
        .iter()
        .find(|c| c.name == "ADocument")
        .ok_or_else(|| anyhow::anyhow!("ADocument not in schema"))?;

    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;
    println!(
        "Global/Latest decompressed: {} bytes. Scanning 4-byte-aligned offsets from 0x100..end-256.",
        d.len()
    );

    // For a candidate offset, compute the cumulative byte cost of
    // walking ADocument's 13 fields per our current wire-encoding
    // guesses. Return the walker's output if successful, else None.
    fn try_walk(adoc: &formats::ClassEntry, bytes: &[u8]) -> Option<Vec<(String, u32, u32)>> {
        let mut cursor = 0;
        let mut out = Vec::new();
        for field in &adoc.fields {
            let Some(ft) = &field.field_type else {
                return None;
            };
            let consumed = match ft {
                formats::FieldType::Pointer { .. } => 8,
                formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. } => 8,
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
                    let total = 2 * col_bytes;
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
                    total
                }
                _ => return None,
            };
            if cursor + consumed > bytes.len() {
                return None;
            }
            // For the last-3 (ElementId-family) fields, also capture
            // the (tag, id) values for scoring.
            let tag_id = if matches!(ft, formats::FieldType::ElementId)
                || matches!(ft, formats::FieldType::ElementIdRef { .. })
            {
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
                (tag, id)
            } else {
                (u32::MAX, u32::MAX)
            };
            out.push((field.name.clone(), tag_id.0, tag_id.1));
            cursor += consumed;
        }
        Some(out)
    }

    // Score: how "sensible" are the last 3 fields as small-int
    // ElementIds with tag=0? Higher = more sensible. Exclude cases
    // where all three ids are 0 (we see that at many random offsets
    // near big zero regions).
    fn score(walk: &[(String, u32, u32)]) -> i64 {
        if walk.len() < 3 {
            return i64::MIN;
        }
        let last3: Vec<(u32, u32)> = walk[walk.len() - 3..]
            .iter()
            .map(|(_, t, i)| (*t, *i))
            .collect();
        let all_zero = last3.iter().all(|(_, i)| *i == 0);
        if all_zero {
            return i64::MIN;
        }
        let mut s: i64 = 0;
        for (t, i) in &last3 {
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
        // Bonus if the three ids look sequential / monotonic / close
        let ids: Vec<u32> = last3.iter().map(|(_, i)| *i).collect();
        let max_i = *ids.iter().max().unwrap();
        let min_i = *ids.iter().min().unwrap();
        if max_i > 0 && max_i - min_i <= 50 {
            s += 25;
        }
        s
    }

    // Scan every byte position (not 4-byte-aligned) — the
    // decompressed stream isn't guaranteed 4-aligned at interior
    // offsets, and empirically the 2024 sample's true ADocument entry
    // (0x0f67) isn't aligned.
    let mut best: Option<(i64, usize, Vec<(String, u32, u32)>)> = None;
    let start = 0x100;
    let end = d.len().saturating_sub(256);
    for offset in start..end {
        if let Some(walk) = try_walk(adoc, &d[offset..]) {
            let sc = score(&walk);
            if best.as_ref().is_none_or(|(bs, _, _)| sc > *bs) {
                best = Some((sc, offset, walk));
            }
        }
    }

    if let Some((sc, off, walk)) = best {
        println!("\nBest candidate: offset 0x{off:06x}, score {sc}");
        for (name, tag, id) in walk.iter() {
            if *tag == u32::MAX {
                println!("  {name:<36} [non-ElementId]");
            } else {
                println!("  {name:<36} tag={tag}  id={id}");
            }
        }
    } else {
        println!("No plausible candidate found.");
    }

    Ok(())
}
