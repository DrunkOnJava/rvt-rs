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
    BEAM_BOX_BODY_SOURCE, RECORD_BBOX_BODY_SOURCE, SPACE_OUTLINE_SAVED_MESH,
    SPACE_OUTLINE_SOURCE_PROPERTY, WALL_BASE_SOURCE_PROPERTY, WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE,
};
use super::mesh_volume::{all_faces_flat, closed_mesh_volume};
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
    meshes.diagnostics.iter().any(|diagnostic| {
        !diagnostic.ends_with(crate::native_graphics_traversal::NESTED_DETAIL_FILTER_DIAGNOSTIC)
            && !diagnostic.ends_with(crate::native_graphics_traversal::VIEW_COMPONENT_DIAGNOSTIC)
    })
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
        // its saved mesh, as the record box is.
        if !matches!(
            text("BodySource"),
            Some(RECORD_BBOX_BODY_SOURCE | BEAM_BOX_BODY_SOURCE)
        ) || profile_resolved
        {
            continue;
        }
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        report.candidates += 1;
        by_id.entry(id).or_default().push(index);
    }
    let hosts = match embed_hosts(rf, model, &by_id) {
        Ok(hosts) => hosts,
        Err(error) => {
            report.embed_error = Some(error);
            BTreeMap::new()
        }
    };
    report.embed_hosts = hosts.len();
    if by_id.is_empty() && hosts.is_empty() {
        return Ok(report);
    }
    let options = crate::native_document::Options {
        selected_ids: by_id
            .keys()
            .chain(hosts.keys())
            .copied()
            .collect::<BTreeSet<_>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        value_source: Some("saved_graphics"),
        ..Default::default()
    };
    // RE-387: the volume of each body drawn from a mesh of flat faces.
    let mut volumes: Vec<(usize, f64)> = Vec::new();
    // Each element is taken as it is meshed and its decoded graphs freed with
    // its batch of shared symbols, rather than all held until the last one is
    // meshed (#1075).
    crate::native_saved_scene::extract_in_batches_with_control(
        rf,
        &options,
        3,
        control,
        |element| {
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
                return Ok(());
            };
            let primitives = &element.meshes.primitives;
            if primitives.is_empty() {
                return Ok(());
            }
            if is_partial(&element.meshes) {
                report.partial += 1;
                return Ok(());
            }
            let IfcEntity::BuildingElement {
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
                return Ok(());
            }
            let volume = all_faces_flat(primitives)
                .then(|| closed_mesh_volume(&vertices, &triangles))
                .flatten();
            report.replaced += 1;
            report.triangles += triangles.len();
            *solid_shape = Some(SolidShape::TriangulatedFaceSet {
                vertices_feet: vertices,
                triangles,
            });
            if let Some(volume) = volume {
                volumes.push((*index, volume));
            }
            for property in &mut set.properties {
                if property.name == "BodySource" {
                    property.value = PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into());
                } else if property.name == SPACE_OUTLINE_SOURCE_PROPERTY {
                    property.value = PropertyValue::Text(SPACE_OUTLINE_SAVED_MESH.into());
                }
            }
            Ok(())
        },
    )?;
    add_mesh_volumes(model, &volumes);
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
/// element (RE-387); a body of another entity gets none.
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
