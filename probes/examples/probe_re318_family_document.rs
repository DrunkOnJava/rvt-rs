//! RE-318 (#810): what the loaded family document of the BIM4LCA office's two
//! door-with-sidelight doors (1756089, 1765267) holds that could size them.
//!
//! FACT under test: Revit's export writes 5.8629 ft as these doors'
//! `OverallWidth`, from `ExporterIFCUtils.GetMinSymbolWidth` on the symbol
//! that draws them (revit-ifc's `GetDoorWindowDimensionFromSymbol`,
//! CLEANROOM.md source 5). RE-215, RE-225 and RE-229 found it in no cut loop,
//! cut solid or stored length of the symbol. A loaded family's own document
//! (RE-135, RE-163, RE-204) holds the forms and reference planes the family
//! was drawn with, among them the void that cuts the opening. This prints,
//! for the family of each symbol the doors name (the symbol that draws the
//! instance and its type), a census of the records in that family's document
//! and the geometry-bearing records themselves (extrusions and other solid
//! forms, reference planes, sketches, curves, dimensions), as JSON, so the
//! width between the family's side planes, or a void's sketch, can be read.
//!
//! The doors are two controls besides: 1382481 and 1722215 (unbounded cut
//! plane, size matches Revit's) and 1623922 (cut solid matches Revit's).
//!
//! Output, one record per line (JSON has no spaces):
//!   door <id> drawn=<symbol> master=<type> drawn_family=<id> master_family=<id>
//!   fdcensus <family> records=<n> <class:count,...>
//!   fdrec <family> <record id> <class> objects=<n>
//!   fdobj <family> <record id> <index> <class> <fields as JSON>   (geometry-bearing records)
//!   fdparam <family> <record id> <caption>                         (ParamElemFamily records)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re318_family_document -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_family_forms::family_records;
use serde_json::Value;

const DOORS: [u64; 5] = [1756089, 1765267, 1382481, 1722215, 1623922];
const OBJECT_LIMIT: usize = 200;
const GEOMETRY: [&str; 12] = [
    "Extrusion",
    "Blend",
    "Sweep",
    "Revolution",
    "Plane",
    "Sketch",
    "Curve",
    "Line",
    "Dimension",
    "Level",
    "Opening",
    "FamilyInstance",
];

fn element_id(value: &Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")?
        .as_i64()
        .and_then(|n| u64::try_from(n).ok())
}

fn cap(text: String, limit: usize) -> String {
    if text.chars().count() <= limit {
        text
    } else {
        text.chars().take(limit).collect::<String>() + "..."
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut drawn: BTreeMap<u64, u64> = BTreeMap::new();
    let mut master: BTreeMap<u64, u64> = BTreeMap::new();
    let options = Options {
        selected_ids: DOORS.iter().copied().collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if let Some(symbol) = graph
            .objects
            .iter()
            .find(|object| object.class_name == "InstanceInfo")
            .and_then(|object| object.fields.get("m_symbolId"))
            .and_then(element_id)
        {
            drawn.insert(id, symbol);
        }
        if let Some(symbol) = graph
            .objects
            .first()
            .and_then(|object| object.fields.get("m_masterSymbolId"))
            .and_then(element_id)
        {
            master.insert(id, symbol);
        }
        Ok(())
    })?;
    let symbols: BTreeSet<u64> = drawn.values().chain(master.values()).copied().collect();
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let options = Options {
        selected_ids: symbols,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        if let Some(family) = graph
            .objects
            .first()
            .and_then(|object| object.fields.get("m_familyId"))
            .and_then(element_id)
        {
            family_of.insert(record.identity.element_id, family);
        }
        Ok(())
    })?;
    let opt = |v: Option<u64>| v.map_or("-".to_string(), |n| n.to_string());
    for door in DOORS {
        let d = drawn.get(&door).copied();
        let m = master.get(&door).copied();
        println!(
            "door {door} drawn={} master={} drawn_family={} master_family={}",
            opt(d),
            opt(m),
            opt(d.and_then(|s| family_of.get(&s).copied())),
            opt(m.and_then(|s| family_of.get(&s).copied())),
        );
    }
    let families: BTreeSet<u64> = family_of.values().copied().collect();
    let documents = family_records(&mut rf, &families, &BTreeSet::new())?;
    for family in &families {
        let Some(records) = documents.get(family) else {
            println!("fdcensus {family} records=0 document-not-found");
            continue;
        };
        let mut census: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, graph) in records {
            if let Some(root) = graph.objects.first() {
                *census.entry(root.class_name.as_str()).or_default() += 1;
            }
        }
        println!(
            "fdcensus {family} records={} {}",
            records.len(),
            census
                .iter()
                .map(|(class, n)| format!("{class}:{n}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        for (id, graph) in records {
            let Some(root) = graph.objects.first() else {
                continue;
            };
            println!(
                "fdrec {family} {id} {} objects={}",
                root.class_name,
                graph.objects.len()
            );
            if root.class_name == "ParamElemFamily" {
                if let Some(caption) = graph
                    .objects
                    .iter()
                    .find_map(|object| object.fields.get("m_caption")?.as_str())
                {
                    println!("fdparam {family} {id} {}", caption.replace(' ', "_"));
                }
            }
            if GEOMETRY.iter().any(|name| root.class_name.contains(*name)) {
                for (index, object) in graph.objects.iter().enumerate().take(OBJECT_LIMIT) {
                    println!(
                        "fdobj {family} {id} {index} {} {}",
                        object.class_name,
                        cap(object.fields.to_string(), 900)
                    );
                }
            }
        }
    }
    Ok(())
}
