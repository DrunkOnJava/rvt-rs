//! RE-157 (probe): where a pipe's inner diameter is stored, for Revit's
//! `Pset_FlowSegmentPipeSegment.InvertElevation` (#35).
//!
//! Revit's IFC4 exports of RE1 Mechanical and Plumbing give every pipe an
//! `InvertElevation`: the height of its inside bottom above its storey, its
//! axis's height less half its inner diameter (pipe 443719: axis 2,850 mm,
//! invert 2,844.67 mm, outer radius 6.35 mm). rvt-rs reads the axis (RE-131)
//! and the nominal size (RE-134), not the inner diameter.
//!
//! From the reference export next to the model the probe takes each pipe
//! segment with an `InvertElevation` and an extruded circle whose
//! extrusion is horizontal, and its inner diameter as twice its axis height
//! (the solid's position, in its storey's frame) less the invert. For each
//! distinct inner diameter it looks, in every partition, for an `f64` within
//! [`TOLERANCE_FEET`] of it in feet, and prints the hits: the innermost
//! verified data object holding each (RE-153) and the `f64`s around it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re157_pipe_inner_diameter -- MODEL.rvt ...

use rvt::RevitFile;
use std::collections::BTreeMap;
use std::path::Path;

/// How close a stored value must be to the export's inner diameter, feet
/// (the export writes millimetres to about 1e-9).
const TOLERANCE_FEET: f64 = 1e-6;
/// Hits printed per diameter.
const HITS_SHOWN: usize = 6;

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

fn numbers(field: &str) -> Vec<f64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| f.trim().parse().ok())
        .collect()
}

/// One pipe of the export: its `Tag`, axis height, outer radius and invert,
/// in the file's length unit.
struct Pipe {
    tag: String,
    axis: f64,
    outer_radius: f64,
    invert: f64,
}

/// Feet per the file's length unit, and every horizontal extruded-circle
/// pipe segment with an `InvertElevation`.
fn pipes(step: &str) -> (f64, Vec<Pipe>) {
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
    let mut inverts: BTreeMap<u64, f64> = BTreeMap::new();
    for (e, a) in ents.values() {
        if e != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(a);
        let Some((_, set_args)) = f
            .get(5)
            .and_then(|s| reference(s))
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        let invert = list(
            split_args(set_args)
                .get(4)
                .map(String::as_str)
                .unwrap_or(""),
        )
        .iter()
        .filter_map(|p| ents.get(p))
        .find_map(|(_, pa)| {
            let pf = split_args(pa);
            if pf.first()?.trim() != "'InvertElevation'" {
                return None;
            }
            pf.get(2)?
                .split_once('(')?
                .1
                .trim_end_matches(')')
                .parse()
                .ok()
        });
        if let Some(invert) = invert {
            for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
                inverts.insert(object, invert);
            }
        }
    }
    let mut out = Vec::new();
    for (id, invert) in inverts {
        let Some((_, a)) = ents.get(&id) else {
            continue;
        };
        let f = split_args(a);
        let tag = f
            .get(7)
            .map(|t| t.trim().trim_matches('\'').to_string())
            .unwrap_or_default();
        // Representation -> shape -> solid.
        let solid = f
            .get(6)
            .and_then(|r| reference(r))
            .and_then(|r| ents.get(&r))
            .and_then(|(_, ra)| list(split_args(ra).get(2)?).first().copied())
            .and_then(|s| ents.get(&s))
            .and_then(|(_, sa)| list(split_args(sa).get(3)?).first().copied())
            .and_then(|s| ents.get(&s));
        let Some((solid_entity, solid_args)) = solid else {
            continue;
        };
        if solid_entity != "IFCEXTRUDEDAREASOLID" {
            continue;
        }
        let sf = split_args(solid_args);
        let profile = sf
            .first()
            .and_then(|p| reference(p))
            .and_then(|p| ents.get(&p));
        let Some(("IFCCIRCLEPROFILEDEF", profile_args)) =
            profile.map(|(e, pa)| (e.as_str(), pa.as_str()))
        else {
            continue;
        };
        let Some(radius) = split_args(profile_args)
            .get(3)
            .and_then(|r| r.trim().parse().ok())
        else {
            continue;
        };
        let Some((_, position)) = sf
            .get(1)
            .and_then(|p| reference(p))
            .and_then(|p| ents.get(&p))
        else {
            continue;
        };
        let pf = split_args(position);
        let Some(z) = pf
            .first()
            .and_then(|p| reference(p))
            .and_then(|p| ents.get(&p))
            .and_then(|(_, pa)| numbers(split_args(pa).first()?).get(2).copied())
        else {
            continue;
        };
        // A horizontal extrusion: the position's local z axis is not up.
        let axis_up = pf
            .get(1)
            .and_then(|d| reference(d))
            .and_then(|d| ents.get(&d))
            .and_then(|(_, da)| numbers(split_args(da).first()?).get(2).copied())
            .is_none_or(|up| up.abs() > 0.5);
        if axis_up {
            continue;
        }
        out.push(Pipe {
            tag,
            axis: z,
            outer_radius: radius,
            invert,
        });
    }
    (feet_per_unit, out)
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn f64_at(b: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
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
    let (feet_per_unit, pipes) = pipes(&std::fs::read_to_string(&reference)?);
    if pipes.is_empty() {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no horizontal pipe with an InvertElevation\"}}"
        )]);
    }
    // inner diameter (feet, rounded to 1e-6) -> (outer diameter, tags)
    let mut diameters: BTreeMap<i64, (f64, Vec<String>)> = BTreeMap::new();
    for pipe in &pipes {
        let inner = 2.0 * (pipe.axis - pipe.invert) * feet_per_unit;
        let slot = diameters
            .entry((inner * 1e6).round() as i64)
            .or_insert((2.0 * pipe.outer_radius * feet_per_unit, Vec::new()));
        slot.1.push(pipe.tag.clone());
    }
    let mut rf = RevitFile::open(path)?;
    let mut streams = Vec::new();
    for name in rf.partition_stream_names() {
        if let Ok(inflated) = rf.inflated_partition(&name) {
            let bytes = inflated.bytes().to_vec();
            let objs = objects(&bytes);
            streams.push((name, bytes, objs));
        }
    }
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"pipes\":{},\"inner_diameters\":{}}}",
        pipes.len(),
        diameters.len()
    )];
    for (key, (outer, tags)) in &diameters {
        let inner = *key as f64 / 1e6;
        let mut count = 0usize;
        let mut shown = Vec::new();
        for (name, b, objs) in &streams {
            for at in 0..b.len().saturating_sub(8) {
                let Some(v) = f64_at(b, at) else { continue };
                // NaN compares false either way, so test for the match.
                if !v.is_finite() || (v - inner).abs() > TOLERANCE_FEET {
                    continue;
                }
                count += 1;
                if shown.len() >= HITS_SHOWN {
                    continue;
                }
                let index = objs.partition_point(|(start, ..)| *start <= at);
                let owner = objs[..index]
                    .iter()
                    .rev()
                    .find(|(_, end, ..)| *end > at)
                    .map(|(start, _, id, class)| format!("{id} class {class:#x} at {start}"));
                let around: Vec<String> = [-24i64, -16, -8, 8, 16, 24]
                    .iter()
                    .map(|d| {
                        usize::try_from(at as i64 + d)
                            .ok()
                            .and_then(|p| f64_at(b, p))
                            .map_or("null".into(), |v| format!("{v:.9}"))
                    })
                    .collect();
                shown.push(format!(
                    "{{\"stream\":{name:?},\"at\":{at},\"object\":{},\"around\":[{}]}}",
                    owner.map_or("null".into(), |o| format!("{o:?}")),
                    around.join(",")
                ));
            }
        }
        out.push(format!(
            "{{\"inner_feet\":{inner:.6},\"inner_mm\":{:.3},\"outer_mm\":{:.3},\"pipes\":{},\
             \"first_tags\":{:?},\"hits\":{count},\"shown\":[{}]}}",
            inner * 304.8,
            outer * 304.8,
            tags.len(),
            tags.iter().take(3).collect::<Vec<_>>(),
            shown.join(",")
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
