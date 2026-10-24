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
//! - its `StairsRunType`: `m_treadThickness`, `m_riserThickness`,
//!   `m_nosingLength`, `m_structuralDepth`, `m_hasMonolithicSupport`,
//!   `m_hasTread`, `m_hasRiser`, `m_riserIsSlanted` and
//!   `m_riserTreadConnection` (1: the tread runs under the riser, Revit's
//!   `RiserToTreadConnectionOption.TreadUnderRiser`).
//!
//! These give [`crate::partition_stairs::RunSketch`] and
//! [`crate::partition_stairs::RunType`], which RE-92 draws the run from. A
//! run that is not straight, does not begin with a riser, or whose treads
//! do not share one depth, one riser height and one direction is left out.

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

/// The element ids `fields` holds at its top level, as `{"m_id": {"m_id64": n}}`.
fn referenced_ids(fields: &Value) -> BTreeSet<u64> {
    fields
        .as_object()
        .into_iter()
        .flat_map(|object| object.values())
        .filter_map(|v| v.get("m_id")?.get("m_id64")?.as_u64())
        .collect()
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
        },
        height,
    ))
}

/// Each run in `runs` whose native records give its sketch and type.
pub fn native_runs(
    rf: &mut RevitFile,
    runs: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, NativeRun>> {
    if runs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: runs.clone(),
        selected_classes: ["StairsRunType".to_string()].into(),
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
            "StairsRun" if runs.contains(&id) => {
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
    Ok(found
        .into_iter()
        .filter_map(|(id, (run, treads))| {
            let mut candidates = referenced_ids(&run)
                .into_iter()
                .filter(|t| types.contains_key(t));
            let type_id = candidates.next()?;
            if candidates.next().is_some() {
                return None;
            }
            let (sketch, riser_height_feet) = sketch(&run, &treads)?;
            Some((
                id,
                NativeRun {
                    sketch,
                    run_type: types[&type_id].clone(),
                    riser_height_feet,
                    end_with_riser: flag(&run, "m_endWithRiser"),
                    type_id,
                },
            ))
        })
        .collect())
}
