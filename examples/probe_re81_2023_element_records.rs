//! RE-81: Revit 2023 element records (#421).
//!
//! FACT: a Revit 2023 project frames its elements behind the same marker as
//! 2024 and 2025, `[tag of Outline][0xFF x 4][tag of ElementParents]` in the
//! file's own schema (RE-80; `3c 01 ff ff ff ff 82 05` on 2023). Relative to
//! the marker, a 2023 record holds its ElementId as a `u32` at -52, its
//! `BuiltInCategory` as an `i64` at -38, its container reference at -30 and
//! its placement kind at -14 (the last two where 2024 has them, at `+0x32`
//! and `+0x42` of a record whose marker is at `+0x50`), and its model box as
//! six `f64` right after the marker. The RE-21 instance rule (placed, no
//! container, a box with volume) then gives the elements Revit's own export
//! writes.
//!
//! Verify against a 2023 project and Revit's IFC export of it:
//!
//! ```text
//! cargo run --profile ci --example probe_re81_2023_element_records -- \
//!     model.rvt model.ifc
//! ```
//!
//! It prints, per category, the instances the rule gives and how many are
//! `Tag`s of the export, then the export's element Tags the rule misses.

use std::collections::{BTreeMap, BTreeSet};

const ID_BACK: usize = 52;
const CATEGORY_BACK: usize = 38;
const CONTAINER_BACK: usize = 30;
const PLACEMENT_BACK: usize = 14;
const PLACEMENT_INSTANCE: u32 = 0xffff_ef7f;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(rvt), Some(ifc)) = (args.get(1), args.get(2)) else {
        anyhow::bail!("usage: probe_re81_2023_element_records FILE.rvt EXPORT.ifc");
    };
    let tags = export_tags(&std::fs::read_to_string(ifc)?);
    let mut rf = rvt::RevitFile::open(rvt)?;
    let version = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let tag = |name: &str| {
        classes
            .classes
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.tag)
    };
    let (Some(outline), Some(parents)) = (tag("Outline"), tag("ElementParents")) else {
        anyhow::bail!("the schema has no Outline or ElementParents class");
    };
    let mut marker = [0xffu8; 8];
    marker[..2].copy_from_slice(&outline.to_le_bytes());
    marker[6..].copy_from_slice(&parents.to_le_bytes());
    let mut instances: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &marker) {
            let Some(start) = at.checked_sub(ID_BACK) else {
                continue;
            };
            let read = |from: usize, len: usize| buf.get(from..from + len);
            let (Some(id), Some(category), Some(container), Some(kind), Some(bbox)) = (
                read(start, 4),
                read(at - CATEGORY_BACK, 8),
                read(at - CONTAINER_BACK, 8),
                read(at - PLACEMENT_BACK, 4),
                read(at + 8, 48),
            ) else {
                continue;
            };
            let category = i64::from_le_bytes(category.try_into()?);
            if !(-2_100_000..-1_990_000).contains(&category)
                || u64::from_le_bytes(container.try_into()?) != u64::MAX
                || u32::from_le_bytes(kind.try_into()?) != PLACEMENT_INSTANCE
            {
                continue;
            }
            let bbox: Vec<f64> = bbox
                .chunks_exact(8)
                .filter_map(|c| c.try_into().ok().map(f64::from_le_bytes))
                .collect();
            if (0..3).all(|k| bbox[k + 3] - bbox[k] > 1e-6) {
                instances
                    .entry(category)
                    .or_default()
                    .insert(u32::from_le_bytes(id.try_into()?));
            }
        }
    }
    println!(
        "Revit {version}, marker {marker:02x?}, {} element Tags in the export",
        tags.len()
    );
    let mut found = BTreeSet::new();
    for (category, ids) in &instances {
        let in_export: BTreeSet<u32> = ids.intersection(&tags).copied().collect();
        println!(
            "category {category}: {} instances, {} Tags of the export, {} not",
            ids.len(),
            in_export.len(),
            ids.len() - in_export.len()
        );
        found.extend(in_export);
    }
    let missed: Vec<&u32> = tags.difference(&found).collect();
    println!("export Tags not found: {} {missed:?}", missed.len());
    Ok(())
}

/// The `Tag` of every building element in an IFC export: its eighth
/// attribute, for entities that are not types, openings, spatial structure
/// or owner histories.
fn export_tags(step: &str) -> BTreeSet<u32> {
    const SKIP: &[&str] = &[
        "IFCOPENINGELEMENT",
        "IFCOWNERHISTORY",
        "IFCPROJECT",
        "IFCSITE",
        "IFCBUILDING",
        "IFCBUILDINGSTOREY",
        "IFCSPACE",
    ];
    let mut out = BTreeSet::new();
    for line in step.lines() {
        let Some((_, rest)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = rest.split_once('(') else {
            continue;
        };
        if entity.ends_with("TYPE") || SKIP.contains(&entity) {
            continue;
        }
        let mut fields = Vec::new();
        let (mut depth, mut quoted, mut current) = (0, false, String::new());
        for c in args.chars() {
            match c {
                '\'' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => depth -= 1,
                ',' if !quoted && depth == 0 => {
                    fields.push(std::mem::take(&mut current));
                    continue;
                }
                _ => {}
            }
            current.push(c);
        }
        fields.push(current);
        if let Some(tag) = fields
            .get(7)
            .and_then(|f| f.trim().trim_matches('\'').parse().ok())
        {
            out.insert(tag);
        }
    }
    out
}
