//! RE-151 (probe): the element records of the openings Revit's export writes
//! with no filling element (#227).
//!
//! Revit's IFC4 export of `2024_Core_Interior.rvt` holds 63 openings no door
//! or window fills: 42 void floor slabs, 20 void shading devices and 1 voids
//! a wall. Each opening's `Tag` is its own ElementId. This reads the
//! reference export next to the model (`<model>_slim.ifc`, else
//! `<model>.ifc`), takes every unfilled `IfcOpeningElement` and the `Tag` of
//! the element it voids, and looks the opening up in the partitions: its
//! chain record (RE-35), the class that record names, and, where the frame
//! decodes, its category, box and reference list. It prints counts by class
//! and category, how often the host's id is in the opening's reference list,
//! and the first ten openings in full.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re151_unfilled_openings -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{self as per};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const EXAMPLES: usize = 10;

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
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
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

/// The last quoted string of an entity's arguments: the `Tag` of an element.
fn tag(args: &str) -> Option<u32> {
    args.rsplit('\'').nth(1)?.parse().ok()
}

/// `(opening tag, host class, host tag)` for every opening nothing fills.
fn unfilled_openings(step: &str) -> Vec<(u32, String, Option<u32>)> {
    let ents = entities(step);
    let mut host_of = BTreeMap::new();
    let mut filled = BTreeSet::new();
    for (entity, args) in ents.values() {
        let r = refs(args);
        match entity.as_str() {
            "IFCRELVOIDSELEMENT" if r.len() >= 2 => {
                host_of.insert(r[r.len() - 1], r[r.len() - 2]);
            }
            "IFCRELFILLSELEMENT" if r.len() >= 2 => {
                filled.insert(r[r.len() - 2]);
            }
            _ => {}
        }
    }
    ents.iter()
        .filter(|(id, (entity, _))| entity == "IFCOPENINGELEMENT" && !filled.contains(*id))
        .filter_map(|(id, (_, args))| {
            let host = host_of.get(id).and_then(|h| ents.get(h));
            Some((
                tag(args)?,
                host.map_or_else(|| "none".to_string(), |(e, _)| e.clone()),
                host.and_then(|(_, a)| tag(a)),
            ))
        })
        .collect()
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
    let openings = unfilled_openings(&std::fs::read_to_string(&reference)?);
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let class_name = |tag: u16| {
        classes
            .classes
            .iter()
            .find(|c| c.tag == tag)
            .map_or_else(|| format!("tag {tag}"), |c| c.name.clone())
    };
    let Some(marker) = per::file_bbox_marker(&mut rf, revit) else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no bbox marker\"}}"
        )]);
    };
    let wanted: BTreeSet<u64> = openings.iter().map(|(t, _, _)| u64::from(*t)).collect();
    // opening id -> (stream, record start, record end, record class, frame)
    let mut found: BTreeMap<
        u64,
        (
            String,
            usize,
            usize,
            String,
            Option<per::PartitionElementRecord>,
        ),
    > = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in per::partition_record_chain(buf, &marker) {
            if !wanted.contains(&span.element_id) {
                continue;
            }
            let frame =
                per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker);
            let class = frame
                .as_ref()
                .map_or_else(|| "undecoded".to_string(), |f| class_name(f.class_tag));
            found.insert(
                span.element_id,
                (stream.clone(), span.start, span.end, class, frame),
            );
        }
    }
    // Every sketch line each host owns, to see which of a loop's lines
    // Revit's opening takes its Tag from.
    let hosts: BTreeSet<u32> = openings.iter().filter_map(|(_, _, h)| *h).collect();
    let mut owned: BTreeMap<u32, Vec<(u64, [f64; 6])>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in per::partition_record_chain(buf, &marker) {
            let Some(f) =
                per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker)
            else {
                continue;
            };
            if let Some(owner) = f.owner_reference.filter(|o| hosts.contains(o)) {
                if f.builtin_category == -2000045 {
                    owned
                        .entry(owner)
                        .or_default()
                        .push((span.element_id, f.bbox_feet));
                }
            }
        }
    }
    let mut owned_lines = Vec::new();
    for (opening, _, host) in openings.iter().take(EXAMPLES) {
        let Some(host) = host else { continue };
        let lines: Vec<String> = owned
            .get(host)
            .into_iter()
            .flatten()
            .map(|(id, b)| format!("[{id},{:.3},{:.3},{:.3},{:.3}]", b[0], b[1], b[3], b[4]))
            .collect();
        owned_lines.push(format!(
            "{{\"host\":{host},\"opening\":{opening},\"owned_sketch_lines\":[{}]}}",
            lines.join(",")
        ));
    }
    let mut by_host: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_category: BTreeMap<i64, usize> = BTreeMap::new();
    let mut lines = Vec::new();
    for (opening, host_class, host) in &openings {
        let entry = by_host.entry(host_class.clone()).or_default();
        entry.0 += 1;
        let Some((stream, start, end, class, frame)) = found.get(&u64::from(*opening)) else {
            continue;
        };
        entry.1 += 1;
        *by_class.entry(class.clone()).or_default() += 1;
        let names_host = frame
            .as_ref()
            .is_some_and(|f| host.is_some_and(|h| f.references.contains(&u64::from(h))));
        entry.2 += usize::from(names_host);
        if let Some(f) = frame {
            *by_category.entry(f.builtin_category).or_default() += 1;
        }
        if lines.len() < EXAMPLES {
            lines.push(format!(
                "{{\"opening\":{opening},\"host_class\":{host_class:?},\"host\":{},\"stream\":{stream:?},\
                 \"record_bytes\":{},\"class\":{class:?},\"frame\":{}}}",
                host.map_or("null".to_string(), |h| h.to_string()),
                end - start,
                frame.as_ref().map_or("null".to_string(), |f| format!(
                    "{{\"category\":{},\"bbox\":{:?},\"references\":{:?},\"owner\":{}}}",
                    f.builtin_category,
                    f.bbox_feet,
                    f.references,
                    f.owner_reference.map_or("null".to_string(), |o| o.to_string())
                ))
            ));
        }
    }
    let by_host: Vec<String> = by_host
        .iter()
        .map(|(h, (n, found, names))| {
            format!("{h:?}:{{\"openings\":{n},\"record_found\":{found},\"names_host\":{names}}}")
        })
        .collect();
    let by_class: Vec<String> = by_class.iter().map(|(c, n)| format!("{c:?}:{n}")).collect();
    let by_category: Vec<String> = by_category
        .iter()
        .map(|(c, n)| format!("\"{c}\":{n}"))
        .collect();
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"unfilled_openings\":{},\"by_host\":{{{}}},\
         \"by_class\":{{{}}},\"by_category\":{{{}}}}}",
        openings.len(),
        by_host.join(","),
        by_class.join(","),
        by_category.join(",")
    )];
    out.extend(lines);
    out.extend(owned_lines);
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
