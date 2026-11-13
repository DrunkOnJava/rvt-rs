//! RE-333: an Area's region from Revit's own plan topology.
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
//!   outer loop vertex count (the ends of its arcs), its holes, its area less
//!   its holes (square feet, arcs counted exactly) and how many of its edges
//!   are arcs, or the reason it has none;
//! - `line`: every area boundary line: its ElementId, its curve (`GLine` or
//!   `GArc`) and the offsets of its parts, `from-to:offset`;
//! - `summary`: the Areas, those with a region and the lines with an offset
//!   that is not zero;
//! - `chain`: on Revit 2024 and later, the partition chain records (RE-35) by
//!   the count of 22-byte entries before their body (the `u16` at `+0x0e`),
//!   how many of them have an `i64` -1 as every entry's second word, and the
//!   Areas and area boundary lines their categories give once the entries are
//!   skipped.
//!
//! Compare the areas with Revit's own export of the file with
//! `tools/re/space_profiles_vs_ifc.py` on rvt-rs's export.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re333_area_plan_topology -- FILE.rvt

use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::{native_area_regions, native_document, partition_element_records};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// An ElementId as either shape the schema saves it in.
fn identifier(value: &Fv) -> Option<i64> {
    let id = value.get("m_id")?;
    id.get("m_id64")
        .or_else(|| id.get("m_id"))
        .and_then(Fv::as_i64)
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
                    if object.fields.get("m_isAreaMeasurementScheme") == Some(&Fv::Bool(true)) =>
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
                        .and_then(Fv::as_array)
                        .map(|parts| {
                            parts
                                .iter()
                                .map(|part| {
                                    let n = |k: &str| {
                                        part.get(k).and_then(Fv::as_f64).unwrap_or(f64::NAN)
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
                let net = region.net_area_feet2().unwrap_or(f64::NAN);
                let arcs = region.outer_arcs.iter().flatten().count()
                    + region.hole_arcs.iter().flatten().flatten().count();
                println!(
                    "area\t{id}\t{}\t{}\t{net:.6}\tarcs {arcs}",
                    region.outer.len(),
                    region.holes.len()
                );
            }
            Some(Err(reason)) => println!("area\t{id}\t-\t-\t{reason}"),
            None => println!("area\t{id}\t-\t-\tnot_read"),
        }
    }
    if let Some(version) = rf.basic_file_info().ok().map(|info| info.version) {
        if let Some(marker) = partition_element_records::file_bbox_marker(&mut rf, version) {
            let mut by_count: BTreeMap<u16, usize> = BTreeMap::new();
            let (mut well_formed, mut areas, mut lines) = (0usize, 0usize, 0usize);
            for stream in rf.partition_stream_names() {
                let Ok(inflated) = rf.inflated_partition(&stream) else {
                    continue;
                };
                let buf = inflated.bytes();
                let word = |at: usize| {
                    buf.get(at..at + 8)
                        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
                };
                for span in partition_element_records::partition_record_chain(buf, &marker) {
                    let count = buf
                        .get(span.start + 0x0e..span.start + 0x10)
                        .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
                    *by_count.entry(count).or_default() += 1;
                    let body = span.start + 0x12 + 22 * usize::from(count);
                    let entries_ok = body + 8 <= span.end
                        && (0..usize::from(count))
                            .all(|k| word(span.start + 0x12 + 22 * k + 8) == Some(u64::MAX));
                    if !entries_ok {
                        continue;
                    }
                    well_formed += 1;
                    match word(body).map(|c| c as i64) {
                        Some(-2_003_200) => areas += 1,
                        Some(-2_000_079) => lines += 1,
                        _ => {}
                    }
                }
            }
            let counts: Vec<String> = by_count.iter().map(|(c, n)| format!("{c}:{n}")).collect();
            println!(
                "chain\trecords by entry count {}\twell formed {well_formed}\tAreas {areas}\tarea boundary lines {lines}",
                counts.join(" ")
            );
        }
    }
    let drawn = regions.values().filter(|r| r.is_ok()).count();
    println!(
        "summary\tareas {}\twith a region {drawn}\tarea boundary lines {lines}\twith an offset {offset_lines}",
        areas.len()
    );
    Ok(())
}
