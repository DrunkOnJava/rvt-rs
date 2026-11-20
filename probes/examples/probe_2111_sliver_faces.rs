//! #2111: which face of a body holds the thin triangles near the Revit vertices that #2107
//! moved past 0.001 ft?
//!
//! FACT under test: on Snowdon Towers Architectural, the triangles nearest those Revit
//! vertices are slivers whose three corners are all points #2107 adds (on a toilet, edges of
//! 0.00533, 0.63243 and 0.63384 ft). Triangle edge lengths do not depend on where the body is
//! placed, so each face of the element's graph and of the symbols it draws is meshed on its
//! own (`native_saved_mesh::face`) and its triangles are matched to given edge lengths within
//! 0.00005 ft. It prints, for each element id given after the file:
//!
//! ```text
//! element id=<ElementId> graphs=<n> faces=<n> meshed=<n>
//! refused id=<ElementId> faces=<n> why=<reason>
//! match id=<ElementId> graph=<ElementId> face=<n> surface=<class> loops=<n> triangles=<n> edges=<ft,ft,ft> standoff=<ft|->
//!   - `standoff`: for a cylinder face, the farthest from the face's own cylinder of 28 points
//!     on a six-step grid over the triangle (3 corners, 15 on its edges, 10 inside)
//! ```
//!
//! With no `ID:L1,L2,L3` given, as in Measure, it reads the slivers #2111 found on Snowdon Towers
//! Architectural (`DEFAULT_TARGETS`); on another file those ids name nothing. Counts and lengths
//! in feet only.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_2111_sliver_faces -- MODEL.rvt ID:L1,L2,L3 [ID:L1,L2,L3 ...]
//! ```

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{face, pointer};

fn ident(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(ident)
}

/// The symbols a graph's `InstanceInfo` objects name.
fn symbols(graph: &ObjectGraph) -> Vec<u64> {
    graph
        .objects
        .iter()
        .filter(|o| o.class_name == "InstanceInfo")
        .filter_map(|o| o.fields.get("m_symbolId"))
        .filter_map(ident)
        .filter_map(|id| u64::try_from(id).ok())
        .filter(|id| *id > 0)
        .collect()
}

fn graphs(rf: &mut RevitFile, ids: &BTreeSet<u64>) -> BTreeMap<u64, ObjectGraph> {
    let mut out = BTreeMap::new();
    // The saved graphics' channel, as `probe_2098_face_cracks` reads it.
    let options = Options {
        selected_ids: ids.clone(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Options::default()
    };
    let read = extract_graphs(rf, &options, |record| {
        if let Some(graph) = record.graph {
            out.insert(record.identity.element_id, graph);
        }
        Ok(())
    });
    if let Err(error) = read {
        println!("graphs: {error}");
    }
    out
}

/// #2111's slivers on Snowdon Towers Architectural: a `Toilet-Domestic-3D` and the `Sink-3 Basin`.
const DEFAULT_TARGETS: [(u64, [f64; 3]); 5] = [
    (773245, [0.00533, 0.63243, 0.63384]),
    (1135932, [0.00711, 2.08178, 2.08294]),
    (1135932, [0.0099, 7.125, 7.132]),
    (1135932, [0.00727, 7.00314, 7.00508]),
    (1135932, [0.00711, 1.95874, 1.9599]),
];

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).filter(|a| !a.starts_with("--"));
    let path = args.next().ok_or_else(|| {
        anyhow::anyhow!("usage: probe_2111_sliver_faces MODEL.rvt ID:L1,L2,L3 ...")
    })?;
    let mut targets: BTreeMap<u64, Vec<[f64; 3]>> = BTreeMap::new();
    for arg in args {
        let Some((id, lengths)) = arg.split_once(':') else {
            continue;
        };
        let mut l: Vec<f64> = lengths.split(',').filter_map(|x| x.parse().ok()).collect();
        if l.len() != 3 {
            continue;
        }
        l.sort_by(f64::total_cmp);
        if let Ok(id) = id.parse::<u64>() {
            targets.entry(id).or_default().push([l[0], l[1], l[2]]);
        }
    }
    if targets.is_empty() {
        for (id, lengths) in DEFAULT_TARGETS {
            targets.entry(id).or_default().push(lengths);
        }
    }
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    for (id, wanted) in &targets {
        // The element's graph, the symbols it draws, and theirs, three levels deep.
        let mut all: BTreeMap<u64, ObjectGraph> = BTreeMap::new();
        let mut next: BTreeSet<u64> = BTreeSet::from([*id]);
        for _ in 0..3 {
            let found = graphs(&mut rf, &next);
            next = found
                .values()
                .flat_map(symbols)
                .filter(|s| !all.contains_key(s) && !found.contains_key(s))
                .collect();
            all.extend(found);
            if next.is_empty() {
                break;
            }
        }
        let (mut faces, mut meshed) = (0usize, 0usize);
        let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
        for (graph_id, g) in &all {
            for fi in 0..g.objects.len() {
                if g.objects[fi].class_name != "Face" {
                    continue;
                }
                faces += 1;
                let mesh = match face(g, fi) {
                    Ok(Some(mesh)) => mesh,
                    Ok(None) => {
                        *refusals.entry("none".into()).or_default() += 1;
                        continue;
                    }
                    Err(error) => {
                        let why: String = format!("{error:#}").chars().take(80).collect();
                        *refusals.entry(why).or_default() += 1;
                        continue;
                    }
                };
                meshed += 1;
                let class = g.objects[fi]
                    .fields
                    .get("m_pSurf")
                    .and_then(|p| pointer(g, fi, p).ok().flatten())
                    .map(|si| g.objects[si].class_name.to_string())
                    .unwrap_or_else(|| "-".into());
                let mut loops = 0usize;
                let mut l = g.objects[fi]
                    .fields
                    .get("m_pFirstLoop")
                    .and_then(|p| pointer(g, fi, p).ok().flatten());
                while let Some(li) = l {
                    loops += 1;
                    if loops > 64 {
                        break;
                    }
                    l = g.objects[li]
                        .fields
                        .get("m_nextLoop")
                        .and_then(|p| pointer(g, li, p).ok().flatten());
                }
                for t in &mesh.triangles {
                    let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                    let d = |x: [f64; 3], y: [f64; 3]| {
                        (0..3).map(|k| (x[k] - y[k]).powi(2)).sum::<f64>().sqrt()
                    };
                    let mut e = [d(a, b), d(b, c), d(c, a)];
                    e.sort_by(f64::total_cmp);
                    if wanted
                        .iter()
                        .any(|w| (0..3).all(|k| (w[k] - e[k]).abs() <= 5e-5))
                    {
                        // How far the triangle stands off its own cylinder, at points spread
                        // over it as well as its edge midpoints and centroid.
                        let standoff = mesh.analytic_surface.as_ref().map(|cyl| {
                            let off = |p: [f64; 3]| {
                                let q: [f64; 3] = std::array::from_fn(|k| p[k] - cyl.center[k]);
                                let along: f64 = (0..3).map(|k| q[k] * cyl.z_vec[k]).sum();
                                let radial: f64 = (0..3)
                                    .map(|k| (q[k] - along * cyl.z_vec[k]).powi(2))
                                    .sum::<f64>()
                                    .sqrt();
                                (radial - cyl.radius).abs()
                            };
                            let mut worst = 0.0_f64;
                            for i in 0..=6 {
                                for j in 0..=(6 - i) {
                                    let (u, v) = (i as f64 / 6.0, j as f64 / 6.0);
                                    let w = 1.0 - u - v;
                                    let p: [f64; 3] =
                                        std::array::from_fn(|k| w * a[k] + u * b[k] + v * c[k]);
                                    worst = worst.max(off(p));
                                }
                            }
                            worst
                        });
                        println!(
                            "match id={id} graph={graph_id} face={fi} surface={class} loops={loops} triangles={} edges={:.5},{:.5},{:.5} standoff={}",
                            mesh.triangles.len(),
                            e[0],
                            e[1],
                            e[2],
                            standoff.map_or("-".to_string(), |x| format!("{x:.6}"))
                        );
                    }
                }
            }
        }
        println!(
            "element id={id} graphs={} faces={faces} meshed={meshed}",
            all.len()
        );
        for (why, n) in &refusals {
            println!("refused id={id} faces={n} why={why}");
        }
    }
    Ok(())
}
