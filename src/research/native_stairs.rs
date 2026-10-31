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
//! ([`native_landings`], #824). A landing that is not monolithic and whose
//! type takes its tread from its runs is its outline's `CurveLoop`, arcs and
//! all, one tread of its runs' type thick ([`native_tread_landings`]).
//!
//! A run's treads all give one `m_runType`, which names the flight's
//! predefined type in Revit's export ([`native_run_shapes`], #323). A spiral
//! run's treads give the spiral's centre and each tread's radii and angles
//! ([`native_spiral_runs`], #357).
//!
//! A stair's, a run's and a stringer's type, its name and the family Revit's
//! export names it after come from the same records on Revit 2025
//! ([`native_stair_parts`], #323), where RE-65's byte anchors are absent.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::RevitFile;
use crate::native_document::{Options, extract_graphs};
use crate::native_json::finite_triple;
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
    finite_triple(v.get(name)?)
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

/// A run's shape and path, from its native records (#323).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunShape {
    /// The `m_runType` every one of its treads gives: 0 on a straight run,
    /// 2 on a spiral one.
    pub run_type: i64,
    /// The plan direction it climbs in, its first tread's `m_runDirProj`.
    pub direction: Option<[f64; 2]>,
}

/// Each run in `runs` whose native `StairsTreadInfo` records all give one
/// `m_runType`, with that value and its climb direction (#323).
pub fn native_run_shapes(
    rf: &mut RevitFile,
    runs: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, RunShape>> {
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
        let treads: Vec<&Value> = graph
            .objects
            .iter()
            .filter(|o| o.class_name == "StairsTreadInfo")
            .map(|o| &o.fields)
            .collect();
        let shapes: BTreeSet<Option<i64>> = treads
            .iter()
            .map(|t| t.get("m_runType").and_then(Value::as_i64))
            .collect();
        if let [Some(run_type)] = shapes.into_iter().collect::<Vec<_>>().as_slice() {
            out.insert(
                record.identity.element_id,
                RunShape {
                    run_type: *run_type,
                    direction: treads
                        .first()
                        .and_then(|t| point(t, "m_runDirProj"))
                        .and_then(unit_plan),
                },
            );
        }
        Ok(())
    })?;
    Ok(out)
}

/// The `IfcStairTypeEnum` values [`stair_predefined_type`] gives.
pub const STAIR_TYPES: &[&str] = &[
    "SPIRAL_STAIR",
    "STRAIGHT_RUN_STAIR",
    "TWO_STRAIGHT_RUN_STAIR",
    "QUARTER_TURN_STAIR",
    "HALF_TURN_STAIR",
    "DOUBLE_RETURN_STAIR",
    "THREE_QUARTER_TURN_STAIR",
];

/// The `IfcStairTypeEnum` value Revit's export gives a stair from its runs,
/// in ElementId order, and its number of landings, as revit-ifc's
/// `StairsExporter.GetValidatedStairType` decides it (CLEANROOM.md source
/// 5): a stair with a spiral run is `SPIRAL_STAIR`; one straight run is
/// `STRAIGHT_RUN_STAIR`; two straight runs and one landing are
/// `TWO_STRAIGHT_RUN_STAIR`, `QUARTER_TURN_STAIR` or `HALF_TURN_STAIR` as
/// the second climbs the same way as the first, square to it or back;
/// three straight runs each square to the one before, with two landings,
/// are `DOUBLE_RETURN_STAIR`; four such runs and three landings are
/// `THREE_QUARTER_TURN_STAIR`. revit-ifc calls three runs
/// `DOUBLE_RETURN_STAIR` only where two of their flight paths start at one
/// height, else `TWO_QUARTER_TURN_STAIR`; Revit's export writes
/// `DOUBLE_RETURN_STAIR` for all ten of the BIM4LCA office's such stairs,
/// whose runs start at three heights each, so a run's path does not start
/// at its own bottom, and the test always holds there. `None` for any
/// other stair, and for one with a run that is neither straight nor spiral
/// (winders and sketched runs are not measured), which Revit writes
/// `.NOTDEFINED.`.
pub fn stair_predefined_type(runs: &[RunShape], landings: usize) -> Option<&'static str> {
    const EPS: f64 = 1e-6;
    if runs.is_empty()
        || runs
            .iter()
            .any(|run| run.run_type != 0 && run.run_type != 2)
    {
        return None;
    }
    if runs.iter().any(|run| run.run_type == 2) {
        return Some("SPIRAL_STAIR");
    }
    let directions: Vec<[f64; 2]> = runs
        .iter()
        .map(|run| run.direction)
        .collect::<Option<_>>()?;
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let square = |pairs: &[[f64; 2]]| pairs.windows(2).all(|w| dot(w[0], w[1]).abs() <= EPS);
    match (directions.as_slice(), landings) {
        ([_], _) => Some("STRAIGHT_RUN_STAIR"),
        ([a, b], 1) => {
            let d = dot(*a, *b);
            if (d - 1.0).abs() <= EPS {
                Some("TWO_STRAIGHT_RUN_STAIR")
            } else if d.abs() <= EPS {
                Some("QUARTER_TURN_STAIR")
            } else if (d + 1.0).abs() <= EPS {
                Some("HALF_TURN_STAIR")
            } else {
                None
            }
        }
        ([_, _, _], 2) if square(&directions) => Some("DOUBLE_RETURN_STAIR"),
        ([_, _, _, _], 3) if square(&directions) => Some("THREE_QUARTER_TURN_STAIR"),
        _ => None,
    }
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

/// A Revit 2025 stair's actual number of risers, riser height and tread
/// depth, feet: its `StairsElement`'s `m_actualNumberOfRisers`,
/// `m_actualRiserHeight` and `m_actualTreadDepth`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeStairCounts {
    pub risers: i64,
    pub riser_height_feet: f64,
    pub tread_depth_feet: f64,
}

/// A Revit 2025 run's stair (`m_stairsId`), its risers (`m_topRiserIndex`
/// less `m_baseRiserIndex`), its height (`m_topElevation` less
/// `m_bottomElevation`, feet) and whether it begins and ends with a riser.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeRunCounts {
    pub stair: u64,
    pub risers: i64,
    pub height_feet: f64,
    pub begin_with_riser: bool,
    pub end_with_riser: bool,
}

/// The counts of each stair in `stairs` and each run in `runs` from their
/// native records, in one pass. On all five R2025 stair models they give the
/// `Pset_StairCommon` and `Pset_StairFlightCommon` values Revit's export
/// writes, as revit-ifc's calculators take them (CLEANROOM.md source 5): a
/// stair's actual risers, riser height and tread depth; a run's risers, its
/// height over its risers and its stair's tread depth.
pub fn native_stair_counts(
    rf: &mut RevitFile,
    stairs: &BTreeSet<u64>,
    runs: &BTreeSet<u64>,
) -> anyhow::Result<(
    BTreeMap<u64, NativeStairCounts>,
    BTreeMap<u64, NativeRunCounts>,
)> {
    let mut stair_counts = BTreeMap::new();
    let mut run_counts = BTreeMap::new();
    if stairs.is_empty() && runs.is_empty() {
        return Ok((stair_counts, run_counts));
    }
    let options = Options {
        selected_ids: stairs.union(runs).copied().collect(),
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(root)) = (
            record.class_name.as_deref(),
            record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first()),
        ) else {
            return Ok(());
        };
        let fields = &root.fields;
        let id = record.identity.element_id;
        match class {
            "StairsElement" if stairs.contains(&id) => {
                if let (Some(risers), Some(riser_height_feet), Some(tread_depth_feet)) = (
                    fields.get("m_actualNumberOfRisers").and_then(Value::as_i64),
                    number(fields, "m_actualRiserHeight").filter(|h| *h > 0.0),
                    number(fields, "m_actualTreadDepth").filter(|d| *d > 0.0),
                ) {
                    stair_counts.insert(
                        id,
                        NativeStairCounts {
                            risers,
                            riser_height_feet,
                            tread_depth_feet,
                        },
                    );
                }
            }
            "StairsRun" if runs.contains(&id) => {
                let index = |name: &str| fields.get(name).and_then(Value::as_i64);
                if let (
                    Some(stair),
                    Some(top),
                    Some(base),
                    Some(top_elevation),
                    Some(bottom_elevation),
                    Some(begin_with_riser),
                    Some(end_with_riser),
                ) = (
                    element_id(fields, "m_stairsId"),
                    index("m_topRiserIndex"),
                    index("m_baseRiserIndex"),
                    number(fields, "m_topElevation"),
                    number(fields, "m_bottomElevation"),
                    flag(fields, "m_beginWithRiser"),
                    flag(fields, "m_endWithRiser"),
                ) {
                    let height_feet = top_elevation - bottom_elevation;
                    if top > base && height_feet > 0.0 {
                        run_counts.insert(
                            id,
                            NativeRunCounts {
                                stair,
                                risers: top - base,
                                height_feet,
                                begin_with_riser,
                                end_with_riser,
                            },
                        );
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok((stair_counts, run_counts))
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
/// whose native records give its treads (see [`NativeSpiralRun`]) and whose type's record
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
    Ok(native_landings_and_type_names(rf, landings)?.0)
}

/// [`native_landings`], with each landing type's name (its
/// `SymbolInfo.m_name`), monolithic or not, from the same pass.
pub fn native_landings_and_type_names(
    rf: &mut RevitFile,
    landings: &BTreeMap<u64, u64>,
) -> anyhow::Result<(BTreeMap<u64, NativeLanding>, BTreeMap<u64, String>)> {
    if landings.is_empty() {
        return Ok(Default::default());
    }
    let options = Options {
        selected_ids: landings.keys().chain(landings.values()).copied().collect(),
        ..Options::default()
    };
    let mut tops: BTreeMap<u64, f64> = BTreeMap::new();
    let mut thicknesses: BTreeMap<u64, f64> = BTreeMap::new();
    let mut names: BTreeMap<u64, String> = BTreeMap::new();
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
            "StairsLandingType" => {
                if let Some(name) = graph
                    .objects
                    .iter()
                    .find(|o| o.class_name == "SymbolInfo")
                    .and_then(|o| o.fields.get("m_name")?.as_str())
                {
                    names.insert(id, name.to_string());
                }
                let monolithic = flag(&root.fields, "m_hasMonolithicSupport") == Some(true);
                if let Some(thickness) =
                    number(&root.fields, "m_thickness").filter(|t| monolithic && *t > 0.0)
                {
                    thicknesses.insert(id, thickness);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let found = tops
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
        .collect();
    Ok((found, names))
}

/// Releases a railing type's top rail height is read on (RE-231).
pub const RAILING_HEIGHT_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// Each railing type in `types` whose native record's `StairsRailingAttr`
/// gives its top rail height (`m_topRailHeight`), feet: Revit's
/// `Pset_RailingCommon.Height` on all 41 railings of the BIM4LCA office
/// (Revit 2024) and RE1 Architecture's one (Revit 2025) (RE-231).
pub fn railing_type_heights(
    rf: &mut RevitFile,
    types: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, f64>> {
    if types.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: types.clone(),
        ..Options::default()
    };
    let mut out = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        if let Some(height) = graph
            .objects
            .iter()
            .find(|o| o.class_name == "StairsRailingAttr")
            .and_then(|o| number(&o.fields, "m_topRailHeight"))
            .filter(|h| *h > 0.0)
        {
            out.insert(record.identity.element_id, height);
        }
        Ok(())
    })?;
    Ok(out)
}

/// A sketch `GLine`'s plan direction (`m_dirVec`) and length, the span of
/// its `m_endParams`, feet.
fn sketch_line(fields: &Value) -> Option<([f64; 2], f64)> {
    let ends = fields.get("m_endParams")?.as_array()?;
    let dir = fields.get("m_dirVec")?.as_array()?;
    let span = (ends.get(1)?.as_f64()? - ends.first()?.as_f64()?).abs();
    let dir = [dir.first()?.as_f64()?, dir.get(1)?.as_f64()?];
    (span.is_finite() && dir.iter().all(|c| c.is_finite())).then_some((dir, span))
}

/// Releases a stair railing's `Length` is read on (RE-254).
pub const RAILING_LENGTH_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// The `Length` Revit's export writes on each railing in `railings` that
/// belongs to a stair (`BaseRailing.m_stairsId`) and whose path, the sketch
/// its record names (`m_sketchId`), is one straight line along one or more
/// of that stair's runs, feet (RE-254). The line climbs at the stair's
/// riser height over its tread depth (its `StairsElement`'s actual counts)
/// for as long as the longest of those runs climbs, its risers less one
/// times the tread depth (`m_topRiserIndex` less `m_baseRiserIndex`, less
/// one), and runs level for the rest: Revit's `Length` on every such railing
/// of the BIM4LCA office and the R2025 stair models. A railing whose path
/// has more lines, or an arc, is left out: over a stair well with landings
/// the climb is not read yet.
pub fn railing_lengths(
    rf: &mut RevitFile,
    railings: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, f64>> {
    if railings.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut held: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    let options = Options {
        selected_ids: railings.clone(),
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let root = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.iter().find(|o| o.class_name == "BaseRailing"));
        if let Some((sketch, stair)) = root.and_then(|root| {
            Some((
                element_id(&root.fields, "m_sketchId")?,
                element_id(&root.fields, "m_stairsId")?,
            ))
        }) {
            held.insert(record.identity.element_id, (sketch, stair));
        }
        Ok(())
    })?;
    if held.is_empty() {
        return Ok(BTreeMap::new());
    }
    let sketches: BTreeSet<u64> = held.values().map(|(sketch, _)| *sketch).collect();
    let stairs: BTreeSet<u64> = held.values().map(|(_, stair)| *stair).collect();
    type Line = ([f64; 2], f64);
    let mut paths: BTreeMap<u64, Option<Vec<Line>>> = BTreeMap::new();
    let mut slopes: BTreeMap<u64, (f64, f64)> = BTreeMap::new();
    let mut runs: Vec<(u64, i64, [f64; 2])> = Vec::new();
    let options = Options {
        selected_ids: sketches.union(&stairs).copied().collect(),
        selected_classes: ["StairsRun".to_string()].into(),
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match record.class_name.as_deref() {
            Some("StairsElement") if stairs.contains(&id) => {
                if let Some(fields) = graph.objects.first().map(|o| &o.fields) {
                    if let (Some(riser), Some(tread)) = (
                        number(fields, "m_actualRiserHeight").filter(|h| *h > 0.0),
                        number(fields, "m_actualTreadDepth").filter(|d| *d > 0.0),
                    ) {
                        slopes.insert(id, (riser, tread));
                    }
                }
            }
            Some("StairsRun") => {
                let Some(fields) = graph.objects.first().map(|o| &o.fields) else {
                    return Ok(());
                };
                let index = |name: &str| fields.get(name).and_then(Value::as_i64);
                let counted = (
                    element_id(fields, "m_stairsId"),
                    index("m_topRiserIndex"),
                    index("m_baseRiserIndex"),
                    number(fields, "m_topElevation"),
                    number(fields, "m_bottomElevation"),
                    flag(fields, "m_beginWithRiser"),
                    flag(fields, "m_endWithRiser"),
                );
                let (
                    Some(stair),
                    Some(top),
                    Some(base),
                    Some(top_z),
                    Some(bottom_z),
                    Some(_),
                    Some(_),
                ) = counted
                else {
                    return Ok(());
                };
                if !stairs.contains(&stair) || top <= base || top_z - bottom_z <= 0.0 {
                    return Ok(());
                }
                let treads: Vec<&Value> = graph
                    .objects
                    .iter()
                    .filter(|o| o.class_name == "StairsTreadInfo")
                    .map(|o| &o.fields)
                    .collect();
                let kinds: BTreeSet<Option<i64>> = treads
                    .iter()
                    .map(|t| t.get("m_runType").and_then(Value::as_i64))
                    .collect();
                if kinds.len() != 1 || kinds.contains(&None) {
                    return Ok(());
                }
                if let Some(direction) = treads
                    .first()
                    .and_then(|t| point(t, "m_runDirProj"))
                    .and_then(unit_plan)
                {
                    runs.push((stair, top - base, direction));
                }
            }
            _ if sketches.contains(&id) => {
                let mut lines = Some(Vec::new());
                for object in &graph.objects {
                    let name = object.class_name.as_str();
                    if !name.starts_with('G')
                        || !(name.contains("Line")
                            || name.contains("Arc")
                            || name.contains("Curve"))
                    {
                        continue;
                    }
                    let line = if name == "GLine" {
                        sketch_line(&object.fields)
                    } else {
                        None
                    };
                    match (line, lines.as_mut()) {
                        (Some(line), Some(found)) => found.push(line),
                        _ => lines = None,
                    }
                }
                paths.insert(id, lines);
            }
            _ => {}
        }
        Ok(())
    })?;
    let mut out = BTreeMap::new();
    for (railing, (sketch, stair)) in held {
        let (Some(Some(path)), Some(&(riser, tread))) = (paths.get(&sketch), slopes.get(&stair))
        else {
            continue;
        };
        let [(dir, span)] = path.as_slice() else {
            continue;
        };
        let longest = runs
            .iter()
            .filter(|(of, _, d)| *of == stair && (dir[0] * d[1] - dir[1] * d[0]).abs() < 1e-6)
            .map(|(_, risers, _)| (risers - 1).max(0) as f64 * tread)
            .fold(0.0_f64, f64::max);
        if longest > 0.0 && *span > 0.0 {
            let climb = span.min(longest);
            out.insert(railing, span + climb * ((riser / tread).hypot(1.0) - 1.0));
        }
    }
    Ok(out)
}

/// Releases a stair's and its parts' names are read from their native
/// records on: the R2025 stair models (#323).
pub const NATIVE_STAIR_NAME_REVIT_VERSIONS: &[u32] = &[2025];

/// A stair's or a stair part's type, and the stair and family Revit's export
/// names it after (#323).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeStairPart {
    /// The stair it is part of; a stair's own id for a stair.
    pub stair: u64,
    /// Its type's ElementId.
    pub type_id: u64,
    /// Its type's `SymbolInfo.m_name`.
    pub type_name: String,
    /// The family Revit's export names it after.
    pub family: &'static str,
}

fn element_id(v: &Value, name: &str) -> Option<u64> {
    v.get(name)?.get("m_id")?.get("m_id64")?.as_u64()
}

/// The stair, type and family of each stair, run and support in `parts`,
/// from their native records, as Revit's export names them (RE-65) on all
/// five R2025 stair models:
///
/// - a `StairsElement`'s type is its `m_typeId`, a `StairsType`, whose
///   `m_constructionMethod` names the family: 0 `Assembled Stair`, 1
///   `Cast-In-Place Stair`;
/// - a `StairsRun`'s stair is its `m_stairsId` and its type the one `runs`
///   gives (its element record's, as for [`native_runs`]), a
///   `StairsRunType` whose `m_hasMonolithicSupport` names the family
///   `Monolithic Run` or `Non-Monolithic Run`;
/// - a `StairsSupport`'s stair is its host's (`m_hostCompId`, a run) and its
///   type its `m_typeId`; its family is `Stringer` where every side of its
///   stair's type that uses that type (`m_leftSideSupportTypeId`,
///   `m_rightSideSupportTypeId`) has support style 1.
///
/// Each type's name is its `SymbolInfo.m_name`. Any other construction or
/// support style, a landing, and a part whose stair or type is not read is
/// left out: none is measured.
pub fn native_stair_parts(
    rf: &mut RevitFile,
    parts: &BTreeSet<u64>,
    runs: &BTreeMap<u64, u64>,
) -> anyhow::Result<BTreeMap<u64, NativeStairPart>> {
    if parts.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut stairs: BTreeMap<u64, u64> = BTreeMap::new();
    let mut run_stairs: BTreeMap<u64, u64> = BTreeMap::new();
    let mut supports: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    let options = Options {
        selected_ids: parts.clone(),
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(root)) = (
            record.class_name.as_deref(),
            record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first()),
        ) else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "StairsElement" => {
                if let Some(type_id) = element_id(&root.fields, "m_typeId") {
                    stairs.insert(id, type_id);
                }
            }
            "StairsRun" => {
                if let Some(stair) = element_id(&root.fields, "m_stairsId") {
                    run_stairs.insert(id, stair);
                }
            }
            "StairsSupport" => {
                if let (Some(host), Some(type_id)) = (
                    element_id(&root.fields, "m_hostCompId"),
                    element_id(&root.fields, "m_typeId"),
                ) {
                    supports.insert(id, (host, type_id));
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let type_ids: BTreeSet<u64> = stairs
        .values()
        .copied()
        .chain(runs.values().copied())
        .chain(supports.values().map(|(_, type_id)| *type_id))
        .collect();
    let mut names: BTreeMap<u64, String> = BTreeMap::new();
    let mut constructions: BTreeMap<u64, i64> = BTreeMap::new();
    // Each stair type's left and right support: its type and its style.
    type Side = (Option<u64>, Option<i64>);
    let mut sides: BTreeMap<u64, [Side; 2]> = BTreeMap::new();
    let mut monolithic: BTreeMap<u64, bool> = BTreeMap::new();
    let options = Options {
        selected_ids: type_ids,
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if let Some(name) = graph
            .objects
            .iter()
            .find(|o| o.class_name == "SymbolInfo")
            .and_then(|o| o.fields.get("m_name")?.as_str())
        {
            names.insert(id, name.to_string());
        }
        let style = |name: &str| root.fields.get(name).and_then(Value::as_i64);
        match class {
            "StairsType" => {
                if let Some(method) = style("m_constructionMethod") {
                    constructions.insert(id, method);
                }
                sides.insert(
                    id,
                    [
                        (
                            element_id(&root.fields, "m_leftSideSupportTypeId"),
                            style("m_leftSupportStyle"),
                        ),
                        (
                            element_id(&root.fields, "m_rightSideSupportTypeId"),
                            style("m_rightSupportStyle"),
                        ),
                    ],
                );
            }
            "StairsRunType" => {
                if let Some(flag) = flag(&root.fields, "m_hasMonolithicSupport") {
                    monolithic.insert(id, flag);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let mut out = BTreeMap::new();
    let mut part = |id: u64, stair: u64, type_id: u64, family: Option<&'static str>| {
        if let (Some(family), Some(type_name)) = (family, names.get(&type_id)) {
            out.insert(
                id,
                NativeStairPart {
                    stair,
                    type_id,
                    type_name: type_name.clone(),
                    family,
                },
            );
        }
    };
    for (&id, &type_id) in &stairs {
        let family = match constructions.get(&type_id) {
            Some(0) => Some("Assembled Stair"),
            Some(1) => Some("Cast-In-Place Stair"),
            _ => None,
        };
        part(id, id, type_id, family);
    }
    for (&id, &stair) in &run_stairs {
        let Some(&type_id) = runs.get(&id) else {
            continue;
        };
        let family = match monolithic.get(&type_id) {
            Some(true) => Some("Monolithic Run"),
            Some(false) => Some("Non-Monolithic Run"),
            None => None,
        };
        part(id, stair, type_id, family);
    }
    for (&id, &(host, type_id)) in &supports {
        let Some(&stair) = run_stairs.get(&host) else {
            continue;
        };
        let styles: Vec<Option<i64>> = stairs
            .get(&stair)
            .and_then(|stair_type| sides.get(stair_type))
            .into_iter()
            .flatten()
            .filter(|(side_type, _)| *side_type == Some(type_id))
            .map(|(_, style)| *style)
            .collect();
        let family = (!styles.is_empty() && styles.iter().all(|style| *style == Some(1)))
            .then_some("Stringer");
        part(id, stair, type_id, family);
    }
    Ok(out)
}

/// One curve of a landing's plan outline, model feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LandingCurve {
    /// A straight edge from the first point to the second.
    Line([f64; 2], [f64; 2]),
    /// A circular arc from the first point through the second to the third.
    Arc([f64; 2], [f64; 2], [f64; 2]),
}

impl LandingCurve {
    /// Where the curve starts.
    pub fn start(&self) -> [f64; 2] {
        match self {
            LandingCurve::Line(a, _) | LandingCurve::Arc(a, _, _) => *a,
        }
    }

    /// Where the curve ends.
    pub fn end(&self) -> [f64; 2] {
        match self {
            LandingCurve::Line(_, b) | LandingCurve::Arc(_, _, b) => *b,
        }
    }
}

/// A landing drawn as one tread: its outline, its top and its thickness,
/// feet (#824).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeTreadLanding {
    pub outline: Vec<LandingCurve>,
    pub top_feet: f64,
    pub thickness_feet: f64,
}

/// How far a landing outline's curves may miss each other's ends, or its
/// top, and still close, feet.
pub const LANDING_LOOP_TOLERANCE_FEET: f64 = 1e-5;

/// The landing's outline: the first closed `CurveLoop` of its record whose
/// curves (`GLine`, `GArc`) all lie at `top` and each start where the one
/// before ends.
fn landing_loop(
    graph: &crate::native_parameters::ObjectGraph,
    top: f64,
) -> Option<Vec<LandingCurve>> {
    let eps = LANDING_LOOP_TOLERANCE_FEET;
    graph
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.class_name == "CurveLoop")
        .find_map(|(index, object)| {
            if flag(&object.fields, "m_open") != Some(false) {
                return None;
            }
            let curves = object.fields.get("m_curves")?.as_array()?;
            let mut out = Vec::new();
            for curve in curves {
                let target = crate::native_saved_mesh::pointer(graph, index, curve).ok()??;
                let curve = graph.objects.get(target)?;
                let params = curve.fields.get("m_endParams")?.as_array()?;
                let [t0, t1] = params.as_slice() else {
                    return None;
                };
                let (t0, t1) = (t0.as_f64()?, t1.as_f64()?);
                let points: Vec<[f64; 3]> = match curve.class_name.as_str() {
                    "GLine" => {
                        let origin = point(&curve.fields, "m_origin")?;
                        let direction = point(&curve.fields, "m_dirVec")?;
                        [t0, t1]
                            .iter()
                            .map(|t| [0, 1, 2].map(|i| origin[i] + direction[i] * t))
                            .collect()
                    }
                    "GArc" => {
                        let centre = point(&curve.fields, "m_center")?;
                        let x = point(&curve.fields, "m_xVec")?;
                        let y = point(&curve.fields, "m_yVec")?;
                        let radius = number(&curve.fields, "m_radius")?;
                        [t0, (t0 + t1) / 2.0, t1]
                            .iter()
                            .map(|t| {
                                [0, 1, 2]
                                    .map(|i| centre[i] + radius * (t.cos() * x[i] + t.sin() * y[i]))
                            })
                            .collect()
                    }
                    _ => return None,
                };
                if points.iter().any(|p| (p[2] - top).abs() > eps) {
                    return None;
                }
                let plan: Vec<[f64; 2]> = points.iter().map(|p| [p[0], p[1]]).collect();
                out.push(match plan.as_slice() {
                    [a, b] => LandingCurve::Line(*a, *b),
                    [a, m, b] => LandingCurve::Arc(*a, *m, *b),
                    _ => return None,
                });
            }
            let joined = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) <= eps;
            (out.len() >= 3
                && (0..out.len()).all(|i| joined(out[i].end(), out[(i + 1) % out.len()].start())))
            .then_some(out)
        })
}

/// Each landing in `landings`, keyed by its id with its own landing type's
/// id, whose type is not monolithic (`m_hasMonolithicSupport`), has a tread
/// (`m_hasTread`) and takes it from its runs (`m_useSameTriserAsRun`), drawn
/// as that tread: its outline is its record's `CurveLoop` at its top
/// (`m_baseElevation`), arcs and all, and its thickness the
/// `m_treadThickness` of its stair's runs' type, the runs in `runs` (keyed
/// by id with their own run type's id) whose `m_stairsId` is the landing's.
/// On the BIM4LCA office's landing 3058084, beside a spiral run, that is
/// Revit's outline, its arc on the spiral's inner circle, and Revit's
/// 0.0984 ft. A landing whose stair's runs name more than one tread
/// thickness, or whose type takes its own tread, is left out: neither is
/// measured.
pub fn native_tread_landings(
    rf: &mut RevitFile,
    landings: &BTreeMap<u64, u64>,
    runs: &BTreeMap<u64, u64>,
) -> anyhow::Result<BTreeMap<u64, NativeTreadLanding>> {
    if landings.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: landings
            .keys()
            .chain(landings.values())
            .chain(runs.keys())
            .chain(runs.values())
            .copied()
            .collect(),
        ..Options::default()
    };
    // Each landing's stair, top and outline.
    type Found = (u64, f64, Vec<LandingCurve>);
    let mut found: BTreeMap<u64, Found> = BTreeMap::new();
    let mut tread_types: BTreeSet<u64> = BTreeSet::new();
    let mut run_stairs: BTreeMap<u64, u64> = BTreeMap::new();
    let mut treads: BTreeMap<u64, f64> = BTreeMap::new();
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
                if let (Some(stair), Some(top)) = (
                    element_id(&root.fields, "m_stairsId"),
                    number(&root.fields, "m_baseElevation"),
                ) {
                    if let Some(outline) = landing_loop(graph, top) {
                        found.insert(id, (stair, top, outline));
                    }
                }
            }
            "StairsLandingType"
                if flag(&root.fields, "m_hasMonolithicSupport") == Some(false)
                    && flag(&root.fields, "m_hasTread") == Some(true)
                    && flag(&root.fields, "m_useSameTriserAsRun") == Some(true) =>
            {
                tread_types.insert(id);
            }
            "StairsRun" if runs.contains_key(&id) => {
                if let Some(stair) = element_id(&root.fields, "m_stairsId") {
                    run_stairs.insert(id, stair);
                }
            }
            "StairsRunType" => {
                if let Some(thickness) =
                    number(&root.fields, "m_treadThickness").filter(|t| *t > 0.0)
                {
                    treads.insert(id, thickness);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(found
        .into_iter()
        .filter(|(id, _)| landings.get(id).is_some_and(|t| tread_types.contains(t)))
        .filter_map(|(id, (stair, top_feet, outline))| {
            let thicknesses: Vec<f64> = run_stairs
                .iter()
                .filter(|(_, run_stair)| **run_stair == stair)
                .map(|(run, _)| runs.get(run).and_then(|t| treads.get(t)).copied())
                .collect::<Option<_>>()?;
            let thickness_feet = *thicknesses.first()?;
            thicknesses
                .iter()
                .all(|t| (t - thickness_feet).abs() <= NATIVE_RUN_TOLERANCE_FEET)
                .then_some((
                    id,
                    NativeTreadLanding {
                        outline,
                        top_feet,
                        thickness_feet,
                    },
                ))
        })
        .collect())
}
