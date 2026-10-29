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
//! Doors and windows are drawn from their saved meshes by default
//! ([`SavedMeshScope::DoorsAndWindows`]: the maintainer, 2026-10-29), where
//! every vertex is Revit's within 0.001 ft on Core Interior's 132 doors and
//! 6 windows and the BIM4LCA office's 269 windows (Measure run
//! 40932136360). Every other class is opt-in ([`SavedMeshScope::All`],
//! `rvt-ifc --saved-meshes`, `rvt-gltf --saved-meshes`): reading saved
//! graphics is slow on large files, and some partial meshes record nothing.

use super::IfcModel;
use super::entities::{BrepTriangle, IfcEntity, PropertyValue, SolidShape};
use super::export_content::RECORD_BBOX_BODY_SOURCE;
use std::collections::{BTreeMap, BTreeSet};

/// `BodySource` of a body drawn from the element's saved graphics.
pub const SAVED_MESH_BODY_SOURCE: &str = "saved_graphics_mesh";

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
}

/// Which bounding-box bodies [`attach_scoped_with_control`] replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SavedMeshScope {
    /// Every element drawn as its record's box (`rvt-ifc --saved-meshes`).
    All,
    /// Doors and windows only, as `rvt-ifc` and `rvt-gltf` do by default.
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

/// Same as [`attach_with_control`], for the bodies `scope` names only.
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
        if text("BodySource") != Some(RECORD_BBOX_BODY_SOURCE) || profile_resolved {
            continue;
        }
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        report.candidates += 1;
        by_id.entry(id).or_default().push(index);
    }
    if by_id.is_empty() {
        return Ok(report);
    }
    let options = crate::native_document::Options {
        selected_ids: by_id.keys().copied().collect::<BTreeSet<_>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene =
        crate::native_saved_scene::extract_at_detail_with_control(rf, &options, 3, control)?;
    for element in &scene.elements {
        let Some(indices) = by_id.get(&element.identity.element_id) else {
            continue;
        };
        // A Tag carried by several entities (a slab's pieces) is ambiguous:
        // leave them as they are.
        let [index] = indices.as_slice() else {
            continue;
        };
        let primitives = &element.meshes.primitives;
        if primitives.is_empty() {
            continue;
        }
        // RE-224: a mesh some face or branch of which did not decode is only
        // part of the body (on the BIM4LCA office 357 such meshes, none of
        // them Revit's body). Only branches left out for their view semantics
        // leave it whole.
        let partial = element.meshes.diagnostics.iter().any(|diagnostic| {
            !diagnostic.ends_with(crate::native_graphics_traversal::NESTED_DETAIL_FILTER_DIAGNOSTIC)
                && !diagnostic
                    .ends_with(crate::native_graphics_traversal::VIEW_COMPONENT_DIAGNOSTIC)
        });
        if partial {
            report.partial += 1;
            continue;
        }
        let IfcEntity::BuildingElement {
            location_feet: Some(origin),
            rotation_radians,
            solid_shape,
            property_set: Some(set),
            ..
        } = &mut model.entities[*index]
        else {
            continue;
        };
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
        if triangles.is_empty() {
            continue;
        }
        report.replaced += 1;
        report.triangles += triangles.len();
        *solid_shape = Some(SolidShape::TriangulatedFaceSet {
            vertices_feet: vertices,
            triangles,
        });
        for property in &mut set.properties {
            if property.name == "BodySource" {
                property.value = PropertyValue::Text(SAVED_MESH_BODY_SOURCE.into());
            }
        }
    }
    Ok(report)
}
