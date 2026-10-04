//! B55 (probe): wall 55840's edited elevation profile and the opening Revit's
//! export cuts from it (RE-151's 63rd unfilled opening, #227).
//!
//! Prints Revit's opening tagged 55859 with every entity it reaches (its
//! placement, its shape and the relation voiding the wall), Revit's wall
//! 55840 and its placement, the wall's own element record, and every sketch
//! line record that names the wall as its owner or that a sketch's curve
//! list puts under it: its box, and its curve read as a line or an arc.
//!
//! Usage: probe_b55_wall_profile <2024_Core_Interior.rvt>

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const WALL: u32 = 55840;
const OPENING: u32 = 55859;

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

/// Print `root` and every entity it reaches, skipping owner histories and
/// the shared contexts, each once.
fn print_tree(ents: &BTreeMap<u64, (String, String)>, root: u64, label: &str) {
    let mut seen = BTreeSet::new();
    let mut stack = vec![(root, 0usize)];
    while let Some((id, depth)) = stack.pop() {
        if !seen.insert(id) || depth > 12 {
            continue;
        }
        let Some((entity, args)) = ents.get(&id) else {
            continue;
        };
        if matches!(
            entity.as_str(),
            "IFCOWNERHISTORY"
                | "IFCGEOMETRICREPRESENTATIONCONTEXT"
                | "IFCGEOMETRICREPRESENTATIONSUBCONTEXT"
                | "IFCBUILDINGSTOREY"
        ) {
            continue;
        }
        println!(
            "{label} {}#{id}={entity}({}",
            "  ".repeat(depth),
            args.chars().take(260).collect::<String>()
        );
        for child in refs(args).into_iter().rev() {
            stack.push((child, depth + 1));
        }
    }
}

fn reference_export(model: &Path) -> Option<String> {
    let stem = model.file_stem()?.to_string_lossy().to_string();
    let dir = model.parent()?;
    [
        dir.join(format!("{stem}_slim.ifc")),
        dir.join(format!("../IFC Exports/{stem}_slim.ifc")),
        dir.join(format!("{stem}.ifc")),
    ]
    .iter()
    .find_map(|path| std::fs::read_to_string(path).ok())
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    if !path.to_string_lossy().contains("Core_Interior") {
        println!("not Core Interior");
        return Ok(());
    }
    if let Some(step) = reference_export(&path) {
        let ents = entities(&step);
        let tagged = |tag: u32| {
            ents.iter()
                .filter(move |(_, (_, args))| args.rsplit('\'').nth(1) == Some(&tag.to_string()))
                .map(|(id, (entity, _))| (*id, entity.clone()))
                .collect::<Vec<_>>()
        };
        for (id, entity) in tagged(OPENING) {
            println!("revit opening {entity} #{id}");
            print_tree(&ents, id, "opening");
            for (rel, (entity, args)) in &ents {
                if entity == "IFCRELVOIDSELEMENT" && refs(args).contains(&id) {
                    println!("revit void relation #{rel}={entity}({args}");
                }
            }
        }
        for (id, entity) in tagged(WALL) {
            if entity.starts_with("IFCWALL") {
                println!("revit wall {entity} #{id}");
                print_tree(&ents, id, "wall");
            }
        }
    } else {
        println!("no reference export found");
    }

    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = per::scan_category_records_multi(
        &mut rf,
        version,
        &[per::OST_WALLS, per::OST_SKETCH_LINES],
        &declared,
    )?;
    for record in records.iter().filter(|r| r.element_id == WALL) {
        println!(
            "wall record {}@{}: bbox {:?} owner {:?} preceding {:?} class {:#x}",
            record.stream,
            record.offset,
            record.bbox_feet,
            record.owner_reference,
            record.preceding_reference,
            record.class_tag
        );
    }
    let sketch: Vec<&per::PartitionElementRecord> = records
        .iter()
        .filter(|r| r.builtin_category == per::OST_SKETCH_LINES)
        .collect();
    let line_ids: BTreeSet<u32> = sketch.iter().map(|r| r.element_id).collect();
    let lists =
        rvt::element_record_plan_profiles::scan_sketch_curve_lists(&mut rf, version, &line_ids);
    let mut wanted: BTreeSet<u32> = sketch
        .iter()
        .filter(|r| r.owner_reference == Some(WALL))
        .map(|r| r.element_id)
        .collect();
    for (owner, curves) in &lists {
        if *owner == WALL || curves.iter().any(|id| wanted.contains(id)) {
            println!("curve list of {owner}: {curves:?}");
            wanted.extend(curves.iter().copied());
        }
    }
    wanted.insert(OPENING);
    let curves =
        rvt::partition_beam_axes::scan_sketch_curves(&mut rf, version, &wanted).unwrap_or_default();
    let mut printed = BTreeSet::new();
    for record in sketch.iter().filter(|r| wanted.contains(&r.element_id)) {
        if !printed.insert(record.element_id) {
            continue;
        }
        let curve = curves.get(&record.element_id);
        println!(
            "sketch line {} owner {:?}: bbox {:?}; line {:?}; arc {:?}",
            record.element_id,
            record.owner_reference,
            record.bbox_feet.map(|v| (v * 1e4).round() / 1e4),
            curve.and_then(|c| c.line).map(|l| (l.start(), l.end())),
            curve.and_then(|c| c.arc).map(|a| (a.centre, a.radius)),
        );
    }
    println!("{} sketch lines under the wall", printed.len());
    Ok(())
}
