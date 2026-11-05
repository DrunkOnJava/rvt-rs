//! #1417: the elements CSV takes each row's level, and which rows it has, from
//! the IFC writer's placement decision.
//!
//! FACT under test: `rvt::ifc::schedule_csv::elements_csv` writes one row for
//! each building element `step_writer::placements` does not fold into another
//! element's body, with the storey `step_writer::storey_of` gives it (the one
//! it is contained in, that of its space, its whole or its host), where it
//! used to write a row for every building element with the element's own
//! `storey_index`.
//!
//! The probe builds the model `rvt-schedule` and Python's `schedule_csv`
//! build (the document exporter alone), counts what the old rule would have
//! written (every building element, the level of its own `storey_index`)
//! against what `elements_csv` writes now, and checks the CSV it writes: its
//! header, a cell for every column, the rows in level-elevation order, and
//! the (ElementId, IFC type, level) of every row against the placements.
//! Then it builds the model `rvt-ifc` writes with no switch (saved meshes
//! on for doors and windows) and says whether its written elements and
//! storeys are the same ones. Counts and IFC types only: no name, id or
//! coordinate is printed.
//!
//! Output, one line per item:
//!   rows before <n> after <n> removed <n>        old rule against the new rows
//!   removed <IFC type> <n>                        the rows left out, by type
//!   moved <n> none-to-storey <n> storey-to-none <n> storey-to-storey <n>
//!   moved-by-type <IFC type> <n>                  the rows whose level moved
//!   moved-by-placement <how> <n>                  and why: part, space, opening
//!   check <name> ok | FAIL <detail>               header, cells, sorted, content
//!   rvt-ifc default written <n> differing <n>     the rvt-ifc model against it
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1417_elements_csv_levels -- MODEL.rvt

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::schedule_csv::{CsvOptions, LengthUnit, element_columns, elements_csv};
use rvt::ifc::step_writer::{Placement, placements, storey_of};
use rvt::ifc::{ExportOptions, Exporter, IfcModel, RvtDocExporter, run_export};

type Key = (String, String, String);

fn how(placement: Option<Placement>) -> &'static str {
    match placement {
        Some(Placement::Folded) => "folded",
        Some(Placement::Opening { .. }) => "opening of its host",
        Some(Placement::InSpace(_)) => "in a space",
        Some(Placement::PartOf(_)) => "part of a whole",
        Some(Placement::OnStorey(_)) => "on a storey",
        Some(Placement::InBuilding) => "in the building",
        _ => "other",
    }
}

fn neutralised(value: &str) -> String {
    if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

fn records(csv: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = csv.trim_start_matches('\u{FEFF}').chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => record.push(std::mem::take(&mut cell)),
            '\r' if !quoted && chars.peek() == Some(&'\n') => {
                chars.next();
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            other => cell.push(other),
        }
    }
    records
}

/// The (ElementId, IFC type, level name) of every building element the export
/// writes, by the placements of `model`.
fn written(model: &IfcModel, placements: &[Option<Placement>]) -> Vec<Key> {
    let mut keys = Vec::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid,
            ..
        } = entity
        else {
            continue;
        };
        if matches!(placements.get(index), Some(Some(Placement::Folded))) {
            continue;
        }
        let level = storey_of(placements, index)
            .and_then(|i| model.building_storeys.get(i))
            .map(|s| s.name.as_str())
            .unwrap_or_default();
        keys.push((
            type_guid.clone().unwrap_or_default(),
            ifc_type.clone(),
            level.to_string(),
        ));
    }
    keys
}

/// How many (ElementId, IFC type, level) keys `a` and `b` hold a different
/// number of times.
fn differing(a: &[Key], b: &[Key]) -> usize {
    let tally = |keys: &[Key]| {
        let mut counts: BTreeMap<Key, i64> = BTreeMap::new();
        for key in keys {
            *counts.entry(key.clone()).or_insert(0) += 1;
        }
        counts
    };
    let (a, b) = (tally(a), tally(b));
    a.keys()
        .chain(b.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|key| a.get(*key) != b.get(*key))
        .count()
}

fn print_counts(label: &str, counts: &BTreeMap<String, usize>) {
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (name, count) in sorted {
        println!("{label} {name} {count}");
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("usage: probe_1417_elements_csv_levels MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export(&mut rf)?;
    let placed = placements(&model);

    let mut before = 0usize;
    let mut removed = 0usize;
    let mut removed_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let (mut none_to_storey, mut storey_to_none, mut storey_to_storey) = (0usize, 0usize, 0usize);
    let mut moved_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut moved_by_placement: BTreeMap<String, usize> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            storey_index,
            ..
        } = entity
        else {
            continue;
        };
        before += 1;
        let placement = placed.get(index).copied().flatten();
        if matches!(placement, Some(Placement::Folded)) {
            removed += 1;
            *removed_by_type.entry(ifc_type.clone()).or_insert(0) += 1;
            continue;
        }
        let old = storey_index.filter(|i| *i < model.building_storeys.len());
        let new = storey_of(&placed, index);
        if old == new {
            continue;
        }
        match (old, new) {
            (None, Some(_)) => none_to_storey += 1,
            (Some(_), None) => storey_to_none += 1,
            _ => storey_to_storey += 1,
        }
        *moved_by_type.entry(ifc_type.clone()).or_insert(0) += 1;
        *moved_by_placement
            .entry(how(placement).to_string())
            .or_insert(0) += 1;
    }
    let csv = elements_csv(&model, &CsvOptions::default());
    let rows = records(&csv);
    let after = rows.len().saturating_sub(1);
    println!("rows before {before} after {after} removed {removed}");
    print_counts("removed", &removed_by_type);
    println!(
        "moved {} none-to-storey {none_to_storey} storey-to-none {storey_to_none} storey-to-storey {storey_to_storey}",
        none_to_storey + storey_to_none + storey_to_storey
    );
    print_counts("moved-by-type", &moved_by_type);
    print_counts("moved-by-placement", &moved_by_placement);

    let mut failed = false;
    let mut check = |name: &str, detail: Option<String>| match detail {
        None => println!("check {name} ok"),
        Some(detail) => {
            failed = true;
            println!("check {name} FAIL {detail}");
        }
    };
    let columns = element_columns(LengthUnit::Feet);
    check(
        "header",
        (rows.first() != Some(&columns)).then(|| format!("{:?}", rows.first())),
    );
    let ragged = rows.iter().filter(|r| r.len() != columns.len()).count();
    check(
        "cells",
        (ragged > 0).then(|| format!("{ragged} rows without {} cells", columns.len())),
    );
    check(
        "count",
        (after != before - removed).then(|| format!("{after} rows, expected {}", before - removed)),
    );
    let elevations: Vec<f64> = rows
        .iter()
        .skip(1)
        .map(|r| {
            r.get(5)
                .and_then(|c| c.parse().ok())
                .unwrap_or(f64::INFINITY)
        })
        .collect();
    let unsorted = elevations.windows(2).filter(|w| w[0] > w[1]).count();
    check(
        "sorted",
        (unsorted > 0).then(|| format!("{unsorted} rows out of level-elevation order")),
    );
    let expected: Vec<Key> = written(&model, &placed)
        .into_iter()
        .map(|(id, ty, level)| (neutralised(&id), neutralised(&ty), neutralised(&level)))
        .collect();
    let actual: Vec<Key> = rows
        .iter()
        .skip(1)
        .filter(|r| r.len() > 4)
        .map(|r| (r[0].clone(), r[1].clone(), r[4].clone()))
        .collect();
    let wrong = differing(&expected, &actual);
    check(
        "content",
        (wrong > 0).then(|| format!("{wrong} (id, type, level) keys differ")),
    );

    let ifc_run = run_export(&mut rf, &ExportOptions::default())?;
    let ifc_written = written(&ifc_run.model, &placements(&ifc_run.model));
    println!(
        "rvt-ifc default written {} differing {}",
        ifc_written.len(),
        differing(&ifc_written, &written(&model, &placed))
    );
    if failed {
        bail!("the elements CSV does not follow the placements");
    }
    Ok(())
}
