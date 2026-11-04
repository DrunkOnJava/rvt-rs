//! Family-instance bodies drawn from the family's own extrusions (#156).
//!
//! A family's document holds each extrusion as the sketch curves it was drawn
//! from (lines and arcs in a sketch plane) and the two offsets it was
//! extruded between. Where the bodies of a family's instances are the meshes
//! Revit saved for display ([`super::saved_meshes`]), [`attach`] rebuilds the
//! family's extrusions and draws the instance from them instead, as
//! `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles of straight
//! segments. An instance with an arc in a profile keeps its mesh: in the one
//! case measured, Revit's vertices lay 0.0014 ft from the extrusions' arcs as
//! IfcOpenShell tessellates them, which the volumes suggest is a difference
//! in where Revit's tessellation of an arc starts (an inference from the
//! volumes, not a measurement; RE-327 section 9).
//!
//! An instance keeps its saved mesh unless the extrusions reproduce it. The
//! family's document holds the family as saved, and a placed instance is
//! drawn at its type's values, so each instance is checked on its own, with
//! no verdict shared between instances of a family: every vertex of its mesh
//! must lie on the extrusions' surfaces, samples of their surfaces must lie
//! on its mesh, and the volumes must agree within the chord loss of the
//! mesh's arcs and 0.01%.
//!
//! Only a family's `ExtrusionElem`, `CurveElem` and `SketchPlane` records are
//! read. A family with a void (an extrusion that cuts), or an extrusion the
//! reader does not rebuild (an ellipse, a sketch whose curves do not close, a
//! plane or a field it does not read), or no extrusion left once the ones
//! whose `Visible` parameter is 0 are set aside, draws nothing. Any other
//! form the family holds (a sweep, a blend, a revolution), and an extrusion
//! whose `Visible` parameter is not read (taken as visible), are not seen:
//! it is the check of an instance against its own mesh that refuses an
//! instance such a form or such an extrusion makes differ.
//!
//! The extrusions are placed by the instance's transform (RE-87, in the
//! family's coordinates to the model's), then into the element's frame as
//! [`super::saved_meshes::attach`] puts the mesh there: the mesh's vertices
//! are the model's less the element's location, turned by minus its rotation
//! about the vertical axis, so the two are in one frame for any rotation of
//! the instance. An instance without a transform that reads keeps its mesh:
//! the transform is read on Revit 2024 to 2027 only (the element-data header
//! is measured for those), and only as a rotation, so a reflected instance's
//! is never read. A tilted or turned instance whose transform is read is
//! placed by it and checked like any other.

use super::IfcModel;
use super::entities::{
    CurveSegment, IfcEntity, IndexedCurve, ProfileDef, PropertyValue, SolidShape,
};
use super::export_content::{SPACE_OUTLINE_FAMILY_BODY, SPACE_OUTLINE_SOURCE_PROPERTY};
use super::saved_meshes::SAVED_MESH_BODY_SOURCE;
use crate::native_form_geometry::{
    Curve, ExtrusionSolid, FormDocument, Frame, Loop, mesh_agreement,
};
use crate::partition_instance_transforms::{InstanceTransform, scan_instance_transforms};
use crate::{native_document, native_family_forms};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

/// `BodySource` of a body drawn from the family's own extrusions.
pub const FAMILY_EXTRUSION_BODY_SOURCE: &str = "family_extrusions";

/// How far a vertex of an instance's mesh may lie from the extrusions' surface
/// for the extrusions to replace it, feet. In the 6,014 printed forms (of
/// 6,172) of Autodesk's sample projects that match their own complete mesh,
/// every mesh vertex lies under 5e-7 ft from the rebuilt surface; the margin
/// is the single-precision rounding of a mesh far from the origin. The
/// tolerances were first 5e-4 ft, 1.5e-3 ft and 1% of the volume, and
/// narrowing them to these changed no replaced body (RE-327 section 9).
const VERTEX_TOLERANCE: f64 = 1e-4;
/// How far a sampled point of the extrusions' surface may lie from the mesh,
/// feet: the 0.001 ft chords the mesh draws arcs with (the largest of the
/// 6,014 printed forms is 0.000998 ft), and a tenth more.
const SURFACE_TOLERANCE: f64 = 1.1e-3;

/// What [`attach`] did. Every instance drawn from a saved mesh is counted
/// once: `candidates = replaced + not_rebuilt + unplaced + unchecked +
/// volume_differs + vertices_differ + surface_differs + arcs + unwritable`.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct FamilyExtrusionReport {
    /// Instances drawn from a saved mesh and alone in their Tag.
    pub candidates: usize,
    /// Instances left alone because another entity carries their Tag.
    pub ambiguous: usize,
    /// The families of the candidates whose document was found.
    pub families: usize,
    /// Of those, families all of whose extrusions rebuilt, with no void.
    pub families_rebuilt: usize,
    /// Candidates now drawn from their family's extrusions.
    pub replaced: usize,
    /// Candidates whose family was not found or not rebuilt.
    pub not_rebuilt: usize,
    /// Candidates of a rebuilt family with no instance transform that reads:
    /// every candidate of a file before Revit 2024, and one whose transform
    /// is not a rotation (a reflection is never read).
    pub unplaced: usize,
    /// Candidates whose mesh is too large to compare (more triangles than the
    /// reader bounds, or more vertices than the comparison budget allows for
    /// the extrusions' curves) or indexes outside its vertices.
    pub unchecked: usize,
    /// Candidates whose mesh the extrusions do not reproduce, by the first
    /// thing that differs: the volume, a vertex, a sampled point of the
    /// extrusions' surface.
    pub volume_differs: usize,
    pub vertices_differ: usize,
    pub surface_differs: usize,
    /// Candidates the extrusions reproduce, left with their mesh because a
    /// profile has an arc: in the one case measured, Revit's vertices lay
    /// 0.0014 ft from the extrusions' arcs, which the volumes suggest is a
    /// difference in where Revit starts its tessellation of an arc (an
    /// inference, RE-327 section 9).
    pub arcs: usize,
    /// Candidates the extrusions reproduce whose profile cannot be written.
    pub unwritable: usize,
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The class `class` records' root ElementId field `field`, by record.
fn root_ids(
    rf: &mut crate::RevitFile,
    ids: &BTreeSet<u64>,
    class: &str,
    field: &str,
) -> anyhow::Result<BTreeMap<u64, u64>> {
    let mut out = BTreeMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let options = native_document::Options {
        selected_ids: ids.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let target = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .filter(|root| root.class_name == class)
            .and_then(|root| root.fields.get(field))
            .and_then(crate::native_json::identifier_i64)
            .and_then(|n| u64::try_from(n).ok());
        if let Some(target) = target {
            out.insert(record.identity.element_id, target);
        }
        Ok(())
    })?;
    Ok(out)
}

/// The solids of a family's visible extrusions, or `None` where it has a
/// void or an extrusion the reader does not rebuild.
fn family_parts(document: &FormDocument) -> Option<Vec<ExtrusionSolid>> {
    let mut parts = Vec::new();
    for id in document.extrusion_ids() {
        let extrusion = document.extrusion(id).ok()?;
        if extrusion.cutting {
            return None;
        }
        if extrusion.visible == Some(0) {
            continue;
        }
        parts.push(extrusion.default_solid().ok()?);
    }
    (!parts.is_empty()).then_some(parts)
}

/// `solid`, a solid in a family's coordinates, in the frame of an element
/// whose instance transform is `transform` and which
/// [`super::saved_meshes::attach`] places at `location`, turned about the
/// vertical axis by `rotation`. `None` where the frame is no longer
/// orthonormal.
fn to_element_frame(
    solid: &ExtrusionSolid,
    transform: &InstanceTransform,
    location: [f64; 3],
    rotation: f64,
) -> Option<ExtrusionSolid> {
    let (sin, cos) = (-rotation).sin_cos();
    let turn = |v: [f64; 3]| [v[0] * cos - v[1] * sin, v[0] * sin + v[1] * cos, v[2]];
    let carry = |v: [f64; 3]| {
        [0usize, 1, 2].map(|i| (0..3).map(|j| transform.axes[j][i] * v[j]).sum::<f64>())
    };
    let moved = carry(solid.frame.origin);
    let world = [
        transform.origin[0] + moved[0] - location[0],
        transform.origin[1] + moved[1] - location[1],
        transform.origin[2] + moved[2] - location[2],
    ];
    let axes = solid.frame.axes.map(|axis| turn(carry(axis)));
    let unit = axes.iter().all(|a| (dot(*a, *a) - 1.0).abs() < 1e-6);
    let square = dot(axes[0], axes[1]).abs() < 1e-6
        && dot(axes[0], axes[2]).abs() < 1e-6
        && dot(axes[1], axes[2]).abs() < 1e-6;
    (unit && square).then(|| ExtrusionSolid {
        profile: solid.profile.clone(),
        frame: Frame {
            origin: turn(world),
            axes,
        },
        low: solid.low,
        high: solid.high,
    })
}

/// One closed `IfcIndexedPolyCurve` over a loop of a profile, in the plane
/// frame IFC builds from the placement's axis and reference direction
/// (`handed` is -1 where the solid's frame is left-handed, which mirrors the
/// profile's y). Counter-clockwise for an outline, clockwise for a hole.
fn indexed_curve(source: &Loop, counter_clockwise: bool, handed: f64) -> Option<IndexedCurve> {
    let oriented = if (source.signed_area() * handed > 0.0) == counter_clockwise {
        source.clone()
    } else {
        source.reversed()
    };
    let mirrored = |p: [f64; 2]| (p[0], handed * p[1]);
    let first = oriented.curves.first()?;
    let mut points = vec![mirrored(first.start_point())];
    let mut segments = Vec::new();
    let mut current = 0usize;
    let count = oriented.curves.len();
    for (n, curve) in oriented.curves.iter().enumerate() {
        let closes = n + 1 == count;
        let push_end = |points: &mut Vec<(f64, f64)>| -> usize {
            if closes {
                0
            } else {
                points.push(mirrored(curve.end_point()));
                points.len() - 1
            }
        };
        match curve {
            Curve::Line { .. } => {
                let end = push_end(&mut points);
                segments.push(CurveSegment::Line([current, end]));
                current = end;
            }
            Curve::Arc { start, end, .. } => {
                if (end - start).abs() > PI + 1e-9 {
                    points.push(mirrored(curve.point_at(0.25)));
                    let quarter = points.len() - 1;
                    points.push(mirrored(curve.point_at(0.5)));
                    let middle = points.len() - 1;
                    points.push(mirrored(curve.point_at(0.75)));
                    let three_quarters = points.len() - 1;
                    let last = push_end(&mut points);
                    segments.push(CurveSegment::Arc([current, quarter, middle]));
                    segments.push(CurveSegment::Arc([middle, three_quarters, last]));
                    current = last;
                } else {
                    points.push(mirrored(curve.point_at(0.5)));
                    let middle = points.len() - 1;
                    let last = push_end(&mut points);
                    segments.push(CurveSegment::Arc([current, middle, last]));
                    current = last;
                }
            }
        }
    }
    Some(IndexedCurve { points, segments })
}

/// A solid as `IfcExtrudedAreaSolid`s, one for each outline of its profile
/// with the holes inside it, placed by the solid's frame.
fn placed_extrusions(solid: &ExtrusionSolid) -> Option<Vec<SolidShape>> {
    let [x, y, z] = solid.frame.axes;
    let handed = if dot(cross(x, y), z) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let depth = solid.high - solid.low;
    if depth <= 1e-9 || !depth.is_finite() {
        return None;
    }
    let profile = &solid.profile;
    let parents = profile.parents();
    let o = solid.frame.origin;
    let origin = [
        o[0] + z[0] * solid.low,
        o[1] + z[1] * solid.low,
        o[2] + z[2] * solid.low,
    ];
    let mut out = Vec::new();
    for (i, outline) in profile.loops.iter().enumerate() {
        if profile.depths[i] & 1 == 1 {
            continue;
        }
        let outer = indexed_curve(outline, true, handed)?;
        let mut voids = Vec::new();
        for (j, hole) in profile.loops.iter().enumerate() {
            if profile.depths[j] & 1 == 1 && parents[j] == Some(i) {
                voids.push(indexed_curve(hole, false, handed)?);
            }
        }
        let profile_def = if voids.is_empty() {
            ProfileDef::IndexedPolyCurve {
                points: outer.points,
                segments: outer.segments,
            }
        } else {
            ProfileDef::IndexedPolyCurveWithVoids { outer, voids }
        };
        out.push(SolidShape::PlacedExtrusion {
            profile: profile_def,
            origin_feet: origin,
            axis: z,
            ref_direction: x,
            depth_feet: depth,
        });
    }
    (!out.is_empty()).then_some(out)
}

/// Draw each family instance whose body is its saved mesh from its family's
/// extrusions, where they reproduce the mesh (see the module docs). Run
/// after [`super::saved_meshes::attach`], whose meshes it checks against.
pub fn attach(
    rf: &mut crate::RevitFile,
    model: &mut IfcModel,
) -> anyhow::Result<FamilyExtrusionReport> {
    attach_with_control(rf, model, &crate::control::WalkerControl::default())
}

/// Same as [`attach`], checking `control` between its reads and for each
/// instance; a cancelled `control` ends it with [`crate::Error::Cancelled`].
pub fn attach_with_control(
    rf: &mut crate::RevitFile,
    model: &mut IfcModel,
    control: &crate::control::WalkerControl,
) -> anyhow::Result<FamilyExtrusionReport> {
    let mut report = FamilyExtrusionReport::default();
    let mut by_id: BTreeMap<u64, usize> = BTreeMap::new();
    let mut ambiguous: BTreeSet<u64> = BTreeSet::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            solid_shape: Some(SolidShape::TriangulatedFaceSet { .. }),
            ..
        } = entity
        else {
            continue;
        };
        let from_mesh = set.properties.iter().any(|p| {
            p.name == "BodySource"
                && matches!(&p.value, PropertyValue::Text(t) if t == SAVED_MESH_BODY_SOURCE)
        });
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        if !from_mesh {
            continue;
        }
        if by_id.insert(id, index).is_some() {
            ambiguous.insert(id);
        }
    }
    for id in &ambiguous {
        by_id.remove(id);
    }
    report.candidates = by_id.len();
    report.ambiguous = ambiguous.len();
    if by_id.is_empty() {
        return Ok(report);
    }

    let instances: BTreeSet<u64> = by_id.keys().copied().collect();
    control.check()?;
    let symbol_of = root_ids(rf, &instances, "FamilyInstance", "m_masterSymbolId")?;
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    control.check()?;
    let family_of_symbol = root_ids(rf, &symbols, "FamilySymbol", "m_familyId")?;
    let families: BTreeSet<u64> = family_of_symbol.values().copied().collect();
    control.check()?;
    let classes: BTreeSet<String> = ["ExtrusionElem", "CurveElem", "SketchPlane"]
        .iter()
        .map(|class| class.to_string())
        .collect();
    let records = native_family_forms::family_records_in(
        rf,
        &families,
        &classes,
        &native_document::Options::default().releases,
        None,
    )?;
    report.families = records.len();
    let mut parts: BTreeMap<u64, Vec<ExtrusionSolid>> = BTreeMap::new();
    for (family, items) in records {
        control.check()?;
        let mut document = FormDocument::default();
        for (id, graph) in items {
            document.insert(id, graph);
        }
        if let Some(solids) = family_parts(&document) {
            parts.insert(family, solids);
        }
    }
    report.families_rebuilt = parts.len();

    let release = rf.basic_file_info()?.version;
    let wanted: BTreeSet<u32> = by_id
        .iter()
        .filter(|(id, _)| {
            symbol_of
                .get(id)
                .and_then(|symbol| family_of_symbol.get(symbol))
                .is_some_and(|family| parts.contains_key(family))
        })
        .filter_map(|(id, _)| u32::try_from(*id).ok())
        .collect();
    control.check()?;
    let transforms = scan_instance_transforms(rf, release, &wanted)?;

    for (id, index) in &by_id {
        control.check()?;
        let Some(solids) = symbol_of
            .get(id)
            .and_then(|symbol| family_of_symbol.get(symbol))
            .and_then(|family| parts.get(family))
        else {
            report.not_rebuilt += 1;
            continue;
        };
        let IfcEntity::BuildingElement {
            location_feet: Some(location),
            rotation_radians,
            solid_shape:
                Some(SolidShape::TriangulatedFaceSet {
                    vertices_feet,
                    triangles,
                }),
            ..
        } = &model.entities[*index]
        else {
            report.unplaced += 1;
            continue;
        };
        let rotation = rotation_radians.unwrap_or(0.0);
        let local: Option<Vec<ExtrusionSolid>> = u32::try_from(*id)
            .ok()
            .and_then(|key| transforms.get(&key))
            .and_then(|transform| {
                solids
                    .iter()
                    .map(|solid| to_element_frame(solid, transform, *location, rotation))
                    .collect()
            });
        let Some(local) = local else {
            report.unplaced += 1;
            continue;
        };
        let corners: Vec<[u32; 3]> = triangles.iter().map(|t| [t.0, t.1, t.2]).collect();
        let Some(agreement) = mesh_agreement(&local, vertices_feet, &corners) else {
            report.unchecked += 1;
            continue;
        };
        if !agreement.volume_agrees() {
            report.volume_differs += 1;
            continue;
        }
        if agreement.vertex_max > VERTEX_TOLERANCE {
            report.vertices_differ += 1;
            continue;
        }
        if agreement.surface_max > SURFACE_TOLERANCE {
            report.surface_differs += 1;
            continue;
        }
        let has_arc = local
            .iter()
            .flat_map(|solid| solid.profile.loops.iter())
            .flat_map(|profile_loop| profile_loop.curves.iter())
            .any(|curve| matches!(curve, Curve::Arc { .. }));
        if has_arc {
            report.arcs += 1;
            continue;
        }
        let placed: Option<Vec<Vec<SolidShape>>> = local.iter().map(placed_extrusions).collect();
        let Some(placed) = placed else {
            report.unwritable += 1;
            continue;
        };
        let IfcEntity::BuildingElement {
            solid_shape,
            property_set: Some(set),
            ..
        } = &mut model.entities[*index]
        else {
            continue;
        };
        *solid_shape = Some(SolidShape::Compound(placed.into_iter().flatten().collect()));
        for property in &mut set.properties {
            if property.name == "BodySource" {
                property.value = PropertyValue::Text(FAMILY_EXTRUSION_BODY_SOURCE.into());
            } else if property.name == SPACE_OUTLINE_SOURCE_PROPERTY {
                property.value = PropertyValue::Text(SPACE_OUTLINE_FAMILY_BODY.into());
            }
        }
        report.replaced += 1;
    }
    Ok(report)
}
