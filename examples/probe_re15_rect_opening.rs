//! RE-15-09 (#89) — ArcWallRectOpening / VWallRectOpening envelopes.
use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

const SCHEMA_FAMILY_MARKER: u32 = 0x0008_8004;

struct FileSpec {
    file: &'static str,
    partition: &'static str,
}
const SPECS: &[FileSpec] = &[
    FileSpec {
        file: "Revit_IFC5_Einhoven.rvt",
        partition: "Partitions/5",
    },
    FileSpec {
        file: "2024_Core_Interior.rvt",
        partition: "Partitions/46",
    },
];

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    if off + 8 > buf.len() {
        return None;
    }
    let v = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    v.is_finite().then_some(v)
}
fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    if off + 4 > buf.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
    ]))
}
fn find_filtered(buf: &[u8], tag: u16) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..buf.len().saturating_sub(3) {
        let v = u16::from_le_bytes([buf[i], buf[i + 1]]);
        if v == tag && buf[i + 2] == 0 && buf[i + 3] == 0 {
            out.push(i);
        }
    }
    out
}

fn probe_tag(buf: &[u8], class_name: &str, tag: u16) {
    let hits = find_filtered(buf, tag);
    println!(
        "\n  --- {class_name} tag 0x{tag:04x}: {} filtered hits ---",
        hits.len()
    );
    if hits.is_empty() {
        return;
    }
    let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
    for w in hits.windows(2) {
        *hist.entry(w[1] - w[0]).or_insert(0) += 1;
    }
    let mut top: Vec<_> = hist.into_iter().collect();
    top.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("    top deltas:");
    for (d, c) in top.iter().take(8) {
        let pct = 100.0 * (*c as f64) / (hits.len().saturating_sub(1).max(1) as f64);
        println!("      delta={d:>6}  count={c:>5}  ({pct:5.1}%)");
    }
    let marker_at_4 = hits
        .iter()
        .filter(|&&off| read_u32(buf, off + 4) == Some(SCHEMA_FAMILY_MARKER))
        .count();
    let mut door_dim_pairs = 0usize;
    let mut samples = Vec::new();
    for &off in &hits {
        let mut best = None;
        for a in (0..120).step_by(8) {
            for b in ((a + 8)..128).step_by(8) {
                let (Some(w), Some(h)) = (read_f64(buf, off + a), read_f64(buf, off + b)) else {
                    continue;
                };
                if (1.5..5.0).contains(&w) && (5.0..9.0).contains(&h) {
                    best = Some((w, h, a));
                    break;
                }
                if (1.5..5.0).contains(&h) && (5.0..9.0).contains(&w) {
                    best = Some((h, w, a));
                    break;
                }
            }
            if best.is_some() {
                break;
            }
        }
        if let Some((w, h, a)) = best {
            door_dim_pairs += 1;
            if samples.len() < 8 {
                samples.push((off, w, h, a));
            }
        }
    }
    println!(
        "    SCHEMA_FAMILY_MARKER @+0x04: {marker_at_4}/{}",
        hits.len()
    );
    println!(
        "    door-plausible (w,h) pairs: {door_dim_pairs}/{}",
        hits.len()
    );
    for (off, w, h, a) in samples {
        println!("      @{off} w={w:.3} h={h:.3} pair_base=+0x{a:02x}");
    }
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    println!("RE-15-09 rect-opening probe — corpus dir: {project_dir}");
    for spec in SPECS {
        let path = format!("{project_dir}/{}", spec.file);
        let mut rf = match RevitFile::open(&path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}: {e}", spec.file);
                continue;
            }
        };
        let formats_raw = rf.read_stream(streams::FORMATS_LATEST).expect("formats");
        let formats_d = compression::inflate_at(&formats_raw, 0).expect("inflate");
        let schema = formats::parse_schema(&formats_d).expect("schema");
        let mut tags = Vec::new();
        for name in ["ArcWallRectOpening", "VWallRectOpening", "ArcWall", "VWall"] {
            if let Some(c) = schema.classes.iter().find(|c| c.name == name) {
                if let Some(t) = c.tag {
                    tags.push((name, t));
                }
            }
        }
        let raw = rf.read_stream(spec.partition).expect("partition");
        let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
            .into_iter()
            .flatten()
            .collect();
        println!(
            "\n=== {} {} — {} B ===",
            spec.file,
            spec.partition,
            concat.len()
        );
        for (name, tag) in tags {
            probe_tag(&concat, name, tag);
        }
    }
}
