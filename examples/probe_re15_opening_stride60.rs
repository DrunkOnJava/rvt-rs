//! RE-15 follow-up — 60 B opening-index + ArcWall thickness sweep.
use rvt::arc_wall_record::{ARC_WALL_TAG, ARC_WALL_VARIANT_STANDARD};
use rvt::rect_opening_index::{ARC_WALL_RECT_OPENING_TAG_2024, OPENING_INDEX_FAMILY_MARKER};
use rvt::{RevitFile, compression};
use std::collections::BTreeMap;

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    if off + 8 > buf.len() {
        return None;
    }
    let v = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    v.is_finite().then_some(v)
}

fn probe_thickness() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let mut rf = RevitFile::open(format!("{project_dir}/Revit_IFC5_Einhoven.rvt")).expect("open");
    let concat: Vec<u8> = compression::inflate_all_chunks(&rf.read_stream("Partitions/5").unwrap())
        .into_iter()
        .flatten()
        .collect();
    let mut offs = Vec::new();
    for i in 0..concat.len().saturating_sub(292) {
        let tag = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if tag != ARC_WALL_TAG || concat[i + 2] != 0 || concat[i + 3] != 0 {
            continue;
        }
        let variant = u16::from_le_bytes([concat[i + 0x10], concat[i + 0x11]]);
        if variant == ARC_WALL_VARIANT_STANDARD {
            offs.push(i);
        }
    }
    println!(
        "=== Einhoven ArcWall thickness sweep — {} records ===",
        offs.len()
    );
    let common = [4.0 / 12.0, 6.0 / 12.0, 8.0 / 12.0, 10.0 / 12.0, 12.0 / 12.0];
    for &target in &common {
        let mut hits = 0usize;
        for &off in &offs {
            for abs in (0x12..0x124).step_by(8) {
                if let Some(v) = read_f64(&concat, off + abs) {
                    if (v - target).abs() < 1e-9 {
                        hits += 1;
                    }
                }
            }
        }
        println!("  {target:.6} ft ({:.0}\"): {hits}", target * 12.0);
    }
}

fn probe_stride60() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let mut rf = RevitFile::open(format!("{project_dir}/2024_Core_Interior.rvt")).expect("open");
    let concat: Vec<u8> =
        compression::inflate_all_chunks(&rf.read_stream("Partitions/46").unwrap())
            .into_iter()
            .flatten()
            .collect();
    let tag = ARC_WALL_RECT_OPENING_TAG_2024;
    let mut hits = Vec::new();
    for i in 0..concat.len().saturating_sub(3) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v == tag && concat[i + 2] == 0 && concat[i + 3] == 0 {
            hits.push(i);
        }
    }
    let mut stride60 = Vec::new();
    for w in hits.windows(2) {
        if w[1] - w[0] == 60 {
            stride60.push(w[0]);
        }
    }
    println!(
        "\n=== 2024 ArcWallRectOpening 0x{tag:04x}: {} filtered, {} delta=60 ===",
        hits.len(),
        stride60.len()
    );
    let mut marker = 0usize;
    for &off in stride60.iter().take(5000) {
        if off + 0x14 <= concat.len() {
            let v = u32::from_le_bytes([
                concat[off + 0x10],
                concat[off + 0x11],
                concat[off + 0x12],
                concat[off + 0x13],
            ]);
            if v == OPENING_INDEX_FAMILY_MARKER {
                marker += 1;
            }
        }
    }
    println!("  family marker 0x40088204 in first 5000 stride-60: {marker}");
    for (i, &off) in stride60.iter().take(3).enumerate() {
        let hex: String = concat[off..off + 60]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  hex#{i} @{off}: {hex}");
    }
    let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
    for &off in stride60.iter().take(2000) {
        *hist.entry(concat[off + 0x10]).or_insert(0) += 1;
    }
    println!("  +0x10 top byte hist: {hist:?}");
}

fn main() {
    probe_thickness();
    probe_stride60();
}
