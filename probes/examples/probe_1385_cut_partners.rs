//! #1385 (RE-335): where the cut partners of a wall that embeds a curtain wall
//! stand, and what their own records say, so that a byte signal can tell the
//! hosts whose end stubs Revit cuts away from the hosts that keep them.
//!
//! FACT under test: a wall that embeds a curtain wall spanning nearly all of it
//! (`WallEmbedWallGStep`, RE-335) has its end stubs, the stretches the curtain
//! wall does not span, cut away by the elements it has `CutPairGStep` partners
//! with where those stubs lie inside the partners' bodies (RE-335 section 3:
//! 2098938 and the columns 1927726 and 1927727), and keeps them where they do
//! not (2103576 has no partner). RE-340 found the partners' boxes cover the
//! stubs of every host Revit raises, and also of 2177158 and, at building
//! permit, 2041479, which keep theirs: this prints what separates them.
//!
//! `probe_1067_native_dump` prints each wall's native steps and relations, and
//! each partner's native root. This prints what it does not:
//!
//!   frame <id> <category> <stream> <offset> <min x> <min y> <min z> <max x> <max y> <max z> <placement kind> <container> <class tag> <references>
//!
//! one line for each element-record frame of the walls, columns, structural
//! columns and structural framing of the file, in feet, full precision, with
//! the frame's counted reference list comma separated; and, for every element a
//! wall that names a `WallEmbedWallGStep` has a `CutPairGStep` or a
//! `CleanPairGStep` with:
//!
//!   partner <id> <class> classes <class:count ...>
//!   partner <id> <class> root <fields json>
//!   partner <id> <class> step <class> <fields json>
//!
//! the partner's record objects: every step whose class ends in `GStep`, and
//! the objects of the classes in `ATTACHMENTS`. A history table is printed as
//! its length under `<name>#`, and a member that is a pointer is left out.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1385_cut_partners -- MODEL.rvt

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::partition_element_records as per;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const WALLS: [&str; 2] = ["SWall", "ArcWall"];
const ATTACHMENTS: [&str; 5] = [
    "ColumnAttachment",
    "AttachmentData",
    "StructAttachInfo",
    "StructAttachRef",
    "CopyGeomToInstGStep",
];
const CLIP: usize = 8000;

fn clip(text: String) -> String {
    if text.len() <= CLIP {
        text
    } else {
        text.chars().take(CLIP).collect()
    }
}

/// Every positive `m_id64` under `value`.
fn ids_under(value: &Value, out: &mut BTreeSet<u64>) {
    match value {
        Value::Object(members) => {
            for (name, member) in members {
                match member.as_u64() {
                    Some(id) if name == "m_id64" && id > 0 => {
                        out.insert(id);
                    }
                    _ => ids_under(member, out),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                ids_under(item, out);
            }
        }
        _ => {}
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

/// The partners of the walls that embed a wall: the elements their
/// `CutPairGStep`s and `CleanPairGStep`s name.
fn partners_of(record: Record, partners: &mut BTreeSet<u64>) -> Result<()> {
    let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref()) else {
        return Ok(());
    };
    if !WALLS.contains(&class) {
        return Ok(());
    }
    if !graph
        .objects
        .iter()
        .any(|object| object.class_name == "WallEmbedWallGStep")
    {
        return Ok(());
    }
    for object in &graph.objects {
        if object.class_name == "CutPairGStep" || object.class_name == "CleanPairGStep" {
            if let Some(other) = object.fields.get("m_otherId") {
                ids_under(other, partners);
            }
        }
    }
    Ok(())
}

fn print_partner(record: Record) -> Result<()> {
    let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref()) else {
        return Ok(());
    };
    let id = record.identity.element_id;
    if let Some(root) = graph.objects.first() {
        println!("partner {id} {class} root {}", trimmed(&root.fields));
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for object in &graph.objects {
        *counts.entry(object.class_name.as_str()).or_default() += 1;
    }
    let histogram: Vec<String> = counts
        .iter()
        .map(|(name, n)| format!("{name}:{n}"))
        .collect();
    println!("partner {id} {class} classes {}", histogram.join(" "));
    for object in graph.objects.iter().skip(1) {
        let name = object.class_name.as_str();
        if name.ends_with("GStep") || ATTACHMENTS.contains(&name) {
            println!(
                "partner {id} {class} step {name} {}",
                trimmed(&object.fields)
            );
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1385_cut_partners MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    let table = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&table);
    let categories = [
        per::OST_WALLS,
        per::OST_COLUMNS,
        per::OST_STRUCTURAL_COLUMNS,
        per::OST_STRUCTURAL_FRAMING,
    ];
    let records = per::scan_category_records_multi(&mut rf, release, &categories, &declared)?;
    println!("release {release} frames {}", records.len());
    for record in &records {
        let b = record.bbox_feet;
        let references: Vec<String> = record.references.iter().map(u64::to_string).collect();
        println!(
            "frame {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            record.element_id,
            record.builtin_category,
            record.stream,
            record.offset,
            b[0],
            b[1],
            b[2],
            b[3],
            b[4],
            b[5],
            record.placement_kind,
            record.container,
            record.class_tag,
            references.join(",")
        );
    }

    let mut partners: BTreeSet<u64> = BTreeSet::new();
    let walls = Options {
        selected_classes: WALLS.iter().map(|name| (*name).to_string()).collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &walls, |record| partners_of(record, &mut partners))?;
    if !partners.is_empty() {
        let selected = Options {
            selected_ids: partners,
            ..Options::default()
        };
        extract_graphs(&mut rf, &selected, print_partner)?;
    }
    Ok(())
}
