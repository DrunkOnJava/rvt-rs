//! RE-225 (#810, #824): what a family symbol with no cut loop cuts in its
//! host, from the `m_complexCutGeoms` of its `FamilySymbol` record.
//!
//! FACT under test: RE-215 reads a door's or window's size from the one
//! `CurveLoop` its symbol's `m_pCutLoops` names. Some symbols name none:
//! two BIM4LCA office doors (1756089, 1765267) whose record box is
//! 0.1137 ft wider than Revit's `OverallWidth`, and the roof hatches whose
//! 600 mm openings Revit's export cuts in roof 2991742. RE-221's dump of the
//! hatch symbol showed `m_complexCutGeoms` on the same record. Hypothesis:
//! it holds the solid the symbol cuts, and that solid's extent along X and Z
//! is the size Revit's export writes (revit-ifc's
//! `ExporterIFCUtils.GetMinSymbolWidth`, CLEANROOM.md source 5).
//!
//! For every door and window the export writes, this follows its
//! `InstanceInfo.m_symbolId` to the symbol, walks every object
//! `m_complexCutGeoms` reaches (pointers followed to depth 4), meshes each
//! `Geometry` object's faces with `native_saved_mesh::face`, and prints the
//! box of all their vertices in the symbol's coordinates.
//!
//! Output:
//!   cut <instance> <IFC class> <symbol> loops=<n> complex=<n>
//!     loop=<width>x<height>|- geom=<x0>,<y0>,<z0>,<x1>,<y1>,<z1>|-
//!     faces=<meshed>/<seen> classes=<class:count,...>|-
//!     failed=<surface class>:<why face() declined it>,...|-
//!
//! Usage:
//!   cargo run --profile ci --example probe_re225_symbol_cut_geoms -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{face, pointer};
use rvt::research::native_symbol_cuts::instance_cut_loops;
use serde_json::Value;

const DEPTH: usize = 4;

#[derive(Default)]
struct CutGeometry {
    complex: usize,
    classes: BTreeMap<String, usize>,
    faces_seen: usize,
    faces_meshed: usize,
    failures: BTreeSet<String>,
    min: [f64; 3],
    max: [f64; 3],
}

fn pointers(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Object(map) if map.contains_key("pointer_token") => out.push(value.clone()),
        Value::Object(map) => map.values().for_each(|v| pointers(v, out)),
        Value::Array(items) => items.iter().for_each(|v| pointers(v, out)),
        _ => {}
    }
}

fn walk(
    graph: &ObjectGraph,
    index: usize,
    depth: usize,
    seen: &mut BTreeSet<usize>,
    cut: &mut CutGeometry,
) {
    if depth > DEPTH || !seen.insert(index) {
        return;
    }
    let Some(object) = graph.objects.get(index) else {
        return;
    };
    *cut.classes.entry(object.class_name.clone()).or_default() += 1;
    if object.class_name == "Geometry" {
        if let Some(faces) = object.fields.get("m_pFaces").and_then(Value::as_array) {
            for p in faces {
                let Ok(Some(fi)) = pointer(graph, index, p) else {
                    continue;
                };
                cut.faces_seen += 1;
                let surface = graph
                    .objects
                    .get(fi)
                    .and_then(|f| f.fields.get("m_pSurf"))
                    .and_then(|s| pointer(graph, fi, s).ok().flatten())
                    .and_then(|si| graph.objects.get(si))
                    .map_or("-".to_string(), |s| s.class_name.clone());
                match face(graph, fi) {
                    Ok(Some(mesh)) => {
                        cut.faces_meshed += 1;
                        for v in &mesh.vertices {
                            for k in 0..3 {
                                cut.min[k] = cut.min[k].min(v[k]);
                                cut.max[k] = cut.max[k].max(v[k]);
                            }
                        }
                    }
                    Ok(None) => {
                        cut.failures.insert(format!("{surface}:unbounded"));
                    }
                    Err(e) => {
                        let reason = format!("{e:#}").replace(' ', "_");
                        cut.failures.insert(format!("{surface}:{reason}"));
                    }
                }
            }
        }
        return;
    }
    let mut next = Vec::new();
    pointers(&object.fields, &mut next);
    for p in next {
        if let Ok(Some(target)) = pointer(graph, index, &p) {
            walk(graph, target, depth + 1, seen, cut);
        }
    }
}

fn cut_geometry(graph: &ObjectGraph) -> CutGeometry {
    let mut cut = CutGeometry {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
        ..Default::default()
    };
    let Some(root) = graph.objects.first() else {
        return cut;
    };
    let Some(list) = root.fields.get("m_complexCutGeoms") else {
        return cut;
    };
    let mut starts = Vec::new();
    pointers(list, &mut starts);
    cut.complex = starts.len();
    let mut seen = BTreeSet::from([0]);
    for p in starts {
        if let Ok(Some(target)) = pointer(graph, 0, &p) {
            walk(graph, target, 1, &mut seen, &mut cut);
        }
    }
    cut
}

fn element_id(value: &Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")?
        .as_i64()
        .and_then(|n| u64::try_from(n).ok())
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if ifc_type.eq_ignore_ascii_case("IFCDOOR")
                || ifc_type.eq_ignore_ascii_case("IFCWINDOW")
            {
                if let Ok(id) = tag.parse() {
                    classes.insert(id, ifc_type.to_ascii_uppercase());
                }
            }
        }
    }
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let loops = instance_cut_loops(&mut rf, &ids)?;
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let mut symbols: BTreeMap<u64, (usize, CutGeometry)> = BTreeMap::new();
    let options = Options {
        selected_ids: ids.clone(),
        selected_classes: BTreeSet::from(["FamilySymbol".to_string()]),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if record.class_name.as_deref() == Some("FamilySymbol") {
            let loop_count = graph
                .objects
                .first()
                .and_then(|root| root.fields.get("m_pCutLoops"))
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            symbols.insert(id, (loop_count, cut_geometry(graph)));
        } else if ids.contains(&id) {
            if let Some(symbol) = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo")
                .and_then(|object| object.fields.get("m_symbolId"))
                .and_then(element_id)
            {
                drawn_by.insert(id, symbol);
            }
        }
        Ok(())
    })?;
    let mut with_geometry = 0;
    for (instance, class) in &classes {
        let Some(symbol) = drawn_by.get(instance) else {
            println!(
                "cut {instance} {class} - loops=- complex=- loop=- geom=- faces=0/0 classes=-"
            );
            continue;
        };
        let (loop_count, cut) = match symbols.get(symbol) {
            Some((n, cut)) => (n.to_string(), cut),
            None => ("-".to_string(), &CutGeometry::default()),
        };
        let loop_text = loops.get(instance).map_or("-".to_string(), |l| {
            format!("{:.6}x{:.6}", l.width_feet(), l.height_feet())
        });
        let geom = if cut.faces_meshed > 0 {
            with_geometry += 1;
            format!(
                "{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                cut.min[0], cut.min[1], cut.min[2], cut.max[0], cut.max[1], cut.max[2]
            )
        } else {
            "-".to_string()
        };
        let class_text = if cut.classes.is_empty() {
            "-".to_string()
        } else {
            cut.classes
                .iter()
                .map(|(c, n)| format!("{c}:{n}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "cut {instance} {class} {symbol} loops={loop_count} complex={} loop={loop_text} geom={geom} faces={}/{} classes={class_text} failed={}",
            cut.complex,
            cut.faces_meshed,
            cut.faces_seen,
            if cut.failures.is_empty() {
                "-".to_string()
            } else {
                cut.failures.iter().cloned().collect::<Vec<_>>().join(",")
            }
        );
    }
    eprintln!(
        "{} doors and windows, {} with a symbol, {} with a loop, {} with meshed cut geometry",
        classes.len(),
        drawn_by.len(),
        loops.len(),
        with_geometry
    );
    Ok(())
}
