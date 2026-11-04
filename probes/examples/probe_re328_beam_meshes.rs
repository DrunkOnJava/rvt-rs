//! RE-328: a beam's candidate bodies against its own saved graphics (#1247).
//!
//! FACT under test: where `beam_body` declines a beam's one location line
//! (`probe_re328_beam_rejections` says why), a body can still be recovered
//! from the line and the record box without ever being wrong. There is no
//! Revit IFC export of a Revit 2024 structural model to score a beam against,
//! so this scores against the mesh Revit saved in the file for the same
//! element (`native_saved_scene`, Revit's own tessellation, not an IFC export),
//! where all of it decodes (`partial` meshes are counted and not scored, as
//! `saved_meshes::attach` keeps their box).
//!
//! For each beam the partition reader recovers with one agreeing line that
//! `beam_check` declines (a `target`), and for the first 400 that run along
//! their line (a `control`, RE-49's own bodies), it takes the mesh's extent
//! along the line, across it level and across it in the line's vertical plane,
//! and compares:
//! - `box_vs_mesh`: the record box against the mesh's axis-aligned box:
//!   `equal`, `mesh_inside` (the box holds the mesh) or `mesh_outside`;
//! - each candidate body: `free` (`box_along`, the record box's extents solved
//!   for a box along the line, length free) and `fit` (`beam_section_fit`, the
//!   line's length and the closure check, without the ends test). The section
//!   across the line is `equal` to the mesh's (0.01 ft), the mesh is
//!   `mesh_inside` it, or it `differs` (the mesh leaves it); the ends along
//!   the line are `equal`, `mesh_inside` (Revit trimmed the beam) or
//!   `mesh_past` (the mesh runs past an end).
//!
//! A candidate is never wrong where its section is `equal` or `mesh_inside`
//! and its ends are not `mesh_past` on every beam it draws. Counts only: no
//! element id, name or coordinate is printed.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re328_beam_meshes -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_graphics_traversal::{
    NESTED_DETAIL_FILTER_DIAGNOSTIC, VIEW_COMPONENT_DIAGNOSTIC,
};
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_beam_axes::{
    BeamReject, BoundedLine, MIN_PLAN_SHARE, beam_check, beam_section_fit, box_along, line_copies,
    supports_revit_version,
};
use rvt::research::partition_schema_mvp::{
    beam_axis_from_fields, element_record_bbox, recover_partition_schema_mvp,
};
use rvt::walker::{DecodedElement, WalkerLimits};

const MATCH_FEET: f64 = 0.01;
const CONTROL_CAP: usize = 400;

struct Target {
    id: u32,
    bbox: [f64; 6],
    line: BoundedLine,
}

struct Drawn {
    id: u32,
    bbox: [f64; 6],
    start: [f64; 3],
    end: [f64; 3],
}

struct Mesh {
    points: Vec<[f64; 3]>,
    partial: bool,
}

struct Extents {
    along: (f64, f64),
    across: (f64, f64),
    up: (f64, f64),
}

fn bump(counts: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *counts.entry(key.into()).or_default() += 1;
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn reject_name(reject: &BeamReject) -> &'static str {
    match reject {
        BeamReject::NotFinite => "not_finite",
        BeamReject::InvertedBox => "inverted_box",
        BeamReject::EndOutside { .. } => "end_outside",
        BeamReject::ZeroLength => "zero_length",
        BeamReject::TooSteep => "too_steep",
        BeamReject::NoSection => "no_section",
        BeamReject::OpenClosure { .. } => "open_closure",
    }
}

fn orientation(direction: [f64; 3]) -> &'static str {
    if direction[0].hypot(direction[1]) < MIN_PLAN_SHARE {
        "steep"
    } else if direction[2].abs() > 1e-9 {
        "sloped"
    } else if direction[0].abs() < 1e-9 || direction[1].abs() < 1e-9 {
        "level_on_axis"
    } else {
        "level_rotated"
    }
}

/// The horizontal axis across the line and the one across it in its vertical
/// plane, as `beam_body` takes them.
fn frame(d: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
    let plan = d[0].hypot(d[1]);
    (plan >= MIN_PLAN_SHARE).then(|| {
        (
            [-d[1] / plan, d[0] / plan, 0.0],
            [-d[2] * d[0] / plan, -d[2] * d[1] / plan, plan],
        )
    })
}

fn range(points: &[[f64; 3]], axis: [f64; 3]) -> (f64, f64) {
    points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), p| {
            let v = dot(*p, axis);
            (low.min(v), high.max(v))
        })
}

fn box_vs_mesh(bbox: [f64; 6], points: &[[f64; 3]]) -> &'static str {
    let low = [0, 1, 2].map(|axis| points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min));
    let high = [0, 1, 2].map(|axis| {
        points
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    if (0..3).all(|axis| {
        (low[axis] - bbox[axis]).abs() <= MATCH_FEET
            && (high[axis] - bbox[axis + 3]).abs() <= MATCH_FEET
    }) {
        "equal"
    } else if (0..3).all(|axis| {
        low[axis] >= bbox[axis] - MATCH_FEET && high[axis] <= bbox[axis + 3] + MATCH_FEET
    }) {
        "mesh_inside"
    } else {
        "mesh_outside"
    }
}

/// A body centred on `centre`, `dims` = `[length, width, depth]` along `d`, `u`
/// and `w`, against the mesh's extents: how the section across the line and
/// the ends along it compare.
fn score(
    centre: [f64; 3],
    d: [f64; 3],
    u: [f64; 3],
    w: [f64; 3],
    dims: [f64; 3],
    mesh: &Extents,
) -> (&'static str, &'static str) {
    let span = |axis: [f64; 3], size: f64| {
        let middle = dot(centre, axis);
        (middle - size / 2.0, middle + size / 2.0)
    };
    let same = |a: (f64, f64), b: (f64, f64)| {
        (a.0 - b.0).abs() <= MATCH_FEET && (a.1 - b.1).abs() <= MATCH_FEET
    };
    let holds = |outer: (f64, f64), inner: (f64, f64)| {
        inner.0 >= outer.0 - MATCH_FEET && inner.1 <= outer.1 + MATCH_FEET
    };
    let (along, across, up) = (span(d, dims[0]), span(u, dims[1]), span(w, dims[2]));
    let section = if same(across, mesh.across) && same(up, mesh.up) {
        "equal"
    } else if holds(across, mesh.across) && holds(up, mesh.up) {
        "mesh_inside"
    } else {
        "differs"
    };
    let ends = if same(along, mesh.along) {
        "equal"
    } else if holds(along, mesh.along) {
        "mesh_inside"
    } else {
        "mesh_past"
    };
    (section, ends)
}

fn centre_of(bbox: [f64; 6]) -> [f64; 3] {
    [
        (bbox[0] + bbox[3]) / 2.0,
        (bbox[1] + bbox[4]) / 2.0,
        (bbox[2] + bbox[5]) / 2.0,
    ]
}

fn extents(points: &[[f64; 3]], d: [f64; 3], u: [f64; 3], w: [f64; 3]) -> Extents {
    Extents {
        along: range(points, d),
        across: range(points, u),
        up: range(points, w),
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    let version = rf.basic_file_info()?.version;
    println!("release {version}");
    println!("layout_supported {}", supports_revit_version(version));
    if !supports_revit_version(version) {
        return Ok(());
    }
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    println!("streams_not_read {}", rf.unread_streams().len());
    let beams: Vec<&DecodedElement> = mvp
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming" && element.id.is_some())
        .collect();
    let mut drawn: Vec<Drawn> = Vec::new();
    let mut unattached: BTreeSet<u32> = BTreeSet::new();
    for beam in &beams {
        let Some(id) = beam.id else { continue };
        match (
            beam_axis_from_fields(&beam.fields),
            element_record_bbox(beam),
        ) {
            (Some((start, end)), Some(bbox)) => drawn.push(Drawn {
                id,
                bbox,
                start,
                end,
            }),
            (None, Some(_)) => {
                unattached.insert(id);
            }
            _ => {}
        }
    }
    drawn.sort_by_key(|beam| beam.id);
    drawn.truncate(CONTROL_CAP);
    let copies = line_copies(&mut rf, version, &unattached)?;
    let mut targets: Vec<Target> = Vec::new();
    for beam in &beams {
        let Some(id) = beam.id else { continue };
        let (Some(found), Some(bbox)) = (copies.get(&id), element_record_bbox(beam)) else {
            continue;
        };
        if let Some((_, first)) = found.lines.first() {
            if found.lines.iter().all(|(_, line)| line == first) {
                targets.push(Target {
                    id,
                    bbox,
                    line: *first,
                });
            }
        }
    }
    println!("beams {}", beams.len());
    println!("targets {}", targets.len());
    println!("controls {}", drawn.len());
    let selected: BTreeSet<u64> = targets
        .iter()
        .map(|t| u64::from(t.id))
        .chain(drawn.iter().map(|d| u64::from(d.id)))
        .collect();
    let mut meshes: BTreeMap<u64, Mesh> = BTreeMap::new();
    if !selected.is_empty() {
        let options = Options {
            selected_ids: selected,
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Default::default()
        };
        let scene = extract_at_detail(&mut rf, &options, 3)?;
        for element in &scene.elements {
            let primitives = &element.meshes.primitives;
            if primitives.is_empty() {
                continue;
            }
            let partial = element.meshes.diagnostics.iter().any(|diagnostic| {
                !diagnostic.ends_with(NESTED_DETAIL_FILTER_DIAGNOSTIC)
                    && !diagnostic.ends_with(VIEW_COMPONENT_DIAGNOSTIC)
            });
            let points: Vec<[f64; 3]> = primitives
                .iter()
                .flat_map(|primitive| primitive.vertices.iter().copied())
                .collect();
            meshes.insert(element.identity.element_id, Mesh { points, partial });
        }
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for target in &targets {
        let (start, end) = (target.line.start(), target.line.end());
        let verdict = beam_check(target.bbox, start, end);
        let why = match &verdict {
            Ok(_) => "accepted",
            Err(reject) => reject_name(reject),
        };
        let d = target.line.direction;
        let key = format!("target {why} {}", orientation(d));
        bump(&mut counts, key.clone());
        let mesh = match meshes.get(&u64::from(target.id)) {
            None => {
                bump(&mut counts, format!("{key} mesh none"));
                continue;
            }
            Some(mesh) if mesh.partial => {
                bump(&mut counts, format!("{key} mesh partial"));
                continue;
            }
            Some(mesh) => mesh,
        };
        bump(&mut counts, format!("{key} mesh whole"));
        bump(
            &mut counts,
            format!(
                "{key} box_vs_mesh {}",
                box_vs_mesh(target.bbox, &mesh.points)
            ),
        );
        let Some((u, w)) = frame(d) else { continue };
        let found = extents(&mesh.points, d, u, w);
        let centre = centre_of(target.bbox);
        match box_along(target.bbox, d) {
            Some(dims) => {
                let (section, ends) = score(centre, d, u, w, dims, &found);
                bump(
                    &mut counts,
                    format!("{key} free section {section} ends {ends}"),
                );
            }
            None => bump(&mut counts, format!("{key} free none")),
        }
        if let Ok(body) = beam_section_fit(target.bbox, start, end) {
            let dims = [body.length_feet, body.width_feet, body.depth_feet];
            let (section, ends) = score(centre, d, u, w, dims, &found);
            bump(
                &mut counts,
                format!("{key} fit section {section} ends {ends}"),
            );
        }
    }
    for beam in &drawn {
        let Ok(body) = beam_check(beam.bbox, beam.start, beam.end) else {
            bump(&mut counts, "MISMATCH control declined");
            continue;
        };
        let d = body.direction;
        let key = format!("control {}", orientation(d));
        bump(&mut counts, key.clone());
        let mesh = match meshes.get(&u64::from(beam.id)) {
            None => {
                bump(&mut counts, format!("{key} mesh none"));
                continue;
            }
            Some(mesh) if mesh.partial => {
                bump(&mut counts, format!("{key} mesh partial"));
                continue;
            }
            Some(mesh) => mesh,
        };
        bump(&mut counts, format!("{key} mesh whole"));
        bump(
            &mut counts,
            format!("{key} box_vs_mesh {}", box_vs_mesh(beam.bbox, &mesh.points)),
        );
        let Some((u, w)) = frame(d) else { continue };
        let found = extents(&mesh.points, d, u, w);
        let dims = [body.length_feet, body.width_feet, body.depth_feet];
        let (section, ends) = score(body.centre, d, u, w, dims, &found);
        bump(
            &mut counts,
            format!("{key} body section {section} ends {ends}"),
        );
    }
    for (key, n) in &counts {
        println!("{key} {n}");
    }
    Ok(())
}
