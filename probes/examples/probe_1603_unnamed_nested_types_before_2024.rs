//! #1603, RE-487: does the number of geometry tags in a Revit 2015 to 2023 door
//! or window's type map, or the map of the types its family's nested instances
//! draw, say which families Revit writes `<Unnamed>` for?
//!
//! FACT to check: RE-485 found `<Unnamed>` all-or-none within every family
//! before 2024 (3 of the spent Revit 2018 project's 7 window families write it
//! on all their windows, 4 on none), and the type map's total tags follow the
//! families there (33 tags: 8 of 8 written; 21: 44 of 45; 13: 0 of 15). Whether
//! that holds on the other projects, and whether the nested instances' own
//! types hold geometry mapped to no material, is not in those counts.
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_unnamed_nested_types_vs_ifc.py`
//! reads:
//!   wnest <tag> <IFC class> map_unset=<0|1|na> found=<0|1> symbol=<id> \
//!        family=<id> tags=<n> unset_tags=<n> nested=<n> nested_read=<n> \
//!        nested_unset=<n>
//! where `map_unset` says whether the drawn symbol's `m_geomTag2MaterialId`
//! maps some geometry to no material (-1), `found` whether its family's
//! document was read, `symbol` and `family` are the ElementIds of the drawn
//! symbol and its family (0 where not read), `tags` the number of geometry tags
//! the symbol's map holds and `unset_tags` how many map to no material; and, of
//! the family's nested family instances, `nested` how many it has,
//! `nested_read` how many draw a type of the family's document whose map was
//! read and `nested_unset` how many of those maps hold a -1.
//!
//! Each record of a family's document is read once, from its last copy. Two
//! more lines per model count what the probe saw, in the wording the held-out
//! scrub keeps:
//!   nested types read: release R families F nested instances N map read M map with an unset tag U
//!   family record copies: release R records N held more than once C copies differing D
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_unnamed_nested_types_before_2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_family_forms::family_records_in;
use rvt::research::native_parameters::ObjectGraph;

const CLASSES: [&str; 2] = ["IFCWINDOW", "IFCDOOR"];

/// The records of a family's document the probe reads: its nested family
/// instances and its types.
const RECORD_CLASSES: [&str; 2] = ["FamilyInstance", "FamilySymbol"];

/// The ElementId in `value`, in either shape: `{"m_id": {"m_id64": n}}` from
/// Revit 2024, `{"m_id": {"m_id": n}}` before it (RE-280).
fn id_in_either(value: Option<&Fv>) -> Option<i64> {
    value.and_then(|v| {
        v.pointer("/m_id/m_id64")
            .or_else(|| v.pointer("/m_id/m_id"))
            .and_then(Fv::as_i64)
    })
}

/// The number of geometry tags a type's map holds and how many map to no
/// material (-1), from the root of its record.
fn map_tags(entries: &[Fv]) -> (usize, usize) {
    let unset = entries
        .iter()
        .filter(|entry| id_in_either(entry.pointer("/second")) == Some(-1))
        .count();
    (entries.len(), unset)
}

/// What the probe reads of one record of a family's document; two copies of a
/// record are equal where they agree on all of it.
#[derive(PartialEq)]
enum Record {
    /// A nested family instance and the type it draws.
    Instance(Option<u64>),
    /// A type's map: its geometry tags and how many map to no material.
    Symbol { tags: usize, unset_tags: usize },
}

fn read_record(graph: &ObjectGraph) -> Option<Record> {
    let root = graph.objects.first()?;
    match root.class_name.as_str() {
        "FamilyInstance" => {
            let symbol = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo")
                .and_then(|object| id_in_either(object.fields.get("m_symbolId")))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0);
            Some(Record::Instance(symbol))
        }
        "FamilySymbol" => {
            let entries = root
                .fields
                .get("m_geomTag2MaterialId")
                .and_then(Fv::as_array)?;
            let (tags, unset_tags) = map_tags(entries);
            Some(Record::Symbol { tags, unset_tags })
        }
        _ => None,
    }
}

/// What one family's document holds of its nested instances.
#[derive(Default)]
struct Nest {
    instances: usize,
    read: usize,
    unset: usize,
}

/// A type of a project: its family and its map's tags.
struct Type {
    family: Option<u64>,
    tags: usize,
    unset_tags: usize,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut elements: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if CLASSES.contains(&upper.as_str()) {
                if let Ok(id) = tag.parse() {
                    elements.entry(id).or_insert(upper);
                }
            }
        }
    }

    // Each instance's drawn symbol (Revit's type, RE-477).
    let mut drawn: BTreeMap<u64, Option<u64>> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: elements.keys().copied().collect(),
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let symbol = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo")
                .and_then(|object| id_in_either(object.fields.get("m_symbolId")))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0);
            drawn.insert(record.identity.element_id, symbol);
            Ok(())
        },
    )?;

    // Each drawn symbol's family and map.
    let symbols: BTreeSet<u64> = drawn.values().filter_map(|symbol| *symbol).collect();
    let mut types: BTreeMap<u64, Type> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: symbols,
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            let Some(root) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
            else {
                return Ok(());
            };
            let Some(entries) = root
                .fields
                .get("m_geomTag2MaterialId")
                .and_then(Fv::as_array)
            else {
                return Ok(());
            };
            let (tags, unset_tags) = map_tags(entries);
            let family = id_in_either(root.fields.get("m_familyId"))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0);
            types.insert(
                record.identity.element_id,
                Type {
                    family,
                    tags,
                    unset_tags,
                },
            );
            Ok(())
        },
    )?;

    // The families' own documents: their nested instances and the types those
    // draw, each record read once from its last copy.
    let families: BTreeSet<u64> = types.values().filter_map(|t| t.family).collect();
    let classes: BTreeSet<String> = RECORD_CLASSES.iter().map(ToString::to_string).collect();
    let documents =
        family_records_in(&mut rf, &families, &classes, &NATIVE_RELEASES, None).unwrap_or_default();
    let mut nests: BTreeMap<u64, Nest> = BTreeMap::new();
    let (mut records_read, mut copied, mut differing) = (0usize, 0usize, 0usize);
    let (mut all_instances, mut all_read, mut all_unset) = (0usize, 0usize, 0usize);
    for (family, records) in &documents {
        let mut latest: BTreeMap<u64, Record> = BTreeMap::new();
        let mut copies: BTreeMap<u64, bool> = BTreeMap::new();
        for (id, graph) in records {
            let Some(record) = read_record(graph) else {
                continue;
            };
            if let Some(previous) = latest.get(id) {
                *copies.entry(*id).or_insert(false) |= *previous != record;
            }
            latest.insert(*id, record);
        }
        records_read += latest.len();
        copied += copies.len();
        differing += copies.values().filter(|differs| **differs).count();
        let maps: BTreeMap<u64, (usize, usize)> = latest
            .iter()
            .filter_map(|(id, record)| match record {
                Record::Symbol { tags, unset_tags } => Some((*id, (*tags, *unset_tags))),
                Record::Instance(_) => None,
            })
            .collect();
        let here = nests.entry(*family).or_default();
        for record in latest.values() {
            let Record::Instance(symbol) = record else {
                continue;
            };
            here.instances += 1;
            if let Some((_, unset_tags)) = symbol.and_then(|id| maps.get(&id)) {
                here.read += 1;
                here.unset += usize::from(*unset_tags > 0);
            }
        }
        all_instances += here.instances;
        all_read += here.read;
        all_unset += here.unset;
    }

    for (tag, class) in &elements {
        let symbol = drawn.get(tag).copied().flatten();
        let kind = symbol.and_then(|id| types.get(&id));
        let map_unset = match kind {
            Some(t) if t.unset_tags > 0 => "1",
            Some(_) => "0",
            None => "na",
        };
        let family = kind.and_then(|t| t.family);
        let nest = family.and_then(|id| nests.get(&id));
        println!(
            "wnest {tag} {class} map_unset={map_unset} found={} symbol={} family={} tags={} unset_tags={} nested={} nested_read={} nested_unset={}",
            u8::from(nest.is_some()),
            symbol.unwrap_or(0),
            family.unwrap_or(0),
            kind.map_or(0, |t| t.tags),
            kind.map_or(0, |t| t.unset_tags),
            nest.map_or(0, |n| n.instances),
            nest.map_or(0, |n| n.read),
            nest.map_or(0, |n| n.unset),
        );
    }
    println!(
        "nested types read: release {version} families {} nested instances {all_instances} map read {all_read} map with an unset tag {all_unset}",
        nests.len()
    );
    println!(
        "family record copies: release {version} records {records_read} held more than once {copied} copies differing {differing}"
    );
    eprintln!(
        "{} doors and windows, {} symbol maps read, {} families with a document",
        elements.len(),
        types.len(),
        documents.len()
    );
    Ok(())
}
