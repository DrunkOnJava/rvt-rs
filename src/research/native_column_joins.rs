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
//!
//! A column whose record holds no `CleanPairGStep` is one Revit's export
//! draws as an extrusion on every BIM4LCA office column: 92 as built and 70
//! at building permit; the 18 columns it draws as a cut mesh at permit each
//! hold one or more (RE-247).

use crate::native_json::element_id;
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

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

/// What [`joins`] reads of each column.
#[derive(Debug, Default)]
pub struct ColumnJoins {
    /// The elements each column is joined to, in the order the joins were
    /// made (its `CutPairGStep` objects by `m_id`), by the column's
    /// ElementId. A column with no join is left out.
    pub order: BTreeMap<u64, Vec<u64>>,
    /// The columns none of whose records holds a `CleanPairGStep` (RE-247).
    pub without_clean_steps: BTreeSet<u64>,
    /// The elements the `CutPairGStep` objects of each column's records
    /// name, by the column's ElementId: one entry for every column whose
    /// pair steps all name an element (RE-291).
    pub cut_partners: BTreeMap<u64, BTreeSet<u64>>,
    /// The same for `CleanPairGStep`.
    pub clean_partners: BTreeMap<u64, BTreeSet<u64>>,
}

/// The joins of each of `columns`, read in one pass. Errs where the native
/// record path does not read the file's release.
pub fn joins(rf: &mut RevitFile, columns: &BTreeSet<u64>) -> anyhow::Result<ColumnJoins> {
    let mut out = BTreeMap::new();
    let mut cleaned: BTreeMap<u64, bool> = BTreeMap::new();
    let mut cut_partners: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    let mut clean_partners: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    let mut unreadable: BTreeSet<u64> = BTreeSet::new();
    if columns.is_empty() {
        return Ok(ColumnJoins::default());
    }
    let options = native_document::Options {
        selected_ids: columns.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let clean = graph
            .objects
            .iter()
            .any(|object| object.class_name == "CleanPairGStep");
        *cleaned.entry(record.identity.element_id).or_default() |= clean;
        for (class, partners) in [
            ("CutPairGStep", &mut cut_partners),
            ("CleanPairGStep", &mut clean_partners),
        ] {
            let steps = graph
                .objects
                .iter()
                .filter(|object| object.class_name == class);
            let held = partners.entry(record.identity.element_id).or_default();
            for object in steps {
                match object.fields.get("m_otherId").and_then(element_id) {
                    Some(other) => {
                        held.insert(other);
                    }
                    None => {
                        unreadable.insert(record.identity.element_id);
                    }
                }
            }
        }
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
    for id in &unreadable {
        cut_partners.remove(id);
        clean_partners.remove(id);
    }
    Ok(ColumnJoins {
        order: out,
        without_clean_steps: cleaned
            .into_iter()
            .filter_map(|(id, clean)| (!clean).then_some(id))
            .collect(),
        cut_partners,
        clean_partners,
    })
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
