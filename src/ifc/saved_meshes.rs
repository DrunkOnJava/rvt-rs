//! Family-instance bodies from the meshes Revit saves for display (#255).
//!
//! A Revit file keeps the graphics it last drew for each element. rosejn's
//! native reader (#255: `native_saved_scene`, `native_saved_mesh` and the
//! schema, graph and identity code under them) decodes them in document
//! feet. Where rvt-rs has only an element record's bounding box, as for
//! doors, windows and furniture, [`attach`] replaces that box with the
//! element's saved mesh.
//!
//! On RE1's four models (Revit 2025) 207 of the 214 bodies replaced are
//! within 0.01 ft of Revit's export (Measure run 37231768480).
//!
//! On Core Interior the saved meshes coincide with the bodies rvt-rs already
//! draws exactly (walls 351 of 360, columns 256 of 256, slabs 80 of 80), and
//! a door's mesh is its panel and frame, 3.50 × 0.83 × 8.25 ft, where its
//! record box is 3.75 ft deep because it includes the swing: the 2.92 ft
//! difference #227 measured against Revit's own export.
//!
//! Every element drawn as its record's box is drawn from its saved mesh by
//! default on Revit 2023 to 2027 ([`default_scope`], [`SavedMeshScope::All`]:
//! the maintainer, 2026-11-09, #1191), as doors and windows were from
//! 2026-10-29 for the CLIs and 2026-10-30 for Python and the viewer, and only
//! where all of the mesh decodes (RE-224). Of the bodies scored on 31 models
//! (Measure run 42612357300), the meshes of 17 classes have every vertex of
//! Revit's export within 0.001 ft ("ref on ours"), windows among them; doors
//! are 675 of 681, sanitary terminals 68 of 94, flow fittings 71 of 89, flow
//! terminals 30 of 35 and walls 14 of 18. The cost is reading the saved
//! graphics of every boxed element: against 0.17.0 the BIM4LCA office as built
//! goes from 25.4 s and 914 MiB to 33.0 s and 1,432 MiB, with its IFC 87 MB
//! against 68, Core Interior from 4.7 s and 305 MiB to 5.3 s and 437 MiB and
//! Snowdon Towers Architectural from 17.6 s and 1,073 MiB to 28.0 s and 2,086
//! MiB, with its IFC 189 MB against 37 (Measure run 42956917709, the median of
//! three exports on one runner per file); #1690 stopped holding every owner's
//! graph at once, and what is left is #1694.
//!
//! A roof drawn as the level block of its sketch outline at its record box's
//! full height is replaced too (#1612): the record box holds the roof's
//! reference surface, not its slope or its thickness, and its saved mesh is
//! what Revit drew. The outline's own properties (`ProfileSource`,
//! `ProfileVertexCount`, `ProfileVoidCount`) and the `SlabShapeRefusal` of
//! any body replaced go with the body they described.
//!
//! One kind of wall is drawn from its saved mesh in every scope that reads
//! graphics, on Revit 2024 to 2027, the releases whose wall records name what
//! embeds in them: a wall that embeds a curtain wall (`WallEmbedWallGStep`,
//! RE-335) whose saved mesh starts at least half a foot above the wall's base
//! (#1385, RE-340). Revit draws such a wall less the curtain wall's prism and less the
//! volumes of the elements that cut its end stubs, which no record read so far
//! tells apart from a wall that keeps them; the mesh Revit last drew is the
//! body, and its lowest vertex is Revit's lowest point on all 113 walls
//! measured. A wall with an opening hosted in it, a Tag shared with another
//! entity, a body that is neither a plain extrusion nor one with a stepped top
//! (RE-320) or a mesh that did not decode whole keeps the body it had, as does
//! one whose mesh starts at its base.

use super::IfcModel;
use super::entities::{BrepTriangle, IfcEntity, Property, PropertySet, PropertyValue, SolidShape};
use super::export_content::{
    BEAM_AXIS_BODY_SOURCE, BEAM_BOX_BODY_SOURCE, INSTANCE_ORIENTED_BODY_SOURCE,
    INSTANCE_TURNED_BODY_SOURCE, RECORD_BBOX_BODY_SOURCE, SAVED_MESH_ELEMENT_ID_SHARED,
    SAVED_MESH_NO_GRAPHICS, SAVED_MESH_NO_MESH, SAVED_MESH_NO_TRIANGLES,
    SAVED_MESH_NOT_CANDIDATE_PREFIX, SAVED_MESH_NOT_OFFERED_PREFIX, SAVED_MESH_PARTIAL_PREFIX,
    SAVED_MESH_REFUSAL_PROPERTY, SLAB_SHAPE_REFUSAL_PROPERTY, SPACE_OUTLINE_SAVED_MESH,
    SPACE_OUTLINE_SOURCE_PROPERTY, WALL_BASE_SOURCE_PROPERTY, WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE,
};
use super::mesh_volume::{
    BeamQuantities, ColumnQuantities, MESH_LENGTH_REACH, Placed, Prism, all_faces_flat,
    beam_quantities, box_plan_width, closed_mesh_volume, column_quantities, mullion_surface_area,
    partner_ends, prism_along,
};
use super::mesh_wall_quantities::{WallFrame, WallMeshQuantities, wall_mesh_quantities};
use crate::native_saved_mesh::{GraphicsMeshes, Primitive};
use crate::native_wall_cuts::WallCutKind;
use std::collections::{BTreeMap, BTreeSet};

/// `BodySource` of a body drawn from the element's saved graphics.
pub const SAVED_MESH_BODY_SOURCE: &str = "saved_graphics_mesh";

/// How far above its base a wall's saved mesh must start, and below its top,
/// for [`attach_scoped_with_control`] to draw a wall that embeds a curtain
/// wall from it, feet (#1385, RE-340). On the BIM4LCA office as built and at
/// building permit the lowest vertex of such a wall's saved mesh is Revit's
/// lowest point within 0.003 ft on 113 of 113 walls: 14 stand 8.2021 or
/// 9.514 ft above their base, the curtain wall's height, and the other 99 on
/// it to within 0.001 ft.
const EMBED_RAISE_MIN_FEET: f64 = 0.5;

/// What [`attach`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct SavedMeshReport {
    /// Elements whose body was the record's bounding box.
    pub candidates: usize,
    /// Of those, elements now drawn from their saved mesh.
    pub replaced: usize,
    /// Of those, elements keeping their box because part of their saved
    /// graphics did not decode (RE-224).
    pub partial: usize,
    /// Triangles written in all.
    pub triangles: usize,
    /// Walls that embed a curtain wall, drawn as plain or stepped extrusions with nothing
    /// hosted in them (#1385, RE-340).
    pub embed_hosts: usize,
    /// Of those, walls now drawn from their saved mesh, Revit having raised
    /// their base to the top of the curtain wall.
    pub embed_raised: usize,
    /// Of those, walls keeping their body because part of their saved
    /// graphics did not decode (RE-224).
    pub embed_partial: usize,
    /// Why the walls' native records were not read, where they were not: no
    /// wall that embeds a curtain wall is then drawn from its mesh.
    pub embed_error: Option<String>,
}

/// Whether some face or branch of `meshes` did not decode (RE-224): on the
/// BIM4LCA office 357 such meshes, none of them Revit's body. Only branches
/// left out for their view semantics leave a mesh whole.
fn is_partial(meshes: &GraphicsMeshes) -> bool {
    meshes
        .diagnostics
        .iter()
        .any(|diagnostic| leaves_partial(diagnostic))
}

/// Whether a decoder diagnostic leaves its mesh partial (see [`is_partial`]).
fn leaves_partial(diagnostic: &str) -> bool {
    !diagnostic.ends_with(crate::native_graphics_traversal::NESTED_DETAIL_FILTER_DIAGNOSTIC)
        && !diagnostic.ends_with(crate::native_graphics_traversal::VIEW_COMPONENT_DIAGNOSTIC)
}

/// A decoder diagnostic as [`SAVED_MESH_REFUSAL_PROPERTY`] says it: without the
/// object it names (the text before its first `: `) and with its digits written
/// as `#`, so that two elements with the same fault say it alike.
fn diagnostic_kind(message: &str) -> String {
    let text = message.split_once(": ").map_or(message, |(_, rest)| rest);
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

/// What [`SAVED_MESH_REFUSAL_PROPERTY`] says of a mesh that did not decode
/// whole: [`SAVED_MESH_PARTIAL_PREFIX`] and the distinct kinds of the
/// diagnostics that left it partial, in order, joined by `; `. None where
/// [`is_partial`] is false.
fn partial_refusal(meshes: &GraphicsMeshes) -> Option<String> {
    let kinds: BTreeSet<String> = meshes
        .diagnostics
        .iter()
        .filter(|diagnostic| leaves_partial(diagnostic))
        .map(|diagnostic| diagnostic_kind(diagnostic))
        .collect();
    (!kinds.is_empty()).then(|| {
        format!(
            "{SAVED_MESH_PARTIAL_PREFIX}{}",
            kinds.into_iter().collect::<Vec<_>>().join("; ")
        )
    })
}

/// Whether an element of `ifc_type` with `properties` is drawn as a box that
/// the pass can replace with its saved mesh: its record's box, a beam's box
/// (RE-328), a family instance's box turned to its angle (RE-87) or a curtain
/// mullion's box along its own axes (RE-106, RE-421: 327 mullions of the BIM4LCA
/// timber office at permit, whose Revit solids have mitred ends), each a
/// stand-in for the mesh; and a roof's level block, whose record box holds the
/// reference surface and not the slope or thickness (#1612). A resolved outline
/// is the body of any other class (a floor's plan, a ceiling's).
fn is_replaceable_box(ifc_type: &str, properties: &[Property]) -> bool {
    let source = properties
        .iter()
        .find_map(|property| match &property.value {
            PropertyValue::Text(text) if property.name == "BodySource" => Some(text.as_str()),
            _ => None,
        });
    let profile_resolved = properties.iter().any(|property| {
        property.name == "ProfileResolved" && matches!(property.value, PropertyValue::Boolean(true))
    });
    matches!(
        source,
        Some(
            RECORD_BBOX_BODY_SOURCE
                | BEAM_BOX_BODY_SOURCE
                | INSTANCE_TURNED_BODY_SOURCE
                | INSTANCE_ORIENTED_BODY_SOURCE
        )
    ) && !(profile_resolved && ifc_type != "IFCROOF")
}

/// Say on each box body that no pass offered a saved mesh why none did
/// (#1645): `why` follows [`SAVED_MESH_NOT_OFFERED_PREFIX`]. Returns how many
/// elements it said it on.
pub fn note_not_offered(model: &mut IfcModel, why: &str) -> usize {
    let boxed: Vec<usize> = model
        .entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                property_set: Some(set),
                ..
            } if is_replaceable_box(ifc_type, &set.properties) => Some(index),
            _ => None,
        })
        .collect();
    for &index in &boxed {
        note_refusal(
            model,
            index,
            format!("{SAVED_MESH_NOT_OFFERED_PREFIX}{why}"),
        );
    }
    boxed.len()
}

/// Say on the element at `index` why its box was not replaced by the mesh Revit
/// saved for it (#1645).
fn note_refusal(model: &mut IfcModel, index: usize, reason: String) {
    let Some(IfcEntity::BuildingElement {
        property_set: Some(set),
        ..
    }) = model.entities.get_mut(index)
    else {
        return;
    };
    set.properties
        .retain(|property| property.name != SAVED_MESH_REFUSAL_PROPERTY);
    set.properties.push(Property {
        name: SAVED_MESH_REFUSAL_PROPERTY.into(),
        value: PropertyValue::Text(reason),
    });
}

/// The vertices and triangles of `primitives`, in the frame of an element at
/// `origin` turned `rotation_radians`, feet.
fn local_mesh(
    primitives: &[Primitive],
    origin: [f64; 3],
    rotation_radians: Option<f64>,
) -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
    let (sin, cos) = (-rotation_radians.unwrap_or(0.0)).sin_cos();
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for primitive in primitives {
        let base = vertices.len() as u32;
        vertices.extend(primitive.vertices.iter().map(|p| {
            let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
            [dx * cos - dy * sin, dx * sin + dy * cos, p[2] - origin[2]]
        }));
        triangles.extend(
            primitive
                .triangles
                .iter()
                .filter(|t| t.iter().all(|&v| (v as usize) < primitive.vertices.len()))
                .map(|t| BrepTriangle(base + t[0], base + t[1], base + t[2])),
        );
    }
    (vertices, triangles)
}

/// Which bounding-box bodies [`attach_scoped_with_control`] replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SavedMeshScope {
    /// Every element drawn as its record's box: the default on Revit 2023 to
    /// 2027 ([`default_scope`]), and `rvt-ifc --saved-meshes` on any release.
    All,
    /// Doors and windows only, which `rvt-ifc` and `rvt-gltf` drew by default
    /// before #1191. No switch selects it now; a caller of the library asks
    /// for it with [`attach_scoped_with_control`].
    DoorsAndWindows,
}

impl SavedMeshScope {
    fn includes(self, ifc_type: &str) -> bool {
        match self {
            Self::All => true,
            Self::DoorsAndWindows => matches!(ifc_type, "IFCDOOR" | "IFCWINDOW"),
        }
    }
}

/// The bodies drawn from saved graphics by default, by `rvt-ifc`, `rvt-gltf`,
/// the Python package and the viewer alike: every boxed element on Revit 2023
/// to 2027, the releases whose saved graphics are read, and none on any
/// other release.
pub fn default_scope(rf: &mut crate::RevitFile) -> Option<SavedMeshScope> {
    let release = rf.basic_file_info().ok()?.version;
    (2023..=2027)
        .contains(&release)
        .then_some(SavedMeshScope::All)
}

/// Replace each bounding-box body with the element's saved mesh, where the
/// file holds one and all of it decodes.
pub fn attach(rf: &mut crate::RevitFile, model: &mut IfcModel) -> anyhow::Result<SavedMeshReport> {
    attach_with_control(rf, model, &crate::control::WalkerControl::default())
}

/// Same as [`attach`], reading the saved graphics through
/// `research::native_saved_scene::extract_at_detail_with_control`.
pub fn attach_with_control(
    rf: &mut crate::RevitFile,
    model: &mut IfcModel,
    control: &crate::control::WalkerControl,
) -> anyhow::Result<SavedMeshReport> {
    attach_scoped_with_control(rf, model, SavedMeshScope::All, control)
}

/// Same as [`attach_with_control`], for the bodies `scope` names only, and
/// for the walls that embed a curtain wall whatever the scope (see the
/// module).
pub fn attach_scoped_with_control(
    rf: &mut crate::RevitFile,
    model: &mut IfcModel,
    scope: SavedMeshScope,
    control: &crate::control::WalkerControl,
) -> anyhow::Result<SavedMeshReport> {
    let mut report = SavedMeshReport::default();
    let mut by_id: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    // A stair or curtain wall is drawn by its parts; its own saved graphics
    // would draw it twice.
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    // #1645: why each box that keeps its box does; a box the pass does not take
    // as a candidate says so before it looks at any saved graphics.
    let mut refusals: Vec<(usize, String)> = Vec::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid,
            property_set: Some(set),
            location_feet,
            ..
        } = entity
        else {
            continue;
        };
        // On Snowdon Towers Electrical 116 of 130 receptacles, outlets, conduit
        // elbows and pendants drawn as a turned box have a whole saved mesh.
        if !is_replaceable_box(ifc_type, &set.properties) {
            continue;
        }
        let id = type_guid.as_deref().and_then(|tag| tag.parse::<u64>().ok());
        let not_taken = if wholes.contains(&index) {
            Some("whole_drawn_by_its_parts")
        } else if location_feet.is_none() {
            Some("no_location")
        } else if id.is_none() {
            Some("no_element_id")
        } else if !scope.includes(ifc_type) {
            Some("class_not_in_scope")
        } else {
            None
        };
        match (id, not_taken) {
            (Some(id), None) => {
                report.candidates += 1;
                by_id.entry(id).or_default().push(index);
            }
            (_, Some(why)) => {
                refusals.push((index, format!("{SAVED_MESH_NOT_CANDIDATE_PREFIX}{why}")));
            }
            (None, None) => {}
        }
    }
    // RE-388: the walls whose quantities their saved mesh gives, whether or not
    // the wall's own body is drawn from it.
    let mut walls: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        if !matches!(ifc_type.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE")
            || !scope.includes(ifc_type)
        {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            walls.entry(id).or_default().push(index);
        }
    }
    // RE-423: the beams drawn along their location line, whose base quantities
    // their saved mesh gives, whether or not the beam's own body is drawn from
    // it: that body is the extrusion of a resolved section and stays so, as a
    // wall's does.
    let mut beam_lines: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        let along_its_line = set.properties.iter().any(|property| {
            property.name == "BodySource"
                && matches!(&property.value, PropertyValue::Text(text) if text == BEAM_AXIS_BODY_SOURCE)
        });
        if !ifc_type.eq_ignore_ascii_case("IFCBEAM") || !along_its_line || !scope.includes(ifc_type)
        {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            beam_lines.entry(id).or_default().push(index);
        }
    }
    // RE-425: the columns whose body is the rectangle their type's section gives
    // over the record's height (a resolved profile, which the pass leaves alone:
    // `is_replaceable_box`), whose base quantities their saved mesh gives all the
    // same. That body is Revit's own form for a plain section and stays so, as a
    // beam's does.
    let mut column_rectangles: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        let boxed = set.properties.iter().any(|property| {
            property.name == "BodySource"
                && matches!(&property.value, PropertyValue::Text(source) if source == RECORD_BBOX_BODY_SOURCE)
        });
        let resolved = set.properties.iter().any(|property| {
            property.name == "ProfileResolved"
                && matches!(property.value, PropertyValue::Boolean(true))
        });
        if !ifc_type.eq_ignore_ascii_case("IFCCOLUMN")
            || !resolved
            || !boxed
            || !scope.includes(ifc_type)
        {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            column_rectangles.entry(id).or_default().push(index);
        }
    }
    let hosts = match embed_hosts(rf, model, &by_id) {
        Ok(hosts) => hosts,
        Err(error) => {
            report.embed_error = Some(error);
            BTreeMap::new()
        }
    };
    report.embed_hosts = hosts.len();
    if by_id.is_empty()
        && hosts.is_empty()
        && walls.is_empty()
        && beam_lines.is_empty()
        && column_rectangles.is_empty()
    {
        for (index, reason) in refusals {
            note_refusal(model, index, reason);
        }
        return Ok(report);
    }
    let options = crate::native_document::Options {
        selected_ids: by_id
            .keys()
            .chain(hosts.keys())
            .chain(walls.keys())
            .chain(beam_lines.keys())
            .chain(column_rectangles.keys())
            .copied()
            .collect::<BTreeSet<_>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        value_source: Some("saved_graphics"),
        ..Default::default()
    };
    // RE-387: the volume of each body drawn from a mesh of flat faces.
    let mut volumes: Vec<(usize, f64)> = Vec::new();
    // RE-418, RE-404: each curtain mullion's mesh read as a prism along its own axis.
    let mut members: Vec<MemberMesh> = Vec::new();
    // RE-418: the axis each curtain mullion runs along, by ElementId, in the model's frame.
    let mullion_axes = rf.mullion_axes();
    // RE-404: the end faces each curtain mullion's solid keeps, by ElementId.
    let kept_ends = rf.mullion_kept_ends();
    // RE-423: what each beam's mesh gives of its base quantities.
    let mut beams: Vec<(usize, BeamQuantities)> = Vec::new();
    // RE-424: what each column's mesh gives of its base quantities.
    let mut columns: Vec<(usize, ColumnQuantities)> = Vec::new();
    // RE-416: the volume of each column's mesh, whatever its faces.
    let mut column_volumes: Vec<(usize, f64)> = Vec::new();
    // RE-388: what the mesh of each wall gives of its quantities.
    let mut wall_quantities: Vec<(usize, WallMeshQuantities)> = Vec::new();
    // #1046: what a wall's mesh needs besides itself to be read as a vertical
    // prism less the openings rvt-rs cuts in it, the wall's own axes and the
    // boxes of those openings, and whether its base is joined to a roof, floor
    // or footing, which takes the areas away.
    let wall_openings = wall_frame_and_openings(model);
    let wall_ids: BTreeSet<u64> = walls.keys().copied().collect();
    let no_base_join =
        crate::native_wall_cuts::walls_with_no_base_join(rf, &wall_ids).unwrap_or_default();
    // RE-396: the width of each curtain panel drawn from a mesh that is a vertical box.
    let mut plate_widths: Vec<(usize, f64)> = Vec::new();
    // #1645: the candidates the scene held graphics for.
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    // Each element is taken as it is meshed and its decoded graphs freed with
    // its batch of shared symbols, rather than all held until the last one is
    // meshed (#1075).
    crate::native_saved_scene::extract_in_batches_with_control(
        rf,
        &options,
        3,
        control,
        |element| {
            if by_id.contains_key(&element.identity.element_id) {
                seen.insert(element.identity.element_id);
            }
            if let Some([index]) = walls.get(&element.identity.element_id).map(Vec::as_slice) {
                let held = wall_openings.get(index);
                let frame = held.map_or(WallFrame { turn: 0.0 }, |held| held.frame);
                let boxes: &[[f64; 6]] = held.map_or(&[], |held| held.boxes.as_slice());
                let found = wall_mesh_quantities(
                    &element.meshes.primitives,
                    &frame,
                    boxes,
                    !no_base_join.contains(&element.identity.element_id),
                );
                if let Some(found) = found {
                    wall_quantities.push((*index, found));
                }
            }
            if let Some([index]) = beam_lines
                .get(&element.identity.element_id)
                .map(Vec::as_slice)
            {
                let found = partial_refusal(&element.meshes)
                    .is_none()
                    .then(|| beam_mesh_quantities(&element.meshes.primitives))
                    .flatten();
                if let Some(found) = found {
                    beams.push((*index, found));
                }
            }
            if let Some([index]) = column_rectangles
                .get(&element.identity.element_id)
                .map(Vec::as_slice)
            {
                let found = partial_refusal(&element.meshes)
                    .is_none()
                    .then(|| column_mesh_quantities(&element.meshes.primitives))
                    .flatten();
                if let Some(found) = found {
                    columns.push((*index, found));
                }
            }
            if let Some(&index) = hosts.get(&element.identity.element_id) {
                raise_embed_host(model, index, &element.meshes, &mut report);
                return Ok(());
            }
            let Some(indices) = by_id.get(&element.identity.element_id) else {
                return Ok(());
            };
            // A Tag carried by several entities (a slab's pieces) is ambiguous:
            // leave them as they are.
            let [index] = indices.as_slice() else {
                refusals.extend(
                    indices
                        .iter()
                        .map(|index| (*index, SAVED_MESH_ELEMENT_ID_SHARED.to_string())),
                );
                return Ok(());
            };
            let primitives = &element.meshes.primitives;
            let partial = partial_refusal(&element.meshes);
            if primitives.is_empty() {
                refusals.push((
                    *index,
                    partial.unwrap_or_else(|| SAVED_MESH_NO_MESH.to_string()),
                ));
                return Ok(());
            }
            if let Some(partial) = partial {
                report.partial += 1;
                refusals.push((*index, partial));
                return Ok(());
            }
            let IfcEntity::BuildingElement {
                ifc_type,
                location_feet: Some(origin),
                rotation_radians,
                solid_shape,
                property_set: Some(set),
                ..
            } = &mut model.entities[*index]
            else {
                return Ok(());
            };
            let (vertices, triangles) = local_mesh(primitives, *origin, *rotation_radians);
            if triangles.is_empty() {
                refusals.push((*index, SAVED_MESH_NO_TRIANGLES.to_string()));
                return Ok(());
            }
            let volume = all_faces_flat(primitives)
                .then(|| closed_mesh_volume(&vertices, &triangles))
                .flatten();
            let plate_width = volume
                .filter(|_| ifc_type.eq_ignore_ascii_case("IFCPLATE"))
                .and_then(|volume| box_plan_width(&vertices, &triangles, volume));
            let column_volume = ifc_type
                .eq_ignore_ascii_case("IFCCOLUMN")
                .then(|| oriented_mesh_volume(primitives, *origin))
                .flatten();
            // RE-418: the mullion's own axis, from its record, turned into the
            // element's frame as its mesh is.
            let member = volume
                .filter(|_| ifc_type.eq_ignore_ascii_case("IFCMEMBER"))
                .and_then(|_| {
                    let id = u32::try_from(element.identity.element_id).ok()?;
                    let axis = *mullion_axes.as_deref()?.get(&id)?;
                    let turn = (*rotation_radians).unwrap_or(0.0);
                    let (sin, cos) = (-turn).sin_cos();
                    let local = [
                        axis[0] * cos - axis[1] * sin,
                        axis[0] * sin + axis[1] * cos,
                        axis[2],
                    ];
                    let prism = prism_along(&vertices, &triangles, local)?;
                    // The end faces' centres in the model's frame, where mullions
                    // are compared with one another.
                    let (sin, cos) = turn.sin_cos();
                    let world = |p: [f64; 3]| {
                        [
                            origin[0] + p[0] * cos - p[1] * sin,
                            origin[1] + p[0] * sin + p[1] * cos,
                            origin[2] + p[2],
                        ]
                    };
                    let placed = Placed {
                        axis,
                        ends: prism.ends.map(|end| world(end.centre)),
                    };
                    let kept = kept_ends.as_deref().and_then(|kept| kept.get(&id).copied());
                    Some(MemberMesh {
                        index: *index,
                        kept,
                        prism,
                        placed,
                    })
                });
            // RE-423: a beam's mesh, where it is a closed solid of flat faces, as
            // the prism Revit's export extrudes its profile into.
            let beam = volume
                .filter(|_| ifc_type.eq_ignore_ascii_case("IFCBEAM"))
                .and_then(|_| beam_quantities(&vertices, &triangles));
            // RE-424: a column's mesh, where it is a closed solid of flat faces, as
            // the vertical prism Revit's export extrudes its profile into.
            let column = volume
                .filter(|_| ifc_type.eq_ignore_ascii_case("IFCCOLUMN"))
                .and_then(|_| column_quantities(&vertices, &triangles));
            report.replaced += 1;
            report.triangles += triangles.len();
            *solid_shape = Some(SolidShape::TriangulatedFaceSet {
                vertices_feet: vertices,
                triangles,
            });
            if let Some(volume) = volume {
                volumes.push((*index, volume));
            }
            if let Some(member) = member {
                members.push(member);
            }
            if let Some(beam) = beam {
                beams.push((*index, beam));
            }
            if let Some(column) = column {
                columns.push((*index, column));
            }
            if let Some(volume) = column_volume {
                column_volumes.push((*index, volume));
            }
            if let Some(width) = plate_width {
                plate_widths.push((*index, width));
            }
            for property in &mut set.properties {
                if property.name == "BodySource" {
                    property.value = PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into());
                } else if property.name == SPACE_OUTLINE_SOURCE_PROPERTY {
                    property.value = PropertyValue::Text(SPACE_OUTLINE_SAVED_MESH.into());
                } else if property.name == "ProfileResolved" {
                    property.value = PropertyValue::Boolean(false);
                }
            }
            // What describes a body that is no longer the outline block or the
            // box: where its outline came from and why its shape was not drawn.
            set.properties.retain(|property| {
                !matches!(
                    property.name.as_str(),
                    "ProfileSource"
                        | "ProfileVertexCount"
                        | "ProfileVoidCount"
                        | SLAB_SHAPE_REFUSAL_PROPERTY
                )
            });
            Ok(())
        },
    )?;
    drop_column_box_quantities(model, &column_volumes);
    let moved = add_mesh_volumes(model, &volumes, &members);
    follow_mesh_span(model, &moved);
    add_mullion_surface_areas(model, &members);
    add_beam_quantities(model, &mut beams);
    add_column_quantities(model, &mut columns);
    add_wall_quantities(model, &wall_quantities);
    add_plate_widths(model, &plate_widths);
    // #1645: a candidate the scene held no graphics for keeps its box too.
    for (id, indices) in &by_id {
        if !seen.contains(id) {
            refusals.extend(
                indices
                    .iter()
                    .map(|index| (*index, SAVED_MESH_NO_GRAPHICS.to_string())),
            );
        }
    }
    for (index, reason) in refusals {
        note_refusal(model, index, reason);
    }
    Ok(report)
}

/// How far a column's box volume may differ from the volume of the mesh Revit
/// saved for it, as a fraction of the mesh's, and still be its solid (RE-416):
/// the mesh of a cylinder drawn with chords is within 1% of it, and a round
/// column's box (the circle's bounding square) is 27% over it.
const COLUMN_BOX_MESH_TOLERANCE: f64 = 0.02;

/// The volume, cubic feet, of the surface `primitives` make, each triangle
/// turned outward by its vertices' normals and measured from `origin`, or none
/// where a primitive has no normals for its vertices or the volume is not
/// positive. It does not need the surface closed or its faces flat, so it reads
/// the chord-drawn side of a cylinder to the accuracy of its chords.
fn oriented_mesh_volume(primitives: &[Primitive], origin: [f64; 3]) -> Option<f64> {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut total = 0.0;
    for primitive in primitives {
        if primitive.normals.len() != primitive.vertices.len() {
            return None;
        }
        for triangle in &primitive.triangles {
            let [i, j, k] = triangle.map(|index| index as usize);
            let a = sub(*primitive.vertices.get(i)?, origin);
            let mut b = sub(*primitive.vertices.get(j)?, origin);
            let mut c = sub(*primitive.vertices.get(k)?, origin);
            let outward = [i, j, k].map(|index| primitive.normals[index]);
            let sum = [
                outward[0][0] + outward[1][0] + outward[2][0],
                outward[0][1] + outward[1][1] + outward[2][1],
                outward[0][2] + outward[1][2] + outward[2][2],
            ];
            if dot(cross(sub(b, a), sub(c, a)), sum) < 0.0 {
                std::mem::swap(&mut b, &mut c);
            }
            total += dot(a, cross(b, c)) / 6.0;
        }
    }
    (total.is_finite() && total > 0.0).then_some(total)
}

/// Take from each column of `found` (an index into the entities and the volume
/// of the mesh its body was drawn from) the quantities its record's box gave
/// where the box is not its solid (RE-416): its `CrossSectionArea`,
/// `OuterSurfaceArea`, `GrossVolume` and `NetVolume`, written for a type whose
/// length parameter `b` is both sides of its box (RE-291), which a round column
/// of that type is as well. Its `Length` stays. The box is its solid where its
/// held `NetVolume`, or its `GrossVolume` where it holds no other, is within
/// [`COLUMN_BOX_MESH_TOLERANCE`] of its mesh's volume. On Autodesk's
/// rac_advanced sample 6 round columns of 450 mm and as many in each of its
/// three later saves hold the square's areas and volumes, 4/pi of the circle's.
fn drop_column_box_quantities(model: &mut IfcModel, found: &[(usize, f64)]) {
    if found.is_empty() {
        return;
    }
    let mut held: BTreeMap<usize, usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == "Qto_ColumnBaseQuantities" {
                held.entry(*element).or_insert(position);
            }
        }
    }
    for &(index, mesh_volume) in found {
        let Some(&position) = held.get(&index) else {
            continue;
        };
        let Some(IfcEntity::ElementPropertySet { set, .. }) = model.entities.get_mut(position)
        else {
            continue;
        };
        let volume_of = |wanted: &str| {
            set.properties
                .iter()
                .find_map(|property| match &property.value {
                    PropertyValue::VolumeCubicFeet(volume) if property.name == wanted => {
                        Some(*volume)
                    }
                    _ => None,
                })
        };
        let Some(box_volume) = volume_of("NetVolume").or_else(|| volume_of("GrossVolume")) else {
            continue;
        };
        if (box_volume - mesh_volume).abs() > COLUMN_BOX_MESH_TOLERANCE * mesh_volume {
            set.properties.retain(|property| {
                !matches!(
                    property.name.as_str(),
                    "CrossSectionArea" | "OuterSurfaceArea" | "GrossVolume" | "NetVolume"
                )
            });
        }
    }
}

/// Give each element of `volumes` (an index into the entities and the volume,
/// cubic feet, of the closed mesh of flat faces its body was drawn from) the
/// `NetVolume` that Revit's export writes for it, in the base quantity set of
/// its entity, where it holds none (RE-387): revit-ifc's `NetVolumeCalculator`
/// takes the volume of the solid it writes (CLEANROOM.md source 5, RE-243),
/// and the mesh is that solid, which a body of flat faces gives exactly. A
/// column gets no `GrossVolume` here: Revit's export omits it on the 18 columns of
/// the BIM4LCA office at building permit that it draws as a cut mesh, and the
/// volume alone does not tell them from the rest ([`add_column_quantities`]
/// gives it where the mesh is a vertical prism, RE-424). A curtain mullion, a curtain
/// panel and a column are the entities measured equal to Revit's on every
/// element (RE-387); a body of another entity gets none. A curtain mullion
/// whose `NetVolume` the box gave and whose mesh has another volume is not an
/// extrusion of its box, and takes the mesh's (RE-403) and the mesh's length
/// (RE-418): the mullions of `members` whose `Length` it changed are returned with
/// the new one, for [`follow_mesh_span`].
fn add_mesh_volumes(
    model: &mut IfcModel,
    volumes: &[(usize, f64)],
    members: &[MemberMesh],
) -> Vec<(usize, f64)> {
    let mut moved = Vec::new();
    if volumes.is_empty() {
        return moved;
    }
    let extents: BTreeMap<usize, f64> = members
        .iter()
        .map(|member| (member.index, member.prism.extent))
        .collect();
    let mut held: BTreeMap<(usize, String), usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name.starts_with("Qto_") {
                held.entry((*element, set.name.clone())).or_insert(position);
            }
        }
    }
    for &(index, volume) in volumes {
        let Some(IfcEntity::BuildingElement { ifc_type, .. }) = model.entities.get(index) else {
            continue;
        };
        let (set_name, names): (&str, &[&str]) = match ifc_type.to_ascii_uppercase().as_str() {
            "IFCMEMBER" => ("Qto_MemberBaseQuantities", &["NetVolume"]),
            "IFCPLATE" => ("Qto_PlateBaseQuantities", &["NetVolume"]),
            "IFCCOLUMN" => ("Qto_ColumnBaseQuantities", &["NetVolume"]),
            _ => continue,
        };
        let volume_property = |name: &str| Property {
            name: name.into(),
            value: PropertyValue::VolumeCubicFeet(volume),
        };
        let key = (index, set_name.to_string());
        match held.get(&key) {
            Some(&position) => {
                if let Some(IfcEntity::ElementPropertySet { set, .. }) =
                    model.entities.get_mut(position)
                {
                    for &name in names {
                        if !has_property(set, name) {
                            set.properties.push(volume_property(name));
                        }
                    }
                    if set_name == "Qto_MemberBaseQuantities" {
                        let extent = extents.get(&index).copied();
                        if let Some(length) = follow_mesh_over_box(set, volume, extent) {
                            moved.push((index, length));
                        }
                    }
                }
            }
            None => {
                held.insert(key, model.entities.len());
                model.entities.push(IfcEntity::ElementPropertySet {
                    element: index,
                    set: PropertySet {
                        name: set_name.into(),
                        properties: names.iter().map(|&name| volume_property(name)).collect(),
                    },
                });
            }
        }
    }
    moved
}

/// How far a curtain mullion's box volume may differ from the volume of the
/// mesh Revit saved for it, as a fraction of the mesh's, before the box is not
/// its solid (RE-403). A mullion whose `NetVolume` is Revit's from its box has
/// the mesh's volume to within 1e-4; one with a 45 degree end differs by 2.6 to
/// 3.7%.
const BOX_MESH_VOLUME_TOLERANCE: f64 = 1e-3;

/// How far, feet, the extent of a mullion's mesh must differ from its written
/// `Length` to replace it (RE-418): the mesh's corners are single precision,
/// which moves an extent by 1e-5 ft, and the box overruns the solid by 0.03 ft
/// or more where it does.
const MESH_LENGTH_MOVES_FEET: f64 = 1e-4;

/// A curtain mullion whose `NetVolume` was written from its box (RE-243) and
/// whose saved mesh has another volume is not an extrusion of that box, as a
/// mullion with a 45 degree end is not: Revit's `NetVolume` is the mesh's
/// (RE-387), and the box's `NetSurfaceArea`, its sides and the end faces its
/// neighbours leave, is not Revit's, so it is left out (RE-403). Its `Length` is
/// the extent of the mesh along its own axis (RE-418): Revit's `Length` is the
/// extent of its own body, which the saved mesh is, and a box that overruns the
/// solid gives a longer one. `extent` is that of the mesh along the mullion's own
/// axis, where it is a prism along it ([`prism_along`]); it replaces the `Length`
/// where it is within `MESH_LENGTH_REACH` of it, and otherwise the written
/// `Length` stays. Returns the new `Length`, where there is one.
fn follow_mesh_over_box(
    set: &mut PropertySet,
    mesh_volume: f64,
    extent: Option<f64>,
) -> Option<f64> {
    if !(mesh_volume.is_finite() && mesh_volume > 0.0) {
        return None;
    }
    let box_volume = set
        .properties
        .iter()
        .find_map(|property| match &property.value {
            PropertyValue::VolumeCubicFeet(volume) if property.name == "NetVolume" => Some(*volume),
            _ => None,
        })?;
    if (box_volume - mesh_volume).abs() <= BOX_MESH_VOLUME_TOLERANCE * mesh_volume {
        return None;
    }
    for property in &mut set.properties {
        if property.name == "NetVolume" {
            property.value = PropertyValue::VolumeCubicFeet(mesh_volume);
        }
    }
    set.properties
        .retain(|property| property.name != "NetSurfaceArea");
    let length = set
        .properties
        .iter_mut()
        .find(|property| property.name == "Length")?;
    let PropertyValue::PositiveLengthFeet(written) = &mut length.value else {
        return None;
    };
    let extent = extent?;
    let off = (extent - *written).abs();
    if off > MESH_LENGTH_REACH * *written || off <= MESH_LENGTH_MOVES_FEET {
        return None;
    }
    *written = extent;
    Some(extent)
}

/// A curtain mullion drawn from its saved mesh, closed and of flat faces, read as
/// a prism along its own axis (RE-418, RE-404).
struct MemberMesh {
    /// The mullion's index into the entities.
    index: usize,
    /// How many of its two end faces its Revit solid keeps, where its curtain
    /// wall's boxes say (RE-268).
    kept: Option<u8>,
    /// The mesh as a prism along the mullion's axis.
    prism: Prism,
    /// Where its two end faces are, in the model's frame.
    placed: Placed,
}

/// Give each curtain mullion of `members` that holds no `NetSurfaceArea` the one
/// Revit's export writes, from its mesh (RE-404): the area of its faces along
/// its own axis and of the end faces its solid keeps
/// ([`mullion_surface_area`]). The mesh is Revit's solid, so its lateral and end
/// areas are Revit's; `kept` is RE-268's rule on the boxes of the mullion's
/// curtain wall, which is read for a mullion on any host, where the box's own
/// `NetSurfaceArea` is written only for one on a plain host. A mullion whose
/// box gave a `NetSurfaceArea` that its mesh did not contradict (RE-403) keeps
/// it. Where a mullion has a partner at an end that `kept` counts, the count
/// misses a continuation on another curtain wall, and the mullion gets none:
/// `kept` must leave every end with a partner (`partner_ends`) among the ends it
/// does not count.
fn add_mullion_surface_areas(model: &mut IfcModel, members: &[MemberMesh]) {
    let mut held: BTreeMap<usize, usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == "Qto_MemberBaseQuantities" {
                held.entry(*element).or_insert(position);
            }
        }
    }
    let placed: Vec<Placed> = members.iter().map(|member| member.placed).collect();
    let partners = partner_ends(&placed);
    for (member, partners) in members.iter().zip(partners) {
        let (Some(kept), Some(&position)) = (member.kept, held.get(&member.index)) else {
            continue;
        };
        let with_partner = partners.iter().filter(|partner| **partner).count();
        if with_partner > usize::from(2 - kept.min(2)) {
            continue;
        }
        let Some(IfcEntity::ElementPropertySet { set, .. }) = model.entities.get_mut(position)
        else {
            continue;
        };
        if has_property(set, "NetSurfaceArea") {
            continue;
        }
        let Some(area) = mullion_surface_area(&member.prism, kept) else {
            continue;
        };
        set.properties.push(Property {
            name: "NetSurfaceArea".into(),
            value: PropertyValue::AreaSquareFeet(area),
        });
    }
}

/// What the saved mesh of a beam drawn along its line gives of its base
/// quantities (RE-423), where all its faces are planes and it is one closed
/// solid that is a prism. The quantities are lengths, areas and a volume, which
/// where the mesh is placed does not change.
fn beam_mesh_quantities(primitives: &[Primitive]) -> Option<BeamQuantities> {
    if !all_faces_flat(primitives) {
        return None;
    }
    let (vertices, triangles) = local_mesh(primitives, [0.0; 3], None);
    beam_quantities(&vertices, &triangles)
}

/// What the saved mesh of a column drawn as its type's rectangle gives of its
/// base quantities (RE-425), where all its faces are planes and it is one closed
/// solid that is a prism along the vertical (RE-424). Where the mesh is placed
/// does not change the lengths, areas and volume.
fn column_mesh_quantities(primitives: &[Primitive]) -> Option<ColumnQuantities> {
    if !all_faces_flat(primitives) {
        return None;
    }
    let (vertices, triangles) = local_mesh(primitives, [0.0; 3], None);
    column_quantities(&vertices, &triangles)
}

/// Give each beam of `found` (an index into the entities and what its mesh gives
/// of its base quantities) the quantities Revit's export writes for it, in the
/// base quantity set it holds or a new one, where that holds none of them
/// (RE-423): `CrossSectionArea`, `OuterSurfaceArea`, `GrossSurfaceArea`,
/// `NetSurfaceArea`, `GrossVolume` and `NetVolume`. They are taken in the order
/// of the entities, so that the sets a beam that has none gets are appended in
/// one order whichever order the elements were meshed in.
fn add_beam_quantities(model: &mut IfcModel, found: &mut [(usize, BeamQuantities)]) {
    if found.is_empty() {
        return;
    }
    found.sort_by_key(|(index, _)| *index);
    let mut held: BTreeMap<usize, usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == "Qto_BeamBaseQuantities" {
                held.entry(*element).or_insert(position);
            }
        }
    }
    for (index, quantities) in found.iter() {
        let values = [
            (
                "CrossSectionArea",
                PropertyValue::AreaSquareFeet(quantities.cross_section_area),
            ),
            (
                "OuterSurfaceArea",
                PropertyValue::AreaSquareFeet(quantities.outer_surface_area),
            ),
            (
                "GrossSurfaceArea",
                PropertyValue::AreaSquareFeet(quantities.surface_area),
            ),
            (
                "NetSurfaceArea",
                PropertyValue::AreaSquareFeet(quantities.surface_area),
            ),
            (
                "GrossVolume",
                PropertyValue::VolumeCubicFeet(quantities.volume),
            ),
            (
                "NetVolume",
                PropertyValue::VolumeCubicFeet(quantities.volume),
            ),
        ];
        match held.get(index) {
            Some(&position) => {
                if let Some(IfcEntity::ElementPropertySet { set, .. }) =
                    model.entities.get_mut(position)
                {
                    for (name, value) in values {
                        if !has_property(set, name) {
                            set.properties.push(Property {
                                name: name.into(),
                                value,
                            });
                        }
                    }
                }
            }
            None => {
                held.insert(*index, model.entities.len());
                model.entities.push(IfcEntity::ElementPropertySet {
                    element: *index,
                    set: PropertySet {
                        name: "Qto_BeamBaseQuantities".into(),
                        properties: values
                            .into_iter()
                            .map(|(name, value)| Property {
                                name: name.into(),
                                value,
                            })
                            .collect(),
                    },
                });
            }
        }
    }
}

/// Give each column of `found` (an index into the entities and what its mesh
/// gives of its base quantities) the quantities Revit's export writes for a
/// column it draws as an extrusion (RE-424): `CrossSectionArea`,
/// `OuterSurfaceArea` and `GrossVolume`, and `Length`, in the base quantity set it
/// holds. A column whose set holds none of the three gets all four, and one
/// that holds all three gets the `Length` it lacks; one that holds some of the
/// three is left as it is, because the cut-column rule writes a
/// `CrossSectionArea` and no more for a column Revit draws as a cut mesh, which has
/// no `OuterSurfaceArea` or `GrossVolume` (RE-291, RE-353). A column drawn from
/// its mesh holds the set: [`add_mesh_volumes`] gave every column whose mesh is
/// a closed solid of flat faces its `NetVolume`. A column that keeps its body,
/// the rectangle its type gives (RE-425), may hold none, and then gets the set
/// with all of them, its `NetVolume` the volume of the prism as Revit's is of an
/// extrusion. A value a column holds is never replaced.
fn add_column_quantities(model: &mut IfcModel, found: &mut [(usize, ColumnQuantities)]) {
    if found.is_empty() {
        return;
    }
    found.sort_by_key(|(index, _)| *index);
    let mut held: BTreeMap<usize, usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == "Qto_ColumnBaseQuantities" {
                held.entry(*element).or_insert(position);
            }
        }
    }
    for (index, quantities) in found.iter() {
        let section = [
            (
                "CrossSectionArea",
                PropertyValue::AreaSquareFeet(quantities.cross_section_area),
            ),
            (
                "OuterSurfaceArea",
                PropertyValue::AreaSquareFeet(quantities.outer_surface_area),
            ),
            (
                "GrossVolume",
                PropertyValue::VolumeCubicFeet(quantities.volume),
            ),
        ];
        let rest = [
            (
                "Length",
                PropertyValue::PositiveLengthFeet(quantities.length),
            ),
            (
                "NetVolume",
                PropertyValue::VolumeCubicFeet(quantities.volume),
            ),
        ];
        let Some(&position) = held.get(index) else {
            held.insert(*index, model.entities.len());
            model.entities.push(IfcEntity::ElementPropertySet {
                element: *index,
                set: PropertySet {
                    name: "Qto_ColumnBaseQuantities".into(),
                    properties: section
                        .into_iter()
                        .chain(rest)
                        .map(|(name, value)| Property {
                            name: name.into(),
                            value,
                        })
                        .collect(),
                },
            });
            continue;
        };
        let Some(IfcEntity::ElementPropertySet { set, .. }) = model.entities.get_mut(position)
        else {
            continue;
        };
        let held_of_three = section
            .iter()
            .filter(|(name, _)| has_property(set, name))
            .count();
        let wanted = match held_of_three {
            0 => section.into_iter().chain(rest).collect::<Vec<_>>(),
            3 => rest.into_iter().collect::<Vec<_>>(),
            _ => continue,
        };
        for (name, value) in wanted {
            if !has_property(set, name) {
                set.properties.push(Property {
                    name: name.into(),
                    value,
                });
            }
        }
    }
}

/// Give the `Span` of each mullion of `moved` (an index into the entities and its
/// new `Length`, feet) the same length, as it has at the record (RE-418).
fn follow_mesh_span(model: &mut IfcModel, moved: &[(usize, f64)]) {
    if moved.is_empty() {
        return;
    }
    let lengths: BTreeMap<usize, f64> = moved.iter().copied().collect();
    for entity in &mut model.entities {
        let IfcEntity::ElementPropertySet { element, set } = entity else {
            continue;
        };
        let Some(&length) = lengths.get(&*element) else {
            continue;
        };
        if set.name != "Pset_MemberCommon" {
            continue;
        }
        for property in &mut set.properties {
            if property.name == "Span" {
                property.value = PropertyValue::PositiveLengthFeet(length);
            }
        }
    }
}

/// How far a wall's held `NetVolume` may differ from the volume of its mesh,
/// relative to the mesh's, and still be the solid's (RE-405): the mesh's corners
/// are single precision, which moves the volume of a small wall at 190 ft by
/// about 1e-4 of it (RE-388).
const WALL_BOX_MESH_TOLERANCE: f64 = 1e-3;

/// How far the vertical span of a wall's mesh may differ from the height of its
/// body, feet, and still confirm it (RE-405): the span is within 1e-4 ft of
/// Revit's `Height` on every wall of the BIM4LCA office (RE-388), and the
/// smallest difference between a body's height and Revit's measured on the
/// BIM4LCA housing is 0.11 ft.
const WALL_HEIGHT_AGREE_FEET: f64 = 5e-4;

/// How far, radians, an element's turn may lie off its wall's own axes for its
/// world box, which is axis aligned, to be that element's extent in the wall's
/// frame (#1046).
const OPENING_TURN_TOLERANCE: f64 = 1e-6;

/// A wall's own axes and the world boxes, feet, of the openings rvt-rs cuts in
/// it, `[min_x, min_y, min_z, max_x, max_y, max_z]` (#1046).
struct WallOpenings {
    frame: WallFrame,
    boxes: Vec<[f64; 6]>,
}

/// #1046: each wall's frame, by its index in `model.entities`, and the boxes of
/// the openings written in it: the rectangle of every element the wall hosts,
/// with the axis across the wall cut to the wall's thickness, placed on that
/// element's own placement and extruded up its height, as the writer emits it
/// (#227, RE-84). A wall whose element is not read, and an opening whose turn is
/// not one of its wall's, is left out, so the wall gets no areas of the mesh for
/// them.
fn wall_frame_and_openings(model: &IfcModel) -> BTreeMap<usize, WallOpenings> {
    let mut out: BTreeMap<usize, WallOpenings> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            rotation_radians,
            ..
        } = entity
        else {
            continue;
        };
        if matches!(ifc_type.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE") {
            out.insert(
                index,
                WallOpenings {
                    frame: WallFrame {
                        turn: rotation_radians.unwrap_or(0.0),
                    },
                    boxes: Vec::new(),
                },
            );
        }
    }
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            host_element_index: Some(host),
            location_feet: Some(location),
            rotation_radians,
            extrusion: Some(body),
            solid_shape: None,
            ..
        } = entity
        else {
            continue;
        };
        // Only a body of a plain rectangle is the opening the writer emits
        // from it; a profile or a solid of another shape explains nothing.
        if body.profile_override.is_some() {
            continue;
        }
        let Some(hosted) = out.get_mut(host) else {
            continue;
        };
        let turn = rotation_radians.unwrap_or(0.0) - hosted.frame.turn;
        if (2.0 * turn).sin().abs() > OPENING_TURN_TOLERANCE {
            continue;
        }
        let cut = model.opening_cuts.get(&index);
        let [x_dim, y_dim] = cut.map_or([body.width_feet, body.depth_feet], |cut| {
            [cut.x_dim_feet, cut.y_dim_feet]
        });
        let [cx, cy] = cut.map_or([0.0, 0.0], |cut| cut.centre_feet);
        let [start, height] = cut
            .and_then(|cut| cut.z_range_feet)
            .map_or([0.0, body.height_feet], |[start, height]| {
                [start, start + height]
            });
        let (sin, cos) = rotation_radians.unwrap_or(0.0).sin_cos();
        let mut x = [f64::INFINITY, f64::NEG_INFINITY];
        let mut y = [f64::INFINITY, f64::NEG_INFINITY];
        for side_x in [-1.0, 1.0] {
            for side_y in [-1.0, 1.0] {
                let (px, py) = (cx + side_x * x_dim / 2.0, cy + side_y * y_dim / 2.0);
                let corner = [
                    location[0] + px * cos - py * sin,
                    location[1] + px * sin + py * cos,
                ];
                x = [x[0].min(corner[0]), x[1].max(corner[0])];
                y = [y[0].min(corner[1]), y[1].max(corner[1])];
            }
        }
        let world = [
            x[0],
            y[0],
            location[2] + start,
            x[1],
            y[1],
            location[2] + start + height,
        ];
        if world.iter().all(|value| value.is_finite()) {
            hosted.boxes.push(world);
        }
    }
    out
}

/// Give each wall of `found` (an index into the entities and what the mesh
/// Revit saved for it gives) the quantities Revit's export writes from its
/// solid, in the base quantity set it holds, where that holds none of them
/// (RE-388): `NetVolume`, and, of a wall with no more than four vertical faces
/// or of one whose mesh is a vertical prism less the openings cut in it and
/// whose base is not attached (#1046), `NetSideArea` and, where its two sides
/// agree on it, `GrossSideArea` and, the wall type's `Width` times it,
/// `GrossVolume`
/// (revit-ifc's `PropertyUtil.CreateWallBaseQuantities`, CLEANROOM.md source 5,
/// RE-270). A wall with no base quantity set, as a wall Revit's export writes
/// none for may be, gets none.
///
/// The quantities of a wall's box (RE-270, RE-293) hold only where the box is
/// the solid (RE-405). A wall whose held `NetVolume` is not the mesh's, by more
/// than [`WALL_BOX_MESH_TOLERANCE`] of it, is a box that is not Revit's solid (a
/// wall joined at an angle, whose footprint is a pentagon, or a box longer than
/// the wall): the quantities taken from the box are dropped, and the mesh gives
/// those it can. The footprint stays where the mesh's vertical span is not the
/// box's height, which says the wall is cut in height and not in plan. A
/// `Height` is the box's, and is written for a wall that gets its areas, where
/// the mesh's vertical span confirms it and is dropped where the span does not
/// (a wall whose lower part a door as wide as the wall cuts away).
fn add_wall_quantities(model: &mut IfcModel, found: &[(usize, WallMeshQuantities)]) {
    if found.is_empty() {
        return;
    }
    let mut held: BTreeMap<usize, usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == "Qto_WallBaseQuantities" {
                held.entry(*element).or_insert(position);
            }
        }
    }
    for (index, quantities) in found {
        let Some(&position) = held.get(index) else {
            continue;
        };
        // The height of the wall's body, where it is the extrusion of its box.
        let body_height = match model.entities.get(*index) {
            Some(IfcEntity::BuildingElement {
                extrusion: Some(body),
                solid_shape: None,
                ..
            }) => Some(body.height_feet),
            _ => None,
        };
        let Some(IfcEntity::ElementPropertySet { set, .. }) = model.entities.get_mut(position)
        else {
            continue;
        };
        let height_agrees = body_height
            .map(|height| (quantities.height_feet - height).abs() <= WALL_HEIGHT_AGREE_FEET);
        let held_volume = set
            .properties
            .iter()
            .find_map(|property| match &property.value {
                PropertyValue::VolumeCubicFeet(volume) if property.name == "NetVolume" => {
                    Some(*volume)
                }
                _ => None,
            });
        if held_volume.is_some_and(|held| {
            (held - quantities.net_volume).abs() > WALL_BOX_MESH_TOLERANCE * quantities.net_volume
        }) {
            // The footprint is the plan's: a box whose height the mesh does not
            // have (a wall cut away from below or above) may still have Revit's.
            let keep_footprint = height_agrees == Some(false);
            set.properties
                .retain(|property| match property.name.as_str() {
                    "NetVolume" | "NetSideArea" | "GrossSideArea" | "GrossVolume" => false,
                    "GrossFootprintArea" => keep_footprint,
                    _ => true,
                });
        }
        if let Some(height) = body_height {
            if height_agrees == Some(false) {
                set.properties.retain(|property| property.name != "Height");
            } else if quantities.sides.is_some() && !has_property(set, "Height") {
                set.properties.push(Property {
                    name: "Height".into(),
                    value: PropertyValue::PositiveLengthFeet(height),
                });
            }
        }
        let width = set
            .properties
            .iter()
            .find_map(|property| match &property.value {
                PropertyValue::PositiveLengthFeet(width) if property.name == "Width" => {
                    Some(*width)
                }
                _ => None,
            });
        let mut add = |name: &str, value: PropertyValue| {
            if !has_property(set, name) {
                set.properties.push(Property {
                    name: name.into(),
                    value,
                });
            }
        };
        add(
            "NetVolume",
            PropertyValue::VolumeCubicFeet(quantities.net_volume),
        );
        let Some(sides) = quantities.sides else {
            continue;
        };
        add("NetSideArea", PropertyValue::AreaSquareFeet(sides.net));
        if let Some(gross) = sides.gross {
            add("GrossSideArea", PropertyValue::AreaSquareFeet(gross));
            if let Some(width) = width {
                add("GrossVolume", PropertyValue::VolumeCubicFeet(width * gross));
            }
        }
    }
}

/// Give each curtain panel of `widths` (an index into the entities and the
/// longer horizontal side, feet, of the box its body was drawn from a mesh as)
/// the `Width` of its `Qto_PlateBaseQuantities`, where that holds none
/// (RE-396). Revit's export writes it on the panels it draws as a box and on no
/// other, and `add_mesh_volumes` has made the set by now for each of these.
fn add_plate_widths(model: &mut IfcModel, widths: &[(usize, f64)]) {
    if widths.is_empty() {
        return;
    }
    let mut held: BTreeMap<usize, usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == "Qto_PlateBaseQuantities" {
                held.entry(*element).or_insert(position);
            }
        }
    }
    for &(index, width) in widths {
        let Some(&position) = held.get(&index) else {
            continue;
        };
        if let Some(IfcEntity::ElementPropertySet { set, .. }) = model.entities.get_mut(position) {
            if !has_property(set, "Width") {
                set.properties.push(Property {
                    name: "Width".into(),
                    value: PropertyValue::LengthFeet(width),
                });
            }
        }
    }
}

/// Whether `set` has a property called `name`.
fn has_property(set: &PropertySet, name: &str) -> bool {
    set.properties.iter().any(|property| property.name == name)
}

/// Whether `set` has the text property `name` with the value `text`.
fn has_text_property(set: &PropertySet, name: &str, text: &str) -> bool {
    set.properties.iter().any(|property| {
        property.name == name && matches!(&property.value, PropertyValue::Text(t) if t == text)
    })
}

/// Whether a wall's body is a plain extrusion or one with RE-320's stepped top,
/// which a saved mesh includes. A base cut at a floor (#1384) is neither.
fn plain_or_stepped(solid_shape: &Option<SolidShape>, property_set: &Option<PropertySet>) -> bool {
    match (solid_shape, property_set) {
        (None, Some(set)) => !has_property(set, WALL_BASE_SOURCE_PROPERTY),
        (Some(_), Some(set)) => {
            has_text_property(set, "BodyHeightSource", WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE)
                && !has_property(set, WALL_BASE_SOURCE_PROPERTY)
        }
        _ => false,
    }
}

/// The walls that embed a curtain wall (`WallEmbedWallGStep`), ElementId to
/// entity index: drawn as plain or stepped extrusions, alone in their Tag
/// (no other entity of any kind has it), not in `taken`, and with nothing
/// hosted in them but the opening their own curtain wall cuts (an
/// `IfcEntity::WallCutOpening` whose Tag is the curtain wall's: Revit's export
/// holds that void and the raised strip both). Nothing where the model has no
/// curtain wall, so a file without one reads no wall record here. An `Err`
/// names why the walls' native records were not read.
fn embed_hosts(
    rf: &mut crate::RevitFile,
    model: &IfcModel,
    taken: &BTreeMap<u64, Vec<usize>>,
) -> Result<BTreeMap<u64, usize>, String> {
    let element_id = |entity: &IfcEntity, wanted: &str| match entity {
        IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } if ifc_type == wanted => tag.parse::<u64>().ok(),
        _ => None,
    };
    let curtain: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| element_id(entity, "IFCCURTAINWALL"))
        .collect();
    if curtain.is_empty() {
        return Ok(BTreeMap::new());
    }
    // Alone in the Tag is decided over every entity that has it, before any
    // piece is dropped for what it is hosting or how its base was cut.
    let mut by_tag: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            ..
        } = entity
        {
            if let Ok(id) = tag.parse::<u64>() {
                by_tag.entry(id).or_default().push(index);
            }
        }
    }
    let mut walls: BTreeMap<u64, usize> = BTreeMap::new();
    for (id, indices) in &by_tag {
        let [index] = indices.as_slice() else {
            continue;
        };
        let IfcEntity::BuildingElement {
            ifc_type,
            location_feet: Some(_),
            extrusion: Some(_),
            solid_shape,
            property_set,
            ..
        } = &model.entities[*index]
        else {
            continue;
        };
        if ifc_type == "IFCWALL"
            && !taken.contains_key(id)
            && plain_or_stepped(solid_shape, property_set)
        {
            walls.insert(*id, *index);
        }
    }
    let ids: BTreeSet<u64> = walls.keys().copied().collect();
    let steps =
        crate::native_wall_cuts::wall_cut_steps(rf, &ids).map_err(|error| format!("{error:#}"))?;
    // What is hosted in a wall: a mesh already holds the holes Revit cuts in
    // it, so an opening that voids it again would cut twice.
    let mut foreign: BTreeSet<usize> = BTreeSet::new();
    let mut cut_by: BTreeMap<usize, BTreeSet<u64>> = BTreeMap::new();
    for entity in &model.entities {
        match entity {
            IfcEntity::BuildingElement {
                host_element_index: Some(host),
                ..
            }
            | IfcEntity::VoidOpening { host, .. }
            | IfcEntity::WallProfileOpening { host, .. }
            | IfcEntity::HostedCutOpening { host, .. } => {
                foreign.insert(*host);
            }
            IfcEntity::WallCutOpening { host, tag, .. } => match tag.parse::<u64>() {
                Ok(cutter) => {
                    cut_by.entry(*host).or_default().insert(cutter);
                }
                Err(_) => {
                    foreign.insert(*host);
                }
            },
            _ => {}
        }
    }
    Ok(walls
        .into_iter()
        .filter_map(|(id, index)| {
            let cutters: BTreeSet<u64> = steps
                .get(&id)
                .into_iter()
                .flatten()
                .filter(|step| step.kind == WallCutKind::Embed && curtain.contains(&step.cutter))
                .map(|step| step.cutter)
                .collect();
            let own_only = cut_by
                .get(&index)
                .is_none_or(|openings| openings.is_subset(&cutters));
            (!cutters.is_empty() && own_only && !foreign.contains(&index)).then_some((id, index))
        })
        .collect())
}

/// Draw the wall at `index`, one that embeds a curtain wall, from its saved
/// `meshes` where they start [`EMBED_RAISE_MIN_FEET`] above its base and end
/// as far below its top: Revit has cut its end stubs away and raised its base
/// (#1385, RE-340). A wall whose mesh starts at its base keeps its body, as
/// does one whose mesh did not decode whole.
fn raise_embed_host(
    model: &mut IfcModel,
    index: usize,
    meshes: &GraphicsMeshes,
    report: &mut SavedMeshReport,
) {
    if meshes.primitives.is_empty() {
        return;
    }
    if is_partial(meshes) {
        report.embed_partial += 1;
        return;
    }
    let IfcEntity::BuildingElement {
        location_feet: Some(origin),
        rotation_radians,
        extrusion: Some(body),
        solid_shape,
        property_set,
        ..
    } = &mut model.entities[index]
    else {
        return;
    };
    let lowest = meshes
        .primitives
        .iter()
        .flat_map(|primitive| primitive.vertices.iter())
        .map(|vertex| vertex[2])
        .fold(f64::INFINITY, f64::min);
    let (base, top) = (origin[2], origin[2] + body.height_feet);
    if !(lowest - base >= EMBED_RAISE_MIN_FEET && top - lowest >= EMBED_RAISE_MIN_FEET) {
        return;
    }
    let (vertices, triangles) = local_mesh(&meshes.primitives, *origin, *rotation_radians);
    if triangles.is_empty() {
        return;
    }
    report.embed_raised += 1;
    *solid_shape = Some(SolidShape::TriangulatedFaceSet {
        vertices_feet: vertices,
        triangles,
    });
    if let Some(set) = property_set {
        let mut named = false;
        for property in &mut set.properties {
            if property.name == "BodySource" {
                property.value = PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into());
                named = true;
            }
        }
        if !named {
            set.properties.push(Property {
                name: "BodySource".into(),
                value: PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into()),
            });
        }
    }
}
