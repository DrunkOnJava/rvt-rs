//! #1165: which sketch line of a floor's void Revit's export tags its opening
//! with.
//!
//! FACT under test (RE-151): the `Tag` of the `IfcOpeningElement` of a hole in
//! a floor's sketch is the lowest ElementId of the sketch lines on the hole.
//! On the BIM4LCA office 9 of the 36 holes (8 of 35 at building permit) are
//! tagged with another line of the hole, always one with a higher id than the
//! lowest. This prints every sketch line the hosts of the export's unfilled
//! openings own, with its joins to the lines before and after it and every
//! object of its record, so that the line Revit picks can be compared with the
//! loop it closes.
//!
//! Reads Revit's export beside the model (`<model>.ifc`, `<model>_slim.ifc` or
//! `<model>_Revit.ifc`). Output, one record per line:
//!   open <tag> <host class> <host tag> <predefined type> <name>
//!   line <id> owner <host> bbox <six feet> refs <ids>
//!   rec <id> obj <class> <fields json>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1165_void_tags -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::partition_element_records::{self as per};
use serde_json::{Map, Value};

const SKETCH_LINES: i64 = -2_000_045;
const CLIP: usize = 6000;

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

struct Opening {
    tag: u64,
    host_class: String,
    host_tag: Option<u64>,
    name: String,
}

/// The openings of a STEP file that nothing fills.
fn read_export(step: &str) -> Vec<Opening> {
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
    for (id, (entity, args)) in &ents {
        if entity == "IFCOPENINGELEMENT" && !filled.contains(id) {
            let host = host_of.get(id).and_then(|h| ents.get(h));
            let Some(opening_tag) = tag(args) else {
                continue;
            };
            openings.push(Opening {
                tag: opening_tag,
                host_class: host.map_or_else(|| "none".to_string(), |(e, _)| e.clone()),
                host_tag: host.and_then(|(_, a)| tag(a)),
                name: name(args),
            });
        }
    }
    openings
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
    let openings = read_export(&std::fs::read_to_string(&export)?);
    for opening in &openings {
        println!(
            "open {} {} {} {}",
            opening.tag,
            opening.host_class,
            opening.host_tag.map_or("-".to_string(), |t| t.to_string()),
            opening.name
        );
    }
    let hosts: BTreeSet<u64> = openings
        .iter()
        .filter(|opening| opening.host_class != "IFCWALL")
        .filter_map(|opening| opening.host_tag)
        .collect();

    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let mut lines: BTreeSet<u64> = BTreeSet::new();
    if let Some(marker) = per::file_bbox_marker(&mut rf, revit) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for span in per::partition_record_chain(buf, &marker) {
                let Some(frame) =
                    per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker)
                else {
                    continue;
                };
                let owned = frame
                    .owner_reference
                    .is_some_and(|owner| hosts.contains(&u64::from(owner)));
                if frame.builtin_category != SKETCH_LINES
                    || !owned
                    || !lines.insert(span.element_id)
                {
                    continue;
                }
                println!(
                    "line {} owner {} bbox {:?} refs {:?}",
                    span.element_id,
                    frame
                        .owner_reference
                        .map_or("-".to_string(), |o| o.to_string()),
                    frame.bbox_feet,
                    frame.references
                );
            }
        }
    }
    let options = Options {
        selected_ids: lines,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record: Record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for object in &graph.objects {
            if !object.class_name.ends_with("GStep") {
                println!(
                    "rec {id} obj {} {}",
                    object.class_name,
                    trimmed(&object.fields)
                );
            }
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
