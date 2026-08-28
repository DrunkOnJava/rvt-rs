//! RE-15 — deeper ArcWall trailer field dump (ElementId / type / elev).
//!
//! Companion to `probe_arcwall_trailer`. Prints per-record decoded
//! candidates and cross-checks ElementIds against Global/ElemTable.
//!
//!     RVT_PROJECT_CORPUS_DIR=... cargo run --release --example probe_arcwall_trailer_decode

use rvt::{RevitFile, compression, elem_table};
use std::collections::{BTreeMap, BTreeSet};

const SINGLE_STRIDE: usize = 292;
const ARC_WALL_TAG: u16 = 0x0191;
const ARC_WALL_VARIANT_STANDARD: u16 = 0x07fa;

fn f64_at(buf: &[u8], off: usize) -> f64 {
    f64::from_le_bytes(buf[off..off + 8].try_into().unwrap())
}
fn u32_at(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let raw = rf.read_stream("Partitions/5").unwrap();
    let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
        .into_iter()
        .flatten()
        .collect();

    let mut offsets = Vec::new();
    for i in 0..concat.len().saturating_sub(SINGLE_STRIDE) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v != ARC_WALL_TAG {
            continue;
        }
        if concat[i + 2] != 0 || concat[i + 3] != 0 {
            continue;
        }
        let variant = u16::from_le_bytes([concat[i + 0x10], concat[i + 0x11]]);
        if variant != ARC_WALL_VARIANT_STANDARD {
            continue;
        }
        offsets.push(i);
    }

    let records = elem_table::parse_records(&mut rf).unwrap_or_default();
    let elem_ids: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    println!("ElemTable ids: {} unique", elem_ids.len());
    println!("standard ArcWalls: {}", offsets.len());

    let f64_slots = [0x0f6usize, 0x0e6, 0x0ee];
    let u32_slots = [0x0feusize, 0x10e, 0x11c];

    let mut equal = 0usize;
    let mut uniq = BTreeSet::new();
    for (n, &off) in offsets.iter().enumerate() {
        let rec = &concat[off..off + SINGLE_STRIDE];
        let id_a = u32_at(rec, 0x10e);
        let id_b = u32_at(rec, 0x11c);
        if id_a == id_b {
            equal += 1;
        }
        uniq.insert(id_a);
        print!("#{n:02} @{off}:");
        for &s in &f64_slots {
            print!(" +0x{s:03x}={:.4}", f64_at(rec, s));
        }
        for &s in &u32_slots {
            let v = u32_at(rec, s);
            let hit = if elem_ids.contains(&v) { "*" } else { "" };
            print!(" +0x{s:03x}=0x{v:08x}{hit}");
        }
        println!();
    }
    println!(
        "\n+0x10e == +0x11c on {equal}/{}; {} unique values at +0x10e",
        offsets.len(),
        uniq.len()
    );

    let mut type_hist: BTreeMap<u32, usize> = BTreeMap::new();
    for &off in &offsets {
        *type_hist.entry(u32_at(&concat, off + 0xfe)).or_default() += 1;
    }
    println!("type_id (+0xfe) histogram: {type_hist:?}");
}
