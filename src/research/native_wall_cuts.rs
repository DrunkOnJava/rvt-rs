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

use crate::graph_value::Fv;
use crate::native_json::{element_id, finite_triple};
use crate::{
    RevitFile, native_document, native_saved_mesh::face, native_saved_mesh::pointer,
    partition_element_records as per,
};
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
    /// The step's own `m_id`, which a `WallShadowCutoutGStep` of another wall
    /// names as its `m_ggId` (#1165).
    pub id: Option<u64>,
    /// Of a `WallShadowCutoutGStep`: the wall it mirrors a cut of
    /// (`m_otherWallId`) and the `m_id` of that wall's step (`m_ggId`).
    pub mirrors: Option<(u64, u64)>,
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
    remember_wall_records(rf, walls)?;
    Ok(walls
        .iter()
        .filter_map(|id| {
            rf.wall_cut_steps_memo
                .get(id)
                .map(|steps| (*id, steps.clone()))
        })
        .collect())
}

/// Reads each `WallShadowCutoutGStep` as the kind of the step of the other
/// wall it mirrors (#1165): the wall it names (`m_otherWallId`) holds an embed
/// or a cutout step of the same cutter whose `m_id` is the shadow's `m_ggId`:
/// all 124 shadow steps of the BIM4LCA office, `rac_advanced` and Snowdon
/// Towers Architectural name one (RE-398). Revit's export writes the cutter's
/// opening in the shadow's wall as it does in the other wall on all 3 of the
/// office's shadow steps. The shadow keeps its own `m_bCutSomething`, which is
/// about its own wall. A shadow whose other wall holds no such step stays a
/// shadow.
pub fn resolve_shadow_steps(steps: &mut BTreeMap<u64, Vec<WallCutStep>>) {
    let mut resolved: Vec<(u64, usize, WallCutStep)> = Vec::new();
    for (wall, list) in steps.iter() {
        for (position, step) in list.iter().enumerate() {
            let (WallCutKind::ShadowCutout, Some((other, id))) = (step.kind, step.mirrors) else {
                continue;
            };
            let mirrored = steps.get(&other).into_iter().flatten().find(|candidate| {
                candidate.id == Some(id)
                    && candidate.cutter == step.cutter
                    && matches!(candidate.kind, WallCutKind::Embed | WallCutKind::Cutout)
            });
            if let Some(mirrored) = mirrored {
                resolved.push((
                    *wall,
                    position,
                    // Only the kind is the mirrored step's: the shadow's own
                    // `m_bCutSomething` is about its own wall (false on 85 of the
                    // 124, where the cutter is not hosted in it) and the other
                    // step's about the other wall (RE-398).
                    WallCutStep {
                        kind: mirrored.kind,
                        ..*step
                    },
                ));
            }
        }
    }
    for (wall, position, step) in resolved {
        if let Some(slot) = steps.get_mut(&wall).and_then(|list| list.get_mut(position)) {
            *slot = step;
        }
    }
}

/// Reads the native records of those of `walls` not asked for before, in one
/// walk, and keeps what it gives on the file's handle: each wall's cut steps
/// and whether its record holds a base join. The openings the walls carry and
/// the walls drawn from their saved mesh ask for the same records, and a walk
/// of them is a pass over the file, so no caller makes a second one.
fn remember_wall_records(rf: &mut RevitFile, walls: &BTreeSet<u64>) -> anyhow::Result<()> {
    let missing: BTreeSet<u64> = walls
        .difference(&rf.wall_cut_steps_asked)
        .copied()
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let read = read_wall_records(rf, &missing)?;
    rf.wall_cut_steps_asked.extend(missing);
    rf.wall_cut_steps_memo.extend(read.steps);
    rf.wall_no_base_join_memo.extend(read.no_base_join);
    Ok(())
}

/// What one walk of walls' native records gives.
struct WallRecords {
    /// The cut steps of each wall whose record was read.
    steps: BTreeMap<u64, Vec<WallCutStep>>,
    /// The walls whose record was read as a wall's and holds no base join.
    no_base_join: BTreeSet<u64>,
}

/// The steps of the native records of `walls` and which of them hold no base
/// join, read now.
fn read_wall_records(rf: &mut RevitFile, walls: &BTreeSet<u64>) -> anyhow::Result<WallRecords> {
    let mut out: BTreeMap<u64, Vec<WallCutStep>> = BTreeMap::new();
    let mut read: BTreeSet<u64> = BTreeSet::new();
    let mut attached: BTreeSet<u64> = BTreeSet::new();
    let options = native_document::Options {
        selected_ids: walls.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        // #1046: a base join is a `JoinToRoofGStep` that does not say it joins
        // the top, in any record read as this wall's.
        if matches!(record.class_name.as_deref(), Some("SWall" | "ArcWall")) {
            let id = record.identity.element_id;
            read.insert(id);
            if graph.objects.iter().any(|object| {
                object.class_name == "JoinToRoofGStep"
                    && object.fields.get("m_baseOrTop").and_then(Fv::as_i64) != Some(1)
            }) {
                attached.insert(id);
            }
        }
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
                let mirrors = if kind == WallCutKind::ShadowCutout {
                    object
                        .fields
                        .get("m_otherWallId")
                        .and_then(element_id)
                        .zip(object.fields.get("m_ggId").and_then(Fv::as_u64))
                } else {
                    None
                };
                steps.push(WallCutStep {
                    kind,
                    cutter,
                    cuts_something,
                    id: object.fields.get("m_id").and_then(Fv::as_u64),
                    mirrors,
                });
            }
        }
        out.entry(record.identity.element_id).or_insert(steps);
        Ok(())
    })?;
    Ok(WallRecords {
        steps: out,
        no_base_join: read.difference(&attached).copied().collect(),
    })
}

/// The walls of `walls` whose native record was read and holds no base join, no
/// `JoinToRoofGStep` whose `m_baseOrTop` is 0 (#1046). Revit's export gives a
/// wall whose body is a vertical prism less its openings the areas of its sides
/// only where its base is not attached: all 170 such walls of the BIM4LCA
/// office as built and at building permit, and none of the 5 that are. A wall
/// whose record is not read, one whose record holds such a step, and one whose
/// step does not say whether it joins the base or the top, is left out, so the
/// set never names a wall that may be attached at its base. The records are the
/// ones [`wall_cut_steps`] reads, in the same walk, once for the file's handle.
pub fn walls_with_no_base_join(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeSet<u64>> {
    if walls.is_empty() {
        return Ok(BTreeSet::new());
    }
    remember_wall_records(rf, walls)?;
    Ok(walls
        .intersection(&rf.wall_no_base_join_memo)
        .copied()
        .collect())
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
            .and_then(Fv::as_array);
        if let Some([a, b]) = corners {
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
fn rows(value: &Fv) -> Option<[[f64; 3]; 3]> {
    let [a, b, c] = value.as_array()? else {
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
