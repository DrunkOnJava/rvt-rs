//! #2104: do an edge's saved points lie where its two cylinders meet?
//!
//! FACT under test: #2107 puts the points it adds along an edge two cylinders share where
//! the two analytic cylinders meet. Against Revit's export of Snowdon Towers Architectural,
//! Revit's vertices near some of those points stand off rvt-rs's surface by up to 0.002 ft,
//! where Revit's mesh is welded. If the edge's own saved points are off the other cylinder by
//! that much, Revit's edge is a curve near the intersection, not on it, and points on the
//! exact intersection depart from it.
//!
//! For every element the export writes and every symbol its graphics name, each edge whose
//! two faces are cylinders and whose saved samples run across both parameters on both sides
//! (rvt-rs's `isoparametric_axis`, copied) is read: each saved sample evaluated on its own
//! side's cylinder, and set against the other side's evaluation of the same sample and
//! against the other cylinder. It prints, for an edge with any of these over 1e-4 ft:
//!   pairedge graph=<ElementId> edge=<n> samples=<n> sides_apart=<ft> off_other=<ft>
//!   - `sides_apart`: the largest distance between the two sides' evaluations of one sample;
//!   - `off_other`: the largest distance of a sample, on either side's cylinder, from the
//!     other cylinder's surface (its distance from that axis less the radius);
//! and last
//!   pairedges edges=<n> off_other_under_1e-6=<n> 1e-6_to_1e-4=<n> 1e-4_to_1e-3=<n>
//!     1e-3_or_more=<n> sides_apart_over_1e-4=<n> largest_off_other=<ft>
//! ElementIds, counts and distances in feet only.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2104_pair_edges -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document;
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::pointer;

type V3 = [f64; 3];

fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

fn instance_symbols(graph: &ObjectGraph) -> Vec<u64> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            object
                .fields
                .get("m_symbolId")
                .and_then(identifier)
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

fn vec<const N: usize>(v: &Fv) -> Option<[f64; N]> {
    let a = v.as_array().filter(|a| a.len() == N)?;
    let mut out = [0.0; N];
    for (k, x) in a.iter().enumerate() {
        out[k] = x.as_f64()?;
    }
    Some(out)
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

struct Cylinder {
    center: V3,
    x: V3,
    y: V3,
    z: V3,
    radius: f64,
}

impl Cylinder {
    fn read(g: &ObjectGraph, si: usize) -> Option<Self> {
        let s = g.objects.get(si)?;
        (s.class_name == "CylSurf").then_some(())?;
        Some(Self {
            center: vec::<3>(s.fields.get("m_center")?)?,
            x: vec::<3>(s.fields.get("m_xVec")?)?,
            y: vec::<3>(s.fields.get("m_yVec")?)?,
            z: vec::<3>(s.fields.get("m_zVec")?)?,
            radius: s.fields.get("m_radius")?.as_f64()?,
        })
    }

    fn eval(&self, uv: [f64; 2]) -> V3 {
        let (sn, cs) = uv[0].sin_cos();
        std::array::from_fn(|k| {
            self.center[k] + self.radius * (cs * self.x[k] + sn * self.y[k]) + uv[1] * self.z[k]
        })
    }

    fn off(&self, p: V3) -> f64 {
        let q: V3 = std::array::from_fn(|k| p[k] - self.center[k]);
        let along = dot(q, self.z);
        let r: V3 = std::array::from_fn(|k| q[k] - along * self.z[k]);
        (dot(r, r).sqrt() - self.radius).abs()
    }
}

/// Constant to 1e-7 along every sample, as rvt-rs's `isoparametric_axis` reads it.
fn isoparametric(samples: &[[f64; 2]]) -> bool {
    let (Some(start), Some(end)) = (samples.first(), samples.last()) else {
        return true;
    };
    (0..2).any(|k| {
        (start[1 - k] - end[1 - k]).abs() < 1e-7
            && (start[k] - end[k]).abs() > 1e-7
            && samples
                .iter()
                .all(|p| (p[1 - k] - start[1 - k]).abs() < 1e-7)
    })
}

#[derive(Default)]
struct Tally {
    edges: usize,
    bins: [usize; 4],
    apart: usize,
    largest: f64,
}

fn scan(id: u64, g: &ObjectGraph, t: &mut Tally) {
    for (ei, e) in g.objects.iter().enumerate() {
        if e.class_name != "Edge" {
            continue;
        }
        let Some(faces) = e.fields.get("m_pFace").and_then(|f| f.as_array()) else {
            continue;
        };
        if faces.len() != 2 {
            continue;
        }
        let cylinder = |side: usize| -> Option<Cylinder> {
            let face = pointer(g, ei, &faces[side]).ok()??;
            let si = pointer(g, face, g.objects[face].fields.get("m_pSurf")?).ok()??;
            Cylinder::read(g, si)
        };
        let (Some(c0), Some(c1)) = (cylinder(0), cylinder(1)) else {
            continue;
        };
        let samples = |k: usize| -> Option<Vec<[f64; 2]>> {
            let ends = e.fields.get("m_firstAndLastEdgePnts")?.as_array()?;
            let interior = e.fields.get("m_interiorEdgePnts")?.as_array()?;
            if ends.len() != 2 {
                return None;
            }
            let mut out = vec![vec::<2>(ends[0].get("uv")?.get(k)?)?];
            for p in interior {
                out.push(vec::<2>(p.get("uv")?.get(k)?)?);
            }
            out.push(vec::<2>(ends[1].get("uv")?.get(k)?)?);
            Some(out)
        };
        let (Some(s0), Some(s1)) = (samples(0), samples(1)) else {
            continue;
        };
        if isoparametric(&s0) || isoparametric(&s1) || s0.len() != s1.len() {
            continue;
        }
        let (mut apart, mut off) = (0.0_f64, 0.0_f64);
        for (a, b) in s0.iter().zip(&s1) {
            let (p0, p1) = (c0.eval(*a), c1.eval(*b));
            let d: V3 = std::array::from_fn(|k| p0[k] - p1[k]);
            apart = apart.max(dot(d, d).sqrt());
            off = off.max(c1.off(p0)).max(c0.off(p1));
        }
        t.edges += 1;
        let bin = match off {
            x if x < 1e-6 => 0,
            x if x < 1e-4 => 1,
            x if x < 1e-3 => 2,
            _ => 3,
        };
        t.bins[bin] += 1;
        t.apart += usize::from(apart > 1e-4);
        t.largest = t.largest.max(off);
        if off > 1e-4 || apart > 1e-4 {
            println!(
                "pairedge graph={id} edge={ei} samples={} sides_apart={apart:.6} off_other={off:.6}",
                s0.len()
            );
        }
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2104_pair_edges MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let model = rvt::ifc::RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        rvt::ifc::ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            rvt::ifc::entities::IfcEntity::BuildingElement {
                type_guid: Some(tag),
                ..
            } => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut options = native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mut tally = Tally::default();
    let mut symbols_of: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    native_document::extract_graphs(&mut rf, &options, |r| {
        if let Some(graph) = &r.graph {
            symbols_of.insert(r.identity.element_id, instance_symbols(graph));
            scan(r.identity.element_id, graph, &mut tally);
        }
        Ok(())
    })?;
    for _ in 0..16 {
        let wanted: BTreeSet<u64> = symbols_of
            .values()
            .flatten()
            .filter(|id| !symbols_of.contains_key(id))
            .copied()
            .collect();
        if wanted.is_empty() {
            break;
        }
        options.selected_ids = wanted.clone();
        native_document::extract_graphs(&mut rf, &options, |r| {
            if let Some(graph) = &r.graph {
                symbols_of.insert(r.identity.element_id, instance_symbols(graph));
                scan(r.identity.element_id, graph, &mut tally);
            }
            Ok(())
        })?;
        for id in wanted {
            symbols_of.entry(id).or_default();
        }
    }
    println!(
        "pairedges edges={} off_other_under_1e-6={} 1e-6_to_1e-4={} 1e-4_to_1e-3={} 1e-3_or_more={} sides_apart_over_1e-4={} largest_off_other={:.6}",
        tally.edges,
        tally.bins[0],
        tally.bins[1],
        tally.bins[2],
        tally.bins[3],
        tally.apart,
        tally.largest
    );
    Ok(())
}
