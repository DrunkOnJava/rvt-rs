//! The elements a wall's native record names as cutting it, and the boxes
//! of those that cut an opening (RE-321, #1165).
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
use crate::{
    RevitFile, native_document, native_saved_mesh::face, native_saved_mesh::pointer,
    partition_element_records as per,
};
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
    /// The step's `m_bCutSomething`, where it has one: whether the element
    /// cuts this wall as the one it is hosted in (a door's own wall) or only
    /// as a further wall it reaches (RE-321).
    pub cuts_something: Option<bool>,
}

/// For each of `walls` (ElementIds), the steps of its native record that
/// name a cutting element, in the order the record lists them. A wall whose
/// record is not read is left out. Errs where the native record path does
/// not read the file's release; Revit 2023 and earlier save the ElementId
/// in another shape, so they give no steps. A wall's steps are read once
/// for the file's handle: a second call for walls already asked for reads
/// only the others.
pub fn wall_cut_steps(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, Vec<WallCutStep>>> {
    if walls.is_empty() {
        return Ok(BTreeMap::new());
    }
    let missing: BTreeSet<u64> = walls
        .difference(&rf.wall_cut_steps_asked)
        .copied()
        .collect();
    if !missing.is_empty() {
        let read = read_wall_cut_steps(rf, &missing)?;
        rf.wall_cut_steps_asked.extend(missing);
        rf.wall_cut_steps_memo.extend(read);
    }
    Ok(walls
        .iter()
        .filter_map(|id| {
            rf.wall_cut_steps_memo
                .get(id)
                .map(|steps| (*id, steps.clone()))
        })
        .collect())
}

/// The steps of the native records of `walls`, read now.
fn read_wall_cut_steps(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, Vec<WallCutStep>>> {
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
                let cuts_something = object.fields.get("m_bCutSomething").and_then(|flag| {
                    flag.as_bool()
                        .or_else(|| flag.as_i64().map(|value| value != 0))
                });
                steps.push(WallCutStep {
                    kind,
                    cutter,
                    cuts_something,
                });
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

/// The model bounding box of each of `ids` as its partition element record
/// gives it, `[min_x, min_y, min_z, max_x, max_y, max_z]` in feet: one for
/// every copy of the record found, in partition order, since an element can
/// be framed in several partitions. An id with no decodable record is left
/// out.
pub fn frame_boxes(rf: &mut RevitFile, ids: &BTreeSet<u64>) -> BTreeMap<u64, Vec<[f64; 6]>> {
    let mut out: BTreeMap<u64, Vec<[f64; 6]>> = BTreeMap::new();
    if ids.is_empty() {
        return out;
    }
    let revit = rf.layout_release().unwrap_or(0);
    let Some(marker) = per::file_bbox_marker(rf, revit) else {
        return out;
    };
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in per::partition_record_chain(buf, &marker) {
            if !ids.contains(&span.element_id) {
                continue;
            }
            if let Some(frame) =
                per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker)
            {
                out.entry(span.element_id)
                    .or_default()
                    .push(frame.bbox_feet);
            }
        }
    }
    out
}

/// A 3 by 3 matrix as its rows.
fn rows(value: &Value) -> Option<[[f64; 3]; 3]> {
    let [a, b, c] = value.as_array()?.as_slice() else {
        return None;
    };
    Some([finite_triple(a)?, finite_triple(b)?, finite_triple(c)?])
}

/// The vertices of the bounded faces of the solid a `FamilySymbol` graph's
/// `m_complexCutGeoms` names (RE-225). A symbol that lists several solids
/// whose boxes do not touch cuts more than one piece, and its box is not the
/// union of theirs: `None`.
fn complex_cut_vertices(graph: &crate::native_parameters::ObjectGraph) -> Option<Vec<[f64; 3]>> {
    let root = graph.objects.first()?;
    let mut solids: Vec<Vec<[f64; 3]>> = Vec::new();
    for solid in root.fields.get("m_complexCutGeoms")?.as_array()? {
        let index = pointer(graph, 0, solid).ok()??;
        let geometry = graph.objects.get(index)?;
        if geometry.class_name != "Geometry" {
            return None;
        }
        let mut vertices = Vec::new();
        for item in geometry.fields.get("m_pFaces")?.as_array()? {
            let face_index = pointer(graph, index, item).ok()??;
            if let Some(mesh) = face(graph, face_index).ok()? {
                vertices.extend(mesh.vertices);
            }
        }
        if !vertices.is_empty() {
            solids.push(vertices);
        }
    }
    let bounds: Vec<[f64; 6]> = solids
        .iter()
        .map(|vertices| vertex_bounds(vertices))
        .collect();
    for (at, a) in bounds.iter().enumerate() {
        for b in &bounds[at + 1..] {
            if (0..3).any(|axis| a[axis + 3] < b[axis] - 1e-6 || b[axis + 3] < a[axis] - 1e-6) {
                return None;
            }
        }
    }
    let vertices: Vec<[f64; 3]> = solids.into_iter().flatten().collect();
    (!vertices.is_empty()).then_some(vertices)
}

/// The box of some vertices, `[min_x, min_y, min_z, max_x, max_y, max_z]`.
fn vertex_bounds(vertices: &[[f64; 3]]) -> [f64; 6] {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for [x, y, z] in vertices {
        bounds = [
            bounds[0].min(*x),
            bounds[1].min(*y),
            bounds[2].min(*z),
            bounds[3].max(*x),
            bounds[4].max(*y),
            bounds[5].max(*z),
        ];
    }
    bounds
}

/// For each of `instances` (family instances' ElementIds), the box, in
/// model coordinates and feet (`[min_x, min_y, min_z, max_x, max_y,
/// max_z]`), of the solid the symbol that draws it cuts (its
/// `InstanceInfo.m_symbolId`, the solid `m_complexCutGeoms` names, RE-225)
/// placed by the instance's own transform (`InstanceInfo.m_Trf`, the
/// rotation `m_3x3` applied to the symbol's coordinates, then `m_or`
/// added). On the BIM4LCA office's fire hose cabinets this is Revit's
/// opening to 0.002 ft (RE-321). An instance with no such solid is left
/// out.
pub fn instance_cut_boxes(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, [f64; 6]>> {
    if instances.is_empty() {
        return Ok(BTreeMap::new());
    }
    // The symbol that draws an instance, the rotation of its transform and
    // the transform's origin.
    type Placement = (u64, [[f64; 3]; 3], [f64; 3]);
    let mut placed: BTreeMap<u64, Placement> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: instances.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(info) = graph
            .objects
            .iter()
            .find(|object| object.class_name == "InstanceInfo")
        else {
            return Ok(());
        };
        let symbol = info.fields.get("m_symbolId").and_then(element_id);
        let matrix = info.fields.pointer("/m_Trf/m_3x3").and_then(rows);
        let origin = info.fields.pointer("/m_Trf/m_or").and_then(finite_triple);
        if let (Some(symbol), Some(matrix), Some(origin)) = (symbol, matrix, origin) {
            placed.insert(record.identity.element_id, (symbol, matrix, origin));
        }
        Ok(())
    })?;
    if placed.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut solids: BTreeMap<u64, Vec<[f64; 3]>> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: placed.values().map(|(symbol, _, _)| *symbol).collect(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        if record.class_name.as_deref() != Some("FamilySymbol") {
            return Ok(());
        }
        if let Some(vertices) = record.graph.as_ref().and_then(complex_cut_vertices) {
            solids.insert(record.identity.element_id, vertices);
        }
        Ok(())
    })?;
    Ok(placed
        .into_iter()
        .filter_map(|(instance, (symbol, matrix, origin))| {
            let vertices = solids.get(&symbol)?;
            let mut low = [f64::INFINITY; 3];
            let mut high = [f64::NEG_INFINITY; 3];
            for vertex in vertices {
                for (axis, row) in matrix.iter().enumerate() {
                    let at =
                        origin[axis] + row[0] * vertex[0] + row[1] * vertex[1] + row[2] * vertex[2];
                    low[axis] = low[axis].min(at);
                    high[axis] = high[axis].max(at);
                }
            }
            low.iter()
                .chain(high.iter())
                .all(|v| v.is_finite())
                .then_some((
                    instance,
                    [low[0], low[1], low[2], high[0], high[1], high[2]],
                ))
        })
        .collect())
}
