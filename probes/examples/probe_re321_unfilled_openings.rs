//! #1165 (RE-321): which records produce the openings Revit's export writes
//! that no door or window fills.
//!
//! FACT under test: each unfilled `IfcOpeningElement` of Revit's export is the
//! cut of an element (its `Tag`) in a host (the element its
//! `IfcRelVoidsElement` voids), and the pair is named by a step of the host's
//! native record (`WallCutoutGStep.m_instId`, RE-266) or by the cutter's own.
//!
//! Reads Revit's export beside the model (`<model>.ifc`, `<model>_slim.ifc` or
//! `<model>_Revit.ifc`). Output, one record per line:
//!   open <tag> <host class> <host tag> <predefined type> <name>
//!   frame <id> <class> category <n> owner <id|-> bbox <six feet> refs <ids>
//!   rec <id> <class> root <fields json>
//!   rec <id> classes <class:count ...>
//!   rec <id> step <class> <fields json>
//!   rec <id> obj <class> <fields json>
//!
//! A `rec` is every wall and floor and roof record, and every record whose id
//! is the tag of an unfilled opening, its host, a fire suppression terminal, a
//! curtain wall or a door or window that cuts one. The first `CONTROLS` other
//! doors and windows are printed in full as controls.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re321_unfilled_openings -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{face, pointer};
use rvt::research::native_symbol_cuts::instance_cut_loops;
use rvt::research::partition_element_records::{self as per};
use serde_json::{Map, Value};

const WALLS: [&str; 2] = ["SWall", "ArcWall"];
const SLABS: [&str; 5] = [
    "Floor",
    "Roof",
    "FootPrintRoof",
    "ProfileRoof",
    "BuildingPad",
];
const OPENINGS: [&str; 2] = ["SWallRectOpening", "ArcWallRectOpening"];
const CLIP: usize = 3000;
const CONTROLS: usize = 12;
const MAX_OBJECTS: usize = 80;

fn clip(text: String) -> String {
    if text.len() <= CLIP {
        text
    } else {
        text.chars().take(CLIP).collect()
    }
}

fn trimmed(fields: &Value) -> String {
    let Some(members) = fields.as_object() else {
        return clip(fields.to_string());
    };
    let mut kept = Map::new();
    for (name, member) in members {
        if name.ends_with("HistTable") {
            let count = member.as_array().map_or(0, Vec::len);
            kept.insert(format!("{name}#"), Value::from(count));
        } else if member.get("pointer_token").is_none() {
            kept.insert(name.clone(), member.clone());
        }
    }
    clip(Value::Object(kept).to_string())
}

/// `#id -> (entity, args)` for every line of a STEP file.
fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

/// An element's `Tag`: the last quoted string of its arguments that is all
/// digits.
fn tag(args: &str) -> Option<u64> {
    args.split('\'')
        .rfind(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_digit()))?
        .parse()
        .ok()
}

fn name(args: &str) -> String {
    args.split('\'').nth(3).unwrap_or("").to_string()
}

fn predefined(args: &str) -> String {
    args.rsplit(',')
        .next()
        .unwrap_or("")
        .trim_end_matches(");")
        .trim_end_matches(')')
        .to_string()
}

struct Opening {
    tag: u64,
    host_class: String,
    host_tag: Option<u64>,
    predefined: String,
    name: String,
}

struct Export {
    openings: Vec<Opening>,
    wanted: BTreeSet<u64>,
    doors: BTreeSet<u64>,
}

fn read_export(step: &str) -> Export {
    let ents = entities(step);
    let mut host_of = BTreeMap::new();
    let mut filled = BTreeSet::new();
    for (entity, args) in ents.values() {
        let r = refs(args);
        match entity.as_str() {
            "IFCRELVOIDSELEMENT" if r.len() >= 2 => {
                host_of.insert(r[r.len() - 1], r[r.len() - 2]);
            }
            "IFCRELFILLSELEMENT" if r.len() >= 2 => {
                filled.insert(r[r.len() - 2]);
            }
            _ => {}
        }
    }
    let mut openings = Vec::new();
    let mut wanted = BTreeSet::new();
    let mut doors = BTreeSet::new();
    for (id, (entity, args)) in &ents {
        match entity.as_str() {
            "IFCOPENINGELEMENT" if !filled.contains(id) => {
                let host = host_of.get(id).and_then(|h| ents.get(h));
                let Some(opening_tag) = tag(args) else {
                    continue;
                };
                let host_tag = host.and_then(|(_, a)| tag(a));
                wanted.insert(opening_tag);
                wanted.extend(host_tag);
                openings.push(Opening {
                    tag: opening_tag,
                    host_class: host.map_or_else(|| "none".to_string(), |(e, _)| e.clone()),
                    host_tag,
                    predefined: predefined(args),
                    name: name(args),
                });
            }
            "IFCFIRESUPPRESSIONTERMINAL" | "IFCCURTAINWALL" => {
                wanted.extend(tag(args));
            }
            "IFCDOOR" | "IFCWINDOW" => {
                doors.extend(tag(args));
            }
            _ => {}
        }
    }
    Export {
        openings,
        wanted,
        doors,
    }
}

fn dump(id: u64, class: &str, objects: &[(&str, &Value)], all: bool) {
    if let Some((_, root)) = objects.first() {
        println!("rec {id} {class} root {}", trimmed(root));
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for &(object, _) in objects {
        *counts.entry(object).or_default() += 1;
    }
    let histogram: Vec<String> = counts
        .iter()
        .map(|(object, n)| format!("{object}:{n}"))
        .collect();
    println!("rec {id} classes {}", histogram.join(" "));
    for &(object, fields) in objects.iter().skip(1).take(MAX_OBJECTS) {
        if object.ends_with("GStep") {
            println!("rec {id} step {object} {}", trimmed(fields));
        } else if all {
            println!("rec {id} obj {object} {}", trimmed(fields));
        }
    }
}

/// The box of the vertices of the solid a symbol's `m_complexCutGeoms` names.
fn cut_box(graph: &ObjectGraph) -> Option<String> {
    let root = graph.objects.first()?;
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for solid in root.fields.get("m_complexCutGeoms")?.as_array()? {
        let Ok(Some(index)) = pointer(graph, 0, solid) else {
            continue;
        };
        let Some(geometry) = graph.objects.get(index) else {
            continue;
        };
        let Some(faces) = geometry
            .fields
            .get("m_pFaces")
            .and_then(rvt::research::graph_value::Fv::as_array)
        else {
            continue;
        };
        for item in faces {
            let Ok(Some(face_index)) = pointer(graph, index, item) else {
                continue;
            };
            if let Ok(Some(mesh)) = face(graph, face_index) {
                for vertex in &mesh.vertices {
                    for axis in 0..3 {
                        lo[axis] = lo[axis].min(vertex[axis]);
                        hi[axis] = hi[axis].max(vertex[axis]);
                    }
                    any = true;
                }
            }
        }
    }
    any.then(|| format!("{lo:?} {hi:?}"))
}

fn probe(path: &str) -> Result<()> {
    let model = PathBuf::from(path);
    let stem = model.with_extension("");
    let export = [
        model.with_extension("ifc"),
        PathBuf::from(format!("{}_slim.ifc", stem.display())),
        PathBuf::from(format!("{}_Revit.ifc", stem.display())),
    ]
    .into_iter()
    .find(|candidate| candidate.exists());
    let Some(export) = export else {
        println!("no Revit export beside the model");
        return Ok(());
    };
    let export = read_export(&std::fs::read_to_string(&export)?);
    for opening in &export.openings {
        println!(
            "open {} {} {} {} {}",
            opening.tag,
            opening.host_class,
            opening.host_tag.map_or("-".to_string(), |t| t.to_string()),
            opening.predefined,
            opening.name
        );
    }
    println!(
        "{} unfilled openings, {} ids wanted, {} doors and windows",
        export.openings.len(),
        export.wanted.len(),
        export.doors.len()
    );

    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let class_name = |tag: u16| {
        classes
            .classes
            .iter()
            .find(|c| c.tag == tag)
            .map_or_else(|| format!("tag {tag}"), |c| c.name.clone())
    };
    if let Some(marker) = per::file_bbox_marker(&mut rf, revit) {
        let mut printed = BTreeSet::new();
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for span in per::partition_record_chain(buf, &marker) {
                let frame =
                    per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker);
                let wanted = export.wanted.contains(&span.element_id)
                    || frame
                        .as_ref()
                        .is_some_and(|f| OPENINGS.contains(&class_name(f.class_tag).as_str()));
                if !wanted || !printed.insert(span.element_id) {
                    continue;
                }
                match frame {
                    Some(f) => println!(
                        "frame {} {} category {} owner {} bbox {:?} refs {:?}",
                        span.element_id,
                        class_name(f.class_tag),
                        f.builtin_category,
                        f.owner_reference.map_or("-".to_string(), |o| o.to_string()),
                        f.bbox_feet,
                        f.references
                    ),
                    None => println!("frame {} undecoded", span.element_id),
                }
            }
        }
    }

    let mut controls = 0usize;
    let mut cutting: BTreeSet<u64> = BTreeSet::new();
    for opening in &export.openings {
        cutting.insert(opening.tag);
    }
    let mut ids: BTreeSet<u64> = export.wanted.clone();
    ids.extend(export.doors.iter().copied());
    let options = Options {
        selected_ids: ids,
        selected_classes: WALLS
            .iter()
            .chain(SLABS.iter())
            .chain(OPENINGS.iter())
            .map(|class| (*class).to_string())
            .collect(),
        ..Options::default()
    };
    let mut symbols: BTreeSet<u64> = BTreeSet::new();
    let mut instances: BTreeSet<u64> = BTreeSet::new();
    let mut rects: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(&mut rf, &options, |record: Record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let as_json: Vec<Value> = graph
            .objects
            .iter()
            .map(|object| object.fields.to_json())
            .collect();
        let objects: Vec<(&str, &Value)> = graph
            .objects
            .iter()
            .zip(&as_json)
            .map(|(object, fields)| (object.class_name.as_str(), fields))
            .collect();
        let structural = WALLS.contains(&class) || SLABS.contains(&class);
        let door = export.doors.contains(&id);
        if class == "FamilyInstance" && (door || export.wanted.contains(&id)) {
            instances.insert(id);
            let symbol = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo")
                .and_then(|object| object.fields.pointer("/m_symbolId/m_id/m_id64"))
                .and_then(rvt::research::graph_value::Fv::as_u64);
            symbols.extend(symbol);
        }
        let all = if structural {
            false
        } else if door {
            if cutting.contains(&id) || export.wanted.contains(&id) {
                true
            } else if controls < CONTROLS {
                controls += 1;
                true
            } else {
                false
            }
        } else {
            true
        };
        for &(object, fields) in &objects {
            if object == "WallRectOpeningGStep" {
                if let Some(cutter) = fields
                    .pointer("/m_instId/m_id/m_id64")
                    .and_then(Value::as_u64)
                {
                    println!(
                        "rectstep {id} {cutter} {}",
                        fields.get("m_bCutSomething").unwrap_or(&Value::Null)
                    );
                    rects.insert(cutter);
                }
            }
        }
        dump(id, class, &objects, all);
        Ok(())
    })?;
    if let Ok(loops) = instance_cut_loops(&mut rf, &instances) {
        for (instance, cut) in &loops {
            println!("loop {instance} {:?} {:?}", cut.min, cut.max);
        }
    }
    let options = Options {
        selected_ids: rects,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record: Record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let class = record.class_name.as_deref().unwrap_or("-");
        let id = record.identity.element_id;
        println!("rectcut {id} {class}");
        for object in &graph.objects {
            if matches!(
                object.class_name.as_str(),
                "RectOpeningData" | "SWallRectOpening" | "ArcWallRectOpening" | "VWallRectOpening"
            ) || object.class_name.ends_with("RectOpening")
            {
                println!(
                    "rectcut {id} obj {} {}",
                    object.class_name,
                    trimmed(&object.fields.to_json())
                );
            }
        }
        Ok(())
    })?;
    let options = Options {
        selected_ids: symbols,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record: Record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if let Some(root) = graph.objects.first() {
            println!("sym {id} root {}", trimmed(&root.fields.to_json()));
        }
        if let Some(found) = cut_box(graph) {
            println!("sym {id} cutbox {found}");
        }
        Ok(())
    })?;
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("error {path}: {error:#}");
        }
    }
}
