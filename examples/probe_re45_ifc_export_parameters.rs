//! RE-45: Revit's IFC export overrides are parameter entries in an
//! element's serialised data.
//!
//! FACT: an element's "Export to IFC As" and "IFC Predefined Type", and a
//! type's "Export Type to IFC As" and "Type IFC Predefined Type", are entries
//! `i64 BuiltInParameter · u32 n · n UTF-16 units` in the element's (or the
//! type's) serialised data. The keys are Autodesk's public
//! `BuiltInParameter` values `IFC_EXPORT_ELEMENT_AS` (−1019014),
//! `IFC_EXPORT_ELEMENT_TYPE_AS` (−1019015), `IFC_EXPORT_PREDEFINEDTYPE`
//! (−1019016) and `IFC_EXPORT_PREDEFINEDTYPE_TYPE` (−1019017). An entry
//! belongs to the element whose data header is the last one before it.
//!
//! On `2024_Core_Interior.rvt` 26 declared elements carry one: 21
//! `IfcShadingDevice` (the 20 exported are Revit's 20 `IFCSHADINGDEVICE`
//! Tags), 3 `ifcSlab` with `ROOF` (the 2 exported are Revit's 2 `IFCSLAB …
//! .ROOF.` rows), one `IfcSpace` and one `ifcFooting`, neither exported. A
//! further 12 entries sit in loaded families' own documents, whose ids
//! `Global/ElemTable` does not declare, and are not read. On
//! `teste_export_2025.rvt` wall type 402 carries `IfcCoveringType` with
//! `CLADDING`, and Revit exports its three walls as `IFCCOVERING …
//! .CLADDING.`.
//!
//! The probe prints every element carrying an export parameter, with its
//! values. Compare an element's entity and `PredefinedType` with the row of
//! the same `Tag` in Revit's export, and a type's with the rows of the
//! elements that name it (`tools/re/export_parameters_vs_ifc.py` does).
//!
//! It then prints every "Export to IFC As" and "Export Type to IFC As"
//! entry where it sits: its stream and offset, its value, the element-data
//! header the owner rule takes (how far back, the id after it, whether
//! `Global/ElemTable` declares that id, and the category of the id's element
//! record) and the next header after the entry. That is the evidence for
//! checking an owner, as on the BIM4LCA office, where the elements read as
//! `IfcShadingDevice` are curtain walls Revit exports as `IfcCurtainWall`
//! (#792).
//!
//! Each entry line also names the data object (RE-153) whose payload holds
//! the entry, the owner rule Revit 2024 and later take (#792), and the probe
//! ends with how often the two owners agree, with the values and owner
//! categories of the entries on which they do not.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re45_ifc_export_parameters -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{
    BUILTIN_CATEGORY_NAMES, scan_category_records_multi,
};
use rvt::research::partition_ifc_export_overrides::{
    EXPORT_PARAMETER_WINDOW, IFC_EXPORT_ELEMENT_AS, IFC_EXPORT_ELEMENT_TYPE_AS,
    scan_export_parameters,
};
use rvt::research::partition_room_parameters::DATA_OBJECT_HEADER;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Categories with no name in [`BUILTIN_CATEGORY_NAMES`] that an entry's
/// owner may still be: Autodesk `BuiltInCategory` `OST_Rooms`, `OST_Areas`
/// and `OST_MEPSpaces`.
const SPACE_CATEGORIES: [(i64, &str); 3] = [
    (-2_000_160, "Rooms"),
    (-2_003_200, "Areas"),
    (-2_003_600, "MEP Spaces"),
];

/// The UTF-16 string at `at` (`u32` length, then the units), when every unit
/// is printable ASCII.
fn utf16_value(buf: &[u8], at: usize) -> Option<String> {
    let units = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=64).contains(&units) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + units * 2)?;
    bytes
        .chunks_exact(2)
        .map(|pair| {
            u8::try_from(u16::from_le_bytes([pair[0], pair[1]]))
                .ok()
                .filter(|byte| byte.is_ascii_graphic() || *byte == b' ')
                .map(char::from)
        })
        .collect()
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let parameters = scan_export_parameters(&mut rf, version, &declared)?;
    println!(
        "release {version}: {} elements carry IFC export parameters",
        parameters.len()
    );
    for (id, p) in parameters {
        let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
        println!(
            "  {id:>9}  export as {:<18} predefined {:<10} type export as {:<18} type predefined {}",
            show(&p.export_as),
            show(&p.predefined_type),
            show(&p.type_export_as),
            show(&p.type_predefined_type),
        );
    }

    let Some(header) = rvt::research::partition_names::element_data_header(version) else {
        println!("entries: no element-data header is measured for release {version}");
        return Ok(());
    };
    let categories: Vec<(i64, &str)> = BUILTIN_CATEGORY_NAMES
        .iter()
        .copied()
        .chain(SPACE_CATEGORIES)
        .collect();
    let ids: Vec<i64> = categories.iter().map(|(id, _)| *id).collect();
    let category_of: BTreeMap<u32, &str> =
        scan_category_records_multi(&mut rf, version, &ids, &declared)?
            .into_iter()
            .filter_map(|record| {
                categories
                    .iter()
                    .find(|(id, _)| *id == record.builtin_category)
                    .map(|(_, name)| (record.element_id, *name))
            })
            .collect();
    println!("entries: stream offset key value | owner header | next header | data object");
    let kind = |raw: u64| {
        let id = u32::try_from(raw).ok();
        format!(
            "{} {}",
            if id.is_some_and(|id| declared.contains(&id)) {
                "declared"
            } else {
                "undeclared"
            },
            id.and_then(|id| category_of.get(&id)).unwrap_or(&"-"),
        )
    };
    let describe = |id: Option<u64>| match id {
        Some(raw) => format!("id {raw} {}", kind(raw)),
        None => "none".into(),
    };
    let mut agree = 0usize;
    let mut no_object = 0usize;
    let mut differ: BTreeMap<String, usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let Ok(objects) = rf.partition_data_objects(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let id_after = |at: usize| {
            buf.get(at + header.len()..at + header.len() + 8)
                .map(|bytes| u64::from_le_bytes(bytes.try_into().expect("8 bytes")))
        };
        for (key, label) in [
            (IFC_EXPORT_ELEMENT_AS, "as"),
            (IFC_EXPORT_ELEMENT_TYPE_AS, "type-as"),
        ] {
            for hit in memchr::memmem::find_iter(buf, &key.to_le_bytes()) {
                let Some(value) = utf16_value(buf, hit + 8) else {
                    continue;
                };
                let from = hit.saturating_sub(EXPORT_PARAMETER_WINDOW);
                let header_owner = memchr::memmem::rfind(&buf[from..hit], &header)
                    .and_then(|at| id_after(from + at));
                let object_owner = objects
                    .partition_point(|(p, _)| *p <= hit)
                    .checked_sub(1)
                    .map(|index| objects[index])
                    .filter(|(p, object)| hit >= p + DATA_OBJECT_HEADER && hit + 8 <= object.end)
                    .map(|(_, object)| u64::from(object.element_id));
                match (header_owner, object_owner) {
                    (_, None) => no_object += 1,
                    (Some(a), Some(b)) if a == b => agree += 1,
                    (a, Some(b)) => {
                        *differ
                            .entry(format!(
                                "{label} {value:?}: header {} / object {}",
                                a.map_or_else(|| "none".to_string(), kind),
                                kind(b)
                            ))
                            .or_default() += 1;
                    }
                }
                let owner = match memchr::memmem::rfind(&buf[from..hit], &header) {
                    Some(at) => {
                        let at = from + at;
                        match id_after(at) {
                            Some(raw) => {
                                let id = u32::try_from(raw).ok();
                                format!(
                                    "-{:#x} id {raw} {} {}",
                                    hit - at,
                                    if id.is_some_and(|id| declared.contains(&id)) {
                                        "declared"
                                    } else {
                                        "undeclared"
                                    },
                                    id.and_then(|id| category_of.get(&id)).unwrap_or(&"-"),
                                )
                            }
                            None => format!("-{:#x} id -", hit - at),
                        }
                    }
                    None => "none in the window".into(),
                };
                let next = match memchr::memmem::find(&buf[hit..], &header) {
                    Some(at) => match id_after(hit + at) {
                        Some(raw) => format!("+{at:#x} id {raw}"),
                        None => format!("+{at:#x} id -"),
                    },
                    None => "none".into(),
                };
                println!(
                    "  entry {stream} {hit:#x} {label} {value:?} | owner {owner} | next {next} | object {}",
                    describe(object_owner)
                );
            }
        }
    }
    println!(
        "owners: header and data object agree {agree}, differ {}, no data object {no_object}",
        differ.values().sum::<usize>()
    );
    for (what, count) in &differ {
        println!("  {count:>5}  {what}");
    }
    Ok(())
}
