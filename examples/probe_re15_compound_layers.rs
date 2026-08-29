//! RE-15-08 (#88) — compound-layer thickness hunt in ArcWall trailers / HostObjAttr.
use rvt::arc_wall_record::{
    ARC_WALL_TAG, ARC_WALL_VARIANT_COMPOUND, ARC_WALL_VARIANT_STANDARD, SCHEMA_FAMILY_MARKER,
};
use rvt::{RevitFile, compression};
use std::collections::BTreeMap;

const CORE_END: usize = 0x73;
const SINGLE_STRIDE: usize = 292;
const HOSTOBJATTR_TAG: u16 = 0x006b;

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    if off + 8 > buf.len() {
        return None;
    }
    let v = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    v.is_finite().then_some(v)
}
fn thickness_plausible(v: f64) -> bool {
    (0.25..2.5).contains(&v)
}
fn layer_plausible(v: f64) -> bool {
    (0.01..1.5).contains(&v)
}

fn find_variant(buf: &[u8], variant: u16) -> Vec<usize> {
    let mut out = Vec::new();
    let min = if variant == ARC_WALL_VARIANT_STANDARD {
        SINGLE_STRIDE
    } else {
        0x20
    };
    for i in 0..buf.len().saturating_sub(min) {
        let tag = u16::from_le_bytes([buf[i], buf[i + 1]]);
        if tag != ARC_WALL_TAG || buf[i + 2] != 0 || buf[i + 3] != 0 {
            continue;
        }
        let v = u16::from_le_bytes([buf[i + 0x10], buf[i + 0x11]]);
        if v == variant {
            out.push(i);
        }
    }
    out
}

fn sweep_layer_runs(buf: &[u8], offsets: &[usize], window: usize, label: &str) {
    let mut runs_found = 0usize;
    let mut samples = Vec::new();
    for &off in offsets {
        let end = (off + window).min(buf.len());
        let slice = &buf[off..end];
        for start in (0..slice.len().saturating_sub(16)).step_by(8) {
            for nlayers in 2..=6 {
                let need = nlayers * 8;
                if start + need > slice.len() {
                    break;
                }
                let mut layers = Vec::new();
                let mut ok = true;
                for i in 0..nlayers {
                    match read_f64(slice, start + i * 8) {
                        Some(v) if layer_plausible(v) => layers.push(v),
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    continue;
                }
                let sum: f64 = layers.iter().sum();
                if thickness_plausible(sum) {
                    runs_found += 1;
                    if samples.len() < 6 {
                        samples.push((off + start, layers));
                    }
                    break;
                }
            }
        }
    }
    println!(
        "  {label}: layer-run candidates: {runs_found} across {} records",
        offsets.len()
    );
    for (at, layers) in samples {
        let sum: f64 = layers.iter().sum();
        println!("    @{at} layers={layers:?} sum={sum:.4}");
    }
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    println!("RE-15-08 compound-layer probe — corpus dir: {project_dir}");
    let mut rf = RevitFile::open(format!("{project_dir}/Revit_IFC5_Einhoven.rvt")).expect("open");
    let concat: Vec<u8> = compression::inflate_all_chunks(&rf.read_stream("Partitions/5").unwrap())
        .into_iter()
        .flatten()
        .collect();
    let standards = find_variant(&concat, ARC_WALL_VARIANT_STANDARD);
    let compounds = find_variant(&concat, ARC_WALL_VARIANT_COMPOUND);
    println!(
        "\n=== Einhoven Partitions/5 — {} B, {} standard, {} compound ===",
        concat.len(),
        standards.len(),
        compounds.len()
    );
    let mut trailers = Vec::new();
    for &off in &standards {
        if off + SINGLE_STRIDE <= concat.len() {
            trailers.push(concat[off + CORE_END..off + SINGLE_STRIDE].to_vec());
        }
    }
    println!("  Captured {} trailers", trailers.len());
    // Report trailer-relative columns with thickness hits (may be 0 — H88-1 falsified).
    for col in (0..(SINGLE_STRIDE - CORE_END)).step_by(8) {
        let thick = trailers
            .iter()
            .filter(|t| read_f64(t, col).map(thickness_plausible).unwrap_or(false))
            .count();
        if thick * 100 / trailers.len().max(1) >= 30 {
            println!(
                "  trailer +0x{:03x}: thick%={}",
                CORE_END + col,
                thick * 100 / trailers.len()
            );
        }
    }
    sweep_layer_runs(&concat, &standards, SINGLE_STRIDE, "ArcWall-standard");
    sweep_layer_runs(&concat, &compounds, 512, "ArcWall-compound");
    let mut real = Vec::new();
    for i in 0..concat.len().saturating_sub(8) {
        let tag = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if tag != HOSTOBJATTR_TAG || concat[i + 2] != 0 || concat[i + 3] != 0 {
            continue;
        }
        for j in (4..32).step_by(4) {
            if i + j + 4 <= concat.len() {
                let v = u32::from_le_bytes([
                    concat[i + j],
                    concat[i + j + 1],
                    concat[i + j + 2],
                    concat[i + j + 3],
                ]);
                if v == SCHEMA_FAMILY_MARKER {
                    real.push(i);
                    break;
                }
            }
        }
    }
    println!("  HostObjAttr real-record candidates: {}", real.len());
    sweep_layer_runs(&concat, &real, 160, "HostObjAttr");
    let _ = BTreeMap::<u8, u8>::new();
    println!(
        "\n  Note: IFC ArcWall depth_feet placeholder = {:.6} ft (8/12).",
        8.0 / 12.0
    );
}
