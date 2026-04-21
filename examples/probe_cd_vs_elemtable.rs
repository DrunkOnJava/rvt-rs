//! RE-22 — correlate ContentDocuments IDs with Global/ElemTable IDs.
//!
//! Extract every u64 value from ContentDocuments that looks like an
//! ID (appears at multiple positions consistent with a record),
//! compare against ElemTable's declared IDs.
//!
//! Also extract CD IDs by reading u64 at every 0x14 offset within a
//! sliding window aligned to observed record starts.

use rvt::{RevitFile, compression, elem_table};
use std::collections::BTreeSet;

fn read_u64(b: &[u8], o: usize) -> Option<u64> {
    if o + 8 > b.len() {
        return None;
    }
    Some(u64::from_le_bytes([
        b[o],
        b[o + 1],
        b[o + 2],
        b[o + 3],
        b[o + 4],
        b[o + 5],
        b[o + 6],
        b[o + 7],
    ]))
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/2024_Core_Interior.rvt");
    let mut rf = RevitFile::open(&path).unwrap();

    // ElemTable IDs.
    let et_records = elem_table::parse_records(&mut rf).unwrap();
    let elem_ids: BTreeSet<u64> = et_records
        .iter()
        .flat_map(|r| [r.id_primary as u64, r.id_secondary as u64])
        .filter(|id| *id != 0 && *id != u64::from(u32::MAX))
        .collect();

    // ContentDocuments IDs — extract u64 at record-aligned offsets.
    let cd_raw = rf.read_stream("Global/ContentDocuments").unwrap();
    let (_, cd) = compression::inflate_at_auto(&cd_raw).unwrap();

    // Anchor at 0x85 per RE-17/20. Stride 40 bytes. id is at offset 0.
    let anchor = 0x85;
    let mut cd_ids: BTreeSet<u64> = BTreeSet::new();
    let mut off = anchor;
    while off + 40 <= cd.len() {
        // Only accept the id if the marker at +16 is 0xFFFFFFFF
        // (so we're confident it's a valid base record, not in the
        // variable-length region).
        if let Some(v) = read_u64(&cd, off) {
            let marker_ok = cd[off + 16..off + 20] == [0xFF, 0xFF, 0xFF, 0xFF];
            if marker_ok && v != 0 && v != u64::MAX {
                cd_ids.insert(v);
            }
        }
        off += 40;
    }

    // Also extract ALL u64 values from the whole CD buffer that look
    // like plausible IDs (small enough, non-sentinel).
    let mut cd_any_u64: BTreeSet<u64> = BTreeSet::new();
    for o in (0..cd.len().saturating_sub(8)).step_by(4) {
        if let Some(v) = read_u64(&cd, o) {
            if v > 0 && v < 1_000_000 {
                cd_any_u64.insert(v);
            }
        }
    }

    let strict_intersection: BTreeSet<u64> = elem_ids.intersection(&cd_ids).copied().collect();
    let loose_intersection: BTreeSet<u64> = elem_ids.intersection(&cd_any_u64).copied().collect();

    println!("2024 Core Interior correlation:");
    println!("  ElemTable distinct IDs: {}", elem_ids.len());
    println!(
        "  CD record-aligned IDs (strict, {} records): {}",
        (cd.len() - anchor) / 40,
        cd_ids.len()
    );
    println!("  CD any-u64 IDs (loose): {}", cd_any_u64.len());
    println!();
    println!(
        "  Strict ∩ ElemTable: {} ({:.1}% of ElemTable covered)",
        strict_intersection.len(),
        100.0 * strict_intersection.len() as f64 / elem_ids.len() as f64
    );
    println!(
        "  Loose  ∩ ElemTable: {} ({:.1}% of ElemTable covered)",
        loose_intersection.len(),
        100.0 * loose_intersection.len() as f64 / elem_ids.len() as f64
    );
    println!();
    println!(
        "  Strict CD ids range: {:?} .. {:?}",
        cd_ids.iter().next(),
        cd_ids.iter().next_back()
    );
    println!(
        "  ElemTable ids range: {:?} .. {:?}",
        elem_ids.iter().next(),
        elem_ids.iter().next_back()
    );

    // Sample overlaps
    let sample_overlap: Vec<u64> = strict_intersection.iter().take(10).copied().collect();
    println!("  Sample strict overlaps: {:?}", sample_overlap);

    // CD ids that are NOT in ElemTable
    let cd_only: Vec<u64> = cd_ids.difference(&elem_ids).take(10).copied().collect();
    println!("  Sample CD ids NOT in ElemTable: {:?}", cd_only);
}
