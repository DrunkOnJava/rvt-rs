//! RE-431 (#1193, W7): what names a railing's type on the BIM4LCA housing.
//!
//! FACT under test: on Revit 2024 and later a railing whose reference list
//! names no type definition takes the one ElementId among the words of its
//! `BaseRailing` data object that is the id of a `StairsRailingAttr` object
//! (B45, RE-153, `railing_type_picks`). `probe_re231_record_lengths` (Measure
//! run 43044928311, on `main` `8ec2ac98`) found no type on any of the 56
//! railings of the timber housing as built or the 43 of the concrete one, and
//! one on each of the office's 41 and the terrain housing files' 2.
//! Hypotheses: a housing railing has several `BaseRailing` copies and the
//! older one names a type the railing no longer has, so its words hold two
//! attribute ids; or the railing's type is an object of another class.
//! Revit's housing export has no `IfcRailingType` (the type is the second
//! part of each railing's Name), so the oracle for a type is its name.
//!
//! Output, one line per `BaseRailing` data object (a copy) of each railing
//! the export writes, in the format `tools/re/railing_types_vs_ifc.py` reads:
//!   copy <tag> <partition> <current 1|0> <bytes> <attrs|-> <word>:<class> ...
//! `current` says the copy is in the partition its element's stored revision
//! routes to (RE-216). The words are every 4-byte word at any offset of the
//! payload, from 1000 up, that is the ElementId of a data object of the
//! file, with the classes of the objects that hold that id (joined by `/`);
//! `attrs` are the ones among them of `StairsRailingAttr` objects, each as
//! `<id>=<name>` with the name its object holds (`?` for none; spaces as
//! `~`; the names of copies that disagree joined by `|`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re431_railing_types -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::current_partitions;
use rvt::research::partition_names::{RailingTypeTags, railing_type_name_in};
use rvt::research::partition_room_parameters::{DATA_OBJECT_HEADER, data_objects};

/// Below this an ElementId is one of the file's own internal elements.
const FIRST_WORD: u32 = 1000;

struct Held {
    id: u32,
    partition: u32,
    bytes: usize,
    words: BTreeSet<u32>,
}

fn partition_of(stream: &str) -> u32 {
    stream
        .strip_prefix("Partitions/")
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let railings: BTreeSet<u32> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCRAILING") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let schema = rf.schema_classes()?;
    let class_names: BTreeMap<u32, &str> = schema
        .classes
        .iter()
        .map(|class| (u32::from(class.tag), class.name.as_str()))
        .collect();
    let tag_of = |name: &str| {
        schema
            .classes
            .iter()
            .find(|class| class.name == name)
            .map(|class| u32::from(class.tag))
    };
    let (Some(railing_tag), Some(attr_tag)) = (tag_of("BaseRailing"), tag_of("StairsRailingAttr"))
    else {
        anyhow::bail!("the schema has no BaseRailing or no StairsRailingAttr class");
    };

    let mut copies: Vec<Held> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if object.class & 0xffff != railing_tag || !railings.contains(&object.element_id) {
                continue;
            }
            let payload = &buf[p + DATA_OBJECT_HEADER..object.end];
            let words = payload
                .windows(4)
                .map(|word| u32::from_le_bytes(word.try_into().expect("4 bytes")))
                .filter(|word| *word >= FIRST_WORD && *word != object.element_id)
                .collect();
            copies.push(Held {
                id: object.element_id,
                partition: partition_of(&stream),
                bytes: object.end - p,
                words,
            });
        }
    }

    let wanted: BTreeSet<u32> = copies
        .iter()
        .flat_map(|copy| copy.words.iter().copied())
        .collect();
    let name_tags = RailingTypeTags::from_classes(&schema);
    let mut classes_of: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut attr_names: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut attr_objects = 0usize;
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            let class = object.class & 0xffff;
            if class == attr_tag {
                attr_objects += 1;
                if wanted.contains(&object.element_id) {
                    let name = name_tags
                        .and_then(|tags| railing_type_name_in(&buf[p..object.end], tags))
                        .map_or_else(|| "?".to_string(), |name| name.replace(' ', "~"));
                    attr_names
                        .entry(object.element_id)
                        .or_default()
                        .insert(name);
                }
            }
            if wanted.contains(&object.element_id) {
                classes_of
                    .entry(object.element_id)
                    .or_default()
                    .insert(class);
            }
        }
    }

    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for copy in &copies {
        holders.entry(copy.id).or_default().insert(copy.partition);
    }
    let current = current_partitions(&mut rf, &holders);

    for copy in &copies {
        let is_current = current.get(&copy.id) == Some(&copy.partition);
        let attrs: Vec<String> = copy
            .words
            .iter()
            .filter(|word| {
                classes_of
                    .get(*word)
                    .is_some_and(|set| set.contains(&attr_tag))
            })
            .map(|word| {
                let names = attr_names.get(word).map_or_else(
                    || "?".to_string(),
                    |names| names.iter().cloned().collect::<Vec<_>>().join("|"),
                );
                format!("{word}={names}")
            })
            .collect();
        let listed: Vec<String> = copy
            .words
            .iter()
            .filter_map(|word| {
                let set = classes_of.get(word)?;
                let names: Vec<&str> = set
                    .iter()
                    .map(|class| class_names.get(class).copied().unwrap_or("?"))
                    .collect();
                Some(format!("{word}:{}", names.join("/")))
            })
            .collect();
        println!(
            "copy {} {} {} {} {} {}",
            copy.id,
            copy.partition,
            u8::from(is_current),
            copy.bytes,
            if attrs.is_empty() {
                "-".to_string()
            } else {
                attrs.join(",")
            },
            listed.join(" ")
        );
    }
    eprintln!(
        "{} railings, {} BaseRailing copies, {} StairsRailingAttr objects",
        railings.len(),
        copies.len(),
        attr_objects
    );
    Ok(())
}
