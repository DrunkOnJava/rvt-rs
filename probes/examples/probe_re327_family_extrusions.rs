//! RE-327 (#156, Discussion #112): are a family's extrusions rebuilt from
//! their own sketch curve records the solids Revit saved for display?
//!
//! FACT under test: an `ExtrusionElem`'s solid is its sketch's curves
//! (`CurveElem` records, each a `GLine` or `GArc` in the sketch plane's
//! coordinates), chained into closed loops and sorted into outlines and
//! holes, swept along the sketch plane's normal between the extrusion's two
//! `Double` parameters (-1001800 and -1001801). Discussion #112 (STE1200's
//! team) reports 85 of 86 extrusions of one 2014 family rebuilt this way.
//!
//! The oracle is the form's own saved display geometry: its channel-103
//! `GElement` record, meshed by `native_saved_mesh`. For each extrusion the
//! probe prints the stages (curves, loops, holes, offsets, whether the loops
//! the extrusion record itself stores are the sketch's) and compares the
//! rebuilt solid with the saved mesh:
//!   - volume: the solid's against the mesh's, within 1% and the most a
//!     chord-tessellated arc loses (two thirds of its chord times the
//!     tessellation's 0.001 ft sagitta, over the arc length and the height);
//!   - bounding box, within 0.01 ft;
//!   - every mesh vertex to the solid's surface, within 0.01 ft;
//!   - samples of the solid's surface to the mesh's triangles, within 0.01 ft.
//! It tries the sketch plane's matrix as columns and as rows, with and
//! without its `m_flipZ`, and tallies which conventions agree.
//!
//! A `.rfa` is read from its own plain groups (`native_form_geometry`); any
//! other file is a project, whose loaded families' documents are read.
//!
//! Output, one record per line:
//!   file <path> release <layout release>
//!   native: <error>
//!   walked groups .. keyed .. unframed .. records ..
//!   varsketch <records> refused <n> <diagnostic>
//!   form <document> <id> void <b> curves <n> loops <n> holes <n> area <ft2> start .. end ..
//!        stored <loops agree|disagree|none> mesh <triangles> ... default <metrics> best <name> <metrics> verdict ..
//!   unbuilt <document> <id> <reason>
//!   tally <key> <n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re327_family_extrusions -- FILE.rfa|FILE.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_family_forms::family_records_in;
use rvt::research::native_form_geometry::{self, Curve, Curve3, ExtrusionSolid, FormDocument};
use rvt::research::native_parameters::{self, GraphLimits, ObjectGraph};
use rvt::research::native_saved_mesh::{self, Primitive};

type V3 = [f64; 3];

const WANTED: [&str; 3] = ["ExtrusionElem", "CurveElem", "SketchPlane"];
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
const TOLERANCE_FEET: f64 = 0.01;
const SAGITTA_FEET: f64 = 0.001;
const FORM_LINES: usize = 400;

struct Mesh {
    vertices: Vec<V3>,
    triangles: Vec<[V3; 3]>,
    volume: f64,
    low: V3,
    high: V3,
    primitives: usize,
    complete: bool,
}

struct Measured {
    deficit: f64,
    ratio: f64,
    slack: f64,
    bbox: f64,
    vertex: f64,
    surface: f64,
}

impl Measured {
    fn volume_ok(&self, mesh_volume: f64) -> bool {
        self.deficit >= -0.01 * mesh_volume && self.deficit <= 0.01 * mesh_volume + self.slack
    }

    fn ok(&self, mesh_volume: f64) -> bool {
        self.volume_ok(mesh_volume)
            && self.bbox <= TOLERANCE_FEET
            && self.vertex <= TOLERANCE_FEET
            && self.surface <= TOLERANCE_FEET
    }

    fn text(&self) -> String {
        format!(
            "ratio {:.5} deficit {:.6} slack {:.6} bbox {:.6} vertex {:.6} surface {:.6}",
            self.ratio, self.deficit, self.slack, self.bbox, self.vertex, self.surface
        )
    }
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// The distance from `p` to triangle `abc` (closest point on a triangle).
fn point_triangle_distance(p: V3, a: V3, b: V3, c: V3) -> f64 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return norm(ap);
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return norm(bp);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return norm(sub(p, add(a, scale(ab, v))));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return norm(cp);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return norm(sub(p, add(a, scale(ac, w))));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return norm(sub(p, add(b, scale(sub(c, b), w))));
    }
    let denominator = 1.0 / (va + vb + vc);
    if !denominator.is_finite() {
        return f64::INFINITY;
    }
    let v = vb * denominator;
    let w = vc * denominator;
    norm(sub(p, add(a, add(scale(ab, v), scale(ac, w)))))
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

fn mesh_of(graph: &ObjectGraph) -> Option<Mesh> {
    let meshes = native_saved_mesh::graphics(graph);
    if meshes.primitives.is_empty() {
        return None;
    }
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for primitive in &meshes.primitives {
        vertices.extend(primitive.vertices.iter().copied());
        for t in &primitive.triangles {
            let point = |i: u32| primitive.vertices.get(usize::try_from(i).ok()?).copied();
            if let (Some(a), Some(b), Some(c)) = (point(t[0]), point(t[1]), point(t[2])) {
                triangles.push([a, b, c]);
            }
        }
    }
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for vertex in &vertices {
        for (axis, value) in vertex.iter().enumerate() {
            low[axis] = low[axis].min(*value);
            high[axis] = high[axis].max(*value);
        }
    }
    Some(Mesh {
        volume: mesh_volume(&meshes.primitives),
        vertices,
        triangles,
        low,
        high,
        primitives: meshes.primitives.len(),
        complete: meshes.diagnostics.is_empty() && meshes.unbounded_faces == 0,
    })
}

fn arc_length(solid: &ExtrusionSolid) -> f64 {
    solid
        .profile
        .loops
        .iter()
        .flat_map(|l| l.curves.iter())
        .map(|curve| match curve {
            Curve::Arc {
                radius, start, end, ..
            } => radius * (end - start).abs(),
            Curve::Line { .. } => 0.0,
        })
        .sum()
}

fn measure(solid: &ExtrusionSolid, mesh: &Mesh) -> Measured {
    let volume = solid.volume();
    let (low, high) = solid.bounding_box();
    let bbox = (0..3)
        .map(|k| {
            (low[k] - mesh.low[k])
                .abs()
                .max((high[k] - mesh.high[k]).abs())
        })
        .fold(0.0, f64::max);
    let vertex = mesh
        .vertices
        .iter()
        .map(|v| solid.signed_distance(*v).abs())
        .fold(0.0, f64::max);
    let budget = (2_000_000 / mesh.triangles.len().max(1)).clamp(50, 4000);
    let samples = solid.surface_samples(12);
    let stride = (samples.len() / budget).max(1);
    let surface = samples
        .iter()
        .step_by(stride)
        .map(|p| {
            mesh.triangles
                .iter()
                .map(|t| point_triangle_distance(*p, t[0], t[1], t[2]))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max);
    Measured {
        deficit: volume - mesh.volume,
        ratio: if mesh.volume > 0.0 {
            volume / mesh.volume
        } else {
            f64::NAN
        },
        slack: 2.0 / 3.0 * arc_length(solid) * SAGITTA_FEET * (solid.high - solid.low),
        bbox,
        vertex,
        surface,
    }
}

struct Run {
    document: FormDocument,
    graphics: BTreeMap<u64, ObjectGraph>,
}

fn standalone(
    rf: &mut RevitFile,
    tally: &mut BTreeMap<String, usize>,
) -> Result<Vec<(String, Run)>> {
    let document = native_form_geometry::open(rf)?;
    let mut forms = FormDocument::default();
    let mut refused: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut varsketch = (0usize, 0usize);
    let walked = native_form_geometry::walk(rf, &document, &[102], |entry| {
        let class = entry.class.unwrap_or("-");
        let wanted = WANTED.contains(&class);
        if !wanted && class != "VarSketch" {
            return Ok(());
        }
        match native_parameters::decode_graph_with_limits(entry.body, &document.registry, &LIMITS) {
            Ok(graph) => {
                if class == "VarSketch" {
                    varsketch.0 += 1;
                } else {
                    forms.insert(entry.id, graph);
                }
            }
            Err(error) => {
                if class == "VarSketch" {
                    varsketch.0 += 1;
                    varsketch.1 += 1;
                }
                let text: String = format!("{error:#}")
                    .chars()
                    .map(|c| if c.is_ascii_digit() { '#' } else { c })
                    .take(160)
                    .collect();
                *refused.entry((class.to_string(), text)).or_default() += 1;
            }
        }
        Ok(())
    })?;
    println!(
        "walked groups {} keyed {} unframed {} records {}",
        walked.groups, walked.keyed_groups, walked.unframed_groups, walked.records
    );
    println!("varsketch {} refused {}", varsketch.0, varsketch.1);
    for ((class, text), n) in &refused {
        println!("refusal {class} {n} {text}");
        *tally.entry(format!("refused {class}")).or_default() += n;
    }
    let wanted: BTreeSet<u64> = forms.extrusion_ids().into_iter().collect();
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
        if !document.extrusion_ids().is_empty() {
            runs.insert(
                family,
                Run {
                    document,
                    graphics: BTreeMap::new(),
                },
            );
        }
    }
    println!("families-with-extrusions {}", runs.len());
    let with_forms: BTreeSet<u64> = runs.keys().copied().collect();
    let ids: BTreeSet<u64> = runs
        .values()
        .flat_map(|run| run.document.extrusion_ids())
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

fn describe_curves(curves: &[Curve3]) -> String {
    let point = |p: V3| format!("({:.4},{:.4},{:.4})", p[0], p[1], p[2]);
    curves
        .iter()
        .take(12)
        .map(|curve| match curve {
            Curve3::Line { from, to } => format!("L{}>{}", point(*from), point(*to)),
            Curve3::Arc {
                center,
                radius,
                start,
                end,
                ..
            } => format!(
                "A{} r{:.4} [{:.4},{:.4}]",
                point(*center),
                radius,
                start,
                end
            ),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn judge(
    label: &str,
    run: &Run,
    tally: &mut BTreeMap<String, usize>,
    lines: &mut usize,
    detailed: &mut usize,
) {
    for id in run.document.extrusion_ids() {
        bump(tally, "forms");
        let extrusion = match run.document.extrusion(id) {
            Ok(extrusion) => extrusion,
            Err(reason) => {
                println!("unbuilt {label} {id} {reason}");
                bump(tally, format!("unbuilt: {reason}"));
                continue;
            }
        };
        let mut variants: Vec<(&str, bool, bool)> =
            vec![("columns", false, true), ("rows", true, true)];
        if extrusion.plane_flipped {
            variants.push(("columns-unflipped", false, false));
            variants.push(("rows-unflipped", true, false));
        }
        let mut built: Vec<(&str, ExtrusionSolid)> = Vec::new();
        let mut refusals: Vec<(&str, String)> = Vec::new();
        for (name, transposed, flip) in variants {
            match extrusion.solid(transposed, flip) {
                Ok(solid) => built.push((name, solid)),
                Err(reason) => refusals.push((name, reason)),
            }
        }
        for (name, reason) in &refusals {
            if !built.is_empty() {
                bump(
                    tally,
                    format!("{name} refused where another builds: {reason}"),
                );
            }
        }
        let Some(first) = built.first() else {
            let reason = refusals
                .first()
                .map(|(_, reason)| reason.clone())
                .unwrap_or_default();
            println!("unbuilt {label} {id} void {} {reason}", extrusion.cutting);
            bump(tally, format!("unbuilt: {reason}"));
            if *detailed < 14 && !reason.contains("sketch plane") {
                *detailed += 1;
                println!(
                    "unbuilt-curves {label} {id} count {} {}",
                    extrusion.curves.len(),
                    describe_curves(&extrusion.curves)
                );
            }
            continue;
        };
        bump(tally, "built");
        if extrusion.cutting {
            bump(tally, "built void");
        }
        let holes = first
            .1
            .profile
            .depths
            .iter()
            .filter(|d| *d % 2 == 1)
            .count();
        let stored = match extrusion.stored_loops_agree {
            Some(true) => "agree",
            Some(false) => "disagree",
            None => "none",
        };
        bump(tally, format!("stored loops {stored}"));
        let head = format!(
            "form {label} {id} void {} curves {} loops {} holes {holes} area {:.6} start {:.6} end {:.6} stored {stored} flipped {}",
            extrusion.cutting,
            extrusion.curve_ids.len(),
            first.1.profile.loops.len(),
            first.1.profile.area,
            extrusion.start,
            extrusion.end,
            extrusion.plane_flipped
        );
        let Some(mesh) = run.graphics.get(&id).and_then(mesh_of) else {
            if *lines < FORM_LINES {
                println!("{head} mesh none");
            }
            *lines += 1;
            bump(
                tally,
                if extrusion.cutting {
                    "no mesh void"
                } else {
                    "no mesh"
                },
            );
            continue;
        };
        bump(tally, "mesh");
        if !mesh.complete {
            bump(tally, "mesh partial");
        }
        let results: Vec<(&str, Measured)> = built
            .iter()
            .map(|(name, solid)| (*name, measure(solid, &mesh)))
            .collect();
        let matching: Vec<&str> = results
            .iter()
            .filter(|(_, m)| m.ok(mesh.volume))
            .map(|(name, _)| *name)
            .collect();
        let best = results
            .iter()
            .min_by(|a, b| {
                a.1.vertex
                    .max(a.1.surface)
                    .total_cmp(&b.1.vertex.max(b.1.surface))
            })
            .unwrap_or(&results[0]);
        let key = if mesh.complete {
            "complete mesh"
        } else {
            "partial mesh"
        };
        if matching.is_empty() {
            bump(tally, format!("DIFFER, {key}"));
            if extrusion.cutting {
                bump(tally, format!("DIFFER void, {key}"));
            }
        } else {
            bump(tally, format!("MATCH, {key}"));
            if extrusion.cutting {
                bump(tally, format!("MATCH void, {key}"));
            }
            for name in &matching {
                bump(tally, format!("matches under {name}"));
            }
            if matching.len() == 1 {
                bump(tally, format!("matches only under {}", matching[0]));
            }
        }
        if *lines < FORM_LINES || matching.is_empty() {
            println!(
                "{head} mesh {} primitives {} complete {} volume {:.6} best {} {} matching {}",
                mesh.triangles.len(),
                mesh.primitives,
                mesh.complete,
                mesh.volume,
                best.0,
                best.1.text(),
                if matching.is_empty() {
                    "DIFFER".to_string()
                } else {
                    matching.join("+")
                }
            );
        }
        *lines += 1;
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re327_family_extrusions FILE"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    let runs = if path.ends_with(".rfa") {
        standalone(&mut rf, &mut tally)
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
    let mut lines = 0usize;
    let mut detailed = 0usize;
    for (label, run) in &runs {
        judge(label, run, &mut tally, &mut lines, &mut detailed);
    }
    for (key, n) in &tally {
        println!("tally {key} {n}");
    }
    Ok(())
}
