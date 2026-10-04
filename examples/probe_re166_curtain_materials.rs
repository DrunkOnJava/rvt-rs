//! RE-166 (probe): where a curtain mullion's and panel's material is stored
//! (B66).
//!
//! Revit's export of RE1 Architecture gives its 10 mullions the material
//! `Aluminum 2` and its 2 system panels `Glass`; rvt-rs gives them none.
//! Mullion and system panel types are system families, so their material is
//! not in the family type's material map (RE-82).
//!
//! From the reference export next to the model the probe takes every
//! `IfcMember` and `IfcPlate` associated with one `IfcMaterial`, with its
//! `Tag` and its type's `Tag`. It finds that material's ElementId by name
//! (RE-58) and looks for it, as a `u64` or a `u32`, in the verified data
//! objects (RE-153) of the element and of its type. Every hit is keyed by
//! whose object it is in, the object's class, and the `i64` just before it (a
//! BuiltInParameter id where the value is a parameter entry) or else its
//! offset, and the probe prints how many elements each key explains.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re166_curtain_materials -- MODEL.rvt ...

use rvt::partition_room_parameters::verified_data_object;
use rvt::{RevitFile, elem_table, partition_materials};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const ENTITIES: [&str; 2] = ["IFCMEMBER", "IFCPLATE"];
const KEYS_SHOWN: usize = 15;

/// `#id -> (entity, args)` for every line of a STEP file.
fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

/// Split a STEP argument list on top-level commas.
fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut depth) = (false, 0usize);
    for c in args.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                current.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                current.push(c);
            }
            ')' if !quoted => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if !quoted && depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}

fn refs(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| f.trim().strip_prefix('#')?.parse().ok())
        .collect()
}

fn tag_of(args: &str) -> Option<u32> {
    split_args(args)
        .get(7)?
        .trim()
        .trim_matches('\'')
        .parse()
        .ok()
}

/// Each element of [`ENTITIES`] with one `IfcMaterial`: its `Tag`, entity,
/// material name and its type's `Tag`.
fn elements(step: &str) -> Vec<(u32, String, String, Option<u32>)> {
    let ents = entities(step);
    let mut types: BTreeMap<u64, u32> = BTreeMap::new();
    for (e, a) in ents.values() {
        if e != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let f = split_args(a);
        let Some(type_tag) = f
            .get(5)
            .and_then(|t| refs(t).first().copied())
            .and_then(|t| ents.get(&t))
            .and_then(|(_, ta)| tag_of(ta))
        else {
            continue;
        };
        for object in refs(f.get(4).map(String::as_str).unwrap_or("")) {
            types.insert(object, type_tag);
        }
    }
    let mut out = Vec::new();
    for (e, a) in ents.values() {
        if e != "IFCRELASSOCIATESMATERIAL" {
            continue;
        }
        let f = split_args(a);
        let Some((material_entity, material_args)) = f
            .get(5)
            .and_then(|m| refs(m).first().copied())
            .and_then(|m| ents.get(&m))
        else {
            continue;
        };
        if material_entity != "IFCMATERIAL" {
            continue;
        }
        let name = split_args(material_args)
            .first()
            .map(|n| n.trim().trim_matches('\'').to_string())
            .unwrap_or_default();
        for object in refs(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some((entity, args)) = ents.get(&object) else {
                continue;
            };
            if !ENTITIES.contains(&entity.as_str()) {
                continue;
            }
            if let Some(tag) = tag_of(args) {
                out.push((
                    tag,
                    entity.clone(),
                    name.clone(),
                    types.get(&object).copied(),
                ));
            }
        }
    }
    out
}

fn i64_at(b: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn probe(path: &str) -> anyhow::Result<Vec<String>> {
    let model = Path::new(path);
    let stem = model.file_stem().unwrap_or_default().to_string_lossy();
    let dir = model.parent().unwrap_or(Path::new("."));
    let reference = [format!("{stem}_slim.ifc"), format!("{stem}.ifc")]
        .into_iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists());
    let Some(reference) = reference else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no reference export\"}}"
        )]);
    };
    let elements = elements(&std::fs::read_to_string(&reference)?);
    if elements.is_empty() {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no member or plate with one material\"}}"
        )]);
    }
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let declared: BTreeSet<u32> = elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .collect();
    let names = partition_materials::scan_material_names(&mut rf, revit, &declared)?;
    let mut by_name: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    for (id, name) in &names {
        by_name.entry(name.as_str()).or_default().push(*id);
    }
    let mut keys: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut materials_found = BTreeMap::new();
    let streams = rf.partition_stream_names();
    for (tag, _, material, type_tag) in &elements {
        let Some(material_ids) = by_name.get(material.as_str()) else {
            continue;
        };
        materials_found.insert(material.clone(), material_ids.clone());
        let owners = [(*tag, "element")]
            .into_iter()
            .chain(type_tag.map(|t| (t, "type")));
        for (owner, whose) in owners {
            for stream in &streams {
                let Ok(inflated) = rf.inflated_partition(stream) else {
                    continue;
                };
                let b = inflated.bytes();
                for start in memchr::memmem::find_iter(b, &owner.to_le_bytes()) {
                    let Some(object) = verified_data_object(b, start) else {
                        continue;
                    };
                    if object.element_id != owner {
                        continue;
                    }
                    let payload = &b[start + 20..object.end];
                    for &material_id in material_ids {
                        for (width, needle) in [
                            (8usize, u64::from(material_id).to_le_bytes().to_vec()),
                            (4, material_id.to_le_bytes().to_vec()),
                        ] {
                            for hit in memchr::memmem::find_iter(payload, &needle) {
                                let at = start + 20 + hit;
                                let before = i64_at(b, at.wrapping_sub(8))
                                    .filter(|id| (-3_000_000..-1_000_000).contains(id));
                                let key = match before {
                                    Some(id) => format!(
                                        "{whose} class {:#x} u{} after id {id}",
                                        object.class,
                                        width * 8
                                    ),
                                    None => format!(
                                        "{whose} class {:#x} u{} at +{}",
                                        object.class,
                                        width * 8,
                                        at - start
                                    ),
                                };
                                keys.entry(key).or_default().insert(*tag);
                            }
                        }
                    }
                }
            }
        }
    }
    let mut ranked: Vec<(&String, &BTreeSet<u32>)> = keys.iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"elements\":{},\"materials\":{:?},\"keys\":{}}}",
        elements.len(),
        materials_found,
        keys.len()
    )];
    for (key, tags) in ranked.into_iter().take(KEYS_SHOWN) {
        out.push(format!("{{\"key\":{key:?},\"elements\":{}}}", tags.len()));
    }
    Ok(out)
}

fn main() {
    // Measure passes flags such as `--records` after the paths.
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        match probe(path) {
            Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
