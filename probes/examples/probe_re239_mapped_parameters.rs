//! RE-239 (#35, W7): which of an element's own parameters hold the values
//! Revit's export of the BIM4LCA office writes in sets IFC4 does not give
//! the element.
//!
//! FACT under test: Revit's export writes `Pset_DoorWindowGlazingType`
//! (`IsLaminated`, `IsTempered`) on 604 walls, 151 curtain walls, the 70
//! openings curtain walls cut (tagged with the curtain wall) and 21
//! coverings, and `Pset_SpaceOccupancyRequirements.OccupancyType` on 234
//! spaces, none of which revit-ifc's own definitions (CLEANROOM.md source 5)
//! puts there. An export-time user-defined property set file maps a
//! parameter to such a set. Hypothesis: each value is one of the element's
//! saved parameters (RE-207), and the file, which the model does not store,
//! only places it. Members and plates are printed too: Revit's export gives
//! the office's curtain mullions and panels an `IsExternal` neither their
//! host nor their type decides (RE-226 §2).
//!
//! Output, one line per wall, curtain wall, covering, member, plate and room
//! the export writes:
//!   par <tag> <IFC class> <parameter id>:<name|->=<value> ...
//! every Yes/No and text value of its saved parameters and its type's,
//! spaces as `_`; then, for each of `MARKERS` (a user-defined property set
//! file's keyword, the name of a set only the office's file defines, and
//! revit-ifc's names for its export setup and the setting that names the
//! file), how often it occurs in every stream, stored and inflated:
//!   scan <marker> <utf8|utf16> <stream|total> <count>
//! and the text after each UTF-16 `ExportUserDefinedPsets`, each path cut
//! to its file name:
//!   ctx <stream> <text>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re239_mapped_parameters -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_element_parameters;
use serde_json::Value;

const CLASSES: [&str; 6] = [
    "IFCWALL",
    "IFCCURTAINWALL",
    "IFCCOVERING",
    "IFCMEMBER",
    "IFCPLATE",
    "IFCSPACE",
];
const MARKERS: [&str; 4] = [
    "PropertySet:",
    "FI_Kohde",
    "ExportUserDefinedPsets",
    "IFCExportConfiguration",
];

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
            let upper = ifc_type.to_ascii_uppercase();
            if CLASSES.contains(&upper.as_str()) {
                if let Ok(id) = tag.parse() {
                    classes.entry(id).or_insert(upper);
                }
            }
        }
    }
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let read = native_element_parameters::read(&mut rf, &ids)?;
    for (id, class) in &classes {
        let values: Vec<String> = read
            .elements
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|parameter| {
                let value = match &parameter.value {
                    Value::Bool(b) => b.to_string(),
                    Value::String(text) => text.replace(' ', "_"),
                    _ => return None,
                };
                Some(format!(
                    "{}:{}={value}",
                    parameter.parameter_id,
                    parameter.name.as_deref().unwrap_or("-").replace(' ', "_")
                ))
            })
            .collect();
        println!("par {id} {class} {}", values.join(" "));
    }
    eprintln!(
        "{} elements, {} with parameters",
        classes.len(),
        read.elements.len()
    );
    let needles: Vec<(&str, &str, Vec<u8>)> = MARKERS
        .iter()
        .flat_map(|marker| {
            let utf16 = marker.encode_utf16().flat_map(u16::to_le_bytes).collect();
            [
                (*marker, "utf8", marker.as_bytes().to_vec()),
                (*marker, "utf16", utf16),
            ]
        })
        .collect();
    let mut totals = vec![0usize; needles.len()];
    for name in rf.stream_names() {
        let Ok(stored) = rf.read_stream(&name) else {
            println!("scan unreadable - {name} 0");
            continue;
        };
        let mut chunks = rvt::compression::inflate_all_chunks_for_stream(&name, &stored);
        chunks.push(stored);
        for ((marker, encoding, needle), total) in needles.iter().zip(totals.iter_mut()) {
            let count: usize = chunks
                .iter()
                .map(|chunk| memchr::memmem::find_iter(chunk, needle).count())
                .sum();
            if count > 0 {
                println!("scan {marker} {encoding} {name} {count}");
            }
            if *encoding == "utf16" && *marker == "ExportUserDefinedPsets" {
                for chunk in &chunks {
                    for at in memchr::memmem::find_iter(chunk, needle) {
                        println!("ctx {name} {}", context(chunk, at));
                    }
                }
            }
            *total += count;
        }
    }
    for ((marker, encoding, _), total) in needles.iter().zip(&totals) {
        println!("scan {marker} {encoding} total {total}");
    }
    Ok(())
}

/// The UTF-16 text at `at` in `chunk`, up to 160 code units or the first
/// control character, with every path between quotes cut to its file name.
fn context(chunk: &[u8], at: usize) -> String {
    let text: String = chunk[at..]
        .chunks_exact(2)
        .take(160)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .map_while(|unit| char::from_u32(u32::from(unit)).filter(|c| !c.is_control()))
        .collect();
    text.split('"')
        .map(|part| match part.rfind(['\\', '/']) {
            Some(cut) => &part[cut + 1..],
            None => part,
        })
        .collect::<Vec<_>>()
        .join("\"")
}
