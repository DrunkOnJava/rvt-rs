//! RE-185 (probe): where a wall's `IsExternal`, `ExtendToStructure` and
//! `FireRating` come from, and where a family type keeps a text parameter
//! RE-77's scan does not see (#35, B28, B42).
//!
//! RE-158 found a door's and a floor's `IsExternal` in its type's
//! `FUNCTION_PARAM` entry, but no wall type carries one, and on RE1 every
//! wall's booleans are false. RE-154 found RE1's wall `FireRating` ("2") under
//! four ids of the type at once. This probe checks three readings against
//! Revit's own IFC4 export next to each model (`<model>.ifc`):
//!
//! - **IsExternal** is the wall type's native `WallType.m_function` (0
//!   Interior, 1 Exterior), read from the type's native record graph.
//! - **ExtendToStructure** is true where the wall's own record graph holds a
//!   `JoinToRoofGStep` object (its top is attached to a roof).
//! - **FireRating** is the type's text entry -1001206, RE-77's Fire Rating; the
//!   probe prints all four of RE-154's ids so the others can be told apart.
//!
//! For each door or window with a `FireRating` text, it also looks for that
//! text in a family-type parameter table, whose entries are
//! `u32 n · UTF-16 × n · (u32 0 | ff ff ff ff · u16 tag) · f64 · i64
//! ElementId (-1: none) · i64 parameter · u32 · u16`: the text belongs to the
//! parameter id that follows it. That id is named from its definition (the
//! data object holding the `autodesk.parameter.group:` string, RE-156).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re185_wall_function -- MODEL.rvt ...

use rvt::research::partition_room_parameters::{PARAMETER_GROUP_PREFIX, enclosing_data_object};
use rvt::{RevitFile, research::native_document};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// RE-154's four candidate ids of a wall type's `FireRating`.
const FIRE_RATING_CANDIDATES: [i64; 4] = [-1_001_206, -1_001_405, -1_002_500, -1_010_103];

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

fn reference(field: &str) -> Option<u64> {
    field.trim().strip_prefix('#')?.parse().ok()
}

fn list(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| reference(f))
        .collect()
}

fn unquote(raw: &str) -> String {
    raw.trim().trim_matches('\'').replace("''", "'")
}

/// One element of Revit's export with the properties this probe reads.
#[derive(Default)]
struct Exported {
    entity: String,
    /// `IsExternal`, `ExtendToStructure`, `LoadBearing`: `.T.` or `.F.`.
    flags: BTreeMap<String, bool>,
    fire_rating: Option<String>,
}

/// Walls, curtain walls, doors and windows of Revit's export, by `Tag`.
fn exported(step: &str) -> BTreeMap<u32, Exported> {
    const ENTITIES: [&str; 5] = [
        "IFCWALL",
        "IFCWALLSTANDARDCASE",
        "IFCCURTAINWALL",
        "IFCDOOR",
        "IFCWINDOW",
    ];
    let ents = entities(step);
    let mut out: BTreeMap<u32, Exported> = BTreeMap::new();
    let mut tag_of: BTreeMap<u64, u32> = BTreeMap::new();
    for (id, (entity, args)) in &ents {
        if !ENTITIES.contains(&entity.as_str()) {
            continue;
        }
        if let Some(tag) = split_args(args)
            .get(7)
            .and_then(|t| unquote(t).parse().ok())
        {
            tag_of.insert(*id, tag);
            out.entry(tag).or_default().entity = entity.clone();
        }
    }
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(args);
        let Some((_, set_args)) = f
            .get(5)
            .and_then(|s| reference(s))
            .and_then(|s| ents.get(&s))
            .filter(|(e, _)| e == "IFCPROPERTYSET")
        else {
            continue;
        };
        let sf = split_args(set_args);
        let mut flags = BTreeMap::new();
        let mut fire_rating = None;
        for prop in list(sf.get(4).map(String::as_str).unwrap_or("")) {
            let Some((pe, pa)) = ents.get(&prop) else {
                continue;
            };
            if pe != "IFCPROPERTYSINGLEVALUE" {
                continue;
            }
            let pf = split_args(pa);
            let (Some(name), Some(value)) = (pf.first().map(|n| unquote(n)), pf.get(2)) else {
                continue;
            };
            let value = value.trim();
            if let Some(b) = value.strip_prefix("IFCBOOLEAN(") {
                flags.insert(name, b.starts_with(".T."));
            } else if name == "FireRating" {
                fire_rating = value
                    .strip_prefix("IFCLABEL(")
                    .map(|v| unquote(v.trim_end_matches(')')));
            }
        }
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            if let Some(e) = tag_of.get(&object).and_then(|t| out.get_mut(t)) {
                e.flags.extend(flags.clone());
                if fire_rating.is_some() {
                    e.fire_rating.clone_from(&fire_rating);
                }
            }
        }
    }
    out
}

/// A field of a native object, as JSON.
fn field<'a>(fields: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    fields.as_object()?.get(name)
}

/// `{"m_id":{"m_id64":n}}` as `n`.
fn element_id(value: &serde_json::Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(serde_json::Value::as_i64)
        .and_then(|n| u64::try_from(n).ok())
}

/// Per wall: its type (`m_WallAttributesId`) and whether its graph holds a
/// `JoinToRoofGStep`. Per wall type: `m_function`.
struct Native {
    wall_type: BTreeMap<u64, u64>,
    joined_to_roof: BTreeSet<u64>,
    function: BTreeMap<u64, i64>,
}

fn native(rf: &mut RevitFile, ids: &BTreeSet<u64>) -> anyhow::Result<Native> {
    let mut out = Native {
        wall_type: BTreeMap::new(),
        joined_to_roof: BTreeSet::new(),
        function: BTreeMap::new(),
    };
    let pass = |rf: &mut RevitFile, ids: BTreeSet<u64>, out: &mut Native| {
        let options = native_document::Options {
            selected_ids: ids,
            ..native_document::Options::default()
        };
        native_document::extract_graphs(rf, &options, |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            for object in &graph.objects {
                let fields = serde_json::to_value(&object.fields)?;
                if let Some(t) = field(&fields, "m_WallAttributesId").and_then(element_id) {
                    out.wall_type.entry(id).or_insert(t);
                }
                if let Some(f) = field(&fields, "m_function").and_then(serde_json::Value::as_i64) {
                    out.function.entry(id).or_insert(f);
                }
                if object.class_name == "JoinToRoofGStep" {
                    out.joined_to_roof.insert(id);
                }
            }
            Ok(())
        })
    };
    pass(rf, ids.clone(), &mut out)?;
    let types: BTreeSet<u64> = out.wall_type.values().copied().collect();
    pass(rf, types, &mut out)?;
    Ok(out)
}

/// `u32 n · UTF-16 × n` at `at`, 1 to 256 printable code units.
fn utf16_entry(buf: &[u8], at: usize) -> Option<String> {
    let n = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=256).contains(&n) {
        return None;
    }
    let units: Vec<u16> = buf
        .get(at + 4..at + 4 + 2 * n)?
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let s = String::from_utf16(&units).ok()?;
    (!s.chars().any(char::is_control)).then_some(s)
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

/// Each parameter definition's ElementId with its name (RE-156's reading,
/// kept per id so that two definitions named `Fire Rating` stay apart).
fn definition_names(rf: &mut RevitFile) -> BTreeMap<i64, String> {
    let needle: Vec<u8> = PARAMETER_GROUP_PREFIX
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut out = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &needle) {
            let Some(group) = hit.checked_sub(4).and_then(|at| utf16_entry(buf, at)) else {
                continue;
            };
            let after = hit + 2 * group.encode_utf16().count();
            let name = (after..after + 64)
                .find_map(|at| utf16_entry(buf, at).filter(|n| n.encode_utf16().count() >= 2));
            if let (Some(name), Some(id)) = (name, enclosing_data_object(buf, hit)) {
                out.insert(i64::from(id), name);
            }
        }
    }
    out
}

/// Where each text is a family-type table entry: the parameter id after it,
/// with how often, and how often the text is anywhere else.
fn table_entries(
    rf: &mut RevitFile,
    texts: &BTreeSet<String>,
) -> BTreeMap<String, (BTreeMap<i64, usize>, usize)> {
    let mut out: BTreeMap<String, (BTreeMap<i64, usize>, usize)> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for text in texts {
            let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let n = (units.len() / 2) as u32;
            let mut needle = n.to_le_bytes().to_vec();
            needle.extend_from_slice(&units);
            for hit in memchr::memmem::find_iter(buf, &needle) {
                let mut at = hit + needle.len();
                // A zero word, or `ff ff ff ff · u16` (a tag), then the double.
                if buf.get(at..at + 4) == Some(&[0xff; 4])
                    && buf.get(at + 4..at + 6) != Some(&[0xff; 2])
                {
                    at += 6;
                } else if buf.get(at..at + 4) == Some(&[0; 4]) {
                    at += 4;
                } else {
                    out.entry(text.clone()).or_default().1 += 1;
                    continue;
                }
                let entry = (buf.get(at + 8..at + 16) == Some(&[0xff; 8]))
                    .then(|| i64_at(buf, at + 16))
                    .flatten();
                let slot = out.entry(text.clone()).or_default();
                match entry {
                    Some(id) => *slot.0.entry(id).or_default() += 1,
                    None => slot.1 += 1,
                }
            }
        }
    }
    out
}

/// Count one reading against Revit's value: equal, different or unread.
fn score(
    tally: &mut BTreeMap<String, [usize; 3]>,
    key: String,
    revit: Option<bool>,
    ours: Option<bool>,
) {
    let Some(revit) = revit else {
        return;
    };
    let slot = tally.entry(key).or_default();
    match ours {
        Some(o) if o == revit => slot[0] += 1,
        Some(_) => slot[1] += 1,
        None => slot[2] += 1,
    }
}

fn probe(path: &str) -> anyhow::Result<()> {
    let model = Path::new(path);
    let file = model
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ifc = model.with_extension("ifc");
    let Ok(step) = std::fs::read_to_string(&ifc) else {
        println!("{{\"file\":{file:?},\"reference\":null}}");
        return Ok(());
    };
    let exported = exported(&step);
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let walls: BTreeSet<u64> = exported
        .iter()
        .filter(|(_, e)| e.entity.contains("WALL"))
        .map(|(t, _)| u64::from(*t))
        .collect();
    let native = native(&mut rf, &walls)?;
    let types: BTreeSet<u32> = native
        .wall_type
        .values()
        .filter_map(|t| u32::try_from(*t).ok())
        .collect();
    let mut fire: BTreeMap<i64, BTreeMap<u32, String>> = BTreeMap::new();
    for id in FIRE_RATING_CANDIDATES {
        fire.insert(
            id,
            rvt::research::partition_room_parameters::scan_text_parameter(
                &mut rf, revit, &types, id,
            ),
        );
    }
    println!(
        "{{\"file\":{file:?},\"revit\":{revit},\"walls\":{}}}",
        walls.len()
    );

    // Per wall: Revit's values against the three readings.
    let mut tally: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    for wall in &walls {
        let e = &exported[&(*wall as u32)];
        let type_id = native.wall_type.get(wall);
        let function = type_id.and_then(|t| native.function.get(t));
        let is_external = e.flags.get("IsExternal").copied();
        let extend = e.flags.get("ExtendToStructure").copied();
        println!(
            "{{\"wall\":{wall},\"entity\":{:?},\"type\":{},\"m_function\":{},\"join_to_roof\":{},\"IsExternal\":{},\"ExtendToStructure\":{},\"FireRating\":{:?},\"type_text\":{{{}}}}}",
            e.entity,
            type_id.map_or("null".into(), u64::to_string),
            function.map_or("null".into(), i64::to_string),
            native.joined_to_roof.contains(wall),
            is_external.map_or("null".into(), |b| b.to_string()),
            extend.map_or("null".into(), |b| b.to_string()),
            e.fire_rating,
            FIRE_RATING_CANDIDATES
                .iter()
                .map(|id| {
                    let v = type_id
                        .and_then(|t| u32::try_from(*t).ok())
                        .and_then(|t| fire[id].get(&t));
                    format!("\"{id}\":{v:?}")
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        score(
            &mut tally,
            "IsExternal = type m_function == 1".into(),
            is_external,
            function.map(|f| *f == 1),
        );
        if e.entity != "IFCCURTAINWALL" {
            score(
                &mut tally,
                "ExtendToStructure = JoinToRoofGStep".into(),
                extend,
                type_id.map(|_| native.joined_to_roof.contains(wall)),
            );
        }
        if let Some(rating) = &e.fire_rating {
            for id in FIRE_RATING_CANDIDATES {
                let v = type_id
                    .and_then(|t| u32::try_from(*t).ok())
                    .and_then(|t| fire[&id].get(&t));
                let slot = tally.entry(format!("FireRating = type {id}")).or_default();
                match v {
                    Some(v) if v == rating => slot[0] += 1,
                    Some(_) => slot[1] += 1,
                    None => slot[2] += 1,
                }
            }
        }
    }
    for (reading, [equal, differ, missing]) in &tally {
        println!(
            "{{\"reading\":{reading:?},\"equal\":{equal},\"differ\":{differ},\"unread\":{missing}}}"
        );
    }

    // Doors and windows: the family-type table entry holding their FireRating.
    let ratings: BTreeSet<String> = exported
        .values()
        .filter(|e| e.entity == "IFCDOOR" || e.entity == "IFCWINDOW")
        .filter_map(|e| e.fire_rating.clone())
        .collect();
    if !ratings.is_empty() {
        let names = definition_names(&mut rf);
        for (text, (ids, elsewhere)) in table_entries(&mut rf, &ratings) {
            let ids: Vec<String> = ids
                .iter()
                .map(|(id, n)| {
                    format!(
                        "{{\"parameter\":{id},\"name\":{:?},\"entries\":{n}}}",
                        names.get(id)
                    )
                })
                .collect();
            println!(
                "{{\"door_window_fire_rating\":{text:?},\"table_entries\":[{}],\"other_occurrences\":{elsewhere}}}",
                ids.join(",")
            );
        }
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re185_wall_function MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        if let Err(err) = probe(path) {
            println!("{{\"file\":{path:?},\"error\":{:?}}}", err.to_string());
        }
    }
}
