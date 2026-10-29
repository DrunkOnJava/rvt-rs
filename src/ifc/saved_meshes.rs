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
    /// Shells left out because they lie wholly inside another shell of the
    /// same body (#1103).
    pub contained_shells: usize,
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
        report.contained_shells += leave_out_contained_shells(&vertices, &mut triangles);
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

type Point = [f64; 3];

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Whether the ray from `origin` along `direction` crosses the triangle
/// (Moller and Trumbore).
fn ray_crosses(origin: Point, direction: Point, [a, b, c]: [Point; 3]) -> bool {
    let (e1, e2) = (sub(b, a), sub(c, a));
    let h = cross(direction, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-12 {
        return false;
    }
    let s = sub(origin, a);
    let u = dot(s, h) / det;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, e1);
    let v = dot(direction, q) / det;
    v >= 0.0 && u + v <= 1.0 && dot(e2, q) / det > 1e-9
}

/// #1103: a saved mesh can hold a closed shell lying wholly inside another
/// shell of the same body, as a part flush with a door's leaf, where Revit's
/// export writes the two as one solid (on the BIM4LCA office four doors, the
/// larger shell alone equal to Revit's body). Such a shell adds volume and no
/// surface, so it is left out. Shells are the triangles joined through
/// vertices within 2e-5 ft of each other (the saved positions of a corner
/// that several faces share differ in their last digits, far from the
/// document origin); a shell enclosing positive volume
/// is inside a larger one when a point just inside each of its faces is
/// inside that one by ray parity. A shell enclosing no positive volume, such
/// as a void, is kept. Returns how many shells were left out.
fn leave_out_contained_shells(vertices: &[Point], triangles: &mut Vec<BrepTriangle>) -> usize {
    const INSET_FEET: f64 = 1e-4;
    const JOIN_FEET: f64 = 2e-5;
    const DIRECTION: Point = [0.4817, 0.5743, 0.6619];
    let mut cells: BTreeMap<[i64; 3], usize> = BTreeMap::new();
    let ids: Vec<usize> = vertices
        .iter()
        .map(|v| {
            let next = cells.len();
            *cells
                .entry(v.map(|c| (c / JOIN_FEET).round() as i64))
                .or_insert(next)
        })
        .collect();
    let mut parent: Vec<usize> = (0..cells.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    // Positions either side of a cell boundary: join each cell with its
    // occupied neighbours.
    for (cell, &id) in &cells {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let near = [cell[0] + dx, cell[1] + dy, cell[2] + dz];
                    if let Some(&other) = cells.get(&near) {
                        let (a, b) = (root(&mut parent, id), root(&mut parent, other));
                        parent[a] = b;
                    }
                }
            }
        }
    }
    let corners: Vec<[Point; 3]> = triangles
        .iter()
        .map(|t| [t.0, t.1, t.2].map(|i| vertices[i as usize]))
        .collect();
    for t in triangles.iter() {
        let first = root(&mut parent, ids[t.0 as usize]);
        for other in [t.1, t.2] {
            let r = root(&mut parent, ids[other as usize]);
            parent[r] = first;
        }
    }
    let mut shells: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, t) in triangles.iter().enumerate() {
        let r = root(&mut parent, ids[t.0 as usize]);
        shells.entry(r).or_default().push(i);
    }
    if shells.len() < 2 {
        return 0;
    }
    struct Shell {
        triangles: Vec<usize>,
        volume: f64,
        min: Point,
        max: Point,
    }
    let shells: Vec<Shell> = shells
        .into_values()
        .map(|triangles| {
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            let mut volume = 0.0;
            for &i in &triangles {
                let [a, b, c] = corners[i];
                volume += dot(a, cross(b, c)) / 6.0;
                for p in [a, b, c] {
                    for ((lo, hi), v) in min.iter_mut().zip(max.iter_mut()).zip(p) {
                        *lo = lo.min(v);
                        *hi = hi.max(v);
                    }
                }
            }
            Shell {
                triangles,
                volume,
                min,
                max,
            }
        })
        .collect();
    let inside = |p: Point, shell: &Shell| {
        shell
            .triangles
            .iter()
            .filter(|&&i| ray_crosses(p, DIRECTION, corners[i]))
            .count()
            % 2
            == 1
    };
    let mut left_out = vec![false; shells.len()];
    for (s, small) in shells.iter().enumerate() {
        if small.volume <= 0.0 || small.triangles.len() < 4 {
            continue;
        }
        let samples: Vec<Point> = small
            .triangles
            .iter()
            .filter_map(|&i| {
                let [a, b, c] = corners[i];
                let n = cross(sub(b, a), sub(c, a));
                let length = dot(n, n).sqrt();
                (length > 1e-12).then(|| {
                    let centre = [
                        (a[0] + b[0] + c[0]) / 3.0,
                        (a[1] + b[1] + c[1]) / 3.0,
                        (a[2] + b[2] + c[2]) / 3.0,
                    ];
                    let k = INSET_FEET / length;
                    [
                        centre[0] - n[0] * k,
                        centre[1] - n[1] * k,
                        centre[2] - n[2] * k,
                    ]
                })
            })
            .collect();
        if samples.is_empty() {
            continue;
        }
        left_out[s] = shells.iter().enumerate().any(|(l, large)| {
            l != s
                && !left_out[l]
                && large.volume > small.volume
                && (0..3).all(|k| {
                    small.min[k] >= large.min[k] - INSET_FEET
                        && small.max[k] <= large.max[k] + INSET_FEET
                })
                && samples.iter().all(|&p| inside(p, large))
        });
    }
    let dropped: BTreeSet<usize> = shells
        .iter()
        .zip(&left_out)
        .filter(|(_, out)| **out)
        .flat_map(|(shell, _)| shell.triangles.iter().copied())
        .collect();
    if dropped.is_empty() {
        return 0;
    }
    let mut index = 0;
    triangles.retain(|_| {
        let keep = !dropped.contains(&index);
        index += 1;
        keep
    });
    left_out.iter().filter(|out| **out).count()
}
