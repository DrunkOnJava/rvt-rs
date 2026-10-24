//! The wall a column is first joined to, and which side of that wall it
//! stands on, from the native records of the column and the wall (RE-202,
//! #355).
//!
//! A column's record holds one `CutPairGStep` per element it is joined to,
//! in the order the joins were made (`m_id`), naming the element in
//! `m_otherId`. A wall's record names its location curve in
//! `VWallDriver.m_pCrv`; its exterior lies on the left of that curve's
//! direction, in plan, unless the wall is flipped (`SWall.m_isFlipped`;
//! `VWallDriver.m_flip` is false on all 198 walls joined to Snowdon Towers'
//! columns). Revit's export gives an architectural column whose type draws
//! with no material the layer of its first joined wall on the side the
//! column stands: 106 of Snowdon Towers' 108 off-centre columns (RE-202).

use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// `{"m_id":{"m_id64":n}}` as `n`.
fn element_id(value: &serde_json::Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(serde_json::Value::as_i64)
        .and_then(|n| u64::try_from(n).ok())
}

/// A straight wall's location line in plan and the unit normal toward its
/// exterior face.
#[derive(Clone, Copy, Debug)]
pub struct WallSide {
    pub origin: [f64; 2],
    pub exterior: [f64; 2],
}

impl WallSide {
    /// Whether `point` lies on the wall's exterior side.
    pub fn is_exterior(&self, point: [f64; 2]) -> bool {
        self.offset(point, self.origin) > 0.0
    }

    /// How far `point` lies toward the exterior from a line through
    /// `through` parallel to the wall.
    pub fn offset(&self, point: [f64; 2], through: [f64; 2]) -> f64 {
        (point[0] - through[0]) * self.exterior[0] + (point[1] - through[1]) * self.exterior[1]
    }
}

/// The elements each of `columns` is joined to, in the order the joins were
/// made (its `CutPairGStep` objects by `m_id`), by the column's ElementId. A
/// column with no join is left out. Errs where the native record path does
/// not read the file's release.
pub fn joins(
    rf: &mut RevitFile,
    columns: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, Vec<u64>>> {
    let mut out = BTreeMap::new();
    if columns.is_empty() {
        return Ok(out);
    }
    let options = native_document::Options {
        selected_ids: columns.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let mut steps: Vec<(i64, u64)> = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "CutPairGStep")
            .filter_map(|object| {
                let step = object.fields.get("m_id")?.as_i64()?;
                Some((step, element_id(object.fields.get("m_otherId")?)?))
            })
            .collect();
        steps.sort_unstable();
        if !steps.is_empty() {
            out.insert(
                record.identity.element_id,
                steps.into_iter().map(|(_, other)| other).collect(),
            );
        }
        Ok(())
    })?;
    Ok(out)
}

/// Each of `walls` whose location curve is a line, by the wall's ElementId.
/// Errs where the native record path does not read the file's release.
pub fn wall_sides(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, WallSide>> {
    let mut out = BTreeMap::new();
    if walls.is_empty() {
        return Ok(out);
    }
    let options = native_document::Options {
        selected_ids: walls.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let Some(flipped) = root
            .fields
            .get("m_isFlipped")
            .and_then(serde_json::Value::as_bool)
        else {
            return Ok(());
        };
        let Some((driver, offset)) = graph.objects.iter().enumerate().find_map(|(i, object)| {
            (object.class_name == "VWallDriver")
                .then(|| object.fields.get("m_pCrv")?["offset"].as_u64())
                .flatten()
                .map(|offset| (i, offset))
        }) else {
            return Ok(());
        };
        let Some(curve) = graph
            .edges
            .iter()
            .find(|edge| edge.source_object_index == driver && edge.pointer_offset as u64 == offset)
            .and_then(|edge| graph.objects.get(edge.target_object_index))
        else {
            return Ok(());
        };
        if curve.class_name != "GLine" {
            return Ok(());
        }
        let xy = |name: &str| -> Option<[f64; 2]> {
            let v = curve.fields.get(name)?.as_array()?;
            Some([v.first()?.as_f64()?, v.get(1)?.as_f64()?])
        };
        let (Some(origin), Some(direction)) = (xy("m_origin"), xy("m_dirVec")) else {
            return Ok(());
        };
        let length = direction[0].hypot(direction[1]);
        if length < 1e-9 {
            return Ok(());
        }
        // Left of the direction, or right where the wall is flipped.
        let sign = if flipped { -1.0 } else { 1.0 };
        let exterior = [-direction[1] / length * sign, direction[0] / length * sign];
        out.insert(record.identity.element_id, WallSide { origin, exterior });
        Ok(())
    })?;
    Ok(out)
}
