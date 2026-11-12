//! #1706: the edges where a plane face meets a curved face in a model's saved graphs, and
//! what the refusals `shared curved edge is not an isoparametric interval` and `shared
//! curved edge changes its fixed parameter` would need to accept them.
//!
//! FACT under test: `native_saved_mesh::refined_plane_edge` refines the plane side of a
//! shared edge only where the edge runs along one parameter of the curved face (the
//! other parameter constant along it) and refuses every other, which leaves 141
//! elements of Snowdon Towers Architectural as record boxes (116 + 25, #1706). A
//! slanted plane across a cylinder meets it along an ellipse, and a mitred rail along
//! a curve that is neither of its parameters. For the edges of the other kind, three
//! things decide whether the plane's polygon can follow the edge's own samples (the
//! graph's `m_interiorEdgePnts`) and need no refinement: how far the plane's chord
//! between two samples is from the curved face's surface at the same parameter; how
//! far the curved face's own boundary, interpolated in its parameters between the
//! samples, leaves the plane; and whether the curved face meshes at all.
//!
//! This reads every record the saved-mesh pass reads (the owners of channel 103 and
//! the shared symbols they name, a level at a time). For each `Edge` with a `Plane`
//! on one side and a `CylSurf`, `SurfRev`, `RuledSurf` or `ConeSurf` on the other it
//! prints, by curved kind: how many edges run along `u`, along `v`, or along neither;
//! for the last, how many samples they hold, the largest distance between the
//! curved surface at a pair of samples' parameter midpoint and the chord's midpoint
//! (the chord's sagitta, feet), the largest distance of that surface point from the
//! plane, and whether the curved face meshes through `native_saved_mesh::face`
//! (and the first reason it does not). Counts only, by owner record for the largest.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1706_shared_edges -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportQualityMode, RvtDocExporter};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parametric_surface::ParametricSurface;
use rvt::research::native_saved_mesh::{face as mesh_face, pointer};

/// Buckets of a distance in feet: at most 1e-4, 5e-4, 1e-3, 5e-3, more.
const BUCKETS: [f64; 4] = [1e-4, 5e-4, 1e-3, 5e-3];
const BAR: f64 = 1e-3;

fn arr<const N: usize>(value: &Fv) -> Option<[f64; N]> {
    let items = value.as_array()?;
    if items.len() < N {
        return None;
    }
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = item.as_f64()?;
    }
    Some(out)
}

fn bucket(distance: f64) -> usize {
    BUCKETS
        .iter()
        .position(|limit| distance <= *limit)
        .unwrap_or(BUCKETS.len())
}

fn bucket_label(index: usize) -> String {
    match BUCKETS.get(index) {
        Some(limit) => format!("<= {limit:e}"),
        None => format!("> {:e}", BUCKETS[BUCKETS.len() - 1]),
    }
}

/// The pass's `instance_symbols`, with `native_metadata::identifier` copied.
fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .into_iter()
        .find_map(|key| value.get(key))
        .and_then(identifier)
}

fn instance_symbols(record: &Record) -> Vec<u64> {
    let Some(graph) = &record.graph else {
        return Vec::new();
    };
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            identifier(&object.fields["m_symbolId"])
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

/// What the edges of one curved kind look like.
#[derive(Default)]
struct Kind {
    along_u: usize,
    along_v: usize,
    neither: usize,
    /// For the edges along neither: the bucket of the largest chord sagitta, of the largest
    /// distance of the interpolated surface point from the plane, and the sample counts.
    sagitta: [usize; 5],
    off_plane: [usize; 5],
    samples: Vec<usize>,
    /// Whether the curved face of such an edge meshes, and the first reasons it does not.
    meshes: usize,
    refused: BTreeMap<String, usize>,
    /// The owner records holding the most edges along neither.
    owners: BTreeMap<u64, usize>,
    /// The classes of the faces' other loops are not read; the face count is.
    faces: BTreeSet<(u64, usize)>,
}

fn scan(owner: u64, record: &Record, kinds: &mut BTreeMap<String, Kind>, other_pairs: &mut usize) {
    let Some(graph) = &record.graph else {
        return;
    };
    let surface_of = |face_index: usize| -> Option<usize> {
        let face = graph.objects.get(face_index)?;
        pointer(graph, face_index, face.fields.get("m_pSurf")?)
            .ok()
            .flatten()
    };
    for (edge_index, edge) in graph.objects.iter().enumerate() {
        if edge.class_name != "Edge" {
            continue;
        }
        let Some(pair) = edge.fields.get("m_pFace").and_then(Fv::as_array) else {
            continue;
        };
        if pair.len() != 2 {
            continue;
        }
        let faces: Vec<Option<usize>> = pair
            .iter()
            .map(|value| pointer(graph, edge_index, value).ok().flatten())
            .collect();
        let (Some(first), Some(second)) = (faces[0], faces[1]) else {
            continue;
        };
        let (Some(first_surface), Some(second_surface)) = (surface_of(first), surface_of(second))
        else {
            continue;
        };
        let class = |surface: usize| graph.objects[surface].class_name.clone();
        let (plane_side, curved_side) = match (
            class(first_surface).as_str(),
            class(second_surface).as_str(),
        ) {
            ("Plane", other) if other != "Plane" => (0, 1),
            (other, "Plane") if other != "Plane" => (1, 0),
            _ => {
                *other_pairs += 1;
                continue;
            }
        };
        let curved_face = [first, second][curved_side];
        let plane_surface = [first_surface, second_surface][plane_side];
        let curved_surface = [first_surface, second_surface][curved_side];
        let kind_name = class(curved_surface);
        if !matches!(
            kind_name.as_str(),
            "CylSurf" | "SurfRev" | "RuledSurf" | "ConeSurf"
        ) {
            continue;
        }
        let entry = kinds.entry(kind_name.clone()).or_default();
        // The samples of the edge in the curved face's parameters.
        let Some(ends) = edge
            .fields
            .get("m_firstAndLastEdgePnts")
            .and_then(Fv::as_array)
        else {
            continue;
        };
        if ends.len() != 2 {
            continue;
        }
        let uv = |sample: &Fv| arr::<2>(&sample["uv"][curved_side]);
        let mut samples = Vec::new();
        let Some(start) = uv(&ends[0]) else {
            continue;
        };
        samples.push(start);
        let mut readable = true;
        for sample in edge
            .fields
            .get("m_interiorEdgePnts")
            .and_then(Fv::as_array)
            .unwrap_or(&[])
        {
            match uv(sample) {
                Some(point) => samples.push(point),
                None => readable = false,
            }
        }
        let Some(end) = uv(&ends[1]) else {
            continue;
        };
        samples.push(end);
        if !readable {
            continue;
        }
        let along_u = (start[1] - end[1]).abs() < 1e-7
            && (start[0] - end[0]).abs() > 1e-7
            && samples.iter().all(|p| (p[1] - start[1]).abs() < 1e-7);
        let along_v = (start[0] - end[0]).abs() < 1e-7
            && (start[1] - end[1]).abs() > 1e-7
            && samples.iter().all(|p| (p[0] - start[0]).abs() < 1e-7);
        if along_u {
            entry.along_u += 1;
            continue;
        }
        if along_v {
            entry.along_v += 1;
            continue;
        }
        entry.neither += 1;
        entry.samples.push(samples.len());
        *entry.owners.entry(owner).or_default() += 1;
        entry.faces.insert((owner, curved_face));

        // The curved surface at a parameter point, and the plane.
        let evaluate: Box<dyn Fn([f64; 2]) -> Option<[f64; 3]>> = if kind_name == "CylSurf" {
            let object = &graph.objects[curved_surface];
            let (Some(center), Some(x), Some(y), Some(z), Some(radius)) = (
                arr::<3>(&object.fields["m_center"]),
                arr::<3>(&object.fields["m_xVec"]),
                arr::<3>(&object.fields["m_yVec"]),
                arr::<3>(&object.fields["m_zVec"]),
                object.fields["m_radius"].as_f64(),
            ) else {
                continue;
            };
            Box::new(move |uv: [f64; 2]| {
                let (s, c) = uv[0].sin_cos();
                Some(std::array::from_fn(|k| {
                    center[k] + radius * (c * x[k] + s * y[k]) + uv[1] * z[k]
                }))
            })
        } else {
            let Ok(surface) = ParametricSurface::read(graph, curved_surface) else {
                continue;
            };
            Box::new(move |uv: [f64; 2]| surface.evaluate(uv[0], uv[1]).ok())
        };
        let plane = &graph.objects[plane_surface];
        let (Some(origin), Some(px), Some(py)) = (
            arr::<3>(&plane.fields["m_origin"]),
            arr::<3>(&plane.fields["m_xVec"]),
            arr::<3>(&plane.fields["m_yVec"]),
        ) else {
            continue;
        };
        let mut normal = [
            px[1] * py[2] - px[2] * py[1],
            px[2] * py[0] - px[0] * py[2],
            px[0] * py[1] - px[1] * py[0],
        ];
        let length = normal.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length < 1e-12 {
            continue;
        }
        for v in &mut normal {
            *v /= length;
        }
        let mut worst_sagitta = 0.0f64;
        let mut worst_off_plane = 0.0f64;
        let mut complete = true;
        for pair in samples.windows(2) {
            let middle = [
                (pair[0][0] + pair[1][0]) / 2.,
                (pair[0][1] + pair[1][1]) / 2.,
            ];
            let (Some(a), Some(b), Some(m)) =
                (evaluate(pair[0]), evaluate(pair[1]), evaluate(middle))
            else {
                complete = false;
                break;
            };
            let chord_middle: [f64; 3] = std::array::from_fn(|k| (a[k] + b[k]) / 2.);
            worst_sagitta = worst_sagitta.max(
                (0..3)
                    .map(|k| (m[k] - chord_middle[k]).powi(2))
                    .sum::<f64>()
                    .sqrt(),
            );
            let off: f64 = (0..3).map(|k| (m[k] - origin[k]) * normal[k]).sum();
            worst_off_plane = worst_off_plane.max(off.abs());
        }
        if complete {
            entry.sagitta[bucket(worst_sagitta)] += 1;
            entry.off_plane[bucket(worst_off_plane)] += 1;
        }
        // Whether the curved face meshes as the pass meshes it.
        match mesh_face(graph, curved_face) {
            Ok(Some(_)) => entry.meshes += 1,
            Ok(None) => *entry.refused.entry("no trim loop".into()).or_default() += 1,
            Err(error) => {
                let message: String = format!("{error:#}").chars().take(70).collect();
                *entry.refused.entry(message).or_default() += 1;
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1706_shared_edges MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
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
    let mut kinds: BTreeMap<String, Kind> = BTreeMap::new();
    let mut other_pairs = 0usize;
    let mut owner_ids = BTreeSet::<u64>::new();
    let mut wanted = BTreeSet::<u64>::new();
    native_document::extract_graphs(&mut rf, &options, |r| {
        let id = r.identity.element_id;
        owner_ids.insert(id);
        wanted.extend(instance_symbols(&r));
        scan(id, &r, &mut kinds, &mut other_pairs);
        Ok(())
    })?;
    let mut attempted = BTreeSet::<u64>::new();
    wanted.retain(|id| !owner_ids.contains(id));
    for _ in 0..128 {
        if wanted.is_empty() {
            break;
        }
        attempted.extend(&wanted);
        options.selected_ids = std::mem::take(&mut wanted);
        let mut found = Vec::new();
        native_document::extract_graphs(&mut rf, &options, |r| {
            found.extend(instance_symbols(&r));
            scan(r.identity.element_id, &r, &mut kinds, &mut other_pairs);
            Ok(())
        })?;
        wanted.extend(
            found
                .into_iter()
                .filter(|id| !owner_ids.contains(id) && !attempted.contains(id)),
        );
    }

    println!("edges between two curved faces or two planes (not read): {other_pairs}");
    for (kind, entry) in &kinds {
        let total = entry.along_u + entry.along_v + entry.neither;
        println!(
            "{kind}: {total} plane-to-curved edges: along u {}, along v {}, along neither {}",
            entry.along_u, entry.along_v, entry.neither
        );
        if entry.neither == 0 {
            continue;
        }
        let mut samples = entry.samples.clone();
        samples.sort_unstable();
        println!(
            "  along neither: samples per edge min {} median {} max {}, curved faces {}, those that mesh {} (refused: {})",
            samples[0],
            samples[samples.len() / 2],
            samples[samples.len() - 1],
            entry.faces.len(),
            entry.meshes,
            entry.refused.values().sum::<usize>()
        );
        for (index, sagitta) in entry.sagitta.iter().enumerate() {
            println!(
                "    largest chord sagitta {}: {sagitta} edges | largest distance of the interpolated surface point from the plane {}: {} edges",
                bucket_label(index),
                bucket_label(index),
                entry.off_plane[index]
            );
        }
        let within_bar: usize = entry.sagitta[..3].iter().sum();
        println!(
            "    edges whose chords stay within {BAR} ft of the surface: {within_bar} of {}",
            entry.sagitta.iter().sum::<usize>()
        );
        let mut reasons: Vec<(&String, &usize)> = entry.refused.iter().collect();
        reasons.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (reason, count) in reasons.iter().take(5) {
            println!("    the curved face is refused ({count}): {reason}");
        }
        let mut owners: Vec<(&u64, &usize)> = entry.owners.iter().collect();
        owners.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (owner, count) in owners.iter().take(8) {
            println!("    record {owner}: {count} edges");
        }
    }
    Ok(())
}
