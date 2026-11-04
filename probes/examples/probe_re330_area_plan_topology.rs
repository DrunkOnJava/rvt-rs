//! RE-330: an Area's region from Revit's own plan topology.
//!
//! FACT under test: for each area scheme on each Level Revit keeps a
//! `LevelRoomPlan` whose `PlanTopology` holds the scheme's area boundary
//! lines cut where they meet (`m_segments`), the closed loops they make
//! (`m_circuits`, each with its area and its sides) and which loop encloses
//! which (`m_components`). Each Area names its loop (`m_cachedCircuitId`).
//! The loop's edges are the lines moved to their right by the area rule's
//! offset (`AreaMeasureCurveData.m_schemeSegments`), met where their curves
//! cross; rebuilt so, the loop has the area Revit saves for the circuit.
//!
//! This probe prints, tab-separated:
//!
//! - `area`: every Area of an area scheme: its ElementId, then its region's
//!   outer loop vertex count, its holes and its area less its holes (square
//!   feet), or the reason it has none;
//! - `line`: every area boundary line: its ElementId, its curve (`GLine` or
//!   `GArc`) and the offsets of its parts, `from-to:offset`;
//! - `summary`: the Areas, those with a region and the lines with an offset
//!   that is not zero.
//!
//! Compare the areas with Revit's own export of the file with
//! `tools/re/space_profiles_vs_ifc.py` on rvt-rs's export.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re330_area_plan_topology -- FILE.rvt

use rvt::RevitFile;
use rvt::research::{native_area_regions, native_document};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// An ElementId as either shape the schema saves it in.
fn identifier(value: &Value) -> Option<i64> {
    let id = value.get("m_id")?;
    id.get("m_id64")
        .or_else(|| id.get("m_id"))
        .and_then(Value::as_i64)
}

fn signed_area(points: &[(f64, f64)]) -> f64 {
    (0..points.len())
        .map(|k| {
            let (a, b) = (points[k], points[(k + 1) % points.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let mut rf = RevitFile::open(&path)?;
    let mut areas = BTreeSet::new();
    let mut lines = 0usize;
    let mut offset_lines = 0usize;
    let options = native_document::Options {
        selected_classes: BTreeSet::from(["RoomElem".to_string(), "CurveElem".to_string()]),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    native_document::extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for object in &graph.objects {
            match object.class_name.as_str() {
                "RoomElem" => {
                    let scheme = object.fields.get("m_areaSchemeId").and_then(identifier);
                    if scheme.is_some_and(|scheme| scheme > 0) {
                        areas.insert(id);
                    }
                }
                "CurveElem"
                    if object.fields.get("m_isAreaMeasurementScheme")
                        == Some(&Value::Bool(true)) =>
                {
                    let curve = graph
                        .objects
                        .iter()
                        .find(|o| o.class_name == "GArc")
                        .or_else(|| graph.objects.iter().find(|o| o.class_name == "GLine"))
                        .map_or("-", |o| o.class_name.as_str());
                    let parts: Vec<String> = graph
                        .objects
                        .iter()
                        .find(|o| o.class_name == "AreaMeasureCurveData")
                        .and_then(|o| o.fields.get("m_schemeSegments"))
                        .and_then(Value::as_array)
                        .map(|parts| {
                            parts
                                .iter()
                                .map(|part| {
                                    let n = |k: &str| {
                                        part.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
                                    };
                                    format!(
                                        "{:.6}-{:.6}:{:.6}",
                                        n("m_startParam"),
                                        n("m_endParam"),
                                        n("m_offset")
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    lines += 1;
                    if parts.iter().any(|part| !part.ends_with(":0.000000")) {
                        offset_lines += 1;
                    }
                    println!("line\t{id}\t{curve}\t{}", parts.join(" "));
                }
                _ => {}
            }
        }
        Ok(())
    })?;
    let regions = native_area_regions::area_regions(&mut rf, &areas);
    for id in &areas {
        match regions.get(id) {
            Some(Ok(region)) => {
                let net = signed_area(&region.outer)
                    + region.holes.iter().map(|h| signed_area(h)).sum::<f64>();
                println!(
                    "area\t{id}\t{}\t{}\t{net:.6}",
                    region.outer.len(),
                    region.holes.len()
                );
            }
            Some(Err(reason)) => println!("area\t{id}\t-\t-\t{reason}"),
            None => println!("area\t{id}\t-\t-\tnot_read"),
        }
    }
    let drawn = regions.values().filter(|r| r.is_ok()).count();
    println!(
        "summary\tareas {}\twith a region {drawn}\tarea boundary lines {lines}\twith an offset {offset_lines}",
        areas.len()
    );
    Ok(())
}
