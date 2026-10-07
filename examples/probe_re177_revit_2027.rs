//! RE-177 probe: Revit 2027 element records, read with the constants the
//! file's own schema gives, against the same model saved by Revit 2026.
//!
//! FACT (to be measured): every per-release constant the partition readers
//! hold is the tag of a class in the file's schema (RE-124), so a 2027 file's
//! schema gives its 2027 values; with them, a 2027 project yields the same
//! elements, under the same ElementIds and classes and with the same fields,
//! as the 2026 copy of the same Autodesk sample. Revit 2027 writes most
//! element frames in the second prologue (no ElementId at `+0x00`, RE-30),
//! type symbols among them, and the enclosing record of the partition's
//! record chain (RE-35) carries their id.
//!
//! For each file it prints the schema tags behind the constants and the
//! frames by prologue and kind. Given two files, it then compares the
//! production elements (`walker::iter_elements`) by ElementId: classes, and
//! every field but the three that name where a record sits (its stream,
//! offset and class tag), listing each `(class, field)` that differs.
//!
//! ```bash
//! cargo run --release --example probe_re177_revit_2027 -- 2026/rac_basic.rvt 2027/rac_basic.rvt
//! ```

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::walker::{DecodedElement, InstanceField};
use std::collections::{BTreeMap, BTreeSet};

/// The classes whose tags the readers hold as per-release constants.
const CLASSES: &[&str] = &[
    "Outline",
    "ElementParents",
    "ElementHeader",
    "Plane",
    "CellList",
    "VerticalRegionsStructure",
    "Material",
    "PatternHelper",
    "PhysicalParamSet",
];

/// Fields that say where a record sits, which differ between any two saves.
const PLACE_FIELDS: &[&str] = &["m_source_stream", "m_source_offset", "m_class_tag"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() || paths.len() > 2 {
        return Err("usage: probe_re177_revit_2027 <file.rvt> [<other release's copy.rvt>]".into());
    }
    let mut sides = Vec::new();
    for path in &paths {
        let mut rf = RevitFile::open(path)?;
        let version = rf.basic_file_info()?.version;
        let schema = rf.schema_classes()?;
        let tags: Vec<String> = CLASSES
            .iter()
            .map(
                |name| match schema.classes.iter().find(|c| c.name == *name) {
                    Some(class) => format!("{name}=0x{:04x}", class.tag),
                    None => format!("{name}=-"),
                },
            )
            .collect();
        println!("file {path}\nrelease {version}\ntags {}", tags.join(" "));
        frames(&mut rf, version)?;
        let elements: Vec<DecodedElement> = rvt::walker::iter_elements(&mut rf)?.collect();
        println!("elements {}", elements.len());
        sides.push(elements);
    }
    if let [a, b] = &sides[..] {
        compare(a, b);
    }
    Ok(())
}

/// Element frames by prologue (ElementId at `+0x00` or not) and kind.
fn frames(rf: &mut RevitFile, version: u32) -> Result<(), Box<dyn std::error::Error>> {
    let Some(marker) = per::schema_bbox_marker(rf) else {
        println!("frames: schema gives no record marker");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(rf)?);
    let mut counts: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(per::BBOX_MARKER_OFFSET) else {
                continue;
            };
            let word = |at: usize| {
                buf.get(at..at + 8)
                    .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
            };
            let (Some(raw), Some(kind)) = (word(offset), word(offset + per::PLACEMENT_KIND_OFFSET))
            else {
                continue;
            };
            let prologue = if u32::try_from(raw).is_ok_and(|id| declared.contains(&id)) {
                "first"
            } else if per::carries_no_element_id(raw) {
                "second"
            } else {
                "other"
            };
            let kind = match (kind & 0xffff_ffff) as u32 {
                per::PLACEMENT_KIND_INSTANCE => "instance",
                per::PLACEMENT_KIND_SYMBOL => "symbol",
                _ => "other",
            };
            *counts.entry((prologue, kind)).or_default() += 1;
        }
    }
    let list: Vec<String> = counts
        .iter()
        .map(|((prologue, kind), n)| format!("{prologue}/{kind}={n}"))
        .collect();
    println!("frames (release {version}) {}", list.join(" "));
    Ok(())
}

/// A field's value, with floats to nine significant digits.
fn canon(value: &InstanceField) -> String {
    match value {
        InstanceField::Float { value, .. } => format!("{value:.8e}"),
        InstanceField::Vector(items) => {
            let items: Vec<String> = items.iter().map(canon).collect();
            format!("[{}]", items.join(","))
        }
        other => format!("{other:?}"),
    }
}

fn fields(element: &DecodedElement) -> BTreeMap<&str, Vec<String>> {
    let mut out: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (name, value) in &element.fields {
        if !PLACE_FIELDS.contains(&name.as_str()) {
            out.entry(name.as_str()).or_default().push(canon(value));
        }
    }
    out
}

fn by_id(side: &[DecodedElement]) -> BTreeMap<u32, &DecodedElement> {
    side.iter().filter_map(|e| Some((e.id?, e))).collect()
}

fn compare(a: &[DecodedElement], b: &[DecodedElement]) {
    let (a, b) = (by_id(a), by_id(b));
    let ids_a: BTreeSet<u32> = a.keys().copied().collect();
    let ids_b: BTreeSet<u32> = b.keys().copied().collect();
    let both: Vec<u32> = ids_a.intersection(&ids_b).copied().collect();
    let same_class = both.iter().filter(|id| a[id].class == b[id].class).count();
    println!(
        "compare ids first {} second {} both {} same class {} only first {} only second {}",
        ids_a.len(),
        ids_b.len(),
        both.len(),
        same_class,
        ids_a.difference(&ids_b).count(),
        ids_b.difference(&ids_a).count()
    );
    let mut agree = 0usize;
    let mut differ: BTreeMap<(String, String), (usize, u32)> = BTreeMap::new();
    for id in both.iter().filter(|id| a[id].class == b[id].class) {
        let (fa, fb) = (fields(a[id]), fields(b[id]));
        let names: BTreeSet<&str> = fa.keys().chain(fb.keys()).copied().collect();
        for name in names {
            if fa.get(name) == fb.get(name) {
                agree += 1;
            } else {
                let entry = differ
                    .entry((a[id].class.clone(), name.to_string()))
                    .or_insert((0, *id));
                entry.0 += 1;
            }
        }
    }
    println!(
        "compare fields agree {agree} differ {}",
        differ.values().map(|(n, _)| n).sum::<usize>()
    );
    for ((class, field), (n, example)) in &differ {
        println!("differs {class} {field} {n} e.g. {example}");
    }
}
