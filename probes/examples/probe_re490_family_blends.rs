//! RE-490: of the blends real files hold, how many does `FormDocument::blend`
//! read, why does it refuse the others, and do the ones it reads match the
//! mesh Revit saved for them?
//!
//! FACT under test: a `BlendElem` of two straight-edged outlines in parallel
//! planes, with the same number of vertices, is the solid whose corners run
//! straight from each vertex of the first sketch, at the first offset
//! (`Double` parameter -1001804), to the vertex of the same index of the
//! second sketch, at the second offset (-1001805). RE-490 measured it on five
//! blends of two Revit 2020 families. This counts it on any file.
//!
//! The oracle is the blend's own saved display geometry: its channel-103
//! `GElement` record, meshed by `native_saved_mesh`. For each blend read:
//!   - volume: the blend's against the mesh's, as a relative difference;
//!   - corners: the farthest any corner lies from the nearest mesh vertex,
//!     with the offsets as read and with the two exchanged.
//!
//! A `.rfa` is read from its own plain groups (`native_form_geometry`); any
//! other file is a project, whose loaded families' documents are read.
//!
//! Output, one record per line (differences and counts only, no coordinate,
//! area or volume of the model):
//!   file <file name> release <layout release>
//!   native: <error>
//!   blend <document> <id> void <b> vertices <n>+<n> mesh <complete|partial|none>
//!        volume_rel <difference> corners <feet> exchanged <feet> verdict <..>
//!   unbuilt <document> <id> <reason>
//!   tally <key> <n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re490_family_blends -- FILE.rfa|FILE.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_family_forms::family_records_in;
use rvt::research::native_form_geometry::{self, Blend, FormDocument};
use rvt::research::native_parameters::{self, GraphLimits, ObjectGraph};
use rvt::research::native_saved_mesh::{self, Primitive};

type V3 = [f64; 3];

const WANTED: [&str; 4] = ["BlendElem", "CurveElem", "SketchPlane", "VarSketch"];
const LIMITS: GraphLimits = GraphLimits {
    max_values: 2_000_000,
    max_objects: 2_000_000,
    max_depth: 128,
};
const MESH_LIMITS: GraphLimits = GraphLimits {
    max_values: 20_000_000,
    max_objects: 20_000_000,
    max_depth: 128,
};
/// How far a corner may lie from a mesh vertex and still be on it, feet.
const CORNER_FEET: f64 = 1e-6;
/// The relative difference under which two volumes agree.
const VOLUME_RELATIVE: f64 = 1e-6;
/// How many `blend` and `unbuilt` lines a file prints.
const LINES: usize = 400;

struct Run {
    document: FormDocument,
    graphics: BTreeMap<u64, ObjectGraph>,
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn distance(a: V3, b: V3) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    dot(d, d).sqrt()
}

fn mesh_volume(primitives: &[Primitive]) -> f64 {
    let mut total = 0.0;
    for primitive in primitives {
        for triangle in &primitive.triangles {
            let point = |i: u32| primitive.vertices.get(usize::try_from(i).ok()?).copied();
            let (Some(a), Some(b), Some(c)) =
                (point(triangle[0]), point(triangle[1]), point(triangle[2]))
            else {
                continue;
            };
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            total += dot(a, cross) / 6.0;
        }
    }
    total.abs()
}

/// The farthest any corner of `blend` lies from its nearest vertex.
fn farthest(blend: &Blend, vertices: &[V3]) -> f64 {
    blend
        .corners()
        .iter()
        .map(|corner| {
            vertices
                .iter()
                .map(|vertex| distance(*corner, *vertex))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn standalone(rf: &mut RevitFile) -> Result<Vec<(String, Run)>> {
    let document = native_form_geometry::open(rf)?;
    let mut forms = FormDocument::default();
    native_form_geometry::walk(rf, &document, &[102], |entry| {
        let class = entry.class.unwrap_or("-");
        if !WANTED.contains(&class) {
            return Ok(());
        }
        if let Ok(graph) =
            native_parameters::decode_graph_with_limits(entry.body, &document.registry, &LIMITS)
        {
            forms.insert(entry.id, graph);
        }
        Ok(())
    })?;
    let wanted: BTreeSet<u64> = forms.blend_ids().into_iter().collect();
    let mut graphics = BTreeMap::new();
    native_form_geometry::walk(rf, &document, &[103], |entry| {
        if entry.class != Some("GElement") || !wanted.contains(&entry.id) {
            return Ok(());
        }
        if let Ok(graph) = native_parameters::decode_graph_with_limits(
            entry.body,
            &document.registry,
            &MESH_LIMITS,
        ) {
            graphics.insert(entry.id, graph);
        }
        Ok(())
    })?;
    Ok(vec![(
        "document".to_string(),
        Run {
            document: forms,
            graphics,
        },
    )])
}

fn project(rf: &mut RevitFile) -> Result<Vec<(String, Run)>> {
    let options = Options {
        selected_classes: BTreeSet::from(["Family".to_string()]),
        releases: native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    let mut families: BTreeSet<u64> = BTreeSet::new();
    native_document::extract_graphs(rf, &options, |record| {
        if record.graph.is_some() {
            families.insert(record.identity.element_id);
        }
        Ok(())
    })?;
    println!("families {}", families.len());
    let classes: BTreeSet<String> = WANTED.iter().map(|c| c.to_string()).collect();
    let records = family_records_in(
        rf,
        &families,
        &classes,
        &native_document::NATIVE_RELEASES,
        None,
    )?;
    let mut runs: BTreeMap<u64, Run> = BTreeMap::new();
    for (family, items) in records {
        let mut document = FormDocument::default();
        for (id, graph) in items {
            document.insert(id, graph);
        }
        if !document.blend_ids().is_empty() {
            runs.insert(
                family,
                Run {
                    document,
                    graphics: BTreeMap::new(),
                },
            );
        }
    }
    println!("families-with-blends {}", runs.len());
    let with_forms: BTreeSet<u64> = runs.keys().copied().collect();
    let ids: BTreeSet<u64> = runs
        .values()
        .flat_map(|run| run.document.blend_ids())
        .collect();
    let graphics = family_records_in(
        rf,
        &with_forms,
        &BTreeSet::from(["GElement".to_string()]),
        &native_document::NATIVE_RELEASES,
        Some(&ids),
    )?;
    for (family, items) in graphics {
        if let Some(run) = runs.get_mut(&family) {
            for (id, graph) in items {
                run.graphics.insert(id, graph);
            }
        }
    }
    Ok(runs
        .into_iter()
        .map(|(family, run)| (format!("family-{family}"), run))
        .collect())
}

fn bump(tally: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *tally.entry(key.into()).or_default() += 1;
}

/// A refusal with its digits written as `#`, so that reasons that differ only
/// in an id or a count are one tally.
fn reason(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .take(120)
        .collect()
}

fn judge(label: &str, run: &Run, tally: &mut BTreeMap<String, usize>, lines: &mut usize) {
    for id in run.document.blend_ids() {
        bump(tally, "blends");
        let blend = match run.document.blend(id) {
            Ok(blend) => blend,
            Err(error) => {
                let why = reason(&error);
                bump(tally, format!("refused: {why}"));
                if *lines < LINES {
                    *lines += 1;
                    println!("unbuilt {label} {id} {why}");
                }
                continue;
            }
        };
        bump(tally, "read");
        let counts = format!(
            "{}+{}",
            blend.sections[0].vertices.len(),
            blend.sections[1].vertices.len()
        );
        let meshes = run.graphics.get(&id).map(native_saved_mesh::graphics);
        let (state, verdict, detail) = match &meshes {
            None => ("none", "no saved mesh".to_string(), String::new()),
            Some(meshes) if meshes.primitives.is_empty() => {
                ("none", "no saved mesh".to_string(), String::new())
            }
            Some(meshes) if !meshes.diagnostics.is_empty() || meshes.unbounded_faces != 0 => {
                ("partial", "saved mesh partial".to_string(), String::new())
            }
            Some(meshes) => {
                let vertices: Vec<V3> = meshes
                    .primitives
                    .iter()
                    .flat_map(|primitive| primitive.vertices.iter().copied())
                    .collect();
                let mesh = mesh_volume(&meshes.primitives);
                let relative = match blend.volume() {
                    Ok(volume) if mesh > 0.0 => Some((volume - mesh).abs() / mesh),
                    _ => None,
                };
                let as_read = farthest(&blend, &vertices);
                let exchanged = farthest(&blend.swapped(), &vertices);
                let volume = match relative {
                    Some(r) if r <= VOLUME_RELATIVE => "volume agrees",
                    Some(_) => "volume differs",
                    None => "volume not compared",
                };
                let corners = if as_read <= CORNER_FEET && exchanged <= CORNER_FEET {
                    "corners on the mesh either way"
                } else if as_read <= CORNER_FEET {
                    "corners on the mesh as read"
                } else if exchanged <= CORNER_FEET {
                    "corners on the mesh only with the offsets exchanged"
                } else {
                    "corners off the mesh either way"
                };
                (
                    "complete",
                    format!("{volume}, {corners}"),
                    format!(
                        " volume_rel {} corners {as_read:.9} exchanged {exchanged:.9}",
                        relative.map_or("-".to_string(), |r| format!("{r:.3e}"))
                    ),
                )
            }
        };
        bump(tally, format!("read: {verdict}"));
        if *lines < LINES {
            *lines += 1;
            println!(
                "blend {label} {id} void {} vertices {counts} mesh {state}{detail} verdict {verdict}",
                blend.cutting
            );
        }
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re490_family_blends FILE"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    // The name only: a path names the machine's folders, which kept evidence must not.
    let name = std::path::Path::new(&path)
        .file_name()
        .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
    println!("file {name} release {release}");
    let runs = if path.ends_with(".rfa") {
        standalone(&mut rf)
    } else {
        project(&mut rf)
    };
    let runs = match runs {
        Ok(runs) => runs,
        Err(error) => {
            println!("native: {error:#}");
            return Ok(());
        }
    };
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    let mut lines = 0usize;
    for (label, run) in &runs {
        judge(label, run, &mut tally, &mut lines);
    }
    for (key, n) in &tally {
        println!("tally {key} {n}");
    }
    Ok(())
}
