//! RE-178 probe: Revit 2019 to 2022 element records, read in Revit 2023's
//! 32-bit layout with the tags each release's own schema gives, against the
//! same model saved by Revit 2023.
//!
//! FACT (to be measured): Revit 2019, 2020, 2021 and 2022 frame their
//! element records as 2023 does (RE-81): a declared `u32` ElementId 52 bytes
//! before the record marker, with the tag of `ElementHeader` 8 bytes after
//! it, and the `BuiltInCategory` 38 bytes before the marker. Every constant
//! the 2023 readers hold is the tag of a class in the file's schema, so with
//! each release's tags (`partition_element_records_2023::schema_tags`), a
//! 2019 to 2022 project yields the elements of its 2023 copy, under the same
//! ElementIds and classes and with the same fields.
//!
//! For each file it prints the schema's tags of those classes and whether
//! they are the table's, then, for every record marker, where a declared
//! `u32` ElementId followed 8 bytes on by `ElementHeader`'s tag sits before
//! it. Given two files, it then compares the production elements
//! (`walker::iter_elements`) by ElementId: classes, and every field but the
//! three that name where a record sits (its stream, offset and class tag),
//! listing each `(class, field)` that differs.
//!
//! ```bash
//! cargo run --release --example probe_re178_revit_2019_2022 -- 2019/rac_basic.rvt 2023/rac_basic.rvt
//! ```

use rvt::RevitFile;
use rvt::partition_element_records_2023 as per32;
use rvt::walker::{DecodedElement, InstanceField};
use std::collections::{BTreeMap, BTreeSet};

/// Fields that say where a record sits, which differ between any two saves.
const PLACE_FIELDS: &[&str] = &["m_source_stream", "m_source_offset", "m_class_tag"];

/// How far before a marker an ElementId is looked for.
const ID_SEARCH: std::ops::RangeInclusive<usize> = 9..=120;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() || paths.len() > 2 {
        return Err(
            "usage: probe_re178_revit_2019_2022 <file.rvt> [<other release's copy.rvt>]".into(),
        );
    }
    let mut sides = Vec::new();
    for path in &paths {
        let mut rf = RevitFile::open(path)?;
        let version = rf.basic_file_info()?.version;
        println!("file {path}\nrelease {version}");
        tags(&mut rf, version)?;
        framing(&mut rf)?;
        let elements: Vec<DecodedElement> = rvt::walker::iter_elements(&mut rf)?.collect();
        println!("elements {}", elements.len());
        sides.push(elements);
    }
    if let [a, b] = &sides[..] {
        compare(a, b);
    }
    Ok(())
}

/// The schema's tags of the classes behind the 32-bit constants, and
/// whether they are [`per32::schema_tags`]'s.
fn tags(rf: &mut RevitFile, version: u32) -> Result<(), Box<dyn std::error::Error>> {
    let schema = rf.schema_classes()?;
    let tag = |name: &str| {
        schema
            .classes
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.tag)
    };
    let table = per32::schema_tags(version);
    let rows = [
        ("CellList", table.map(|t| t.cell_list)),
        ("Material", table.map(|t| t.material)),
        ("PhysicalParamSet", table.map(|t| t.physical_param_set)),
        ("PatternHelper", table.map(|t| t.pattern_helper)),
        (
            "VerticalRegionsStructure",
            table.map(|t| t.vertical_regions_structure),
        ),
        ("Outline", None),
        ("ElementParents", None),
        ("ElementHeader", None),
    ];
    let mut agree = true;
    let mut list = Vec::new();
    for (name, held) in rows {
        let read = tag(name);
        if held.is_some() && held != read {
            agree = false;
        }
        list.push(match read {
            Some(read) => format!("{name}=0x{read:04x}"),
            None => format!("{name}=-"),
        });
    }
    let verdict = match table {
        Some(_) if agree => "agree",
        Some(_) => "DISAGREE",
        None => "no table",
    };
    println!("tags {} (table: {verdict})", list.join(" "));
    Ok(())
}

/// For each record marker, the distance back to a declared `u32`
/// ElementId followed 8 bytes on by `ElementHeader`'s tag.
fn framing(rf: &mut RevitFile) -> Result<(), Box<dyn std::error::Error>> {
    let Some((marker, header)) = per32::record_marker(rf) else {
        println!("framing: schema gives no record marker");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(rf)?);
    let (mut markers, mut none) = (0usize, 0usize);
    let mut at_distance: BTreeMap<usize, usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            markers += 1;
            let found = ID_SEARCH.clone().find(|&back| {
                let Some(at) = hit.checked_sub(back) else {
                    return false;
                };
                let u32_at = |at: usize| {
                    buf.get(at..at + 4)
                        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
                };
                let u16_at = |at: usize| {
                    buf.get(at..at + 2)
                        .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
                };
                u32_at(at).is_some_and(|id| declared.contains(&id))
                    && u16_at(at + 8) == Some(header)
            });
            match found {
                Some(back) => *at_distance.entry(back).or_default() += 1,
                None => none += 1,
            }
        }
    }
    let list: Vec<String> = at_distance
        .iter()
        .map(|(back, n)| format!("-{back}={n}"))
        .collect();
    println!(
        "framing markers {markers} id-before-marker {} none {none}",
        list.join(" ")
    );
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
    let mut only: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for id in ids_a.difference(&ids_b) {
        *only.entry(("first", a[id].class.as_str())).or_default() += 1;
    }
    for id in ids_b.difference(&ids_a) {
        *only.entry(("second", b[id].class.as_str())).or_default() += 1;
    }
    for ((side, class), n) in &only {
        println!("only {side} {class} {n}");
    }
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
