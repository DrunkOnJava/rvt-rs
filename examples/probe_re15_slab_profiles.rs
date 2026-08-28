//! RE-15-07 (#87) — slab / floor boundary profile candidates.
use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

struct Spec {
    file: &'static str,
    partition: &'static str,
}
const SPECS: &[Spec] = &[
    Spec {
        file: "Revit_IFC5_Einhoven.rvt",
        partition: "Partitions/5",
    },
    Spec {
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
fn plan_coord(v: f64) -> bool {
    v.is_finite() && v.abs() < 500.0
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

struct LoopCandidate {
    offset: usize,
    n_points: usize,
    span_x: f64,
    span_y: f64,
    closed_err: f64,
}

fn scan_closed_polylines(buf: &[u8], nmin: usize, nmax: usize) -> Vec<LoopCandidate> {
    let mut found = Vec::new();
    let step = if buf.len() > 5_000_000 { 64 } else { 16 };
    let limit = buf.len().saturating_sub(nmax * 16 + 16);
    let mut i = 0usize;
    while i < limit {
        for n in nmin..=nmax {
            let need = n * 16;
            if i + need + 16 > buf.len() {
                break;
            }
            let mut pts = Vec::with_capacity(n);
            let mut ok = true;
            for k in 0..n {
                let (Some(x), Some(y)) = (read_f64(buf, i + k * 16), read_f64(buf, i + k * 16 + 8))
                else {
                    ok = false;
                    break;
                };
                if !plan_coord(x) || !plan_coord(y) || (x.abs() < 1e-9 && y.abs() < 1e-9) {
                    ok = false;
                    break;
                }
                pts.push((x, y));
            }
            if !ok || pts.len() < nmin {
                continue;
            }
            let mut uniq = pts.clone();
            uniq.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            });
            uniq.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6);
            if uniq.len() < 3 {
                continue;
            }
            let (x0, y0) = pts[0];
            let (xn, yn) = pts[pts.len() - 1];
            let mut closed_err = (xn - x0).hypot(yn - y0);
            if let (Some(nx), Some(ny)) = (read_f64(buf, i + n * 16), read_f64(buf, i + n * 16 + 8))
            {
                closed_err = closed_err.min((nx - x0).hypot(ny - y0));
            }
            if closed_err > 0.05 {
                continue;
            }
            let min_x = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let max_x = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let min_y = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let max_y = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            let span_x = max_x - min_x;
            let span_y = max_y - min_y;
            if span_x < 2.0 || span_y < 2.0 {
                continue;
            }
            found.push(LoopCandidate {
                offset: i,
                n_points: pts.len(),
                span_x,
                span_y,
                closed_err,
            });
            break;
        }
        i += step;
    }
    found
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    println!("RE-15-07 slab-profile probe — corpus dir: {project_dir}");
    for spec in SPECS {
        let mut rf = match RevitFile::open(format!("{project_dir}/{}", spec.file)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}: {e}", spec.file);
                continue;
            }
        };
        let formats_d =
            compression::inflate_at(&rf.read_stream(streams::FORMATS_LATEST).unwrap(), 0).unwrap();
        let schema = formats::parse_schema(&formats_d).unwrap();
        let slab_tag = schema
            .classes
            .iter()
            .find(|c| c.name == "AnalyticalModelSlab")
            .and_then(|c| c.tag);
        let concat: Vec<u8> =
            compression::inflate_all_chunks(&rf.read_stream(spec.partition).unwrap())
                .into_iter()
                .flatten()
                .collect();
        println!(
            "\n=== {} {} — {} B ===",
            spec.file,
            spec.partition,
            concat.len()
        );
        if let Some(tag) = slab_tag {
            let hits = find_filtered(&concat, tag);
            println!(
                "  AnalyticalModelSlab tag 0x{tag:04x}: {} filtered hits",
                hits.len()
            );
            let mut deltas: BTreeMap<usize, usize> = BTreeMap::new();
            for w in hits.windows(2) {
                *deltas.entry(w[1] - w[0]).or_insert(0) += 1;
            }
            let mut top: Vec<_> = deltas.into_iter().collect();
            top.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            for (d, c) in top.iter().take(6) {
                println!("      delta {d:>6} × {c}");
            }
        }
        let loops = scan_closed_polylines(&concat, 4, 8);
        println!("  Closed plan-polyline candidates: {}", loops.len());
        for (i, lp) in loops.iter().take(8).enumerate() {
            println!(
                "    #{i} @{} n={} span=({:.2}×{:.2}) err={:.4}",
                lp.offset, lp.n_points, lp.span_x, lp.span_y, lp.closed_err
            );
        }
    }
}
