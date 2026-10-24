//! The outline a door or window cuts in its host, from the native record of
//! the family symbol that draws it (RE-215).
//!
//! A `FamilySymbol` record's `m_pCutLoops` points at one `CurveLoop`, whose
//! `m_curves` are the `GLine`s of the outline the symbol cuts in its host,
//! in the symbol's XZ plane. A `FamilyInstance` names its type in
//! `m_masterSymbolId`, and the symbol that draws it in its `InstanceInfo`
//! object's `m_symbolId`. The two differ where the instance's own parameters
//! change the family's geometry: Core Interior's windows are of type 17335,
//! whose loop is 3.3136 ft square, and are drawn by symbol 19275, whose loop
//! is 4.9213 by 8.8583 ft. Revit's exporter sizes a door or window from that
//! symbol (revit-ifc's `ExporterIFCUtils.GetOriginalSymbol`, CLEANROOM.md
//! source 5). The loop's extent along X and along Z is the `OverallWidth`
//! and `OverallHeight` Revit's export writes on every door and window
//! measured with a cut loop: Core Interior's 132 doors and 6 windows, RE1
//! Architecture's 4 doors, and the door and 3 windows of each of the R2025
//! models 12 to 17 (`reports/element-framing/RE-215-symbol-cut-loops.md`).

use crate::{RevitFile, native_document, native_saved_mesh::pointer};
use std::collections::{BTreeMap, BTreeSet};

/// The farthest apart, in feet, two ends of the loop's lines may be and
/// still join, and the most the loop may stray from its plane.
const JOIN_TOLERANCE_FEET: f64 = 1e-6;

/// The box of the outline a family symbol cuts in its host, in the symbol's
/// own coordinates, feet: its width runs along X and its height along Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutLoop {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl CutLoop {
    /// The loop's extent along the symbol's X axis, feet.
    pub fn width_feet(&self) -> f64 {
        self.max[0] - self.min[0]
    }

    /// The loop's extent along the symbol's Z axis, feet.
    pub fn height_feet(&self) -> f64 {
        self.max[2] - self.min[2]
    }
}

/// `{"m_id":{"m_id64":n}}` as `n`.
fn element_id(value: &serde_json::Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(serde_json::Value::as_i64)
        .and_then(|n| u64::try_from(n).ok())
}

fn point(value: Option<&serde_json::Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    let [x, y, z] = items.as_slice() else {
        return None;
    };
    let p = [x.as_f64()?, y.as_f64()?, z.as_f64()?];
    p.iter().all(|v| v.is_finite()).then_some(p)
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// The cut loop of a `FamilySymbol` graph: one loop, every curve a `GLine`,
/// each line's start the previous line's end, closed, flat in the symbol's
/// XZ plane and with some width and height. Anything else is `None`.
fn cut_loop(graph: &crate::native_parameters::ObjectGraph) -> Option<CutLoop> {
    let root = graph.objects.first()?;
    let [loop_pointer] = root.fields.get("m_pCutLoops")?.as_array()?.as_slice() else {
        return None;
    };
    let index = pointer(graph, 0, loop_pointer).ok()??;
    let curve_loop = graph.objects.get(index)?;
    if curve_loop.class_name != "CurveLoop"
        || curve_loop
            .fields
            .get("m_open")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
    {
        return None;
    }
    let mut ends = Vec::new();
    for curve in curve_loop.fields.get("m_curves")?.as_array()? {
        let line = graph.objects.get(pointer(graph, index, curve).ok()??)?;
        if line.class_name != "GLine" {
            return None;
        }
        let origin = point(line.fields.get("m_origin"))?;
        let direction = point(line.fields.get("m_dirVec"))?;
        let [start, end] = point2(line.fields.get("m_endParams"))?;
        let at = |t: f64| [0, 1, 2].map(|i| origin[i] + direction[i] * t);
        ends.push((at(start), at(end)));
    }
    if ends.len() < 3 {
        return None;
    }
    let joined = (0..ends.len())
        .all(|i| distance(ends[i].1, ends[(i + 1) % ends.len()].0) <= JOIN_TOLERANCE_FEET);
    if !joined {
        return None;
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for (p, _) in &ends {
        for ((low, high), v) in min.iter_mut().zip(max.iter_mut()).zip(p) {
            *low = low.min(*v);
            *high = high.max(*v);
        }
    }
    let cut = CutLoop { min, max };
    (max[1] - min[1] <= JOIN_TOLERANCE_FEET && cut.width_feet() > 0.0 && cut.height_feet() > 0.0)
        .then_some(cut)
}

fn point2(value: Option<&serde_json::Value>) -> Option<[f64; 2]> {
    let items = value?.as_array()?;
    let [a, b] = items.as_slice() else {
        return None;
    };
    let p = [a.as_f64()?, b.as_f64()?];
    p.iter().all(|v| v.is_finite()).then_some(p)
}

/// For each of `instances` (family instances' ElementIds), the cut loop of
/// the symbol that draws it (its `InstanceInfo.m_symbolId`). An instance
/// that names no symbol, or whose symbol has no loop [`CutLoop`] reads, is
/// left out. One pass reads the instances and every family symbol, since
/// which symbols they name is known only once they are read. Errs where the
/// native record path does not read the file's release.
pub fn instance_cut_loops(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, CutLoop>> {
    if instances.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let mut loops: BTreeMap<u64, CutLoop> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: instances.clone(),
        selected_classes: BTreeSet::from(["FamilySymbol".to_string()]),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if record.class_name.as_deref() == Some("FamilySymbol") {
            if let Some(cut) = cut_loop(graph) {
                loops.insert(id, cut);
            }
        } else if instances.contains(&id) {
            if let Some(symbol) = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo")
                .and_then(|object| object.fields.get("m_symbolId"))
                .and_then(element_id)
            {
                drawn_by.insert(id, symbol);
            }
        }
        Ok(())
    })?;
    Ok(drawn_by
        .into_iter()
        .filter_map(|(instance, symbol)| Some((instance, *loops.get(&symbol)?)))
        .collect())
}
