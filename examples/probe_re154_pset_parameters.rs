//! RE-154 (probe): which stored text parameter each text property of Revit's
//! common property sets comes from (#35).
//!
//! Revit's IFC4 export writes standard property sets (`Pset_WallCommon`,
//! `Pset_DoorCommon`, `Pset_CoveringCommon` ...) whose text values
//! (`IfcLabel`, `IfcText`, `IfcIdentifier`) come from the element's
//! parameters or its type's. A text parameter is stored as an entry
//! `id · u32 n · UTF-16 × n`, `id` an `i64` BuiltInParameter (RE-117), inside
//! a data object `[i32 ElementId][i32 0][u32 Adler-32][i32 size][i32 class]`
//! whose payload ends in its size again and whose Adler-32 verifies (RE-153).
//!
//! The probe lists every verified object of every partition in one pass, and
//! every text entry whose id is a BuiltInParameter (-1,000,000 to
//! -2,000,000) with the innermost object holding it. From the reference export
//! next to the model (`<model>_slim.ifc`, else `<model>.ifc`) it takes every
//! element with a numeric `Tag`, its type's `Tag` (`IfcRelDefinesByType`),
//! and every text value of its property sets. For each `(set, property)` it
//! prints how many elements carry it, how many values are found among the
//! element's own entries and among its type's, and the parameter ids that
//! hold them.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re154_pset_parameters -- MODEL.rvt ...

use rvt::RevitFile;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

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

/// A STEP string literal's text, with `''` and `\X\hh` / `\X2\…\X0\` decoded.
fn decode(raw: &str) -> String {
    let raw = raw.trim().trim_matches('\'');
    let mut out = String::new();
    let mut rest = raw;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(tail) = rest.strip_prefix("\\X2\\") {
            let end = tail.find("\\X0\\").unwrap_or(tail.len());
            let units: Vec<u16> = tail.as_bytes()[..end]
                .chunks(4)
                .filter_map(|c| u16::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
                .collect();
            out.push_str(&String::from_utf16_lossy(&units));
            rest = tail.get(end + 4..).unwrap_or("");
        } else if let Some(tail) = rest.strip_prefix("\\X\\") {
            let byte = tail.get(..2).and_then(|h| u8::from_str_radix(h, 16).ok());
            out.push(byte.map_or('?', char::from));
            rest = tail.get(2..).unwrap_or("");
        } else {
            out.push('\\');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out.replace("''", "'")
}

/// One text property value Revit wrote on an element.
struct Written {
    element: u32,
    type_id: Option<u32>,
    set: String,
    property: String,
    value: String,
}

/// Every text property value on an element with a numeric `Tag`.
fn written(step: &str) -> Vec<Written> {
    let ents = entities(step);
    let tag_of = |id: u64| -> Option<u32> {
        let (entity, args) = ents.get(&id)?;
        if entity.ends_with("TYPE") || entity == "IFCOPENINGELEMENT" {
            return None;
        }
        split_args(args)
            .get(7)
            .map(|t| decode(t))
            .and_then(|t| t.parse().ok())
    };
    let mut type_of: BTreeMap<u64, u32> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let f = split_args(args);
        let Some(type_tag) = f
            .get(5)
            .and_then(|t| reference(t))
            .and_then(|t| ents.get(&t))
            .and_then(|(_, a)| split_args(a).get(7).map(|t| decode(t)))
            .and_then(|t| t.parse().ok())
        else {
            continue;
        };
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            type_of.insert(object, type_tag);
        }
    }
    let mut out = Vec::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(args);
        let Some((set_entity, set_args)) = f
            .get(5)
            .and_then(|s| reference(s))
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET" {
            continue;
        }
        let sf = split_args(set_args);
        let set = sf.get(2).map(|n| decode(n)).unwrap_or_default();
        let mut values = Vec::new();
        for prop in list(sf.get(4).map(String::as_str).unwrap_or("")) {
            let Some((pe, pa)) = ents.get(&prop) else {
                continue;
            };
            if pe != "IFCPROPERTYSINGLEVALUE" {
                continue;
            }
            let pf = split_args(pa);
            let (Some(name), Some(value)) = (pf.first(), pf.get(2)) else {
                continue;
            };
            let value = value.trim();
            let text = ["IFCLABEL(", "IFCTEXT(", "IFCIDENTIFIER("]
                .iter()
                .find_map(|p| value.strip_prefix(p))
                .map(|v| decode(v.trim_end_matches(')')));
            if let Some(text) = text.filter(|t| !t.is_empty()) {
                values.push((decode(name), text));
            }
        }
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some(element) = tag_of(object) else {
                continue;
            };
            for (property, value) in &values {
                out.push(Written {
                    element,
                    type_id: type_of.get(&object).copied(),
                    set: set.clone(),
                    property: property.clone(),
                    value: value.clone(),
                });
            }
        }
    }
    out
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut s) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += u32::from(x);
            s += a;
        }
        a %= 65521;
        s %= 65521;
    }
    (s << 16) | a
}

/// Every verified data object in `b`: `(start, end, ElementId)`.
fn objects(b: &[u8]) -> Vec<(usize, usize, u32)> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + 20 <= b.len() {
        if u32_at(b, p + 4) == Some(0) {
            if let (Some(id), Some(sum), Some(size), Some(class)) = (
                u32_at(b, p),
                u32_at(b, p + 8),
                u32_at(b, p + 12),
                u32_at(b, p + 16),
            ) {
                let size = size as usize;
                let end = p + 20 + size;
                if size >= 4 && end <= b.len() && u32_at(b, end - 4) == Some(size as u32) {
                    let mut data = class.to_le_bytes().to_vec();
                    data.extend_from_slice(&b[p + 20..end]);
                    data.truncate(size);
                    if adler32(&data) == sum {
                        out.push((p, end, id));
                    }
                }
            }
        }
        p += 1;
    }
    out
}

fn string_at(b: &[u8], at: usize) -> Option<String> {
    let n = u32_at(b, at)? as usize;
    if !(1..=512).contains(&n) {
        return None;
    }
    let units: Vec<u16> = b
        .get(at + 4..at + 4 + 2 * n)?
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    (!text.chars().any(char::is_control)).then_some(text)
}

#[derive(Default)]
struct Tally {
    written: usize,
    on_element: usize,
    on_type: usize,
    params: BTreeMap<String, usize>,
    unmatched: Vec<String>,
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
    let written = written(&std::fs::read_to_string(&reference)?);
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    if revit < 2024 {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"revit\":{revit},\"skipped\":\"before 2024\"}}"
        )]);
    }
    // object ElementId -> value -> parameter ids
    let mut entries: BTreeMap<u32, BTreeMap<String, BTreeSet<i64>>> = BTreeMap::new();
    let mut objects_seen = 0usize;
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let b = inflated.bytes();
        let objs = objects(b);
        objects_seen += objs.len();
        let mut at = 0;
        while at + 12 <= b.len() {
            let id = i64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"));
            if (-2_000_000..=-1_000_000).contains(&id) {
                if let Some(value) = string_at(b, at + 8) {
                    // the innermost object holding the entry: the latest start
                    // before it whose end is past it
                    let index = objs.partition_point(|(start, _, _)| *start <= at);
                    if let Some((_, _, owner)) =
                        objs[..index].iter().rev().find(|(_, end, _)| *end > at)
                    {
                        entries
                            .entry(*owner)
                            .or_default()
                            .entry(value)
                            .or_default()
                            .insert(id);
                    }
                }
            }
            at += 1;
        }
    }
    let mut tallies: BTreeMap<(String, String), Tally> = BTreeMap::new();
    for w in &written {
        let tally = tallies
            .entry((w.set.clone(), w.property.clone()))
            .or_default();
        tally.written += 1;
        let own = entries.get(&w.element).and_then(|e| e.get(&w.value));
        let typed = w
            .type_id
            .and_then(|t| entries.get(&t))
            .and_then(|e| e.get(&w.value));
        tally.on_element += usize::from(own.is_some());
        tally.on_type += usize::from(typed.is_some());
        for (place, ids) in [("element", own), ("type", typed)] {
            for id in ids.into_iter().flatten() {
                *tally.params.entry(format!("{place} {id}")).or_default() += 1;
            }
        }
        if own.is_none() && typed.is_none() && tally.unmatched.len() < 3 {
            tally.unmatched.push(format!("{}:{:?}", w.element, w.value));
        }
    }
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"verified_objects\":{objects_seen},\
         \"objects_with_text_entries\":{},\"written_text_values\":{}}}",
        entries.len(),
        written.len()
    )];
    for ((set, property), t) in &tallies {
        let params: Vec<String> = t.params.iter().map(|(k, v)| format!("{k:?}:{v}")).collect();
        out.push(format!(
            "{{\"set\":{set:?},\"property\":{property:?},\"written\":{},\"on_element\":{},\"on_type\":{},\
             \"params\":{{{}}},\"unmatched\":{:?}}}",
            t.written,
            t.on_element,
            t.on_type,
            params.join(","),
            t.unmatched
        ));
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
