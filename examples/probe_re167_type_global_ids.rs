//! RE-167 (probe): whose GlobalId Revit's exporter gives a type object
//! (B60).
//!
//! revit-ifc writes a family instance's type with the GlobalId of
//! `ExporterIFCUtils.GetOriginalSymbol(instance)` and the `Tag` of the
//! instance's symbol. Where the two differ, the type's GlobalId is not its
//! Tag's. For each type object of Revit's export beside the model, the probe
//! finds the element of the file whose own GlobalId (RE-48) it is, that
//! element's schema class, and where each of the type's instances holds that
//! element's id in its own data object (RE-153): the offsets from the
//! object's start.
//!
//! Usage: probe_re167_type_global_ids <model.rvt>

use rvt::RevitFile;
use rvt::partition_room_parameters::data_objects;
use rvt::revit_global_ids::revit_global_ids;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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

fn first_quoted(args: &str) -> Option<String> {
    Some(args.split('\'').nth(1)?.to_string())
}

fn quoted_numbers(args: &str) -> Vec<u32> {
    args.split('\'')
        .skip(1)
        .step_by(2)
        .filter_map(|s| s.parse().ok())
        .collect()
}

struct Row {
    entity: String,
    tag: Option<u32>,
    gid: String,
    elements: Vec<u32>,
    origin: Option<u32>,
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export");
        return Ok(());
    }
    let ents = entities(&std::fs::read_to_string(&reference)?);
    let mut rf = RevitFile::open(&path)?;
    let ids = revit_global_ids(&mut rf)?;
    let owner: BTreeMap<&str, u32> = ids.iter().map(|(id, gid)| (gid.as_str(), *id)).collect();
    let classes = rf.schema_classes()?;
    let class_name = |class: u32| {
        classes
            .classes
            .iter()
            .find(|c| u32::from(c.tag) == class & 0xffff)
            .map_or_else(|| format!("{class:#x}"), |c| c.name.clone())
    };
    let mut rows = Vec::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let all = refs(args);
        let Some((&ty, related)) = all.split_last() else {
            continue;
        };
        let Some((ty_entity, ty_args)) = ents.get(&ty) else {
            continue;
        };
        let Some(gid) = first_quoted(ty_args) else {
            continue;
        };
        let elements: Vec<u32> = related
            .iter()
            .skip(1)
            .filter_map(|e| ents.get(e))
            .filter_map(|(_, a)| quoted_numbers(a).last().copied())
            .collect();
        rows.push(Row {
            entity: ty_entity.clone(),
            tag: quoted_numbers(ty_args).last().copied(),
            origin: owner.get(gid.as_str()).copied(),
            gid,
            elements,
        });
    }
    let wanted: BTreeSet<u32> = rows
        .iter()
        .flat_map(|r| r.elements.iter().copied().chain(r.origin))
        .collect();
    let mut objects: BTreeMap<u32, Vec<(String, Vec<u8>)>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if wanted.contains(&object.element_id) {
                objects
                    .entry(object.element_id)
                    .or_default()
                    .push((class_name(object.class), buf[p..object.end].to_vec()));
            }
        }
    }
    let mut summary: BTreeMap<String, usize> = BTreeMap::new();
    for row in &rows {
        let relation = match (row.origin, row.tag) {
            (None, _) => "no element of the file has it".to_string(),
            (Some(o), Some(t)) if o == t => "the Tag's own".to_string(),
            (Some(o), _) if row.elements.contains(&o) => "the instance's own".to_string(),
            (Some(o), _) => {
                let class: BTreeSet<&str> = objects
                    .get(&o)
                    .map(|v| v.iter().map(|(c, _)| c.as_str()).collect())
                    .unwrap_or_default();
                format!("another element's ({o}, objects {class:?})")
            }
        };
        let kind = relation.split(" (").next().unwrap_or("").to_string();
        *summary
            .entry(format!("{}: {kind}", row.entity))
            .or_default() += 1;
        let mut held = Vec::new();
        if let Some(origin) = row.origin.filter(|o| Some(*o) != row.tag) {
            let needle = origin.to_le_bytes();
            for element in &row.elements {
                for (class, bytes) in objects.get(element).into_iter().flatten() {
                    let at: Vec<usize> = memchr::memmem::find_iter(bytes, &needle).collect();
                    held.push(format!("{element} {class} size {} at {at:?}", bytes.len()));
                }
            }
        }
        println!(
            "{} tag {:?} {} elements {:?}: {relation}; held {held:?}",
            row.entity, row.tag, row.gid, row.elements
        );
    }
    println!("summary: {summary:?}");
    Ok(())
}
