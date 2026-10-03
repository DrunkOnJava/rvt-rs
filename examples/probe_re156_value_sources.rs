//! RE-156 (probe): where in the file the text values of Revit's property sets
//! are, when they are not BuiltInParameter text entries (#35).
//!
//! RE-154 looked for each text value of Revit's property sets among the
//! entries `id · u32 n · UTF-16 × n` whose `i64` id is a BuiltInParameter,
//! and found none for `SerialNumber`, `Finish` and others. This probe looks
//! for the values anywhere: in every stream of the file, inflated where it
//! inflates and raw otherwise, as UTF-16LE and as UTF-8.
//!
//! From the reference export next to the model (`<model>_slim.ifc`, else
//! `<model>.ifc`) it takes every text value of every property set on an
//! element with a numeric `Tag`, and for each `(set, property)` up to
//! [`VALUES_PER_PROPERTY`] distinct values of at least [`MIN_LEN`]
//! characters. For each value it prints how many times each encoding occurs
//! in each stream, and for the first [`HITS_SHOWN`] hits: the `u32` before
//! the text (a length prefix when it equals the length), the `i64` before
//! that (a parameter id when the prefix is a length), the 24 bytes before
//! the text, and the innermost verified data object holding it (RE-153), by
//! ElementId and class word. For each positive id found before a
//! length-prefixed value (a project parameter's ElementId), it prints the
//! strings in that id's own data objects. Each property's name is looked for the same
//! way, under the set `(property name)`, since a parameter of that name
//! would hold it in the element defining the parameter.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re156_value_sources -- MODEL.rvt ...

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

/// Every verified data object in `b`: `(start, end, ElementId, class)`.
fn objects(b: &[u8]) -> Vec<(usize, usize, u32, u32)> {
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
                        out.push((p, end, id, class));
                    }
                }
            }
        }
        p += 1;
    }
    out
}

/// Distinct values looked for per `(set, property)`.
const VALUES_PER_PROPERTY: usize = 3;
/// Shortest value looked for: shorter ones meet unrelated bytes by chance.
const MIN_LEN: usize = 3;
/// Hits printed per value.
const HITS_SHOWN: usize = 4;
/// Strings printed per data object of a parameter id.
const STRINGS_SHOWN: usize = 12;

fn i64_at(b: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// One stream's bytes and its verified data objects.
struct Stream {
    name: String,
    inflated: bool,
    bytes: Vec<u8>,
    objects: Vec<(usize, usize, u32, u32)>,
}

fn load(rf: &mut RevitFile) -> Vec<Stream> {
    let mut out = Vec::new();
    for name in rf.stream_names() {
        let inflated = rf
            .inflated_partition(&name)
            .map(|s| s.bytes().to_vec())
            .unwrap_or_default();
        let (bytes, was_inflated) = if inflated.is_empty() {
            (rf.read_stream(&name).unwrap_or_default(), false)
        } else {
            (inflated, true)
        };
        let objects = if name.starts_with("Partitions/") || name.starts_with("Global/") {
            objects(&bytes)
        } else {
            Vec::new()
        };
        out.push(Stream {
            name,
            inflated: was_inflated,
            bytes,
            objects,
        });
    }
    out
}

/// Hit counts by stream and encoding, the hits shown, and the positive ids
/// before length-prefixed hits.
type Hits = (BTreeMap<String, usize>, Vec<String>, BTreeSet<i64>);

fn hits(streams: &[Stream], value: &str) -> Hits {
    let utf16: Vec<u8> = value.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let units = value.encode_utf16().count();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown = Vec::new();
    let mut ids = BTreeSet::new();
    for stream in streams {
        for (encoding, needle) in [("utf16", utf16.as_slice()), ("utf8", value.as_bytes())] {
            for at in memchr::memmem::find_iter(&stream.bytes, needle) {
                let b = &stream.bytes;
                *counts
                    .entry(format!("{} {encoding}", stream.name))
                    .or_default() += 1;
                let prefix = at.checked_sub(4).and_then(|p| u32_at(b, p));
                let expected = if encoding == "utf16" {
                    units
                } else {
                    value.len()
                };
                let length_prefixed = prefix == Some(expected as u32);
                let id = if length_prefixed {
                    at.checked_sub(12).and_then(|p| i64_at(b, p))
                } else {
                    None
                };
                ids.extend(id.filter(|i| *i > 0));
                if shown.len() >= HITS_SHOWN {
                    continue;
                }
                let index = stream.objects.partition_point(|(start, ..)| *start <= at);
                let owner = stream.objects[..index]
                    .iter()
                    .rev()
                    .find(|(_, end, ..)| *end > at)
                    .map(|(start, _, id, class)| format!("{id} class {class:#x} at {start}"));
                let before = &b[at.saturating_sub(24)..at];
                shown.push(format!(
                    "{{\"stream\":{:?},\"inflated\":{},\"encoding\":{encoding:?},\"at\":{at},\
                     \"length_prefixed\":{length_prefixed},\"id\":{},\"object\":{},\"before\":{:?}}}",
                    stream.name,
                    stream.inflated,
                    id.map_or("null".into(), |i| i.to_string()),
                    owner.map_or("null".into(), |o| format!("{o:?}")),
                    hex(before)
                ));
            }
        }
    }
    (counts, shown, ids)
}

/// The length-prefixed UTF-16 strings (`u32 n · UTF-16 × n`, 2 to 64
/// printable characters) inside every verified data object of ElementId
/// `id`, by stream, up to [`STRINGS_SHOWN`] per object.
fn object_strings(streams: &[Stream], id: i64) -> Vec<String> {
    let Ok(id) = u32::try_from(id) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for stream in streams {
        for (start, end, _, class) in stream.objects.iter().filter(|o| o.2 == id) {
            let b = &stream.bytes[*start..*end];
            let mut strings = Vec::new();
            let mut at = 0;
            while at + 4 <= b.len() && strings.len() < STRINGS_SHOWN {
                let n = u32_at(b, at).unwrap_or(0) as usize;
                let raw = if (2..=64).contains(&n) {
                    b.get(at + 4..at + 4 + 2 * n)
                } else {
                    None
                };
                let text = raw
                    .and_then(|raw| {
                        let units: Vec<u16> = raw
                            .chunks_exact(2)
                            .map(|p| u16::from_le_bytes([p[0], p[1]]))
                            .collect();
                        String::from_utf16(&units).ok()
                    })
                    .filter(|t| t.chars().all(|c| !c.is_control() && (c as u32) < 0x3000));
                match text {
                    Some(text) => {
                        at += 4 + 2 * n;
                        strings.push(text);
                    }
                    None => at += 1,
                }
            }
            out.push(format!(
                "{{\"stream\":{:?},\"object\":{id},\"class\":\"{class:#x}\",\"at\":{start},\"strings\":{strings:?}}}",
                stream.name
            ));
        }
    }
    out
}

/// (set, property) -> value -> (an element carrying it, that element's type).
type Samples = BTreeMap<(String, String), BTreeMap<String, (u32, Option<u32>)>>;

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
    let mut values: Samples = BTreeMap::new();
    for w in &written {
        let slot = values
            .entry((w.set.clone(), w.property.clone()))
            .or_default();
        if w.value.chars().count() >= MIN_LEN
            && (slot.len() < VALUES_PER_PROPERTY || slot.contains_key(&w.value))
        {
            slot.entry(w.value.clone())
                .or_insert((w.element, w.type_id));
        }
        // The property's own name, as a parameter of that name would hold
        // it in the element that defines the parameter.
        values
            .entry(("(property name)".to_string(), w.property.clone()))
            .or_default()
            .entry(w.property.clone())
            .or_insert((w.element, w.type_id));
    }
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let streams = load(&mut rf);
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"streams\":{},\"properties\":{}}}",
        streams.len(),
        values.len()
    )];
    let mut parameter_ids = BTreeSet::new();
    for ((set, property), sample) in &values {
        for (value, (element, type_id)) in sample {
            let (counts, shown, ids) = hits(&streams, value);
            parameter_ids.extend(ids);
            let counts: Vec<String> = counts.iter().map(|(k, n)| format!("{k:?}:{n}")).collect();
            out.push(format!(
                "{{\"set\":{set:?},\"property\":{property:?},\"value\":{value:?},\
                 \"element\":{element},\"type\":{},\"counts\":{{{}}},\"hits\":[{}]}}",
                type_id.map_or("null".into(), |t| t.to_string()),
                counts.join(","),
                shown.join(",")
            ));
        }
    }
    for id in parameter_ids {
        let objects = object_strings(&streams, id);
        out.push(format!(
            "{{\"parameter\":{id},\"objects\":[{}]}}",
            objects.join(",")
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
