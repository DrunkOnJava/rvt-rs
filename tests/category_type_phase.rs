//! Spike 257: an element's Category, family and type, and Phase Created, in
//! the property sets and under the names Revit's own IFC export uses
//! (`Other`, `Identity Data`, `Phasing`).
//!
//! For each `<name>.rvt` with Revit's `<name>.ifc` beside it, every value
//! rvt-rs writes for an element (matched by `Tag`) must equal Revit's, and
//! rvt-rs must write Category and Phase Created for at least 99% of the
//! elements Revit writes them for.
//!
//! Runs against `RVT_REVIT2024_SAMPLES_DIR` (Autodesk's Revit 2024 sample
//! projects with Revit 2024 IFC4 exports). Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The checked properties: an element's own, and its type object's (Revit
/// writes Category and Family Name in the type's `Other` set, and Type Name
/// in its `Identity Data`).
const CHECKED: [(&str, &str); 8] = [
    ("Other", "Category"),
    ("Other", "Family"),
    ("Other", "Family Name"),
    ("Other", "Family and Type"),
    ("Other", "Type"),
    ("Other", "Type Id"),
    ("Identity Data", "Type Name"),
    ("Phasing", "Phase Created"),
];

/// Top-level comma-separated attributes of a STEP entity's argument list.
fn attributes(args: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut quoted) = (Vec::new(), String::new(), 0, false);
    for c in args.chars() {
        match c {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out
}

fn refs(attr: &str) -> Vec<u64> {
    attr.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

fn unquote(attr: &str) -> Option<String> {
    let start = attr.find('\'')?;
    let end = attr.rfind('\'')?;
    (end > start).then(|| decode(&attr[start + 1..end]))
}

/// A STEP string's text: `''`, `\X\hh` (ISO 8859-1) and `\X2\hhhh…\X0\`
/// (UTF-16), which Revit's writer and ours use differently.
fn decode(raw: &str) -> String {
    let (mut out, mut rest) = (String::new(), raw.replace("''", "'"));
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        if let Some(hex) = tail.strip_prefix("\\X2\\") {
            let end = hex.find("\\X0\\").unwrap_or(hex.len());
            let units: Vec<u16> = (0..end / 4)
                .filter_map(|i| u16::from_str_radix(&hex[4 * i..4 * i + 4], 16).ok())
                .collect();
            out.push_str(&String::from_utf16_lossy(&units));
            rest = hex.get(end + 4..).unwrap_or("").to_string();
        } else if let Some(hex) = tail.strip_prefix("\\X\\").filter(|h| h.len() >= 2) {
            out.extend(u8::from_str_radix(&hex[..2], 16).ok().map(char::from));
            rest = hex[2..].to_string();
        } else {
            out.push('\\');
            rest = tail[1..].to_string();
        }
    }
    out + &rest
}

/// `Tag -> (set, property) -> value` for the `CHECKED` properties of every
/// element that has a `Tag`.
fn checked_values(step: &str) -> BTreeMap<String, BTreeMap<(String, String), String>> {
    let mut ents: BTreeMap<u64, (String, Vec<String>)> = BTreeMap::new();
    for line in step.lines() {
        let Some((id, body)) = line.strip_prefix('#').and_then(|r| r.split_once('=')) else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let args = args.trim_end().trim_end_matches(';').trim_end_matches(')');
        if let Ok(id) = id.trim().parse() {
            ents.insert(id, (entity.trim().to_string(), attributes(args)));
        }
    }
    let mut out: BTreeMap<String, BTreeMap<(String, String), String>> = BTreeMap::new();
    for (entity, attrs) in ents.values() {
        // An element's own sets, and its type object's (`HasPropertySets`),
        // the latter keyed `type <set>`.
        let (related, sets, prefix) = match entity.as_str() {
            "IFCRELDEFINESBYPROPERTIES" if attrs.len() >= 6 => (&attrs[4], refs(&attrs[5]), ""),
            "IFCRELDEFINESBYTYPE" if attrs.len() >= 6 => {
                let Some((_, ty)) = refs(&attrs[5]).first().and_then(|t| ents.get(t)) else {
                    continue;
                };
                (
                    &attrs[4],
                    ty.get(5).map(|a| refs(a)).unwrap_or_default(),
                    "type ",
                )
            }
            _ => continue,
        };
        for pset in sets.iter().filter_map(|p| ents.get(p)).map(|(_, a)| a) {
            let values = set_values(&ents, pset, prefix);
            for element in refs(related) {
                let Some(tag) = ents
                    .get(&element)
                    .and_then(|(_, a)| a.get(7))
                    .and_then(|t| unquote(t))
                    .filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
                else {
                    continue;
                };
                out.entry(tag).or_default().extend(values.iter().cloned());
            }
        }
    }
    out
}

/// The `CHECKED` values of the `IfcPropertySet` with attributes `pset`, its
/// set name prefixed with `prefix`.
fn set_values(
    ents: &BTreeMap<u64, (String, Vec<String>)>,
    pset: &[String],
    prefix: &str,
) -> Vec<((String, String), String)> {
    let (Some(set), Some(props)) = (pset.get(2).and_then(|a| unquote(a)), pset.get(4)) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for p in refs(props) {
        let Some((e, a)) = ents.get(&p) else { continue };
        if e != "IFCPROPERTYSINGLEVALUE" {
            continue;
        }
        let (Some(name), Some(value)) = (unquote(&a[0]), a.get(2).and_then(|v| unquote(v))) else {
            continue;
        };
        if CHECKED.contains(&(set.as_str(), name.as_str())) {
            values.push(((format!("{prefix}{set}"), name), value));
        }
    }
    values
}

#[test]
fn category_type_and_phase_match_revits_export() {
    let Some(dir) = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return;
    };
    let mut files = 0;
    for entry in std::fs::read_dir(&dir).expect("samples dir") {
        let rvt = entry.expect("entry").path();
        let reference = rvt.with_extension("ifc");
        if rvt.extension().is_none_or(|e| e != "rvt") || !reference.exists() {
            continue;
        }
        files += 1;
        let theirs = checked_values(&std::fs::read_to_string(&reference).expect("reference"));
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let ours = checked_values(&write_step(&result.model));
        let mut wrong = Vec::new();
        let mut right: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut revit: BTreeMap<(String, String), usize> = BTreeMap::new();
        for (tag, values) in &ours {
            let Some(want) = theirs.get(tag) else {
                continue;
            };
            for key in want.keys() {
                *revit.entry(key.clone()).or_default() += 1;
            }
            for (key, value) in values {
                match want.get(key) {
                    Some(w) if w == value => *right.entry(key.clone()).or_default() += 1,
                    Some(w) => wrong.push((tag.clone(), key.clone(), w.clone(), value.clone())),
                    None => {}
                }
            }
        }
        eprintln!("{}: right {right:?} of {revit:?}", rvt.display());
        assert!(
            wrong.is_empty(),
            "{}: {:?}",
            rvt.display(),
            &wrong[..wrong.len().min(5)]
        );
        for key in [("Other", "Category"), ("Phasing", "Phase Created")] {
            let key = (key.0.to_string(), key.1.to_string());
            let (r, n) = (
                right.get(&key).copied().unwrap_or(0),
                revit.get(&key).copied().unwrap_or(0),
            );
            assert!(
                n > 0 && r * 100 >= n * 99,
                "{}: {key:?} {r} of {n}",
                rvt.display()
            );
        }
    }
    if files == 0 {
        eprintln!("skipping: no .rvt with a Revit .ifc in {}", dir.display());
    }
}
