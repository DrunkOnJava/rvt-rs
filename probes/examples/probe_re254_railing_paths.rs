//! RE-254 (#1016, W7): the curves each railing's path sketch holds, for
//! finding Revit's `Qto_RailingBaseQuantities.Length`.
//!
//! FACT under test: revit-ifc's `LengthCalculator` (CLEANROOM.md source 5)
//! takes a railing's `Length` from its `CURVE_ELEM_LENGTH` parameter, its
//! path's length. A railing's own native record (`BaseRailing`) holds no
//! curve: it names its path's sketch in `m_sketchId`. Hypothesis: that
//! sketch's record holds the path as `GLine` and `GArc` objects whose
//! lengths add up to it. `tools/re/railing_length_vs_ifc.py` compares the
//! sums with Revit's export.
//!
//! Output, one line per railing the export writes:
//!   rail <tag> <line length sum> <arc length sum> <curve classes ...>
//! lengths in feet, each `GLine` the span of its `m_endParams` and each
//! `GArc` its radius times the span of its `m_endParams` (radians), from the
//! record of the sketch its `m_sketchId` names (`-` where none is read);
//! then `classes <tag> <sketch id> <sketch record class> <object
//! class>=<count> ...` per railing, and for the first three sketches each
//! field of each object of their record as `field <sketch id> <class> <name>
//! <value as JSON, cut at 400 characters>`. Last, for each railing whose
//! record names a stair (`BaseRailing.m_stairsId`), `stair <tag> <feet>
//! <slope>`: its path's length where each line parallel to a run of that
//! stair climbs at the stair's riser height over its tread depth (its
//! `StairsElement` counts) and every other line and arc is level; and
//! `stair2 <tag> <feet>`: the path's plan length plus, for each run of the
//! stair, its risers less one times the tread depth climbing at that slope;
//! `offset <tag> <feet> flipped <bool>`, each railing's
//! `BaseRailing.m_placementOffset` and `m_flipped`, and
//! `line <sketch> <origin x y z> <direction x y z> <start> <end>` for each
//! of its sketch's lines;
//! and, where the path is one line and runs along one or more of the
//! stair's runs, `stair3 <tag> <feet>`: that line, its part as long as the
//! longest of those runs' climbs (risers less one times the tread depth)
//! climbing at the slope.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re254_railing_paths -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_stairs::{native_run_shapes, native_stair_counts};
use serde_json::Value;

fn pair(value: Option<&Value>) -> Option<(f64, f64)> {
    match value?.as_array()?.as_slice() {
        [a, b] => Some((a.as_f64()?, b.as_f64()?)),
        _ => None,
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCRAILING") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut sketches: BTreeMap<u64, u64> = BTreeMap::new();
    let mut railing_stairs: BTreeMap<u64, u64> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: ids.clone(),
            ..Options::default()
        },
        |record| {
            let sketch = record.graph.as_ref().and_then(|graph| {
                let object = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "BaseRailing")?;
                let id = object
                    .fields
                    .get("m_sketchId")?
                    .pointer("/m_id/m_id64")?
                    .as_i64()?;
                u64::try_from(id).ok()
            });
            if let Some(sketch) = sketch {
                sketches.insert(record.identity.element_id, sketch);
            }
            if let Some(offset) = record.graph.as_ref().and_then(|graph| {
                graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "BaseRailing")?
                    .fields
                    .get("m_placementOffset")?
                    .as_f64()
            }) {
                let flipped = record
                    .graph
                    .as_ref()
                    .and_then(|graph| {
                        graph
                            .objects
                            .iter()
                            .find(|object| object.class_name == "BaseRailing")?
                            .fields
                            .get("m_flipped")?
                            .as_bool()
                    })
                    .map_or("-".to_string(), |f| f.to_string());
                println!(
                    "offset {} {offset} flipped {flipped}",
                    record.identity.element_id
                );
            }
            let stair = record.graph.as_ref().and_then(|graph| {
                let object = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "BaseRailing")?;
                let id = object
                    .fields
                    .get("m_stairsId")?
                    .pointer("/m_id/m_id64")?
                    .as_i64()?;
                u64::try_from(id).ok()
            });
            if let Some(stair) = stair {
                railing_stairs.insert(record.identity.element_id, stair);
            }
            Ok(())
        },
    )?;
    let wanted: BTreeSet<u64> = sketches.values().copied().collect();
    let dumped: BTreeSet<u64> = wanted.iter().copied().take(3).collect();
    let mut rows: BTreeMap<u64, (f64, f64, Vec<String>)> = BTreeMap::new();
    let mut lines_of: BTreeMap<u64, Vec<([f64; 2], f64)>> = BTreeMap::new();
    type PointAndDirection = ([f64; 2], [f64; 2]);
    let mut line_points: BTreeMap<u64, Vec<PointAndDirection>> = BTreeMap::new();
    let mut census: BTreeMap<u64, (String, BTreeMap<String, usize>)> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: wanted.clone(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let entry = census.entry(id).or_default();
            entry.0 = record.class_name.clone().unwrap_or_default();
            let (mut lines, mut arcs, mut classes) = (0.0, 0.0, Vec::new());
            for object in &graph.objects {
                *entry.1.entry(object.class_name.clone()).or_default() += 1;
                if dumped.contains(&id) {
                    if let Some(fields) = object.fields.as_object() {
                        for (name, value) in fields {
                            let value = value.to_string();
                            let cut: String = value.chars().take(400).collect();
                            println!("field {id} {} {name} {cut}", object.class_name);
                        }
                    }
                }
                let name = object.class_name.as_str();
                if !name.starts_with('G')
                    || !(name.contains("Line") || name.contains("Arc") || name.contains("Curve"))
                {
                    continue;
                }
                classes.push(name.to_string());
                let Some((a, b)) = pair(object.fields.get("m_endParams")) else {
                    continue;
                };
                match name {
                    "GLine" => {
                        lines += (b - a).abs();
                        let origin = object.fields.get("m_origin").and_then(Value::as_array);
                        let dir = object.fields.get("m_dirVec").and_then(Value::as_array);
                        if let (Some(o), Some(d)) = (origin, dir) {
                            let point = |v: &Vec<Value>| {
                                [
                                    v.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
                                    v.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
                                ]
                            };
                            line_points
                                .entry(id)
                                .or_default()
                                .push((point(o), point(d)));
                            let at = |v: &Vec<Value>, i: usize| {
                                v.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN)
                            };
                            println!(
                                "line {id} {} {} {} {} {} {} {a} {b}",
                                at(o, 0),
                                at(o, 1),
                                at(o, 2),
                                at(d, 0),
                                at(d, 1),
                                at(d, 2)
                            );
                        }
                        if let Some((dx, dy)) = object
                            .fields
                            .get("m_dirVec")
                            .and_then(Value::as_array)
                            .and_then(|v| Some((v.first()?.as_f64()?, v.get(1)?.as_f64()?)))
                        {
                            lines_of
                                .entry(id)
                                .or_default()
                                .push(([dx, dy], (b - a).abs()));
                        }
                    }
                    "GArc" => {
                        if let Some(radius) = object.fields.get("m_radius").and_then(Value::as_f64)
                        {
                            arcs += radius * (b - a).abs();
                        }
                    }
                    _ => {}
                }
            }
            rows.insert(id, (lines, arcs, classes));
            Ok(())
        },
    )?;
    for id in &ids {
        match sketches.get(id).and_then(|sketch| rows.get(sketch)) {
            Some((lines, arcs, classes)) => {
                println!("rail {id} {lines} {arcs} {}", classes.join(" "))
            }
            None => println!("rail {id} - -"),
        }
    }
    for (id, sketch) in &sketches {
        let Some((class, objects)) = census.get(sketch) else {
            println!("classes {id} {sketch} -");
            continue;
        };
        let counts: Vec<String> = objects.iter().map(|(c, n)| format!("{c}={n}")).collect();
        println!("classes {id} {sketch} {class} {}", counts.join(" "));
    }
    let mut run_stairs: BTreeMap<u64, u64> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_classes: ["StairsRun".to_string()].into(),
            ..Options::default()
        },
        |record| {
            let stair = record.graph.as_ref().and_then(|graph| {
                let id = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "StairsRun")?
                    .fields
                    .get("m_stairsId")?
                    .pointer("/m_id/m_id64")?
                    .as_i64()?;
                u64::try_from(id).ok()
            });
            if let Some(stair) = stair {
                run_stairs.insert(record.identity.element_id, stair);
            }
            Ok(())
        },
    )?;
    let shapes = native_run_shapes(&mut rf, &run_stairs.keys().copied().collect())?;
    let stairs: BTreeSet<u64> = railing_stairs.values().copied().collect();
    let held_runs: BTreeSet<u64> = run_stairs
        .iter()
        .filter(|(_, stair)| stairs.contains(stair))
        .map(|(run, _)| *run)
        .collect();
    let (counts, run_counts) = native_stair_counts(&mut rf, &stairs, &held_runs)?;
    for (stair, count) in &counts {
        println!(
            "stairinfo {stair} risers {} riser {} tread {}",
            count.risers, count.riser_height_feet, count.tread_depth_feet
        );
    }
    // Each run's plan footprint, from its decoded sketch (RE-52): which side
    // of a railing's line along it the run lies on.
    let version = rf.basic_file_info()?.version;
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let bodies: BTreeMap<u64, rvt::research::partition_schema_mvp::StairRunBody> = mvp
        .products
        .iter()
        .filter(|element| element.class == "StairsRun")
        .filter_map(|element| {
            Some((
                u64::from(element.id?),
                rvt::research::partition_schema_mvp::stair_run_body_from_fields(&element.fields)?,
            ))
        })
        .collect();
    for (id, stair) in &railing_stairs {
        let Some(lines) = sketches.get(id).and_then(|sketch| line_points.get(sketch)) else {
            continue;
        };
        let mut turn = 0.0_f64;
        for pair in lines.windows(2) {
            let (a, b) = (pair[0].1, pair[1].1);
            turn += (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]);
        }
        let Some(&(origin, dir)) = lines.first() else {
            continue;
        };
        let side = run_stairs
            .iter()
            .filter(|(_, s)| *s == stair)
            .filter_map(|(run, _)| bodies.get(run))
            .filter(|body| (dir[0] * body.climb[1] - dir[1] * body.climb[0]).abs() < 1e-6)
            .map(|body| {
                let centre = [
                    body.origin[0] + body.across[0] * body.width_feet / 2.0,
                    body.origin[1] + body.across[1] * body.width_feet / 2.0,
                ];
                let to = [centre[0] - origin[0], centre[1] - origin[1]];
                let cross = dir[0] * to[1] - dir[1] * to[0];
                (cross.abs(), cross.signum())
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, side)| side);
        println!(
            "side {id} run {} turn {turn}",
            side.map_or("-".to_string(), |side| side.to_string())
        );
    }
    for (run, counted) in &run_counts {
        let direction = shapes.get(run).and_then(|shape| shape.direction);
        println!(
            "runinfo {run} stair {} risers {} height {} begin {} end {} direction {:?}",
            counted.stair,
            counted.risers,
            counted.height_feet,
            counted.begin_with_riser,
            counted.end_with_riser,
            direction
        );
    }
    for (id, stair) in &railing_stairs {
        let Some(count) = counts.get(stair) else {
            println!("stair {id} - -");
            continue;
        };
        let slope = count.riser_height_feet / count.tread_depth_feet;
        let directions: Vec<[f64; 2]> = run_stairs
            .iter()
            .filter(|(_, s)| *s == stair)
            .filter_map(|(run, _)| shapes.get(run)?.direction)
            .collect();
        let arcs = sketches
            .get(id)
            .and_then(|sketch| rows.get(sketch))
            .map_or(0.0, |r| r.1);
        let lines = sketches.get(id).and_then(|sketch| lines_of.get(sketch));
        let length: f64 = lines.map_or(0.0, |lines| {
            lines
                .iter()
                .map(|(dir, span)| {
                    let along = directions
                        .iter()
                        .any(|d| (dir[0] * d[1] - dir[1] * d[0]).abs() < 1e-6);
                    if along {
                        span * slope.hypot(1.0)
                    } else {
                        *span
                    }
                })
                .sum()
        }) + arcs;
        println!("stair {id} {length} {slope}");
        let plan =
            lines.map_or(0.0, |lines| lines.iter().map(|(_, span)| span).sum::<f64>()) + arcs;
        let climbing: f64 = run_counts
            .values()
            .filter(|run| run.stair == *stair)
            .map(|run| (run.risers - 1).max(0) as f64 * count.tread_depth_feet)
            .sum();
        println!("stair2 {id} {}", plan + climbing * (slope.hypot(1.0) - 1.0));
        if let (Some([(dir, span)]), true) = (lines.map(Vec::as_slice), arcs == 0.0) {
            let along: Vec<u64> = run_stairs
                .iter()
                .filter(|(_, s)| *s == stair)
                .filter(|(run, _)| {
                    shapes
                        .get(*run)
                        .and_then(|shape| shape.direction)
                        .is_some_and(|d| (dir[0] * d[1] - dir[1] * d[0]).abs() < 1e-6)
                })
                .map(|(run, _)| *run)
                .collect();
            let longest = along
                .iter()
                .filter_map(|run| run_counts.get(run))
                .map(|run| (run.risers - 1).max(0) as f64 * count.tread_depth_feet)
                .fold(0.0_f64, f64::max);
            if longest > 0.0 {
                let climb = span.min(longest);
                println!("stair3 {id} {}", span + climb * (slope.hypot(1.0) - 1.0));
            }
        }
    }
    eprintln!(
        "{} railings, {} sketches named, {} sketch records",
        ids.len(),
        wanted.len(),
        rows.len()
    );
    Ok(())
}
