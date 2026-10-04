//! B44 follow-up: where a pipe fitting's NominalDiameter is lost between the
//! connector scan and the written IFC. For the fittings Revit's export gives a
//! `NominalDiameter` (the model's `.ifc` beside it), prints how many the scan
//! finds, how many the schema pass and the walker carry the field for, how
//! many property sets the exporter builds, and how many the STEP text holds.
//!
//! Usage: probe_b44_stages <model.rvt>

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{RvtDocExporter, write_step};
use rvt::partition_schema_mvp::FITTING_NOMINAL_DIAMETER_FIELD;
use rvt::walker::WalkerLimits;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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

fn references(field: &str) -> Vec<u64> {
    field
        .split(|c: char| !(c == '#' || c.is_ascii_digit()))
        .filter_map(|f| f.strip_prefix('#')?.parse().ok())
        .collect()
}

fn tag_of(args: &str) -> Option<u32> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut field = 0;
    let mut current = String::new();
    for c in args.chars() {
        match c {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                if field == 7 {
                    break;
                }
                field += 1;
                current.clear();
                continue;
            }
            _ => {}
        }
        if field == 7 {
            current.push(c);
        }
    }
    current.trim().trim_matches('\'').parse().ok()
}

/// Each element's `Tag` -> the STEP line of its `NominalDiameter` property.
fn nominal_diameters(step: &str) -> BTreeMap<u32, String> {
    let ents = entities(step);
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let refs = references(args);
        let Some((set_entity, set_args)) = refs.last().and_then(|r| ents.get(r)) else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET" {
            continue;
        }
        let Some(property) = references(set_args)
            .iter()
            .filter_map(|r| ents.get(r))
            .find(|(_, a)| a.starts_with("'NominalDiameter'"))
        else {
            continue;
        };
        for object in &refs[..refs.len() - 1] {
            if let Some(tag) = ents.get(object).and_then(|(_, a)| tag_of(a)) {
                out.insert(tag, format!("{}({})", property.0, property.1));
            }
        }
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export at {}", reference.display());
        return Ok(());
    }
    let theirs = nominal_diameters(&std::fs::read_to_string(&reference)?);
    if theirs.is_empty() {
        println!("no NominalDiameter in the reference export");
        return Ok(());
    }
    let tags: BTreeSet<u32> = theirs.keys().copied().collect();
    println!(
        "reference: {} elements with a NominalDiameter; first {:?}",
        tags.len(),
        theirs.iter().next()
    );

    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("version {version}");

    let scanned =
        rvt::partition_fitting_sizes::scan_fitting_nominal_diameters(&mut rf, version, &tags)?;
    let shown: Vec<u32> = tags
        .iter()
        .copied()
        .take(4)
        .chain([443957, 447853])
        .collect();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for &id in &shown {
            for start in memchr::memmem::find_iter(buf, &id.to_le_bytes()) {
                let Some(object) = rvt::partition_room_parameters::verified_data_object(buf, start)
                else {
                    continue;
                };
                if object.element_id != id {
                    continue;
                }
                let payload =
                    &buf[start + rvt::partition_room_parameters::DATA_OBJECT_HEADER..object.end];
                let pairs = rvt::partition_fitting_sizes::nominal_diameters(payload);
                let near: Vec<String> = (0..payload.len().saturating_sub(8))
                    .filter_map(|at| {
                        let v = f64::from_le_bytes(payload[at..at + 8].try_into().ok()?);
                        ((v - 0.0492126).abs() < 1e-6 || (v - 0.0656168).abs() < 1e-6).then(|| {
                            let from = at.saturating_sub(38);
                            format!(
                                "+{} {}",
                                at + 20,
                                payload[from..at]
                                    .iter()
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<String>()
                            )
                        })
                    })
                    .take(4)
                    .collect();
                println!(
                    "object {id} in {stream} at {start} size {}: pairs {pairs:?}; diameter hits {near:?}",
                    object.end - start
                );
            }
        }
    }
    println!(
        "stage 1, connector scan given the reference tags: {} of {}; first {:?}",
        scanned.len(),
        tags.len(),
        scanned.iter().take(3).collect::<Vec<_>>()
    );

    let has_field = |fields: &[(String, rvt::walker::InstanceField)]| {
        fields
            .iter()
            .any(|(name, _)| name == FITTING_NOMINAL_DIAMETER_FIELD)
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        WalkerLimits::default(),
    )?;
    let mut classes: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for element in mvp.into_elements() {
        if element.id.is_some_and(|id| tags.contains(&id)) {
            let entry = classes.entry(element.class.clone()).or_default();
            entry.0 += 1;
            entry.1 += usize::from(has_field(&element.fields));
        }
    }
    println!("stage 2, schema pass, class -> (elements, with the field): {classes:?}");

    let mut classes: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for element in rvt::walker::iter_elements(&mut rf)? {
        if element.id.is_some_and(|id| tags.contains(&id)) {
            let entry = classes.entry(element.class.clone()).or_default();
            entry.0 += 1;
            entry.1 += usize::from(has_field(&element.fields));
        }
    }
    println!("stage 3, walker, class -> (elements, with the field): {classes:?}");

    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let sets = model
        .entities
        .iter()
        .filter(|e| matches!(e, IfcEntity::ElementPropertySet { set, .. } if set.name == "Pset_PipeFittingTypeCommon"))
        .count();
    let fittings: Vec<String> = model
        .entities
        .iter()
        .filter_map(|e| match e {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid,
                ..
            } if type_guid
                .as_deref()
                .and_then(|t| t.parse::<u32>().ok())
                .is_some_and(|t| tags.contains(&t)) =>
            {
                Some(ifc_type.clone())
            }
            _ => None,
        })
        .collect();
    let mut by_type: BTreeMap<&str, usize> = BTreeMap::new();
    for t in &fittings {
        *by_type.entry(t.as_str()).or_default() += 1;
    }
    println!(
        "stage 4, exporter: {sets} Pset_PipeFittingTypeCommon property sets; entities with the tags by type {by_type:?}"
    );

    let step = write_step(&model);
    let ours = nominal_diameters(&step);
    println!(
        "stage 5, STEP: {} elements with a NominalDiameter, {} of them reference tags; first {:?}",
        ours.len(),
        ours.keys().filter(|t| tags.contains(t)).count(),
        ours.iter().next()
    );
    for line in step
        .lines()
        .filter(|l| l.contains("NominalDiameter"))
        .take(3)
    {
        println!("  {line}");
    }
    Ok(())
}
