//! RE-327 (#156, Discussion #112), discovery run 2: where does a family's
//! own document keep its extrusions, their sketches, their sketch planes and
//! their start and end offsets, and does the file hold a saved display mesh
//! of each form to measure a rebuilt solid against?
//!
//! FACT under test: Discussion #112 reports a chain from sketch curve
//! records to closed loops to extrusion parameters to solids, 85 of 86
//! extrusions of one 2014 family (STE1200's team). `native_document` refuses
//! every family file ("invalid increment episode range", run 41776509790),
//! so the records are read by `native_form_geometry` from the family file's
//! own plain groups, decoded by the file's schema, and related by the
//! ElemTable owner and by the ElementId fields the schema names
//! (`SketchBase::m_userId`, `Sketch::m_sketchPlaneId`,
//! `CurveElem::m_sketchPlaneId`).
//!
//! This prints, for one family file:
//!   file <path> release <layout release>
//!   native: <error>                                 where the framing refuses the release
//!   shape <class> <tag, parent, fields>             the schema of the classes the chain uses
//!   walked <channel> groups .. keyed .. unframed .. records ..
//!   census <channel> <class> records <n> refused <m>
//!   refusal <class> <n> <diagnostic>                why graphs did not decode
//!   copies <n> ids in several partitions
//!   ext <id> <class> owner <id/class> cutting <b> material <id> kids <class:n,..> params <..>
//!   tree <depth> <id> <class> owner <id>            the ownership subtree of the first forms
//!   rec / obj <id> <index> <class> <fields as JSON> the graphs of those forms, their children
//!                                                   and the records they reference
//!   saved <id> <class> ...                          the form's channel-103 saved graphics
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re327_family_extrusions -- FAMILY.rfa

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::native_form_geometry;
use rvt::research::native_parameters::{self, GraphLimits, ObjectGraph};
use rvt::research::native_saved_mesh::{self, Primitive};
use rvt::research::schema_registry::{Class, Field, Registry};
use serde_json::Value;

const FORMS: [&str; 5] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
];
const CORE: [&str; 9] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "VarSketch",
    "SketchPlane",
    "CurveElem",
    "ParamValueSetDouble",
    "ParamValueDouble",
];
const KEYWORDS: [&str; 7] = [
    "Sketch",
    "Plane",
    "Curve",
    "Extrusion",
    "Form",
    "Line",
    "Arc",
];
const FULL_DUMPS: usize = 2;
const EXTRA_DUMPS: usize = 4;
const OBJECTS_PER_RECORD: usize = 16;
const JSON_CAP: usize = 1600;
const TREE_LINES: usize = 90;
const LIMITS: GraphLimits = GraphLimits {
    max_values: 2_000_000,
    max_objects: 2_000_000,
    max_depth: 128,
};

struct Kept {
    class: String,
    stream: String,
    owner: Option<u64>,
    graph: Option<ObjectGraph>,
}

fn cap(text: String, limit: usize) -> String {
    if text.chars().count() <= limit {
        text
    } else {
        text.chars().take(limit).collect::<String>() + "..."
    }
}

fn ident(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    if let Some(n) = inner.as_i64() {
        return Some(n);
    }
    inner
        .get("m_id64")
        .or_else(|| inner.get("m_id"))
        .and_then(Value::as_i64)
}

fn collect_ids(value: &Value, out: &mut BTreeSet<u64>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = ident(value)
                .and_then(|n| u64::try_from(n).ok())
                .filter(|n| *n > 0)
            {
                out.insert(id);
            }
            for inner in map.values() {
                collect_ids(inner, out);
            }
        }
        Value::Array(items) => {
            for inner in items {
                collect_ids(inner, out);
            }
        }
        _ => {}
    }
}

fn describe_field(registry: &Registry, field: &Field) -> String {
    let mut text = format!(
        "{}:{}.{}.{}",
        field.name, field.base, field.modifier, field.uninterpreted_flags
    );
    for reference in &field.references {
        text.push('>');
        text.push_str(
            registry
                .class(reference.tag)
                .map_or("?", |c| c.name.as_str()),
        );
    }
    text
}

fn describe_class(registry: &Registry, class: &Class) -> String {
    let parent = if class.parent_reference.tag >= 12 {
        registry
            .class(class.parent_reference.tag)
            .map_or("?", |c| c.name.as_str())
    } else {
        "none"
    };
    let fields: Vec<String> = class
        .fields
        .iter()
        .map(|f| describe_field(registry, f))
        .collect();
    format!(
        "tag={} parent={parent} fields={}: {}",
        class.tag,
        class.fields.len(),
        fields.join(",")
    )
}

fn lineage<'a>(registry: &'a Registry, name: &str) -> Vec<&'a Class> {
    let mut out: Vec<&Class> = Vec::new();
    let mut next = registry.named(name);
    while let Some(class) = next {
        out.push(class);
        if out.len() > 16 || class.parent_reference.tag < 12 {
            break;
        }
        next = registry.class(class.parent_reference.tag);
    }
    out
}

fn print_shape(registry: &Registry, printed: &mut BTreeSet<String>, name: &str) {
    for class in lineage(registry, name) {
        if printed.insert(class.name.clone()) {
            println!("shape {} {}", class.name, describe_class(registry, class));
        }
    }
}

fn param_sets(graph: &ObjectGraph) -> String {
    let mut parts: Vec<String> = Vec::new();
    for object in &graph.objects {
        if !object.class_name.starts_with("ParamValueSet") {
            continue;
        }
        let entries = object.fields.get("m_paramSet").and_then(Value::as_array);
        let Some(entries) = entries.filter(|e| !e.is_empty()) else {
            parts.push(format!(
                "{}:raw={}",
                object.class_name,
                cap(object.fields.to_string(), 200)
            ));
            continue;
        };
        for entry in entries {
            let id = entry.get("m_paramId").and_then(ident);
            let value = entry
                .get("m_value")
                .map_or_else(|| cap(entry.to_string(), 120), Value::to_string);
            parts.push(format!(
                "{}:{}={}",
                object.class_name.trim_start_matches("ParamValueSet"),
                id.map_or_else(|| "?".to_string(), |n| n.to_string()),
                cap(value, 60)
            ));
        }
    }
    parts.join(";")
}

fn dump(label: &str, id: u64, kept: &Kept) {
    let objects = kept.graph.as_ref().map_or(0, |g| g.objects.len());
    println!(
        "rec {label} {id} {} owner {} objects {objects}",
        kept.class,
        kept.owner
            .map_or_else(|| "-".to_string(), |o| o.to_string())
    );
    let Some(graph) = kept.graph.as_ref() else {
        return;
    };
    for (index, object) in graph.objects.iter().enumerate().take(OBJECTS_PER_RECORD) {
        println!(
            "obj {id} {index} {} {}",
            object.class_name,
            cap(object.fields.to_string(), JSON_CAP)
        );
    }
}

fn print_tree(
    kept: &BTreeMap<u64, Kept>,
    children: &BTreeMap<u64, Vec<u64>>,
    id: u64,
    depth: usize,
    budget: &mut usize,
) {
    if *budget == 0 || depth > 6 {
        return;
    }
    *budget -= 1;
    let Some(record) = kept.get(&id) else {
        return;
    };
    println!(
        "tree {depth} {id} {} owner {}",
        record.class,
        record
            .owner
            .map_or_else(|| "-".to_string(), |o| o.to_string())
    );
    if let Some(kids) = children.get(&id) {
        for kid in kids {
            print_tree(kept, children, *kid, depth + 1, budget);
        }
    }
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
            total += (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0;
        }
    }
    total
}

fn mesh_box(primitives: &[Primitive]) -> Option<([f64; 3], [f64; 3])> {
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for primitive in primitives {
        for vertex in &primitive.vertices {
            any = true;
            for (axis, value) in vertex.iter().enumerate() {
                low[axis] = low[axis].min(*value);
                high[axis] = high[axis].max(*value);
            }
        }
    }
    any.then_some((low, high))
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re327_family_extrusions FAMILY.rfa"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");
    let document = match native_form_geometry::open(&mut rf) {
        Ok(document) => document,
        Err(error) => {
            println!("native: {error:#}");
            return Ok(());
        }
    };
    let registry = &document.registry;

    let mut printed: BTreeSet<String> = BTreeSet::new();
    for name in CORE {
        print_shape(registry, &mut printed, name);
        if registry.named(name).is_none() {
            println!("shape {name} absent");
        }
    }

    let mut owner_of: BTreeMap<u64, u64> = BTreeMap::new();
    match elem_table::parse_records(&mut rf) {
        Ok(records) => {
            for record in &records {
                if let Some(owner) = record.owner_id {
                    owner_of.insert(u64::from(record.id_secondary), u64::from(owner));
                }
            }
            println!(
                "elemtable records {} with-owner {}",
                records.len(),
                owner_of.len()
            );
        }
        Err(error) => println!("elemtable: {error:#}"),
    }

    let mut kept: BTreeMap<u64, Kept> = BTreeMap::new();
    let mut census: BTreeMap<(u64, String), (usize, usize)> = BTreeMap::new();
    let mut refusals: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut copies: BTreeMap<u64, usize> = BTreeMap::new();
    let walked = native_form_geometry::walk(&mut rf, &document, &[101, 102, 103], |entry| {
        let class = entry.class.unwrap_or("-").to_string();
        let counts = census.entry((entry.channel, class.clone())).or_default();
        counts.0 += 1;
        if entry.channel != 102 {
            return Ok(());
        }
        *copies.entry(entry.id).or_default() += 1;
        let graph = match native_parameters::decode_graph_with_limits(entry.body, registry, &LIMITS)
        {
            Ok(graph) => Some(graph),
            Err(error) => {
                counts.1 += 1;
                let digits: String = format!("{error:#}")
                    .chars()
                    .map(|c| if c.is_ascii_digit() { '#' } else { c })
                    .collect();
                *refusals
                    .entry((class.clone(), cap(digits, 160)))
                    .or_default() += 1;
                None
            }
        };
        kept.insert(
            entry.id,
            Kept {
                class,
                stream: entry.stream.to_string(),
                owner: owner_of.get(&entry.id).copied(),
                graph,
            },
        );
        Ok(())
    });
    match walked {
        Ok(walked) => println!(
            "walked groups {} keyed {} unframed {} records {} kept {}",
            walked.groups,
            walked.keyed_groups,
            walked.unframed_groups,
            walked.records,
            kept.len()
        ),
        Err(error) => {
            println!("native: {error:#}");
            return Ok(());
        }
    }
    for ((channel, class), (records, refused)) in &census {
        println!("census {channel} {class} records {records} refused {refused}");
    }
    for ((class, diagnostic), count) in &refusals {
        println!("refusal {class} {count} {diagnostic}");
    }
    println!("copies {}", copies.values().filter(|n| **n > 1).count());
    for class in &registry.classes {
        if census.keys().any(|(_, c)| *c == class.name)
            && KEYWORDS.iter().any(|k| class.name.contains(k))
        {
            print_shape(registry, &mut printed, &class.name);
        }
    }

    let mut children: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for (id, record) in &kept {
        if let Some(owner) = record.owner {
            children.entry(owner).or_default().push(*id);
        }
    }
    let forms: Vec<u64> = kept
        .iter()
        .filter(|(_, k)| FORMS.contains(&k.class.as_str()))
        .map(|(id, _)| *id)
        .collect();
    for id in &forms {
        let record = &kept[id];
        let owner_class = record
            .owner
            .and_then(|o| kept.get(&o))
            .map_or("-", |k| k.class.as_str());
        let mut kids: BTreeMap<&str, usize> = BTreeMap::new();
        for kid in children.get(id).into_iter().flatten() {
            if let Some(k) = kept.get(kid) {
                *kids.entry(k.class.as_str()).or_default() += 1;
            }
        }
        let kids_text: Vec<String> = kids.iter().map(|(c, n)| format!("{c}:{n}")).collect();
        let root = record.graph.as_ref().and_then(|g| g.objects.first());
        println!(
            "ext {id} {} stream {} owner {}/{owner_class} cutting {} material {} kids {} params {}",
            record.class,
            record.stream,
            record
                .owner
                .map_or_else(|| "-".to_string(), |o| o.to_string()),
            root.and_then(|o| o.fields.get("m_cutting"))
                .map_or_else(|| "?".to_string(), Value::to_string),
            root.and_then(|o| o.fields.get("m_materialId"))
                .and_then(ident)
                .map_or_else(|| "?".to_string(), |n| n.to_string()),
            kids_text.join(","),
            record.graph.as_ref().map_or_else(String::new, param_sets),
        );
    }

    let mut dumped: BTreeSet<u64> = BTreeSet::new();
    for id in forms.iter().take(FULL_DUMPS) {
        let mut budget = TREE_LINES;
        print_tree(&kept, &children, *id, 0, &mut budget);
        let mut wanted: BTreeSet<u64> = BTreeSet::from([*id]);
        wanted.extend(children.get(id).into_iter().flatten().copied());
        let mut second: BTreeSet<u64> = BTreeSet::new();
        for member in &wanted {
            second.extend(children.get(member).into_iter().flatten().copied());
        }
        wanted.extend(second.into_iter().take(12));
        let mut referenced: BTreeSet<u64> = BTreeSet::new();
        for member in &wanted {
            if let Some(graph) = kept.get(member).and_then(|k| k.graph.as_ref()) {
                for object in &graph.objects {
                    collect_ids(&object.fields, &mut referenced);
                }
            }
        }
        referenced.retain(|r| kept.contains_key(r) && !wanted.contains(r));
        let listing: Vec<String> = referenced
            .iter()
            .take(40)
            .map(|r| format!("{r}/{}", kept[r].class))
            .collect();
        println!("refs {id} {}", listing.join(","));
        for member in &wanted {
            if dumped.insert(*member) {
                dump("member", *member, &kept[member]);
            }
        }
        for r in &referenced {
            let class = kept[r].class.as_str();
            if KEYWORDS.iter().any(|k| class.contains(k)) && dumped.insert(*r) {
                dump("ref", *r, &kept[r]);
            }
        }
    }
    for wanted in ["SketchPlane", "CurveElem", "VarSketch"] {
        let mut shown = 0usize;
        for (id, record) in &kept {
            if shown >= EXTRA_DUMPS {
                break;
            }
            if record.class == wanted && dumped.insert(*id) {
                dump("extra", *id, record);
                shown += 1;
            }
        }
    }
    let mut seen_classes: BTreeSet<String> = BTreeSet::new();
    for id in &dumped {
        if let Some(graph) = kept.get(id).and_then(|k| k.graph.as_ref()) {
            for object in graph.objects.iter().take(OBJECTS_PER_RECORD) {
                seen_classes.insert(object.class_name.clone());
            }
        }
    }
    for class in seen_classes.iter().take(80) {
        print_shape(registry, &mut printed, class);
    }

    if forms.is_empty() {
        println!("saved none: no form record");
        return Ok(());
    }
    let wanted_forms: BTreeSet<u64> = forms.iter().copied().collect();
    let mut with_graphics = 0usize;
    let saved = native_form_geometry::walk(&mut rf, &document, &[103], |entry| {
        if !wanted_forms.contains(&entry.id) {
            return Ok(());
        }
        let class = entry.class.unwrap_or("-");
        let graph = match native_parameters::decode_graph_with_limits(
            entry.body,
            registry,
            &GraphLimits {
                max_values: 20_000_000,
                max_objects: 20_000_000,
                max_depth: 128,
            },
        ) {
            Ok(graph) => graph,
            Err(error) => {
                println!(
                    "saved {} {class} refused {}",
                    entry.id,
                    cap(format!("{error:#}"), 160)
                );
                return Ok(());
            }
        };
        with_graphics += 1;
        let meshes = native_saved_mesh::graphics(&graph);
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        let vertices: usize = meshes.primitives.iter().map(|p| p.vertices.len()).sum();
        let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
        for object in &graph.objects {
            *classes.entry(object.class_name.as_str()).or_default() += 1;
        }
        let top: Vec<String> = classes
            .iter()
            .take(10)
            .map(|(c, n)| format!("{c}:{n}"))
            .collect();
        let bounds = mesh_box(&meshes.primitives)
            .map_or_else(|| "-".to_string(), |(lo, hi)| format!("{lo:?}..{hi:?}"));
        println!(
            "saved {} {class} objects {} primitives {} triangles {triangles} vertices {vertices} unbounded {} diagnostics {} volume {:.6} box {bounds} classes {}",
            entry.id,
            graph.objects.len(),
            meshes.primitives.len(),
            meshes.unbounded_faces,
            meshes.diagnostics.len(),
            mesh_volume(&meshes.primitives),
            top.join(","),
        );
        for message in meshes.diagnostics.iter().take(2) {
            println!(
                "saved-diagnostic {} {}",
                entry.id,
                cap(message.clone(), 200)
            );
        }
        Ok(())
    });
    match saved {
        Ok(walked) => println!(
            "saved-summary forms {} with-graphics {with_graphics} groups {} unframed {}",
            forms.len(),
            walked.groups,
            walked.unframed_groups
        ),
        Err(error) => println!("saved-error {error:#}"),
    }
    Ok(())
}
