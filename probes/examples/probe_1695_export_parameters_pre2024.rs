//! #1695: where do Revit 2015 to 2023 files keep an element's IFC export
//! parameters, the four of RE-45 ("Export to IFC As", "IFC Predefined Type" and
//! the two a type sets), which `scan_export_parameters` reads only from 2024?
//!
//! FACT 1 to check: on Revit 2024 and later an entry is `i64 BuiltInParameter ·
//! u32 n · n UTF-16 units`. The probe counts, per key, the entries with a value
//! in that form (`i64`) and in a narrower one, `i32 BuiltInParameter · u32 n ·
//! n UTF-16 units`, as `i32`, because the 2015 to 2023 element ids are 4 bytes
//! wide and the key may be too. The narrow key's four bytes are also the first
//! four of the wide one, so a hit whose next four bytes are `ff ff ff ff` is
//! counted as the wide form (`i64`).
//!
//! FACT 2 to check: the element whose data holds an entry is the last element
//! data header before it (`element_data_layout`, a `u32` id from Revit 2008 to
//! 2023, RE-111), within `EXPORT_PARAMETER_WINDOW`, and that id is declared in
//! `Global/ElemTable`. The probe counts the entries that resolve to a declared
//! owner, the ones with no header in range and the ones whose owner is
//! undeclared, then the `(owner, key)` pairs that are set to two different
//! values (the scan drops those).
//!
//! FACT 3 to check, when FACT 1 finds nothing: an export parameter's value is
//! one of a few IFC words. For each enumerator of `IfcFurnitureTypeEnum` the
//! probe finds every `u32 n · UTF-16` string whose length is that word's and
//! prints a histogram of the 8 bytes before it, which names the key's own
//! encoding if a release keeps it in another form.
//!
//! Counts only, one `export parameters pre2024:` line per figure, in the fixed
//! wording `tools/ci/scrub-holdout-results.sh` keeps for a held-out model. A
//! value is printed only when it is on a fixed list of IFC entity names or
//! predefined type words, in the list's spelling, and every other text, which a
//! user types, is counted as `other-text`. The bytes before a furniture word are
//! raw file bytes, which can be an id, a length or an offset, so they are printed
//! under their own tag (`export parameters pre2024 bytes:`), which the scrub does
//! not keep; no id, offset or name reaches a held-out result.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1695_export_parameters_pre2024 -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_ifc_export_overrides::{
    EXPORT_PARAMETER_WINDOW, IFC_EXPORT_ELEMENT_AS, IFC_EXPORT_ELEMENT_TYPE_AS,
    IFC_EXPORT_PREDEFINEDTYPE, IFC_EXPORT_PREDEFINEDTYPE_TYPE,
};
use rvt::research::partition_names::element_data_layout;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const KEYS: [(i64, &str); 4] = [
    (IFC_EXPORT_ELEMENT_AS, "export as"),
    (IFC_EXPORT_ELEMENT_TYPE_AS, "type export as"),
    (IFC_EXPORT_PREDEFINEDTYPE, "predefined type"),
    (IFC_EXPORT_PREDEFINEDTYPE_TYPE, "type predefined type"),
];

/// `IfcFurnitureTypeEnum`, the words searched for when no key is found.
const ENUMERATORS: [&str; 10] = [
    "BED",
    "CHAIR",
    "DESK",
    "FILECABINET",
    "SHELF",
    "SOFA",
    "TABLE",
    "TECHNICALCABINET",
    "USERDEFINED",
    "NOTDEFINED",
];

/// IFC entity names an "as" key's value is printed as; any other text is
/// `other-text`.
const ENTITIES: [&str; 29] = [
    "IfcBeam",
    "IfcBuildingElementProxy",
    "IfcColumn",
    "IfcCovering",
    "IfcCoveringType",
    "IfcCurtainWall",
    "IfcDoor",
    "IfcDuctSegment",
    "IfcFlowSegment",
    "IfcFlowTerminal",
    "IfcFooting",
    "IfcFurniture",
    "IfcFurnitureType",
    "IfcLightFixture",
    "IfcMember",
    "IfcOpeningElement",
    "IfcPipeSegment",
    "IfcPlate",
    "IfcRailing",
    "IfcRamp",
    "IfcRoof",
    "IfcSanitaryTerminal",
    "IfcShadingDevice",
    "IfcSlab",
    "IfcSpace",
    "IfcStair",
    "IfcWall",
    "IfcWasteTerminal",
    "IfcWindow",
];

/// Predefined type words a predefined key's value is printed as, besides
/// [`ENUMERATORS`]; any other text is `other-text`.
const PREDEFINED_WORDS: [&str; 12] = [
    "BASESLAB",
    "CLADDING",
    "ELEMENTEDWALL",
    "FLOOR",
    "LANDING",
    "MOVABLE",
    "PARAPET",
    "PARTITIONING",
    "PLUMBINGWALL",
    "ROOF",
    "SHEAR",
    "SOLIDWALL",
];

/// The longest value an entry holds, in UTF-16 units.
const MAX_VALUE_UNITS: usize = 64;

/// Starts every count line, which the held-out scrub keeps by its wording.
const TAG: &str = "export parameters pre2024:";

/// Starts the lines of raw bytes, which the scrub does not keep.
const BYTES_TAG: &str = "export parameters pre2024 bytes:";

/// The most byte patterns printed per file for FACT 3.
const PATTERNS_PRINTED: usize = 8;

/// The value at `at` (`u32` length, then the units) as `RE-45` reads it: ASCII
/// letters, digits and underscores, and for an entity key an `Ifc` name.
fn export_value(buf: &[u8], at: usize, entity: bool) -> Option<String> {
    let units = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=MAX_VALUE_UNITS).contains(&units) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + units * 2)?;
    let mut value = String::with_capacity(units);
    for pair in bytes.chunks_exact(2) {
        let byte = u8::try_from(u16::from_le_bytes([pair[0], pair[1]])).ok()?;
        if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return None;
        }
        value.push(char::from(byte));
    }
    if entity {
        let prefixed = value
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("ifc"));
        if !prefixed || value.len() == 3 || value.contains('_') {
            return None;
        }
    }
    Some(value)
}

/// How a value is printed: its spelling on the fixed list for its key, else
/// `other-text`, since the text is a user's.
fn printable(value: &str, entity: bool) -> &'static str {
    let lists: [&[&'static str]; 2] = if entity {
        [&ENTITIES[..], &[][..]]
    } else {
        [&ENUMERATORS[..], &PREDEFINED_WORDS[..]]
    };
    lists
        .iter()
        .flat_map(|list| list.iter().copied())
        .find(|word| word.eq_ignore_ascii_case(value))
        .unwrap_or("other-text")
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let layout = element_data_layout(version);
    println!("{TAG} release {version}");
    println!(
        "{TAG} element data layout {}",
        match layout {
            Some(layout) if layout.wide_id => "u64-id",
            Some(_) => "u32-id",
            None => "none-measured",
        }
    );

    // (key label, form) -> value -> entries with that value.
    let mut valued: BTreeMap<(&str, &str), BTreeMap<&str, usize>> = BTreeMap::new();
    // (owner, key) -> the values its entries hold.
    let mut owned: BTreeMap<(u32, i64), BTreeSet<String>> = BTreeMap::new();
    let (mut no_header, mut undeclared) = (0usize, 0usize);
    // The 8 bytes before a keyless enumerator string -> how many.
    let mut before: BTreeMap<String, usize> = BTreeMap::new();
    let mut objects_found = 0usize;
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        if let Ok(objects) = rf.partition_data_objects(&stream) {
            objects_found += objects.len();
        }
        let buf = inflated.bytes();
        for (key, label) in KEYS {
            let entity = key == IFC_EXPORT_ELEMENT_AS || key == IFC_EXPORT_ELEMENT_TYPE_AS;
            let narrow = i32::try_from(key).expect("a BuiltInParameter id fits 32 bits");
            for hit in memchr::memmem::find_iter(buf, &narrow.to_le_bytes()) {
                let wide = buf.get(hit + 4..hit + 8) == Some(&[0xff; 4][..]);
                let value_at = if wide { hit + 8 } else { hit + 4 };
                let Some(value) = export_value(buf, value_at, entity) else {
                    continue;
                };
                *valued
                    .entry((label, if wide { "i64" } else { "i32" }))
                    .or_default()
                    .entry(printable(&value, entity))
                    .or_default() += 1;
                let owner = layout.and_then(|layout| {
                    let from = hit.saturating_sub(EXPORT_PARAMETER_WINDOW);
                    let start = memchr::memmem::rfind(&buf[from..hit], &layout.header)?;
                    layout.id_at(buf, from + start + layout.header.len())
                });
                match owner {
                    None => no_header += 1,
                    Some(owner) if !declared.contains(&owner) => undeclared += 1,
                    Some(owner) => {
                        owned.entry((owner, key)).or_default().insert(value);
                    }
                }
            }
        }
        for word in ENUMERATORS {
            let mut needle = u32::try_from(word.len())
                .expect("a short word")
                .to_le_bytes()
                .to_vec();
            for unit in word.encode_utf16() {
                needle.extend_from_slice(&unit.to_le_bytes());
            }
            for hit in memchr::memmem::find_iter(buf, &needle) {
                if let Some(prefix) = hit.checked_sub(8).and_then(|from| buf.get(from..hit)) {
                    *before.entry(hex(prefix)).or_default() += 1;
                }
            }
        }
    }

    println!("{TAG} data objects the 2024 reader finds {objects_found}");
    for ((label, form), values) in &valued {
        for (value, count) in values {
            println!(
                "{TAG} entries {} {form} {value} {count}",
                label.replace(' ', "-")
            );
        }
    }
    println!(
        "{TAG} entries with a value in either form {}",
        valued.values().flat_map(BTreeMap::values).sum::<usize>()
    );
    let disagreeing = owned.values().filter(|values| values.len() > 1).count();
    println!(
        "{TAG} owners: pairs {} over {} declared owners, set to two values {disagreeing}, no element data header in range {no_header}, undeclared owner {undeclared}",
        owned.len(),
        owned
            .keys()
            .map(|(owner, _)| owner)
            .collect::<BTreeSet<_>>()
            .len()
    );

    let mut patterns: Vec<(&String, &usize)> = before.iter().collect();
    patterns.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!(
        "{TAG} strings equal to an IfcFurnitureTypeEnum word {} in {} distinct 8-byte prefixes",
        before.values().sum::<usize>(),
        before.len()
    );
    for (prefix, count) in patterns.into_iter().take(PATTERNS_PRINTED) {
        println!(
            "{BYTES_TAG} prefix {} count {count}",
            prefix.replace(' ', "")
        );
    }
    Ok(())
}
