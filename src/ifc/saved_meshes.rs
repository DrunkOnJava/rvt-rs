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
//! graphics of every boxed element: on the BIM4LCA office as built 24.4 s
//! against 21.2 s and a peak of 1,423 MiB against 913, with its IFC 83 MB
//! against 68, and 1.01 times the time and the same peak on Core Interior
//! (Measure run 42614230465, median of 3 on one runner).
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
    BEAM_BOX_BODY_SOURCE, INSTANCE_TURNED_BODY_SOURCE, RECORD_BBOX_BODY_SOURCE,
    SAVED_MESH_ELEMENT_ID_SHARED, SAVED_MESH_NO_GRAPHICS, SAVED_MESH_NO_MESH,
    SAVED_MESH_NO_TRIANGLES, SAVED_MESH_PARTIAL_PREFIX, SAVED_MESH_REFUSAL_PROPERTY,
    SLAB_SHAPE_REFUSAL_PROPERTY, SPACE_OUTLINE_SAVED_MESH, SPACE_OUTLINE_SOURCE_PROPERTY,
    WALL_BASE_SOURCE_PROPERTY, WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE,
};
use super::mesh_volume::{all_faces_flat, box_plan_width, closed_mesh_volume};
use super::mesh_wall_quantities::{WallMeshQuantities, wall_mesh_quantities};
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
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            location_feet: Some(_),
            ..
        } = entity
        else {
            continue;
        };
        if !scope.includes(ifc_type) {
            continue;
        }
        let text = |name: &str| {
            set.properties.iter().find_map(|p| match &p.value {
                PropertyValue::Text(t) if p.name == name => Some(t.as_str()),
                _ => None,
            })
        };
        let profile_resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        // A beam RE-328 draws as its box along its line is a stand-in for
        // its saved mesh, as the record box is, and so is a family instance
        // RE-87 draws as its record box turned to its angle: on Snowdon
        // Towers Electrical 116 of 130 such receptacles, outlets, conduit
        // elbows and pendants have a whole saved mesh. So is the level block of
        // a roof's sketch outline: the record box holds the roof's reference
        // surface, not its slope or its thickness (#1612). A resolved outline
        // is the body of any other class (a floor's plan, a ceiling's).
        if !matches!(
            text("BodySource"),
            Some(RECORD_BBOX_BODY_SOURCE | BEAM_BOX_BODY_SOURCE | INSTANCE_TURNED_BODY_SOURCE)
        ) || (profile_resolved && ifc_type != "IFCROOF")
        {
            continue;
        }
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        report.candidates += 1;
        by_id.entry(id).or_default().push(index);
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
    let hosts = match embed_hosts(rf, model, &by_id) {
        Ok(hosts) => hosts,
        Err(error) => {
            report.embed_error = Some(error);
            BTreeMap::new()
        }
    };
    report.embed_hosts = hosts.len();
    if by_id.is_empty() && hosts.is_empty() && walls.is_empty() {
        return Ok(report);
    }
    let options = crate::native_document::Options {
        selected_ids: by_id
            .keys()
            .chain(hosts.keys())
            .chain(walls.keys())
            .copied()
            .collect::<BTreeSet<_>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        value_source: Some("saved_graphics"),
        ..Default::default()
    };
    // RE-387: the volume of each body drawn from a mesh of flat faces.
    let mut volumes: Vec<(usize, f64)> = Vec::new();
    // RE-388: what the mesh of each wall gives of its quantities.
    let mut wall_quantities: Vec<(usize, WallMeshQuantities)> = Vec::new();
    // RE-396: the width of each curtain panel drawn from a mesh that is a vertical box.
    let mut plate_widths: Vec<(usize, f64)> = Vec::new();
    // #1645: the candidates the scene held graphics for, and why each box that
    // keeps its box does.
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut refusals: Vec<(usize, String)> = Vec::new();
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
                if let Some(found) = wall_mesh_quantities(&element.meshes.primitives) {
                    wall_quantities.push((*index, found));
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
            report.replaced += 1;
            report.triangles += triangles.len();
            *solid_shape = Some(SolidShape::TriangulatedFaceSet {
                vertices_feet: vertices,
                triangles,
            });
            if let Some(volume) = volume {
                volumes.push((*index, volume));
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
    add_mesh_volumes(model, &volumes);
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

/// Give each element of `volumes` (an index into the entities and the volume,
/// cubic feet, of the closed mesh of flat faces its body was drawn from) the
/// `NetVolume` that Revit's export writes for it, in the base quantity set of
/// its entity, where it holds none (RE-387): revit-ifc's `NetVolumeCalculator`
/// takes the volume of the solid it writes (CLEANROOM.md source 5, RE-243),
/// and the mesh is that solid, which a body of flat faces gives exactly. A
/// column gets no `GrossVolume`: Revit's export omits it on the 18 columns of
/// the BIM4LCA office at building permit that it draws as a cut mesh, and
/// writes it on 6 that nothing tells from them. A curtain mullion, a curtain
/// panel and a column are the entities measured equal to Revit's on every
/// element (RE-387); a body of another entity gets none. A curtain mullion
/// whose `NetVolume` the box gave and whose mesh has another volume is not an
/// extrusion of its box, and takes the mesh's (RE-403).
fn add_mesh_volumes(model: &mut IfcModel, volumes: &[(usize, f64)]) {
    if volumes.is_empty() {
        return;
    }
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
                        follow_mesh_over_box(set, volume);
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
}

/// How far a curtain mullion's box volume may differ from the volume of the
/// mesh Revit saved for it, as a fraction of the mesh's, before the box is not
/// its solid (RE-403). A mullion whose `NetVolume` is Revit's from its box has
/// the mesh's volume to within 1e-4; one with a 45 degree end differs by 2.6 to
/// 3.7%.
const BOX_MESH_VOLUME_TOLERANCE: f64 = 1e-3;

/// A curtain mullion whose `NetVolume` was written from its box (RE-243) and
/// whose saved mesh has another volume is not an extrusion of that box, as a
/// mullion with a 45 degree end is not: Revit's `NetVolume` is the mesh's
/// (RE-387), and the box's `NetSurfaceArea`, its sides and the end faces its
/// neighbours leave, is not Revit's, so it is left out (RE-403).
fn follow_mesh_over_box(set: &mut PropertySet, mesh_volume: f64) {
    if !(mesh_volume.is_finite() && mesh_volume > 0.0) {
        return;
    }
    let box_volume = set
        .properties
        .iter()
        .find_map(|property| match &property.value {
            PropertyValue::VolumeCubicFeet(volume) if property.name == "NetVolume" => Some(*volume),
            _ => None,
        });
    let Some(box_volume) = box_volume else {
        return;
    };
    if (box_volume - mesh_volume).abs() <= BOX_MESH_VOLUME_TOLERANCE * mesh_volume {
        return;
    }
    for property in &mut set.properties {
        if property.name == "NetVolume" {
            property.value = PropertyValue::VolumeCubicFeet(mesh_volume);
        }
    }
    set.properties
        .retain(|property| property.name != "NetSurfaceArea");
}

/// Give each wall of `found` (an index into the entities and what the mesh
/// Revit saved for it gives) the quantities Revit's export writes from its
/// solid, in the base quantity set it holds, where that holds none of them
/// (RE-388): `NetVolume`, and of a wall with no more than four vertical faces
/// `NetSideArea` and, where its two sides agree on it, `GrossSideArea` and, the
/// wall type's `Width` times it, `GrossVolume`
/// (revit-ifc's `PropertyUtil.CreateWallBaseQuantities`, CLEANROOM.md source 5,
/// RE-270). A wall with no base quantity set, as a wall Revit's export writes
/// none for may be, gets none.
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
        let Some(IfcEntity::ElementPropertySet { set, .. }) = model.entities.get_mut(position)
        else {
            continue;
        };
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
