//! RE-165 (probe): where a pipe fitting's nominal size is stored, for Revit's
//! `Pset_PipeFittingTypeCommon.NominalDiameter` (B44, #35).
//!
//! Revit's IFC4 export of RE1 Plumbing gives each of its 42 pipe fittings a
//! `NominalDiameter` (10 to 50 mm). rvt-rs writes none.
//!
//! From the reference export next to the model the probe takes each element
//! with a `NominalDiameter` and its `Tag`. In the verified data objects whose
//! header holds that ElementId (RE-153) it looks for an `f64` within
//! [`TOLERANCE_FEET`] of the diameter or of its half, in feet. Every hit is
//! keyed by what precedes it: the object's class, whether it is the diameter
//! or the radius, and the `i64` just before it (a parameter id where the
//! value is a parameter entry) or else its offset from the object's start.
//! The probe prints how many fittings each key explains, so a key that
//! explains all of them is the stored value. For four fittings ([`DUMPED`])
//! it also prints every hit with the bytes before it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re165_fitting_nominal_size -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_room_parameters::verified_data_object;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// How close a stored value must be to the export's diameter, feet.
const TOLERANCE_FEET: f64 = 1e-6;
/// Keys printed, most fittings first.
const KEYS_SHOWN: usize = 15;
/// Fittings whose every hit is dumped: an elbow and a tee (both found at the
/// common offset), and the cap and the vent tee that are not.
const DUMPED: [u32; 4] = [443813, 443941, 443957, 447853];
/// Bytes shown before each dumped hit.
const DUMP_BEFORE: usize = 64;

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

fn list(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| f.trim().strip_prefix('#')?.parse().ok())
        .collect()
}

/// Feet per the file's length unit, and each element's `Tag` with its
/// `NominalDiameter` in that unit.
fn nominal_diameters(step: &str) -> (f64, BTreeMap<u32, f64>) {
    let ents = entities(step);
    let feet_per_unit = ents
        .values()
        .filter(|(e, a)| e == "IFCSIUNIT" && a.contains(".LENGTHUNIT."))
        .find_map(|(_, a)| {
            Some(match split_args(a).get(2)?.trim() {
                ".MILLI." => 0.001 / 0.3048,
                _ => 1.0 / 0.3048,
            })
        })
        .unwrap_or(1.0 / 0.3048);
    let mut out = BTreeMap::new();
    for (e, a) in ents.values() {
        if e != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(a);
        let Some((_, set_args)) = f
            .get(5)
            .and_then(|s| s.trim().strip_prefix('#')?.parse::<u64>().ok())
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        let diameter = list(
            split_args(set_args)
                .get(4)
                .map(String::as_str)
                .unwrap_or(""),
        )
        .iter()
        .filter_map(|p| ents.get(p))
        .find_map(|(_, pa)| {
            let pf = split_args(pa);
            if pf.first()?.trim() != "'NominalDiameter'" {
                return None;
            }
            let value = pf.get(2)?;
            let inner = value.rsplit_once('(')?.1.split(')').next()?;
            inner.trim().parse::<f64>().ok()
        });
        let Some(diameter) = diameter else {
            continue;
        };
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some((_, oa)) = ents.get(&object) else {
                continue;
            };
            let tag = split_args(oa)
                .get(7)
                .and_then(|t| t.trim().trim_matches('\'').parse::<u32>().ok());
            if let Some(tag) = tag {
                out.insert(tag, diameter);
            }
        }
    }
    (feet_per_unit, out)
}

fn f64_at(b: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
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
    let (feet_per_unit, diameters) = nominal_diameters(&std::fs::read_to_string(&reference)?);
    if diameters.is_empty() {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no NominalDiameter in the reference export\"}}"
        )]);
    }
    let mut rf = RevitFile::open(path)?;
    // key -> the fittings it explains
    let mut keys: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut with_objects: BTreeSet<u32> = BTreeSet::new();
    // For the fittings in DUMPED: every hit's offset, and the bytes before it.
    let mut dumps: Vec<String> = Vec::new();
    for name in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&name) else {
            continue;
        };
        let b = inflated.bytes();
        for (&tag, &diameter) in &diameters {
            let feet = diameter * feet_per_unit;
            for start in memchr::memmem::find_iter(b, &tag.to_le_bytes()) {
                let Some(object) = verified_data_object(b, start) else {
                    continue;
                };
                if object.element_id != tag {
                    continue;
                }
                with_objects.insert(tag);
                for at in start + 20..object.end.saturating_sub(8) {
                    let Some(v) = f64_at(b, at) else { continue };
                    if !v.is_finite() {
                        continue;
                    }
                    let what = if (v - feet).abs() <= TOLERANCE_FEET {
                        "diameter"
                    } else if (v - feet / 2.0).abs() <= TOLERANCE_FEET {
                        "radius"
                    } else {
                        continue;
                    };
                    let before = i64_at(b, at.wrapping_sub(8))
                        .filter(|id| (-3_000_000..0).contains(id) || (1..10_000_000).contains(id));
                    let key = match before {
                        Some(id) => format!("class {:#x} {what} after id {id}", object.class),
                        None => format!("class {:#x} {what} at +{}", object.class, at - start),
                    };
                    keys.entry(key).or_default().insert(tag);
                    if DUMPED.contains(&tag) {
                        let from = at.saturating_sub(DUMP_BEFORE);
                        let hex: String = b[from..at + 8]
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect();
                        dumps.push(format!(
                            "{{\"dump\":{tag},\"class\":\"{:#x}\",\"size\":{},\"what\":\"{what}\",\"at\":{},\"hex_before\":\"{hex}\"}}",
                            object.class,
                            object.end - start,
                            at - start
                        ));
                    }
                }
            }
        }
    }
    let mut ranked: Vec<(&String, &BTreeSet<u32>)> = keys.iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let sizes: BTreeSet<String> = diameters.values().map(|d| format!("{d}")).collect();
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"fittings\":{},\"with_own_objects\":{},\"sizes\":{:?},\"keys\":{}}}",
        diameters.len(),
        with_objects.len(),
        sizes,
        keys.len()
    )];
    for (key, tags) in ranked.into_iter().take(KEYS_SHOWN) {
        let missing: Vec<u32> = diameters
            .keys()
            .filter(|tag| !tags.contains(tag))
            .take(5)
            .copied()
            .collect();
        out.push(format!(
            "{{\"key\":{key:?},\"fittings\":{},\"first_missing\":{missing:?}}}",
            tags.len()
        ));
    }
    out.extend(dumps);
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
