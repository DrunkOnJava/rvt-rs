//! RE-445: a project's display units are its `UnitsElem`'s format map.
//!
//! FACT: from Revit 2021 the project's `UnitsElem` (`m_famId` unset) holds
//! `m_units.m_formatOptionsMap`, one entry per spec: the spec's `ForgeTypeId`
//! and its `FormatOptions`, whose `m_unitTypeId` is the display unit. Autodesk's
//! 2024 Core Interior gives length `feetFractionalInches`, area `squareFeet`
//! and volume `cubicFeet`; the metric samples give millimetres, square and
//! cubic metres. A family document inside the project keeps a `UnitsElem` of
//! its own, with `m_famId` naming the family. Revit 2015 to 2020 save an
//! array without specs instead, which is counted and not read.
//!
//! The probe prints, per file, the release, how many `UnitsElem` records it
//! holds and how many name a family, the shape of the project's, how many
//! specs it lists and the display unit of the six specs an IFC unit
//! assignment takes, as `native_project_units::read` returns them; then the
//! format maps of the partition's string records and how many equal the
//! `UnitsElem` map, and what `native_project_units::resolve` returns.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re445_project_units -- FILE.rvt...

use rvt::RevitFile;
use rvt::research::{native_document, native_project_units};
use std::collections::BTreeSet;

const SIX: [&str; 6] = [
    "autodesk.spec.aec:length",
    "autodesk.spec.aec:area",
    "autodesk.spec.aec:volume",
    "autodesk.spec.aec:angle",
    "autodesk.spec.aec.structural:mass",
    "autodesk.spec.aec:time",
];

fn family_of(fields: &serde_json::Value) -> Option<i64> {
    let id = &fields["m_famId"]["m_id"];
    id["m_id64"].as_i64().or_else(|| id["m_id"].as_i64())
}

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let mut rf = match RevitFile::open(&path) {
            Ok(rf) => rf,
            Err(error) => {
                println!("{path}\tunreadable\t{error}");
                continue;
            }
        };
        let release = rf.layout_release().ok();
        let options = native_document::Options {
            selected_classes: BTreeSet::from(["UnitsElem".to_string()]),
            releases: native_document::NATIVE_RELEASES,
            ..native_document::Options::default()
        };
        let (mut records, mut family, mut shapes) = (0, 0, Vec::new());
        let walk = native_document::extract_graphs(&mut rf, &options, |record| {
            let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
                return Ok(());
            };
            records += 1;
            if family_of(&root.fields).is_some_and(|id| id != -1) {
                family += 1;
                return Ok(());
            }
            let units = &root.fields["m_units"];
            shapes.push(if let Some(map) = units["m_formatOptionsMap"].as_array() {
                format!("map:{}", map.len())
            } else if let Some(array) = units["m_formatOptionsArr"].as_array() {
                format!("array:{}", array.len())
            } else {
                "other".to_string()
            });
            Ok(())
        });
        let started = std::time::Instant::now();
        let read = native_project_units::read(&mut rf);
        let read_ms = started.elapsed().as_millis();
        let six = match &read {
            Ok(Some(units)) => SIX
                .iter()
                .map(|spec| {
                    units
                        .unit_of(spec)
                        .map_or("-", |unit| unit.unit.as_str())
                        .trim_start_matches("autodesk.unit.unit:")
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(" "),
            Ok(None) => "not read".to_string(),
            Err(error) => format!("error: {error}"),
        };
        let strings = rf.partition_string_records()?;
        let maps = native_project_units::string_record_maps(&strings);
        let longest = maps.iter().map(|m| m.specs.len()).max().unwrap_or(0);
        let same = |a: &native_project_units::ProjectUnits,
                    b: &native_project_units::ProjectUnits| {
            a.specs.len() == b.specs.len()
                && a.specs.iter().all(|(spec, unit)| {
                    b.specs
                        .get(spec)
                        .is_some_and(|other| other.unit == unit.unit)
                })
        };
        let against = match &read {
            Ok(Some(units)) => {
                let equal = maps.iter().filter(|m| same(m, units)).count();
                format!(
                    "string maps {} (sizes {:?}), {equal} equal to UnitsElem",
                    maps.len(),
                    maps.iter().map(|m| m.specs.len()).collect::<Vec<_>>()
                )
            }
            _ => format!(
                "string maps {} (sizes {:?}), longest {longest}",
                maps.len(),
                maps.iter().map(|m| m.specs.len()).collect::<Vec<_>>()
            ),
        };
        let resolved = match native_project_units::resolve(&mut rf) {
            Ok(Some(units)) => SIX
                .iter()
                .map(|spec| {
                    units
                        .unit_of(spec)
                        .map_or("-", |unit| unit.unit.as_str())
                        .trim_start_matches("autodesk.unit.unit:")
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(" "),
            Ok(None) => "none".to_string(),
            Err(error) => format!("error: {error}"),
        };
        println!(
            "{path}\t{}\trecords {records} family {family}\tproject {}\t{}\t{against}\tresolved {resolved}\tread {read_ms} ms{}",
            release.map_or("?".to_string(), |r| r.to_string()),
            if shapes.is_empty() {
                "-".to_string()
            } else {
                shapes.join(",")
            },
            six,
            walk.err().map_or(String::new(), |e| format!("\twalk: {e}")),
        );
    }
    Ok(())
}
