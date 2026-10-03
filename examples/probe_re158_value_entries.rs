//! RE-158 (probe): where Revit keeps the non-text parameter values its
//! property sets carry (B28, B32, #35).
//!
//! RE-154 and RE-156 found the text values: an entry `id · u32 n · UTF-16`
//! in the element's own data object, `id` a BuiltInParameter or a project or
//! shared parameter's ElementId. This probe looks for the others: booleans
//! (`IsExternal`, `LoadBearing`, `ExtendToStructure`) and integers
//! (`NumberOfPoles`).
//!
//! - For each project or shared parameter definition (RE-156), it prints
//!   where its id occurs, the data object holding each hit, and the 16 bytes
//!   after it.
//! - For each element of the reference export with a boolean, logical or
//!   integer property, and for that element's type, it lists every
//!   BuiltInParameter id (-1,000,000 to -2,000,000) in their data objects
//!   whose first or second word after holds a small value (0 to 1,000). For
//!   each `(set, property)` it prints the `(place, id, word)` whose value
//!   equals Revit's on every element that has one, when that is at least
//!   half of the elements Revit gives the property.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re158_value_entries -- MODEL.rvt ...

use rvt::RevitFile;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Hits printed per definition.
const HITS_SHOWN: usize = 6;
/// Candidates printed per property.
const CANDIDATES_SHOWN: usize = 8;

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

/// One non-text property value Revit wrote on an element: booleans and
/// logicals as 1 or 0 (unknown left out), integers as themselves.
struct Written {
    element: u32,
    type_id: Option<u32>,
    set: String,
    property: String,
    value: i64,
}

fn tag(ents: &BTreeMap<u64, (String, String)>, id: u64) -> Option<u32> {
    let (_, args) = ents.get(&id)?;
    split_args(args)
        .get(7)
        .map(|t| decode(t))
        .and_then(|t| t.parse().ok())
}

fn written(step: &str) -> Vec<Written> {
    let ents = entities(step);
    let mut type_of: BTreeMap<u64, u32> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let f = split_args(args);
        let Some(type_tag) = f
            .get(5)
            .and_then(|t| reference(t))
            .and_then(|t| tag(&ents, t))
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
            let Some(("IFCPROPERTYSINGLEVALUE", pa)) =
                ents.get(&prop).map(|(e, a)| (e.as_str(), a.as_str()))
            else {
                continue;
            };
            let pf = split_args(pa);
            let (Some(name), Some(value)) = (pf.first(), pf.get(2)) else {
                continue;
            };
            let value = value.trim();
            let number = if value.ends_with("(.T.)") {
                Some(1)
            } else if value.ends_with("(.F.)") {
                Some(0)
            } else if let Some(n) = value.strip_prefix("IFCINTEGER(") {
                n.trim_end_matches(')').parse().ok()
            } else {
                None
            };
            if let Some(number) = number {
                values.push((decode(name), number));
            }
        }
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some(element) = tag(&ents, object) else {
                continue;
            };
            for (property, value) in &values {
                out.push(Written {
                    element,
                    type_id: type_of.get(&object).copied(),
                    set: set.clone(),
                    property: property.clone(),
                    value: *value,
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

fn i64_at(b: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
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

/// Every verified data object in `b` (RE-153), by ElementId: `(start, end)`.
fn objects(b: &[u8]) -> BTreeMap<u32, Vec<(usize, usize)>> {
    let mut out: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
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
                        out.entry(id).or_default().push((p, end));
                    }
                }
            }
        }
        p += 1;
    }
    out
}

/// Where a candidate value sits: in the element's own object or its type's,
/// after which BuiltInParameter, and which word after it.
type Key = (&'static str, i64, usize);

/// A partition: its name, inflated bytes and verified data objects.
type Stream = (String, Vec<u8>, BTreeMap<u32, Vec<(usize, usize)>>);

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
    let definitions = rvt::partition_room_parameters::find_parameter_definitions(&mut rf, revit);
    let mut streams: Vec<Stream> = Vec::new();
    for name in rf.partition_stream_names() {
        if let Ok(inflated) = rf.inflated_partition(&name) {
            let bytes = inflated.bytes().to_vec();
            let objs = objects(&bytes);
            streams.push((name, bytes, objs));
        }
    }
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"written\":{},\"definitions\":{}}}",
        written.len(),
        definitions.len()
    )];
    // (a) What follows each project or shared parameter's id.
    for (name, id) in &definitions {
        let needle = i64::from(*id).to_le_bytes();
        let mut count = 0usize;
        let mut shown = Vec::new();
        for (stream, b, objs) in &streams {
            for at in memchr::memmem::find_iter(b, &needle) {
                count += 1;
                if shown.len() >= HITS_SHOWN {
                    continue;
                }
                let owner = objs
                    .iter()
                    .find(|(_, spans)| spans.iter().any(|(s, e)| *s < at && at < *e))
                    .map(|(owner, _)| *owner);
                let after = &b[at + 8..(at + 24).min(b.len())];
                shown.push(format!(
                    "{{\"stream\":{stream:?},\"at\":{at},\"object\":{},\"after\":{:?}}}",
                    owner.map_or("null".into(), |o| o.to_string()),
                    hex(after)
                ));
            }
        }
        out.push(format!(
            "{{\"definition\":{name:?},\"id\":{id},\"hits\":{count},\"shown\":[{}]}}",
            shown.join(",")
        ));
    }
    // (b) For each element and its type, every BuiltInParameter entry in its
    // objects with a small value in either of the next two words.
    let mut candidates: BTreeMap<u32, BTreeMap<Key, i64>> = BTreeMap::new();
    let elements: BTreeSet<u32> = written
        .iter()
        .flat_map(|w| [Some(w.element), w.type_id])
        .flatten()
        .collect();
    for (_, b, objs) in &streams {
        for id in &elements {
            for (start, end) in objs.get(id).into_iter().flatten() {
                for at in *start + 20..end.saturating_sub(16) {
                    let Some(bip) = i64_at(b, at) else { continue };
                    if !(-2_000_000..=-1_000_000).contains(&bip) {
                        continue;
                    }
                    for word in [8usize, 12] {
                        if let Some(v) = u32_at(b, at + word).filter(|v| *v <= 1000) {
                            candidates
                                .entry(*id)
                                .or_default()
                                .insert(("own", bip, word), i64::from(v));
                        }
                    }
                }
            }
        }
    }
    // (set, property) -> key -> (agree, total)
    let mut scores: BTreeMap<(String, String), BTreeMap<Key, (usize, usize)>> = BTreeMap::new();
    for w in &written {
        let score = scores
            .entry((w.set.clone(), w.property.clone()))
            .or_default();
        for (place, holder) in [("own", Some(w.element)), ("type", w.type_id)] {
            let Some(found) = holder.and_then(|h| candidates.get(&h)) else {
                continue;
            };
            for ((_, bip, word), value) in found {
                let slot = score.entry((place, *bip, *word)).or_default();
                slot.1 += 1;
                slot.0 += usize::from(*value == w.value);
            }
        }
    }
    for ((set, property), score) in &scores {
        let n = written
            .iter()
            .filter(|w| &w.set == set && &w.property == property)
            .count();
        let values: BTreeSet<i64> = written
            .iter()
            .filter(|w| &w.set == set && &w.property == property)
            .map(|w| w.value)
            .collect();
        let mut best: Vec<(&Key, &(usize, usize))> = score
            .iter()
            .filter(|(_, (agree, total))| agree == total && *total * 2 >= n)
            .collect();
        best.sort_by_key(|(key, (_, total))| (std::cmp::Reverse(*total), **key));
        let best: Vec<String> = best
            .iter()
            .take(CANDIDATES_SHOWN)
            .map(|((place, bip, word), (agree, total))| {
                format!("\"{place} {bip} +{word}\":\"{agree}/{total}\"")
            })
            .collect();
        out.push(format!(
            "{{\"set\":{set:?},\"property\":{property:?},\"written\":{n},\"values\":{values:?},\
             \"candidates\":{{{}}}}}",
            best.join(",")
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
