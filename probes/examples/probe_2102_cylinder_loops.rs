//! #2102: what a cylinder face's two trim loops are, in the surface's parameters.
//!
//! FACT under test: on the BIM4LCA office, the `Stay_Wall_Mounted_:VAR-1` bodies that
//! #2102's `band_mesh` draws come out at 1.11 of Revit's volume, with shells missing a
//! round end face. Their cylinder faces with two loops were read as "a hole" or "two
//! separate regions" from the two rings' boxes alone. This prints each such face's
//! loops as the file saves them, so the reading can be checked: the envelope's span,
//! and for each loop its edges, whether it closes in the parameters, its box and its
//! signed area (its winding).
//!
//! For the graphs of the elements named with `--owners=<id>,<id>` (all exported
//! elements without it) and every symbol they reach, it prints:
//!   cyl graph=<ElementId> face=<index> loops=<n> env_u=<lo>..<hi> env_v=<lo>..<hi>
//!   loop graph=<ElementId> face=<index> k=<n> edges=<n> points=<n> closes=<yes|no>
//!     u=<lo>..<hi> v=<lo>..<hi> area=<signed, in the parameters> err=<why|->
//! then `total faces=<n>`. Parameters are radians along u and feet along v.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2102_cylinder_loops -- MODEL.rvt [--owners=1832497]

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document;
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::pointer;

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

fn uv(v: &Fv) -> Option<[f64; 2]> {
    let a = v.as_array().filter(|a| a.len() == 2)?;
    Some([a[0].as_f64()?, a[1].as_f64()?])
}

fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() <= 1e-7 && (a[1] - b[1]).abs() <= 1e-7
}

/// One loop's edges joined end to end, as `ring_impl` joins them, and whether it closes.
fn read_loop(
    g: &ObjectGraph,
    face: usize,
    li: usize,
) -> Result<(usize, Vec<[f64; 2]>, bool), String> {
    let mut cur = pointer(
        g,
        li,
        g.objects[li].fields.get("m_next").ok_or("no m_next")?,
    )
    .map_err(|e| e.to_string())?
    .ok_or("empty loop")?;
    let mut seen = BTreeSet::new();
    let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
    while cur != li {
        if !seen.insert(cur) || seen.len() > 10_000 {
            return Err("edge chain cycle".into());
        }
        let e = &g.objects[cur];
        let faces = e
            .fields
            .get("m_pFace")
            .and_then(Fv::as_array)
            .ok_or("no faces")?;
        let side = faces
            .iter()
            .position(|p| matches!(pointer(g, cur, p), Ok(Some(f)) if f == face))
            .ok_or("no side")?;
        let ends = e
            .fields
            .get("m_firstAndLastEdgePnts")
            .and_then(Fv::as_array)
            .ok_or("no ends")?;
        let interior = e
            .fields
            .get("m_interiorEdgePnts")
            .and_then(Fv::as_array)
            .ok_or("no interior")?;
        let at = |p: &Fv| {
            p.get("uv")
                .and_then(|u| u.as_array())
                .and_then(|a| a.get(side))
                .and_then(uv)
        };
        let mut pts = vec![ends.first().and_then(at).ok_or("end")?];
        for p in interior {
            pts.push(at(p).ok_or("interior")?);
        }
        pts.push(ends.get(1).and_then(at).ok_or("end")?);
        segments.push(pts);
        let next = e
            .fields
            .get("m_next")
            .and_then(Fv::as_array)
            .and_then(|a| a.get(side))
            .ok_or("no next")?;
        cur = pointer(g, cur, next)
            .map_err(|e| e.to_string())?
            .ok_or("null next")?;
    }
    let edges = segments.len();
    let mut out = segments.first().cloned().ok_or("no edges")?;
    for s in &segments[1..] {
        let last = *out.last().unwrap_or(&[f64::NAN; 2]);
        if close(last, s[0]) {
            out.extend_from_slice(&s[1..]);
        } else if close(last, s[s.len() - 1]) {
            out.extend(s.iter().rev().skip(1).copied());
        } else {
            return Err("disconnected".into());
        }
    }
    let closes = out.len() > 2 && close(out[0], out[out.len() - 1]);
    Ok((edges, out, closes))
}

fn report(id: u64, g: &ObjectGraph, faces: &mut usize) {
    for (fi, face) in g.objects.iter().enumerate() {
        if face.class_name != "Face" {
            continue;
        }
        let Some(Ok(Some(si))) = face.fields.get("m_pSurf").map(|p| pointer(g, fi, p)) else {
            continue;
        };
        if g.objects[si].class_name != "CylSurf" {
            continue;
        }
        let mut loops = Vec::new();
        let mut li = face
            .fields
            .get("m_pFirstLoop")
            .and_then(|p| pointer(g, fi, p).ok().flatten());
        while let Some(l) = li {
            if loops.contains(&l) || loops.len() > 16 {
                break;
            }
            loops.push(l);
            li = g.objects[l]
                .fields
                .get("m_nextLoop")
                .and_then(|p| pointer(g, l, p).ok().flatten());
        }
        if loops.len() < 2 {
            continue;
        }
        *faces += 1;
        let corner = |k: usize, j: usize| {
            g.objects[si]
                .fields
                .get("m_Envelope")
                .and_then(|e| e.get("m_corners"))
                .and_then(Fv::as_array)
                .and_then(|c| c.get(k))
                .and_then(Fv::as_array)
                .and_then(|p| p.get(j))
                .and_then(Fv::as_f64)
                .unwrap_or(f64::NAN)
        };
        println!(
            "cyl graph={id} face={fi} loops={} env_u={:.6}..{:.6} env_v={:.6}..{:.6}",
            loops.len(),
            corner(0, 0),
            corner(1, 0),
            corner(0, 1),
            corner(1, 1)
        );
        for (k, &l) in loops.iter().enumerate() {
            match read_loop(g, fi, l) {
                Ok((edges, pts, closes)) => {
                    let lo = |j: usize| pts.iter().map(|p| p[j]).fold(f64::INFINITY, f64::min);
                    let hi = |j: usize| pts.iter().map(|p| p[j]).fold(f64::NEG_INFINITY, f64::max);
                    let area: f64 = pts
                        .iter()
                        .zip(pts.iter().cycle().skip(1))
                        .take(pts.len())
                        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                        .sum::<f64>()
                        / 2.;
                    println!(
                        "loop graph={id} face={fi} k={k} edges={edges} points={} closes={} u={:.6}..{:.6} v={:.6}..{:.6} area={area:.6} err=-",
                        pts.len(),
                        if closes { "yes" } else { "no" },
                        lo(0),
                        hi(0),
                        lo(1),
                        hi(1)
                    );
                }
                Err(why) => println!(
                    "loop graph={id} face={fi} k={k} err={}",
                    why.replace(' ', "_")
                ),
            }
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.iter().find(|a| !a.starts_with("--")).ok_or_else(|| {
        anyhow::anyhow!("usage: probe_2102_cylinder_loops MODEL.rvt [--owners=..]")
    })?;
    let named: BTreeSet<u64> = args
        .iter()
        .find_map(|a| a.strip_prefix("--owners="))
        .map(|list| list.split(',').filter_map(|s| s.parse().ok()).collect())
        .unwrap_or_default();
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let ids: BTreeSet<u64> = if named.is_empty() {
        let model = rvt::ifc::RvtDocExporter.export_with_mode_and_limits(
            &mut rf,
            rvt::ifc::ExportQualityMode::Scaffold,
            rvt::walker::WalkerLimits::default(),
        )?;
        model
            .entities
            .iter()
            .filter_map(|entity| match entity {
                rvt::ifc::entities::IfcEntity::BuildingElement {
                    type_guid: Some(tag),
                    ..
                } => tag.parse().ok(),
                _ => None,
            })
            .collect()
    } else {
        named
    };
    let mut options = native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mut done: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut faces = 0usize;
    for _ in 0..16 {
        let mut found: Vec<(u64, Vec<u64>)> = Vec::new();
        native_document::extract_graphs(&mut rf, &options, |r| {
            if let Some(graph) = &r.graph {
                report(r.identity.element_id, graph, &mut faces);
                found.push((r.identity.element_id, instance_symbols(graph)));
            }
            Ok(())
        })?;
        for (id, symbols) in found {
            done.insert(id, symbols);
        }
        let wanted: BTreeSet<u64> = done
            .values()
            .flatten()
            .filter(|id| !done.contains_key(id))
            .copied()
            .collect();
        if wanted.is_empty() {
            break;
        }
        options.selected_ids = wanted;
    }
    for (id, symbols) in &done {
        if !symbols.is_empty() {
            println!(
                "symbols graph={id} names={}",
                symbols
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
    }
    println!("total faces={faces}");
    Ok(())
}
