//! RE-318 (#810): where the 5.8629 ft `OverallWidth` of the BIM4LCA office's
//! two door-with-sidelight doors (1756089, 1765267) comes from.
//!
//! FACT under test: Revit's export sizes a door from the symbol that draws it
//! (revit-ifc's `GetDoorWindowDimensionFromSymbol`, which feeds
//! `ExporterIFCUtils.GetOriginalSymbol`'s symbol to `GetMinSymbolWidth`,
//! CLEANROOM.md source 5). RE-215 (cut loops), RE-225 (cut solid) and RE-229
//! (stored lengths) leave these two doors 0.1137 ft short of Revit's width:
//! the record box is 5.9766 ft, the bounded faces of the cut solid span
//! 3.1168 ft, and no stored length is 5.8629 ft. This prints what is left to
//! test for every door and window the export writes:
//!
//! 1. the cut solid's unbounded planes (origin, axes, orientation), and the
//!    `FamilySymbol` root's `m_closurePlanes`, `m_cutDirs`, `m_cutIndices`
//!    and `m_cutPlaneHeights`, which RE-215 dumped only for symbols with a
//!    loop; the two sidelight symbols' every field and object;
//! 2. the symbol's saved graphics (the 3D geometry Revit drew): each
//!    primitive's owner, face tag, render style, material and box, for the
//!    symbol's own record (symbol frame) and for the instance's (document
//!    frame, with the shared symbol's primitives tagged by `owner`).
//!
//! Output, one record per line (JSON has no spaces):
//!   inst <id> <class> symbol=<drawn by> master=<type> host=<m_hostId> trf=<InstanceInfo.m_Trf>
//!   symroot <symbol> family=<id> loops=<m_pCutLoops> closure=.. dirs=.. indices=.. heights=..
//!   symclasses <symbol> objects=<n> <class:count,...>
//!   symface <symbol> <solid> <face> surface=<class> mesh=<x0,y0,z0,x1,y1,z1|unbounded|error:..>
//!     origin=.. x=.. y=.. orient=.. [face=<fields>]
//!   symfield <symbol> <name> <value>          (sidelight symbols only)
//!   symobj <symbol> <index> <class> <fields>  (sidelight symbols only)
//!   scene <id> <instance|symbol> status=.. prims=<n> unbounded=<n> diag=<first>
//!   prim <id> <instance|symbol> owner=<symbol|-> tag=<face tag> style=<render style>
//!     material=<id|-> verts=<n> box=<x0,y0,z0,x1,y1,z1>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re318_door_width_sources -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{face, pointer};
use rvt::research::native_saved_scene;
use serde_json::Value;

const FULL: [u64; 2] = [2058852, 1766487];
const OBJECT_LIMIT: usize = 400;

struct Instance {
    drawn_by: Option<u64>,
    master: Option<u64>,
    host: Option<u64>,
    transform: String,
}

fn cap(text: String, limit: usize) -> String {
    if text.chars().count() <= limit {
        text
    } else {
        text.chars().take(limit).collect::<String>() + "..."
    }
}

fn compact(value: Option<&Value>) -> String {
    value.map_or("-".to_string(), |v| v.to_string())
}

fn pointers(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Object(map) if map.contains_key("pointer_token") => out.push(value.clone()),
        Value::Object(map) => map.values().for_each(|v| pointers(v, out)),
        Value::Array(items) => items.iter().for_each(|v| pointers(v, out)),
        _ => {}
    }
}

fn element_id(value: &Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")?
        .as_i64()
        .and_then(|n| u64::try_from(n).ok())
}

fn bounds(vertices: &[[f64; 3]]) -> Option<[f64; 6]> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for v in vertices {
        for (k, value) in v.iter().enumerate() {
            b[k] = b[k].min(*value);
            b[k + 3] = b[k + 3].max(*value);
        }
    }
    b.iter().all(|x| x.is_finite()).then_some(b)
}

fn box_text(b: Option<[f64; 6]>) -> String {
    b.map_or("-".to_string(), |b| {
        format!(
            "{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    })
}

/// The lines describing one `FamilySymbol` graph that cuts no loop.
fn summarize(graph: &ObjectGraph, id: u64, full: bool) -> Vec<String> {
    let mut out = Vec::new();
    let Some(root) = graph.objects.first() else {
        return out;
    };
    let get = |name: &str| {
        root.fields
            .get(name)
            .map_or("-".to_string(), |v| cap(v.to_string(), 1500))
    };
    out.push(format!(
        "symroot {id} family={} loops={} closure={} dirs={} indices={} heights={}",
        root.fields
            .get("m_familyId")
            .and_then(element_id)
            .map_or("-".to_string(), |f| f.to_string()),
        get("m_pCutLoops"),
        get("m_closurePlanes"),
        get("m_cutDirs"),
        get("m_cutIndices"),
        get("m_cutPlaneHeights"),
    ));
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for object in &graph.objects {
        *census.entry(object.class_name.as_str()).or_default() += 1;
    }
    out.push(format!(
        "symclasses {id} objects={} {}",
        graph.objects.len(),
        census
            .iter()
            .map(|(class, n)| format!("{class}:{n}"))
            .collect::<Vec<_>>()
            .join(",")
    ));
    let mut solids = Vec::new();
    if let Some(list) = root.fields.get("m_complexCutGeoms") {
        pointers(list, &mut solids);
    }
    for (n, solid) in solids.iter().enumerate() {
        let Ok(Some(gi)) = pointer(graph, 0, solid) else {
            continue;
        };
        let Some(geometry) = graph.objects.get(gi) else {
            continue;
        };
        let Some(faces) = geometry.fields.get("m_pFaces").and_then(Value::as_array) else {
            out.push(format!("symsolid {id} {n} class={}", geometry.class_name));
            continue;
        };
        for (k, face_pointer) in faces.iter().enumerate() {
            let Ok(Some(fi)) = pointer(graph, gi, face_pointer) else {
                continue;
            };
            let Some(face_object) = graph.objects.get(fi) else {
                continue;
            };
            let surface = face_object
                .fields
                .get("m_pSurf")
                .and_then(|s| pointer(graph, fi, s).ok().flatten())
                .and_then(|si| graph.objects.get(si));
            let class = surface.map_or("-", |s| s.class_name.as_str());
            let plane = surface.filter(|s| s.class_name == "Plane").map_or(
                "origin=- x=- y=- orient=-".to_string(),
                |s| {
                    format!(
                        "origin={} x={} y={} orient={}",
                        compact(s.fields.get("m_origin")),
                        compact(s.fields.get("m_xVec")),
                        compact(s.fields.get("m_yVec")),
                        compact(s.fields.get("m_orientFlag")),
                    )
                },
            );
            let (mesh, bounded) = match face(graph, fi) {
                Ok(Some(m)) => (box_text(bounds(&m.vertices)), true),
                Ok(None) => ("unbounded".to_string(), false),
                Err(e) => (
                    format!("error:{}", format!("{e:#}").replace(' ', "_")),
                    false,
                ),
            };
            let detail = if full || !bounded {
                format!(" face={}", cap(face_object.fields.to_string(), 900))
            } else {
                String::new()
            };
            out.push(format!(
                "symface {id} {n} {k} surface={class} mesh={mesh} {plane}{detail}"
            ));
        }
    }
    if full {
        if let Some(map) = root.fields.as_object() {
            for (name, value) in map {
                out.push(format!(
                    "symfield {id} {name} {}",
                    cap(value.to_string(), 600)
                ));
            }
        }
        for (index, object) in graph.objects.iter().enumerate().take(OBJECT_LIMIT) {
            out.push(format!(
                "symobj {id} {index} {} {}",
                object.class_name,
                cap(object.fields.to_string(), 1200)
            ));
        }
    }
    out
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
    let mut instances: BTreeMap<u64, Instance> = BTreeMap::new();
    let mut symbol_lines: BTreeMap<u64, Vec<String>> = BTreeMap::new();
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
            let uncut = graph
                .objects
                .first()
                .and_then(|root| root.fields.get("m_pCutLoops"))
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty);
            if uncut {
                symbol_lines.insert(id, summarize(graph, id, FULL.contains(&id)));
            }
        } else if ids.contains(&id) {
            let info = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo");
            let root = graph.objects.first();
            instances.insert(
                id,
                Instance {
                    drawn_by: info
                        .and_then(|o| o.fields.get("m_symbolId"))
                        .and_then(element_id),
                    master: root
                        .and_then(|o| o.fields.get("m_masterSymbolId"))
                        .and_then(element_id),
                    host: root
                        .and_then(|o| o.fields.get("m_hostId"))
                        .and_then(element_id),
                    transform: compact(info.and_then(|o| o.fields.get("m_Trf"))),
                },
            );
        }
        Ok(())
    })?;
    let opt = |v: Option<u64>| v.map_or("-".to_string(), |n| n.to_string());
    let mut used: BTreeSet<u64> = BTreeSet::new();
    for (id, class) in &classes {
        match instances.get(id) {
            Some(i) => {
                used.extend(i.drawn_by);
                println!(
                    "inst {id} {class} symbol={} master={} host={} trf={}",
                    opt(i.drawn_by),
                    opt(i.master),
                    opt(i.host),
                    i.transform
                );
            }
            None => println!("inst {id} {class} -"),
        }
    }
    for (id, lines) in &symbol_lines {
        if used.contains(id) {
            for line in lines {
                println!("{line}");
            }
        }
    }
    let scene_options = Options {
        selected_ids: ids.iter().chain(used.iter()).copied().collect(),
        ..Options::default()
    };
    match native_saved_scene::extract(&mut rf, &scene_options) {
        Ok(scene) => {
            for element in &scene.elements {
                let kind = if ids.contains(&element.id) {
                    "instance"
                } else {
                    "symbol"
                };
                let meshes = &element.meshes;
                println!(
                    "scene {} {kind} status={} prims={} unbounded={} diag={}",
                    element.id,
                    element.status,
                    meshes.primitives.len(),
                    meshes.unbounded_faces,
                    meshes
                        .diagnostics
                        .first()
                        .map_or("-".to_string(), |d| cap(d.replace(' ', "_"), 200)),
                );
                for p in &meshes.primitives {
                    println!(
                        "prim {} {kind} owner={} tag={} style={} material={} verts={} box={}",
                        element.id,
                        opt(p.source_owner_id),
                        p.face_tag,
                        p.render_style_id,
                        p.material_id.map_or("-".to_string(), |m| m.to_string()),
                        p.vertices.len(),
                        box_text(bounds(&p.vertices)),
                    );
                }
            }
        }
        Err(e) => eprintln!("saved scene: {e:#}"),
    }
    eprintln!(
        "{} doors and windows, {} with a record, {} uncut symbols used",
        classes.len(),
        instances.len(),
        used.iter()
            .filter(|s| symbol_lines.contains_key(*s))
            .count()
    );
    Ok(())
}
