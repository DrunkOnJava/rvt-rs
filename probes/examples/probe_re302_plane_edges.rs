//! RE-302: how far an edge a plane face shares with a curved face lies from
//! the plane, at its samples and between them (#1247).
//!
//! FACT under test: `native_saved_mesh::refined_plane_edge` makes a plane
//! face's boundary follow the curved face beside it. On the BIM4LCA office,
//! once RE-302 meshes trimmed ruled surfaces and surfaces of revolution, 43
//! elements are rejected because such an edge "leaves adjacent plane": either
//! the curved surface, evaluated at the edge's own UV samples, does not reach
//! the plane's points (1e-7 ft), or the surface between two samples strays
//! from the plane by more than the 0.001 ft chord bound. This measures both.
//!
//! Output: one `edge` line per shared edge that is not isoparametric on its
//! curved side, for the first 40 per file, with its record's ElementId, the
//! curved surface's class, its sample count, the largest distance between a
//! sample's curved and planar points, the largest distance of a sample's
//! curved point from the plane, and the largest distance from the plane of
//! the curved surface at the UV midpoint between two samples; then `census`
//! lines counting edges by surface class, isoparametric or not, and decade of
//! each distance.
//!
//! Second FACT under test (after RE-302 traced such edges, the WCs it then
//! drew are 1.03 to 1.05 of Revit's volume): whether the trace could have
//! followed the wrong branch. `edge_on_plane` (at f4d82ed7, on
//! `research/1247-ellipse-profiles`) moves each of 7 points between
//! two samples across the line between them, within that line's length on
//! either side, onto the plane, taking the sign change nearest the line
//! among 32 steps. For every such point this counts the sign changes those
//! 32 steps see: one `branch` line per record with a non-isoparametric
//! shared edge, `traced` points, how many saw more than one sign change
//! (`multi`), and how many saw none (`none`); then `census branch` lines by
//! surface class and count.
//!
//! Run it at 827718ff, where the RE-302 readers read every profile: on
//! `main` a surface with a `GEllipse` profile is not read, so its edges are
//! skipped.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re302_plane_edges -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parametric_surface::ParametricSurface;
use rvt::research::native_saved_mesh::pointer;
use serde_json::Value;

type Point = [f64; 3];

fn vec3(v: &Value) -> Option<Point> {
    let a = v.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ])
}

fn uv(v: &Value) -> Option<[f64; 2]> {
    let a = v.as_array()?;
    Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?])
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn decade(d: f64) -> String {
    if d <= 0. {
        "0".into()
    } else {
        format!("1e{}", d.log10().ceil() as i32)
    }
}

enum Curved {
    Cylinder([Point; 4], f64),
    Parametric(Box<ParametricSurface>),
}

impl Curved {
    fn read(g: &ObjectGraph, si: usize) -> Option<(Self, &'static str)> {
        let s = &g.objects[si];
        match s.class_name.as_str() {
            "CylSurf" => Some((
                Self::Cylinder(
                    [
                        vec3(&s.fields["m_center"])?,
                        vec3(&s.fields["m_xVec"])?,
                        vec3(&s.fields["m_yVec"])?,
                        vec3(&s.fields["m_zVec"])?,
                    ],
                    s.fields["m_radius"].as_f64()?,
                ),
                "CylSurf",
            )),
            "SurfRev" | "RuledSurf" => Some((
                Self::Parametric(Box::new(ParametricSurface::read(g, si).ok()?)),
                if s.class_name == "SurfRev" {
                    "SurfRev"
                } else {
                    "RuledSurf"
                },
            )),
            _ => None,
        }
    }

    fn eval(&self, p: [f64; 2]) -> Option<Point> {
        match self {
            Self::Cylinder([c, x, y, z], r) => {
                let (s, k) = p[0].sin_cos();
                Some(std::array::from_fn(|i| {
                    c[i] + r * (k * x[i] + s * y[i]) + p[1] * z[i]
                }))
            }
            Self::Parametric(surface) => surface.evaluate(p[0], p[1]).ok(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re302_plane_edges FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let options = Options {
        selected_ids: rvt::elem_table::declared_ids(&records)
            .into_iter()
            .map(u64::from)
            .collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let mut printed = 0;
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    let mut branches: BTreeMap<(u64, &'static str), [usize; 3]> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let Some(g) = record.graph.as_ref() else {
            return Ok(());
        };
        for (ei, edge) in g.objects.iter().enumerate() {
            if edge.class_name != "Edge" {
                continue;
            }
            let Some(faces) = edge.fields["m_pFace"].as_array() else {
                continue;
            };
            if faces.len() != 2 {
                continue;
            }
            let mut surfaces = [None, None];
            for (side, face) in faces.iter().enumerate() {
                let Ok(Some(face)) = pointer(g, ei, face) else {
                    continue;
                };
                if let Ok(Some(si)) = pointer(g, face, &g.objects[face].fields["m_pSurf"]) {
                    surfaces[side] = Some(si);
                }
            }
            let [Some(s0), Some(s1)] = surfaces else {
                continue;
            };
            let (plane_side, plane) = match (
                g.objects[s0].class_name.as_str(),
                g.objects[s1].class_name.as_str(),
            ) {
                ("Plane", other) if other != "Plane" => (0, s0),
                (other, "Plane") if other != "Plane" => (1, s1),
                _ => continue,
            };
            let curved_side = 1 - plane_side;
            let Some((curved, kind)) = Curved::read(g, if plane_side == 0 { s1 } else { s0 })
            else {
                continue;
            };
            let pf = &g.objects[plane].fields;
            let (Some(origin), Some(px), Some(py)) = (
                vec3(&pf["m_origin"]),
                vec3(&pf["m_xVec"]),
                vec3(&pf["m_yVec"]),
            ) else {
                continue;
            };
            let n = [
                px[1] * py[2] - px[2] * py[1],
                px[2] * py[0] - px[0] * py[2],
                px[0] * py[1] - px[1] * py[0],
            ];
            let len = dot(n, n).sqrt();
            if len <= 1e-12 {
                continue;
            }
            let n = n.map(|v| v / len);
            let off =
                |q: Point| dot([q[0] - origin[0], q[1] - origin[1], q[2] - origin[2]], n).abs();
            let Some(ends) = edge.fields["m_firstAndLastEdgePnts"].as_array() else {
                continue;
            };
            if ends.len() != 2 {
                continue;
            }
            let mut samples = vec![&ends[0]];
            if let Some(inner) = edge.fields["m_interiorEdgePnts"].as_array() {
                samples.extend(inner);
            }
            samples.push(&ends[1]);
            let pairs: Option<Vec<([f64; 2], [f64; 2])>> = samples
                .iter()
                .map(|s| Some((uv(&s["uv"][curved_side])?, uv(&s["uv"][plane_side])?)))
                .collect();
            let Some(pairs) = pairs else {
                continue;
            };
            let first = pairs[0].0;
            let last = pairs[pairs.len() - 1].0;
            let iso = (0..2).any(|axis| {
                (first[axis] - last[axis]).abs() > 1e-7
                    && pairs
                        .iter()
                        .all(|(c, _)| (c[1 - axis] - first[1 - axis]).abs() < 1e-7)
            });
            let mut gap = 0f64;
            let mut sample_off = 0f64;
            let mut mid_off = 0f64;
            let mut failed = false;
            for (c, p) in &pairs {
                let Some(q) = curved.eval(*c) else {
                    failed = true;
                    break;
                };
                let r: Point = std::array::from_fn(|i| origin[i] + p[0] * px[i] + p[1] * py[i]);
                gap = gap.max(
                    dot(
                        [q[0] - r[0], q[1] - r[1], q[2] - r[2]],
                        [q[0] - r[0], q[1] - r[1], q[2] - r[2]],
                    )
                    .sqrt(),
                );
                sample_off = sample_off.max(off(q));
            }
            for w in pairs.windows(2) {
                let m = [(w[0].0[0] + w[1].0[0]) / 2., (w[0].0[1] + w[1].0[1]) / 2.];
                match curved.eval(m) {
                    Some(q) => mid_off = mid_off.max(off(q)),
                    None => failed = true,
                }
            }
            if failed {
                *census.entry(format!("{kind} unevaluated")).or_default() += 1;
                continue;
            }
            if !iso {
                let height =
                    |q: Point| dot([q[0] - origin[0], q[1] - origin[1], q[2] - origin[2]], n);
                let entry = branches
                    .entry((record.identity.element_id, kind))
                    .or_default();
                for w in pairs.windows(2) {
                    let (a, b) = (w[0].0, w[1].0);
                    let d = [b[0] - a[0], b[1] - a[1]];
                    let span = d[0].hypot(d[1]);
                    if span <= 1e-12 {
                        continue;
                    }
                    let across = [-d[1] / span, d[0] / span];
                    for i in 1..8 {
                        let t = f64::from(i) / 8.;
                        let m = [a[0] + t * d[0], a[1] + t * d[1]];
                        let values: Vec<Option<f64>> = (0..=32)
                            .map(|k| {
                                let s = -span + 2. * span * f64::from(k) / 32.;
                                curved
                                    .eval([m[0] + s * across[0], m[1] + s * across[1]])
                                    .map(height)
                            })
                            .collect();
                        let changes = values
                            .windows(2)
                            .filter(|v| matches!((v[0], v[1]), (Some(x), Some(y)) if x == 0. || x * y < 0.))
                            .count();
                        entry[0] += 1;
                        if changes > 1 {
                            entry[1] += 1;
                        }
                        if changes == 0 {
                            entry[2] += 1;
                        }
                        *census
                            .entry(format!("branch {kind} changes={}", changes.min(3)))
                            .or_default() += 1;
                    }
                }
            }
            let shape = if iso { "iso" } else { "general" };
            *census
                .entry(format!(
                    "{kind} {shape} gap={} sample_off={} mid_off={}",
                    decade(gap),
                    decade(sample_off),
                    decade(mid_off)
                ))
                .or_default() += 1;
            if !iso && printed < 40 {
                printed += 1;
                println!(
                    "edge owner={} {kind} samples={} gap={gap:.3e} sample_off={sample_off:.3e} mid_off={mid_off:.3e} first_uv={first:?} last_uv={last:?}",
                    record.identity.element_id,
                    pairs.len()
                );
            }
        }
        Ok(())
    })?;
    for ((owner, kind), [traced, multi, none]) in &branches {
        println!("branch owner={owner} {kind} traced={traced} multi={multi} none={none}");
    }
    for (key, n) in &census {
        println!("census {n} {key}");
    }
    Ok(())
}
