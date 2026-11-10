//! RE-397 (#1635): what each detail level of a railing's saved graphics adds,
//! against Revit's body.
//!
//! FACT under test: with the instances RE-397 draws, the saved mesh of a `KAI-3`
//! railing is 1.195 and 1.198 of the volume of Revit's body, each of its 39
//! posts 0.1793 ft3 where Revit's is 0.1126 (48 vertices and 92 triangles, the
//! extents 0.263 by 0.291 by 2.133 ft), and the saved graphics hold detail
//! families whose geometry depends on the detail level the walk selects (3).
//! Revit's export may have read another. This reads the railings the scorer lists
//! at each detail level, 1 to 3, and prints what the mesh of each holds.
//!
//! Output, one line per railing and level, then one per primitive of the
//! railings in `PRIMITIVES` at level 3:
//!   level <railing> <detail> <status> primitives=<n> triangles=<n> volume=<ft3>
//!   prim <railing> <i> vertices=<n> triangles=<n> signed_volume=<ft3> extents=<x> <y> <z>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_railing_detail_levels -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;

/// The railings of the office as built that the scorer lists: `KAI-3` (2618205 and
/// 2614259), the small `KAI-5` (2903645), two `KAI-4` (1467318, 1469144) and `KAI-1`.
const RAILINGS: [u64; 6] = [2618205, 2614259, 2903645, 1467318, 1469144, 1485880];
const PRIMITIVES: [u64; 2] = [2618205, 2903645];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: RAILINGS.iter().copied().collect::<BTreeSet<u64>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    for detail in 1..=3 {
        let scene = extract_at_detail(&mut rf, &options, detail)?;
        for element in &scene.elements {
            let id = element.identity.element_id;
            if !RAILINGS.contains(&id) {
                continue;
            }
            let mut volume6 = 0.0f64;
            let mut triangles = 0usize;
            for primitive in &element.meshes.primitives {
                for triangle in &primitive.triangles {
                    let [a, b, c] = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                    let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                        continue;
                    };
                    triangles += 1;
                    volume6 += a[0] * (b[1] * c[2] - b[2] * c[1])
                        - a[1] * (b[0] * c[2] - b[2] * c[0])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]);
                }
            }
            println!(
                "level {id} {detail} {} primitives={} triangles={triangles} volume={:.6}",
                element.status,
                element.meshes.primitives.len(),
                (volume6 / 6.0).abs()
            );
            if detail != 3 || !PRIMITIVES.contains(&id) {
                continue;
            }
            for (index, primitive) in element.meshes.primitives.iter().enumerate() {
                let mut low = [f64::INFINITY; 3];
                let mut high = [f64::NEG_INFINITY; 3];
                for p in &primitive.vertices {
                    for k in 0..3 {
                        low[k] = low[k].min(p[k]);
                        high[k] = high[k].max(p[k]);
                    }
                }
                let mut signed6 = 0.0f64;
                for triangle in &primitive.triangles {
                    let [a, b, c] = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                    let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                        continue;
                    };
                    signed6 += a[0] * (b[1] * c[2] - b[2] * c[1])
                        - a[1] * (b[0] * c[2] - b[2] * c[0])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]);
                }
                println!(
                    "prim {id} {index} vertices={} triangles={} signed_volume={:.6} extents={:.3} {:.3} {:.3}",
                    primitive.vertices.len(),
                    primitive.triangles.len(),
                    signed6 / 6.0,
                    high[0] - low[0],
                    high[1] - low[1],
                    high[2] - low[2],
                );
            }
        }
    }
    Ok(())
}
