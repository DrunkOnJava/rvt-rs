//! The elements a wall's native record names as cutting it, and the
//! rectangle of a rectangular wall opening (RE-321, #1165).
//!
//! A wall's `SWall` record lists the geometry steps Revit applies to its
//! body. Four of them name the element that cuts it in `m_instId`: a family
//! instance's `WallCutoutGStep` (a door, window or cabinet), a wall
//! embedded in it (`WallEmbedWallGStep`, a curtain wall), a rectangular
//! wall opening (`WallRectOpeningGStep`) and a `WallShadowCutoutGStep`.
//! revit-ifc's `OpeningUtil.AddOpeningsToElement` numbers the host's
//! openings from 1 in the order Revit's `GetOpeningData` returns them, and
//! on the BIM4LCA office that order is the order of these steps in the
//! record (CLEANROOM.md source 5, checked against 148 of 148 openings whose
//! GlobalId hashes the opening number).

use crate::native_json::{element_id, finite_triple};
use crate::{RevitFile, native_document};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Which step of a wall's record names a cutting element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallCutKind {
    /// `WallEmbedWallGStep`: a wall embedded in this one.
    Embed,
    /// `WallCutoutGStep`: a family instance's cutout.
    Cutout,
    /// `WallRectOpeningGStep`: a rectangular opening of the wall.
    RectOpening,
    /// `WallShadowCutoutGStep`.
    ShadowCutout,
}

/// A step of a wall's record that names a cutting element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallCutStep {
    /// The step's kind.
    pub kind: WallCutKind,
    /// The ElementId of the cutting element, `m_instId`.
    pub cutter: u64,
}

/// For each of `walls` (ElementIds), the steps of its native record that
/// name a cutting element, in the order the record lists them. A wall whose
/// record is not read is left out. Errs where the native record path does
/// not read the file's release; Revit 2023 and earlier save the ElementId
/// in another shape, so they give no steps.
pub fn wall_cut_steps(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, Vec<WallCutStep>>> {
    if walls.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut out: BTreeMap<u64, Vec<WallCutStep>> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: walls.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let mut steps = Vec::new();
        for object in &graph.objects {
            let kind = match object.class_name.as_str() {
                "WallEmbedWallGStep" => WallCutKind::Embed,
                "WallCutoutGStep" => WallCutKind::Cutout,
                "WallRectOpeningGStep" => WallCutKind::RectOpening,
                "WallShadowCutoutGStep" => WallCutKind::ShadowCutout,
                _ => continue,
            };
            if let Some(cutter) = object.fields.get("m_instId").and_then(element_id) {
                steps.push(WallCutStep { kind, cutter });
            }
        }
        out.entry(record.identity.element_id).or_insert(steps);
        Ok(())
    })?;
    Ok(out)
}

/// For each of `ids` (rectangular wall openings' ElementIds), the box of the
/// two corners its `RectOpeningData.m_aPoint` saves, `[min_x, min_y, min_z,
/// max_x, max_y, max_z]` in feet: the opening's rectangle in the plane of
/// its wall, along the wall and up (RE-321). It is Revit's opening along the
/// wall and up on all 30 of the BIM4LCA office as built and 24 at building
/// permit, where the element record's bounding box, which is padded to
/// 8 ft high, is not. An opening with no such object is left out.
pub fn rect_opening_boxes(
    rf: &mut RevitFile,
    ids: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, [f64; 6]>> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut out = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: ids.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let corners = graph
            .objects
            .iter()
            .find(|object| object.class_name == "RectOpeningData")
            .and_then(|object| object.fields.get("m_aPoint"))
            .and_then(Value::as_array);
        if let Some([a, b]) = corners.map(Vec::as_slice) {
            if let (Some(a), Some(b)) = (finite_triple(a), finite_triple(b)) {
                out.insert(
                    record.identity.element_id,
                    [
                        a[0].min(b[0]),
                        a[1].min(b[1]),
                        a[2].min(b[2]),
                        a[0].max(b[0]),
                        a[1].max(b[1]),
                        a[2].max(b[2]),
                    ],
                );
            }
        }
        Ok(())
    })?;
    Ok(out)
}
