//! Family-instance bodies drawn from the family's own extrusions (#156).
//!
//! A family's document holds each extrusion as the sketch curves it was drawn
//! from (lines and arcs in a sketch plane) and the two offsets it was
//! extruded between. Where the bodies of a family's instances are the meshes
//! Revit saved for display ([`super::saved_meshes`]), [`attach`] rebuilds the
//! family's extrusions and draws the instance from them instead, as
//! `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles whose arcs are
//! arcs, the form Revit's own export writes.
//!
//! An instance keeps its saved mesh unless the extrusions reproduce it. The
//! family's document holds the family as saved, and a placed instance is
//! drawn at its type's values, so each instance is checked on its own: every
//! vertex of its mesh must lie on the extrusions' surfaces, samples of their
//! surfaces must lie on its mesh, and the volumes must agree within 1% and
//! the chord loss of the mesh's arcs. A family with a void, a hidden form it
//! cannot read, a form that is not an extrusion or a profile the reader does
//! not draw (an ellipse, a sketch whose curves do not close) draws nothing,
//! and neither does an instance whose mesh the extrusions do not reproduce:
//! the instance keeps the body it has.
//!
//! The extrusions are placed as the family document gives them, which is the
//! instance's own frame when its transform is a rotation about the vertical
//! axis and a translation; an instance placed any other way fails the check
//! and keeps its mesh.

use super::IfcModel;
use super::entities::{
    CurveSegment, IfcEntity, IndexedCurve, ProfileDef, PropertyValue, SolidShape,
};
use super::saved_meshes::SAVED_MESH_BODY_SOURCE;
use crate::native_form_geometry::{Curve, ExtrusionSolid, FormDocument, Loop, mesh_agreement};
use crate::{native_document, native_family_forms};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

/// `BodySource` of a body drawn from the family's own extrusions.
pub const FAMILY_EXTRUSION_BODY_SOURCE: &str = "family_extrusions";

/// How far a vertex of an instance's mesh may lie from the extrusions' surface
/// for the extrusions to replace it, feet. Every vertex of the 6,093 matching
/// extrusions of Autodesk's sample projects lies within 1e-6 ft.
const VERTEX_TOLERANCE: f64 = 5e-4;
/// How far a sampled point of the extrusions' surface may lie from the mesh,
/// feet: the 0.001 ft chords the mesh draws arcs with, and half again.
const SURFACE_TOLERANCE: f64 = 1.5e-3;

/// What [`attach`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct FamilyExtrusionReport {
    /// Instances drawn from a saved mesh.
    pub candidates: usize,
    /// The families of those instances whose document was found.
    pub families: usize,
    /// Of those, families all of whose extrusions rebuilt, with no void.
    pub families_rebuilt: usize,
    /// Candidates now drawn from their family's extrusions.
    pub replaced: usize,
    /// Candidates of a rebuilt family whose mesh the extrusions did not
    /// reproduce, which keep it.
    pub kept: usize,
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
            if profile.depths[j] % 2 == 1 && parents[j] == Some(i) {
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
        report.candidates += 1;
        if by_id.insert(id, index).is_some() {
            ambiguous.insert(id);
        }
    }
    for id in &ambiguous {
        by_id.remove(id);
    }
    if by_id.is_empty() {
        return Ok(report);
    }

    let instances: BTreeSet<u64> = by_id.keys().copied().collect();
    let symbol_of = root_ids(rf, &instances, "FamilyInstance", "m_masterSymbolId")?;
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    let family_of_symbol = root_ids(rf, &symbols, "FamilySymbol", "m_familyId")?;
    let families: BTreeSet<u64> = family_of_symbol.values().copied().collect();
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
        let mut document = FormDocument::default();
        for (id, graph) in items {
            document.insert(id, graph);
        }
        if let Some(solids) = family_parts(&document) {
            parts.insert(family, solids);
        }
    }
    report.families_rebuilt = parts.len();

    for (id, index) in &by_id {
        let Some(solids) = symbol_of
            .get(id)
            .and_then(|symbol| family_of_symbol.get(symbol))
            .and_then(|family| parts.get(family))
        else {
            continue;
        };
        let IfcEntity::BuildingElement {
            solid_shape:
                Some(SolidShape::TriangulatedFaceSet {
                    vertices_feet,
                    triangles,
                }),
            ..
        } = &model.entities[*index]
        else {
            continue;
        };
        let corners: Vec<[u32; 3]> = triangles.iter().map(|t| [t.0, t.1, t.2]).collect();
        let reproduced = mesh_agreement(solids, vertices_feet, &corners).is_some_and(|a| {
            a.volume_agrees()
                && a.vertex_max <= VERTEX_TOLERANCE
                && a.surface_max <= SURFACE_TOLERANCE
        });
        let placed: Option<Vec<Vec<SolidShape>>> = solids.iter().map(placed_extrusions).collect();
        let (true, Some(placed)) = (reproduced, placed) else {
            report.kept += 1;
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
            }
        }
        report.replaced += 1;
    }
    Ok(report)
}
