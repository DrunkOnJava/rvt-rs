//! A Revit 2025 stair run's sketch and type, read by member name from its
//! native records (#357).
//!
//! RE-47 and RE-92 read a run's riser height, tread depth, riser count and
//! ends from byte anchors measured on Revit 2024, and none of them is found
//! on Revit 2025. The run's native record names them instead:
//!
//! - `StairsRun`: `m_actualRunWidth`, `m_bottomElevation`,
//!   `m_beginWithRiser` and `m_endWithRiser`;
//! - one `StairsTreadInfo` per tread: `m_origin`, where the tread's own
//!   riser meets the run's centre line, at the riser's top; `m_actDepth`;
//!   `m_runDirProj`, the plan direction the run climbs in; `m_riserDir`,
//!   along the riser; and `m_runType`, 0 for a straight run;
//! - the `StairsRunType` of the run's own type, which its native record does
//!   not name: the caller reads it from the run's element record, the one
//!   `OST_StairsRuns` type definition its reference list names (RE-28), as
//!   on Revit 2024. That is the type Revit's export gives the flight on all
//!   five R2025 stair models and all 35 runs of each BIM4LCA office file
//!   (RE-217): `m_treadThickness`, `m_riserThickness`,
//!   `m_nosingLength`, `m_structuralDepth`, `m_hasMonolithicSupport`,
//!   `m_hasTread`, `m_hasRiser`, `m_riserIsSlanted` and
//!   `m_riserTreadConnection` (1: the tread runs under the riser, Revit's
//!   `RiserToTreadConnectionOption.TreadUnderRiser`).
//!
//! These give [`crate::partition_stairs::RunSketch`] and
//! [`crate::partition_stairs::RunType`], which RE-92 draws the run from. A
//! run that is not straight, does not begin with a riser, or whose treads
//! do not share one depth, one riser height and one direction is left out.
//!
//! A landing's top is its `StairsLanding.m_baseElevation`, and a monolithic
//! landing type's thickness its `StairsLandingType.m_thickness`
//! ([`native_landings`], #824).
//!
//! A run's treads all give one `m_runType`, which names the flight's
//! predefined type in Revit's export ([`native_run_shapes`], #323). A spiral
//! run's treads give the spiral's centre and each tread's radii and angles
//! ([`native_spiral_runs`], #357).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::RevitFile;
use crate::native_document::{Options, extract_graphs};
use crate::partition_stairs::{RunSketch, RunType};

/// Releases these records are measured on: the R2025 stair models (#357).
pub const NATIVE_RUN_REVIT_VERSIONS: &[u32] = &[2025];

/// How closely a run's treads must agree with each other, feet.
pub const NATIVE_RUN_TOLERANCE_FEET: f64 = 1e-6;

/// A run's sketch, its type and how it ends, from its native records.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeRun {
    pub sketch: RunSketch,
    pub run_type: RunType,
    pub riser_height_feet: f64,
    pub end_with_riser: Option<bool>,
    pub type_id: u64,
}

fn number(v: &Value, name: &str) -> Option<f64> {
    v.get(name)?.as_f64().filter(|x| x.is_finite())
}

fn flag(v: &Value, name: &str) -> Option<bool> {
    v.get(name)?.as_bool()
}

fn point(v: &Value, name: &str) -> Option<[f64; 3]> {
    let a = v.get(name)?.as_array()?;
    match a.as_slice() {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?])
            .filter(|p| p.iter().all(|c| c.is_finite())),
        _ => None,
    }
}

fn unit_plan(p: [f64; 3]) -> Option<[f64; 2]> {
    let length = p[0].hypot(p[1]);
    (length > 0.5 && p[2].abs() <= NATIVE_RUN_TOLERANCE_FEET)
        .then_some([p[0] / length, p[1] / length])
}

fn run_type(fields: &Value, name: Option<String>) -> Option<RunType> {
    Some(RunType {
        structural_depth_feet: number(fields, "m_structuralDepth")?,
        tread_thickness_feet: number(fields, "m_treadThickness")?,
        riser_thickness_feet: number(fields, "m_riserThickness")?,
        nosing_length_feet: number(fields, "m_nosingLength")?,
        monolithic: flag(fields, "m_hasMonolithicSupport")?,
        treads: flag(fields, "m_hasTread")?,
        risers: flag(fields, "m_hasRiser")?,
        slanted_risers: flag(fields, "m_riserIsSlanted")?,
        tread_under_riser: fields.get("m_riserTreadConnection")?.as_i64()? == 1,
        name,
    })
}

/// Where the run's underside is cut ([`RunSketch::base_cut_feet`]): at its
/// `m_extendBelowBase` below its base where it starts at its stair's base
/// (`m_baseRiserIndex` 0), not at all where it starts on a landing. On the
/// BIM4LCA office the five flights at their stair's base with no extension
/// are cut at the base, as RE-92 draws them; those 300 mm extended
/// (`m_extendBelowBase` -0.98425 ft) and those that start on a landing run
/// their underside on to their first riser line (#357).
fn base_cut(run: &Value) -> Option<Option<f64>> {
    Some(match run.get("m_baseRiserIndex")?.as_i64()? {
        0 => Some(number(run, "m_extendBelowBase")?),
        _ => None,
    })
}

/// Each run in `runs` whose native `StairsRun` record gives where its
/// underside is cut ([`RunSketch::base_cut_feet`]: at its
/// `m_extendBelowBase` where `m_baseRiserIndex` is 0, not at all otherwise),
/// for runs drawn from their byte anchors (RE-52), whose sketch does not
/// carry it.
pub fn native_run_base_cuts(
    rf: &mut RevitFile,
    runs: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, Option<f64>>> {
    if runs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: runs.clone(),
        ..Options::default()
    };
    let mut out = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        if record.class_name.as_deref() != Some("StairsRun") {
            return Ok(());
        }
        let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        else {
            return Ok(());
        };
        if let Some(cut) = base_cut(&root.fields) {
            out.insert(record.identity.element_id, cut);
        }
        Ok(())
    })?;
    Ok(out)
}

/// The run's sketch and riser height from its root and its treads, in
/// climbing order.
fn sketch(run: &Value, treads: &[Value]) -> Option<(RunSketch, f64)> {
    if !flag(run, "m_beginWithRiser")? {
        return None;
    }
    let width = number(run, "m_actualRunWidth").filter(|w| *w > 0.0)?;
    let base = number(run, "m_bottomElevation")?;
    let first = treads.first()?;
    let climb = unit_plan(point(first, "m_runDirProj")?)?;
    let across = unit_plan(point(first, "m_riserDir")?)?;
    if (climb[0] * across[0] + climb[1] * across[1]).abs() > NATIVE_RUN_TOLERANCE_FEET {
        return None;
    }
    let depth = number(first, "m_actDepth").filter(|d| *d > 0.0)?;
    let o0 = point(first, "m_origin")?;
    let height = o0[2] - base;
    if !(height.is_finite() && height > 0.0) {
        return None;
    }
    let origin = [
        o0[0] - across[0] * width / 2.0,
        o0[1] - across[1] * width / 2.0,
        base,
    ];
    let mut risers = Vec::with_capacity(treads.len() + 1);
    for (k, tread) in treads.iter().enumerate() {
        let o = point(tread, "m_origin")?;
        let direction = unit_plan(point(tread, "m_runDirProj")?)?;
        let same_direction = direction
            .iter()
            .zip(climb)
            .all(|(a, b)| (a - b).abs() <= NATIVE_RUN_TOLERANCE_FEET);
        let along = (o[0] - origin[0]) * climb[0] + (o[1] - origin[1]) * climb[1];
        let side = (o[0] - origin[0]) * across[0] + (o[1] - origin[1]) * across[1];
        let consistent = tread.get("m_runType")?.as_i64()? == 0
            && same_direction
            && (number(tread, "m_actDepth")? - depth).abs() <= NATIVE_RUN_TOLERANCE_FEET
            && (along - depth * k as f64).abs() <= NATIVE_RUN_TOLERANCE_FEET * (k as f64 + 1.0)
            && (side - width / 2.0).abs() <= NATIVE_RUN_TOLERANCE_FEET
            && (o[2] - base - height * (k as f64 + 1.0)).abs()
                <= NATIVE_RUN_TOLERANCE_FEET * (k as f64 + 1.0);
        if !consistent {
            return None;
        }
        risers.push(along);
    }
    risers.push(depth * treads.len() as f64);
    Some((
        RunSketch {
            origin,
            climb,
            across,
            width_feet: width,
            risers,
            base_cut_feet: base_cut(run)?,
        },
        height,
    ))
}

/// Releases a run's shape is measured on: the BIM4LCA office (2024) and
/// the R2025 stair models (#323).
pub const NATIVE_RUN_SHAPE_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// The `IfcStairFlightTypeEnum` value Revit's export writes for a run whose
/// treads' `m_runType` is `run_type`: `STRAIGHT` for 0 and `SPIRAL` for 2,
/// the two values measured (#323).
pub fn flight_predefined_type(run_type: i64) -> Option<&'static str> {
    match run_type {
        0 => Some("STRAIGHT"),
        2 => Some("SPIRAL"),
        _ => None,
    }
}

/// Each run in `runs` whose native `StairsTreadInfo` records all give one
/// `m_runType`, with that value (#323).
pub fn native_run_shapes(
    rf: &mut RevitFile,
    runs: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, i64>> {
    if runs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: runs.clone(),
        ..Options::default()
    };
    let mut out = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        if record.class_name.as_deref() != Some("StairsRun") {
            return Ok(());
        }
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let shapes: BTreeSet<Option<i64>> = graph
            .objects
            .iter()
            .filter(|o| o.class_name == "StairsTreadInfo")
            .map(|o| o.fields.get("m_runType").and_then(Value::as_i64))
            .collect();
        if let [Some(shape)] = shapes.into_iter().collect::<Vec<_>>().as_slice() {
            out.insert(record.identity.element_id, *shape);
        }
        Ok(())
    })?;
    Ok(out)
}

/// The run types, and each run's `StairsRun` fields with its treads'
/// `StairsTreadInfo` fields in riser order, of the runs in `runs` and their
/// types.
type RunGraphs = (BTreeMap<u64, RunType>, BTreeMap<u64, (Value, Vec<Value>)>);

fn run_graphs(rf: &mut RevitFile, runs: &BTreeMap<u64, u64>) -> anyhow::Result<RunGraphs> {
    let options = Options {
        selected_ids: runs.keys().chain(runs.values()).copied().collect(),
        ..Options::default()
    };
    let mut types: BTreeMap<u64, RunType> = BTreeMap::new();
    let mut found: BTreeMap<u64, (Value, Vec<Value>)> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "StairsRunType" => {
                let name = graph
                    .objects
                    .iter()
                    .find(|o| o.class_name == "SymbolInfo")
                    .and_then(|o| o.fields.get("m_name")?.as_str().map(String::from));
                if let Some(run_type) = run_type(&root.fields, name) {
                    types.insert(id, run_type);
                }
            }
            "StairsRun" if runs.contains_key(&id) => {
                let mut treads: Vec<(i64, Value)> = graph
                    .objects
                    .iter()
                    .filter(|o| o.class_name == "StairsTreadInfo")
                    .map(|o| {
                        let index = o.fields["m_startRiser"]["m_specifiedIdx"]
                            .as_i64()
                            .unwrap_or(i64::MAX);
                        (index, o.fields.clone())
                    })
                    .collect();
                treads.sort_by_key(|(index, _)| *index);
                found.insert(
                    id,
                    (
                        root.fields.clone(),
                        treads.into_iter().map(|(_, t)| t).collect(),
                    ),
                );
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok((types, found))
}

/// Each run in `runs`, keyed by its id with its own run type's id, whose
/// native records give its sketch and whose type's record gives its
/// construction.
pub fn native_runs(
    rf: &mut RevitFile,
    runs: &BTreeMap<u64, u64>,
) -> anyhow::Result<BTreeMap<u64, NativeRun>> {
    if runs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let (types, found) = run_graphs(rf, runs)?;
    Ok(found
        .into_iter()
        .filter_map(|(id, (run, treads))| {
            let type_id = runs[&id];
            let run_type = types.get(&type_id)?.clone();
            let (sketch, riser_height_feet) = sketch(&run, &treads)?;
            Some((
                id,
                NativeRun {
                    sketch,
                    run_type,
                    riser_height_feet,
                    end_with_riser: flag(&run, "m_endWithRiser"),
                    type_id,
                },
            ))
        })
        .collect())
}

/// Releases a spiral run's treads are measured on: the BIM4LCA office
/// (2024, #357).
pub const NATIVE_SPIRAL_RUN_REVIT_VERSIONS: &[u32] = &[2024];

/// One tread of a spiral run: its top, feet, and the angles about the
/// spiral's centre, radians, of the two riser lines it lies between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralTread {
    pub top_feet: f64,
    pub angles: [f64; 2],
}

/// A spiral run's treads and type, from its native records (#357).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeSpiralRun {
    /// The spiral's centre, model feet.
    pub centre: [f64; 2],
    /// The treads' inner and outer radius, feet.
    pub radii_feet: [f64; 2],
    /// In riser order, each one riser higher than the one before.
    pub treads: Vec<SpiralTread>,
    /// `StairsRun.m_isMirrored`.
    pub mirrored: bool,
    pub run_type: RunType,
    pub type_id: u64,
}

/// A spiral run's centre, radii and treads (#357). Each `StairsTreadInfo` of
/// a spiral run (`m_runType` 2) holds the spiral's centre in its `m_origin`,
/// at its own top, and its plan extent in `m_bndBox`, as `[radius, angle]`
/// corners: from its inner to its outer radius, the run's width, and between
/// the angles of its two riser lines, which wrap by a turn where the run
/// passes -pi. `None` unless every tread is a spiral
/// tread about one centre, between the same two radii, and each tread lies
/// next to the one before it and above it.
fn spiral(run: &Value, treads: &[Value]) -> Option<([f64; 2], [f64; 2], Vec<SpiralTread>)> {
    let close = |a: f64, b: f64| (a - b).abs() <= NATIVE_RUN_TOLERANCE_FEET;
    // The stored angles wrap by a turn where a run passes -pi (the BIM4LCA
    // office's run 3057703: -3.44719 then 2.836).
    let turn = std::f64::consts::TAU;
    let wrapped = |a: f64| a - turn * (a / turn).round();
    let mut centre: Option<[f64; 2]> = None;
    let mut radii: Option<[f64; 2]> = None;
    let mut out: Vec<SpiralTread> = Vec::with_capacity(treads.len());
    for tread in treads {
        if tread.get("m_runType")?.as_i64()? != 2 {
            return None;
        }
        let origin = point(tread, "m_origin")?;
        let corners = tread.get("m_bndBox")?.get("m_corners")?.as_array()?;
        let corner = |index: usize| -> Option<[f64; 2]> {
            let pair = corners.get(index)?.as_array()?;
            match pair.as_slice() {
                [a, b] => {
                    Some([a.as_f64()?, b.as_f64()?]).filter(|p| p.iter().all(|c| c.is_finite()))
                }
                _ => None,
            }
        };
        let ([r0, a0], [r1, a1]) = (corner(0)?, corner(1)?);
        if !(0.0 <= r0 && r0 < r1 && a0 < a1) {
            return None;
        }
        let here = [origin[0], origin[1]];
        match (centre, radii) {
            (Some(c), Some(r)) => {
                if !(close(c[0], here[0])
                    && close(c[1], here[1])
                    && close(r[0], r0)
                    && close(r[1], r1))
                {
                    return None;
                }
            }
            _ => {
                centre = Some(here);
                radii = Some([r0, r1]);
            }
        }
        if let Some(before) = out.last() {
            let next_to = close(wrapped(before.angles[0] - a1), 0.0)
                || close(wrapped(before.angles[1] - a0), 0.0);
            if !(next_to && origin[2] > before.top_feet) {
                return None;
            }
        }
        out.push(SpiralTread {
            top_feet: origin[2],
            angles: [a0, a1],
        });
    }
    // Every tread climbs the same way round, and the radii span the run.
    let (centre, radii) = (centre?, radii?);
    let turns: BTreeSet<bool> = out
        .windows(2)
        .map(|pair| wrapped(pair[1].angles[0] - pair[0].angles[0]) < 0.0)
        .collect();
    let width = number(run, "m_actualRunWidth")?;
    (out.len() >= 2 && turns.len() == 1 && close(width, radii[1] - radii[0]))
        .then_some((centre, radii, out))
}

/// Each spiral run in `runs`, keyed by its id with its own run type's id,
/// whose native records give its treads ([`spiral`]) and whose type's record
/// gives its construction (#357).
pub fn native_spiral_runs(
    rf: &mut RevitFile,
    runs: &BTreeMap<u64, u64>,
) -> anyhow::Result<BTreeMap<u64, NativeSpiralRun>> {
    if runs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let (types, found) = run_graphs(rf, runs)?;
    Ok(found
        .into_iter()
        .filter_map(|(id, (run, treads))| {
            let type_id = runs[&id];
            let run_type = types.get(&type_id)?.clone();
            let (centre, radii_feet, treads) = spiral(&run, &treads)?;
            Some((
                id,
                NativeSpiralRun {
                    centre,
                    radii_feet,
                    treads,
                    mirrored: flag(&run, "m_isMirrored")?,
                    run_type,
                    type_id,
                },
            ))
        })
        .collect())
}

/// Releases a landing's top and thickness are measured on: the BIM4LCA
/// office (2024, #824).
pub const NATIVE_LANDING_REVIT_VERSIONS: &[u32] = &[2024];

/// A monolithic landing's top elevation and its type's thickness, feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeLanding {
    pub top_feet: f64,
    pub thickness_feet: f64,
}

/// Each landing in `landings`, keyed by its id with its own landing type's
/// id, whose `StairsLanding` record gives its top (`m_baseElevation`) and
/// whose `StairsLandingType` is monolithic (`m_hasMonolithicSupport`) with a
/// thickness (`m_thickness`). On the BIM4LCA office the top is Revit's on
/// all 22 landings, and a monolithic landing's underside is that thickness
/// below it but where a flight's soffit meets it. A landing whose type is
/// not monolithic is left out: the office's one is 0.0984 ft thick in
/// Revit's export against its type's 0.995 ft.
pub fn native_landings(
    rf: &mut RevitFile,
    landings: &BTreeMap<u64, u64>,
) -> anyhow::Result<BTreeMap<u64, NativeLanding>> {
    if landings.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: landings.keys().chain(landings.values()).copied().collect(),
        ..Options::default()
    };
    let mut tops: BTreeMap<u64, f64> = BTreeMap::new();
    let mut thicknesses: BTreeMap<u64, f64> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "StairsLanding" if landings.contains_key(&id) => {
                if let Some(top) = number(&root.fields, "m_baseElevation") {
                    tops.insert(id, top);
                }
            }
            "StairsLandingType" if flag(&root.fields, "m_hasMonolithicSupport") == Some(true) => {
                if let Some(thickness) = number(&root.fields, "m_thickness").filter(|t| *t > 0.0) {
                    thicknesses.insert(id, thickness);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(tops
        .into_iter()
        .filter_map(|(id, top_feet)| {
            let thickness_feet = *thicknesses.get(&landings[&id])?;
            Some((
                id,
                NativeLanding {
                    top_feet,
                    thickness_feet,
                },
            ))
        })
        .collect())
}
