//! The volume of the mesh a body is drawn from, where it is a volume (RE-387).
//!
//! A saved mesh need not be closed ([`super::entities::SolidShape::TriangulatedFaceSet`]),
//! and the signed sum of its triangles' tetrahedra is a volume only where it
//! is. [`closed_mesh_volume`] gives one only where the triangles, their
//! vertices welded, form a closed surface wound one way: every directed edge
//! has the opposite edge, once, and nothing else. [`all_faces_flat`] says
//! whether every face the mesh was cut from is a plane, because the volume of
//! a polyhedron is its solid's, where a tessellated curved face is not.

use super::entities::BrepTriangle;
use crate::native_saved_mesh::Primitive;
use crate::vector_math::{cross, dot, sub};
use std::collections::{HashMap, HashSet};

/// Points this close, feet, are one vertex.
const WELD_FEET: f64 = 1e-6;

/// How far apart, per component, the normals of one face may be and the face
/// still be a plane.
const NORMAL_TOLERANCE: f64 = 1e-9;

/// Whether every face of `primitives` has one normal on all its vertices: a
/// planar face, which a saved mesh holds exactly, where a curved face holds
/// the normal it has at each vertex. A face with no normal, and no face at
/// all, is not flat.
pub(super) fn all_faces_flat(primitives: &[Primitive]) -> bool {
    !primitives.is_empty()
        && primitives.iter().all(|primitive| {
            let mut normals = primitive.normals.iter();
            let Some(first) = normals.next() else {
                return false;
            };
            normals.all(|normal| {
                normal
                    .iter()
                    .zip(first)
                    .all(|(a, b)| (a - b).abs() <= NORMAL_TOLERANCE)
            })
        })
}

/// The volume, cubic feet, of the closed surface `triangles` make of
/// `vertices`, or none where they make no closed surface: a vertex that is not
/// finite or not there, no triangle left, an edge no other triangle shares or
/// one that three do, two triangles that wind the same way along an edge, or
/// no volume at all.
pub(super) fn closed_mesh_volume(vertices: &[[f64; 3]], triangles: &[BrepTriangle]) -> Option<f64> {
    closed_mesh(vertices, triangles).map(|mesh| mesh.volume)
}

/// A closed surface a mesh makes (RE-423): its volume, cubic feet, how many
/// vertices and triangles it has once welded, which give its Euler characteristic,
/// and how many separate pieces it is in (the characteristic of several is the sum
/// of theirs, so a solid with a hole through it beside a solid without one has the
/// characteristic of a single solid with none).
struct ClosedMesh {
    volume: f64,
    vertices: usize,
    triangles: usize,
    components: usize,
}

/// The root of `x` in the union-find forest `parent`, shortening the path to it.
fn find_root(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let up = parent[parent[x as usize] as usize];
        parent[x as usize] = up;
        x = up;
    }
    x
}

/// Join the pieces holding `a` and `b`.
fn join_roots(parent: &mut [u32], a: u32, b: u32) {
    let (a, b) = (find_root(parent, a), find_root(parent, b));
    if a != b {
        parent[b as usize] = a;
    }
}

/// The closed surface `triangles` make of `vertices`, as [`closed_mesh_volume`]
/// reads it.
fn closed_mesh(vertices: &[[f64; 3]], triangles: &[BrepTriangle]) -> Option<ClosedMesh> {
    let mut welded: HashMap<[i64; 3], u32> = HashMap::new();
    let mut ids = Vec::with_capacity(vertices.len());
    let mut centre = [0.0; 3];
    for point in vertices {
        if !point.iter().all(|coordinate| coordinate.is_finite()) {
            return None;
        }
        let key = (*point).map(|coordinate| (coordinate / WELD_FEET).round() as i64);
        let next = u32::try_from(welded.len()).ok()?;
        ids.push(*welded.entry(key).or_insert(next));
        for (total, coordinate) in centre.iter_mut().zip(point) {
            *total += coordinate;
        }
    }
    if vertices.is_empty() {
        return None;
    }
    let count = vertices.len() as f64;
    centre = centre.map(|sum| sum / count);
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    let mut used: HashSet<u32> = HashSet::new();
    let mut parent: Vec<u32> = (0..u32::try_from(welded.len()).ok()?).collect();
    let mut counted = 0usize;
    let mut sum = 0.0;
    for BrepTriangle(a, b, c) in triangles {
        let (ia, ib, ic) = (
            *ids.get(*a as usize)?,
            *ids.get(*b as usize)?,
            *ids.get(*c as usize)?,
        );
        if ia == ib || ib == ic || ic == ia {
            continue;
        }
        used.extend([ia, ib, ic]);
        join_roots(&mut parent, ia, ib);
        join_roots(&mut parent, ib, ic);
        counted += 1;
        for edge in [(ia, ib), (ib, ic), (ic, ia)] {
            *edges.entry(edge).or_insert(0) += 1;
        }
        let at = |index: &u32| {
            vertices
                .get(*index as usize)
                .copied()
                .map(|p| sub(p, centre))
        };
        sum += dot(at(a)?, cross(at(b)?, at(c)?));
    }
    if edges.is_empty() {
        return None;
    }
    if edges
        .iter()
        .any(|(&(from, to), &seen)| seen != 1 || edges.get(&(to, from)) != Some(&1))
    {
        return None;
    }
    let volume = sum.abs() / 6.0;
    let components = used
        .iter()
        .map(|vertex| find_root(&mut parent, *vertex))
        .collect::<HashSet<u32>>()
        .len();
    (volume.is_finite() && volume > 0.0).then_some(ClosedMesh {
        volume,
        vertices: used.len(),
        triangles: counted,
        components,
    })
}

/// How close, relative, the volume of a mesh must be to the product of the
/// extents of its box for the mesh to be that box.
const BOX_TOLERANCE: f64 = 1e-4;

/// The `Width` Revit's export writes for a curtain panel drawn as a vertical
/// box (RE-396): the longer of its two horizontal extents. `vertices` are in
/// the element's own frame, feet, `volume` is the closed volume of `triangles`
/// over them; none where the mesh is not a box (its volume is not the product
/// of the extents of the corners the triangles use, which Revit's export gives
/// no `Width`) or the box is not upright (its smallest extent is not
/// horizontal).
pub(super) fn box_plan_width(
    vertices: &[[f64; 3]],
    triangles: &[BrepTriangle],
    volume: f64,
) -> Option<f64> {
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for BrepTriangle(a, b, c) in triangles {
        for index in [a, b, c] {
            let point = vertices.get(*index as usize)?;
            for ((lo, hi), coordinate) in low.iter_mut().zip(high.iter_mut()).zip(point) {
                *lo = lo.min(*coordinate);
                *hi = hi.max(*coordinate);
            }
        }
    }
    let [dx, dy, dz] = [high[0] - low[0], high[1] - low[1], high[2] - low[2]];
    if !(dx.is_finite() && dy.is_finite() && dz.is_finite() && dx > 0.0 && dy > 0.0 && dz > 0.0) {
        return None;
    }
    let is_box = (dx * dy * dz - volume).abs() <= BOX_TOLERANCE * volume;
    (is_box && dx.min(dy) < dz).then_some(dx.max(dy))
}

/// Triangles whose doubled area, square feet, is under this have no plane.
const MIN_TWICE_AREA: f64 = 2e-11;

/// Two triangles lie on one plane where one minus the cosine of their normals
/// is under this (an angle of 0.26 degrees) and their planes are closer than
/// [`PLANE_FEET`]: a mesh whose corners are single precision, 3e-5 ft at 300 ft
/// from the origin, turns the normal of a 0.05 ft face by up to 6e-4 radians.
const PLANE_COSINE: f64 = 1e-5;

/// How close, feet, two parallel planes are one (0.2 mm).
const PLANE_FEET: f64 = 7e-4;

/// A plane is parallel to an axis where its normal's cosine with the axis is
/// under this (0.06 degrees).
const PARALLEL_COSINE: f64 = 1e-3;

/// The most planes a mesh may have to be read as a prism (RE-418): a mullion's
/// section of n sides has n + 2, and the most of the 27,059 mullions of the
/// BIM4LCA housing and the Autodesk samples is 13 (an eleven-sided section). A
/// member bent along a curve, a spiral stair's stringer, has hundreds, and is
/// not read: finding its planes would cost more than any rule gets from them.
const MAX_PLANES: usize = 32;

/// A plane's normal is square to an axis where its cosine with it is over this
/// (0.8 degrees).
const SQUARE_COSINE: f64 = 0.9999;

/// One of the two end planes of a mesh read as a prism ([`Prism`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PrismEnd {
    /// Whether the plane is square to the axis; a mitred end is not.
    pub square: bool,
    /// The area of the mesh's triangles on the plane, square feet.
    pub area: f64,
    /// The centroid of the plane's triangles, in the mesh's frame, feet.
    pub centre: [f64; 3],
}

/// A mesh read as a prism along one axis (RE-418, RE-404).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Prism {
    /// The distance between the first and the last of the mesh's corners along
    /// the axis, feet: a mitred end counts at its longer side.
    pub extent: f64,
    /// The area of the triangles on the planes parallel to the axis, square
    /// feet.
    pub lateral: f64,
    /// The two planes that are not parallel to it.
    pub ends: [PrismEnd; 2],
}

/// The mesh `triangles` make of `vertices` read as a prism along `axis` (RE-418):
/// every plane of its triangles but two (its ends) parallel to it. A curtain
/// mullion with a flat or a mitred end is a prism along its own axis; one with a
/// gabled end, whose end is two planes, is not, though it is along the axis of
/// its section's side, which is why the axis is the mullion's own, from its
/// record, and not found from the mesh.
///
/// None where `axis` is no direction, a vertex is not there or not finite, no
/// triangle has an area, there are more than [`MAX_PLANES`] planes, or the
/// planes not parallel to the axis are not two.
pub(super) fn prism_along(
    vertices: &[[f64; 3]],
    triangles: &[BrepTriangle],
    axis: [f64; 3],
) -> Option<Prism> {
    let length = dot(axis, axis).sqrt();
    if !(length.is_finite() && length > 0.0) {
        return None;
    }
    let axis = axis.map(|component| component / length);
    // Each plane: its unit normal, its offset, the area of its triangles and the
    // moment of their centroids about the origin, area times centroid.
    let mut planes: Vec<([f64; 3], f64, f64, [f64; 3])> = Vec::new();
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for BrepTriangle(a, b, c) in triangles {
        let (Some(&p), Some(&q), Some(&r)) = (
            vertices.get(*a as usize),
            vertices.get(*b as usize),
            vertices.get(*c as usize),
        ) else {
            return None;
        };
        if ![p, q, r]
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
        {
            return None;
        }
        let normal = cross(sub(q, p), sub(r, p));
        let twice_area = dot(normal, normal).sqrt();
        if twice_area < MIN_TWICE_AREA {
            continue;
        }
        for corner in [p, q, r] {
            let along = dot(corner, axis);
            low = low.min(along);
            high = high.max(along);
        }
        let normal = normal.map(|component| component / twice_area);
        let offset = dot(normal, p);
        let known = planes.iter().position(|(other, other_offset, ..)| {
            let cosine = dot(normal, *other);
            cosine.abs() > 1.0 - PLANE_COSINE
                && (offset - cosine.signum() * *other_offset).abs() < PLANE_FEET
        });
        let area = twice_area / 2.0;
        let moment = [
            (p[0] + q[0] + r[0]) / 3.0 * area,
            (p[1] + q[1] + r[1]) / 3.0 * area,
            (p[2] + q[2] + r[2]) / 3.0 * area,
        ];
        match known {
            Some(position) => {
                let plane = &mut planes[position];
                plane.2 += area;
                for (sum, part) in plane.3.iter_mut().zip(moment) {
                    *sum += part;
                }
            }
            None if planes.len() == MAX_PLANES => return None,
            None => planes.push((normal, offset, area, moment)),
        }
    }
    let mut lateral = 0.0;
    let mut ends = Vec::new();
    for (normal, _, area, moment) in &planes {
        let cosine = dot(*normal, axis).abs();
        if cosine > PARALLEL_COSINE {
            ends.push(PrismEnd {
                square: cosine > SQUARE_COSINE,
                area: *area,
                centre: moment.map(|sum| sum / area),
            });
        } else {
            lateral += area;
        }
    }
    let ends: [PrismEnd; 2] = ends.try_into().ok()?;
    Some(Prism {
        extent: high - low,
        lateral,
        ends,
    })
}

/// How far, as a fraction of a mullion's written `Length`, the extent of its
/// mesh along its own axis may be from it and be its length (RE-418): the box
/// overruns the solid by up to 14% on the 49 mullions of the BIM4LCA timber
/// office at permit that it moves.
pub(super) const MESH_LENGTH_REACH: f64 = 0.35;

/// How far, as a fraction of the larger, the areas of a prism's two flat ends
/// may differ and the ends still be the two of one section.
const END_AREA_TOLERANCE: f64 = 1e-4;

/// The `NetSurfaceArea` of a curtain mullion drawn from its mesh (RE-404): the
/// area of the faces along it plus that of the end faces Revit's solid keeps,
/// `kept` of its two ends (0, 1 or 2, RE-268). revit-ifc's
/// `NetSurfaceAreaCalculator` (CLEANROOM.md source 5) sums the faces of the solid
/// it writes. Of two flat ends of one area each is one section's, and `kept`
/// of them count. Of a mitred end and a flat one, the mitred end always counts
/// (on every one of the 184 of the BIM4LCA housing's four large files and the
/// 58 of the office's two permit files) and the flat one where `kept` is 2; a
/// mullion with a mitred end and `kept` 0 contradicts that and gets none. Of two
/// mitred ends, which a bent joint at each end makes, both count and `kept` must
/// be 2 (RE-433: Revit's value counts every end of all 159 of the BIM4LCA
/// office's mullions that have a bent partner, the 131 of them with two mitred
/// ends included). A prism with any other ends gets none.
pub(super) fn mullion_surface_area(prism: &Prism, kept: u8) -> Option<f64> {
    let [a, b] = prism.ends;
    if a.square && b.square {
        let same = (a.area - b.area).abs() <= END_AREA_TOLERANCE * a.area.max(b.area);
        (same && kept <= 2).then(|| prism.lateral + f64::from(kept) * a.area)
    } else if a.square != b.square && (1..=2).contains(&kept) {
        let (flat, mitre) = if a.square { (a, b) } else { (b, a) };
        let flat = if kept == 2 { flat.area } else { 0.0 };
        Some(prism.lateral + mitre.area + flat)
    } else if !a.square && !b.square && kept == 2 {
        Some(prism.lateral + a.area + b.area)
    } else {
        None
    }
}

/// How close, as one minus the cosine, two edges' directions are one (an angle
/// of 0.26 degrees).
const EDGE_COSINE: f64 = 1e-5;

/// The most distinct edge directions a mesh may have to be tried as a beam's
/// axis (RE-423): a box has 3 and a prism over a twelve-sided section about 25.
/// A mesh with more is a curved or a free-form solid.
const MAX_AXIS_CANDIDATES: usize = 48;

/// Two prisms' extents are one where they differ by no more than this fraction of
/// the longer.
const EXTENT_TIE: f64 = 1e-6;

/// How far, as a fraction of the mesh's volume, the volume may be from a
/// prism's end area times its extent.
const PRISM_VOLUME_TOLERANCE: f64 = 1e-4;

/// The base quantities of a beam read from the mesh of its body (RE-423),
/// square and cubic feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BeamQuantities {
    /// The area of the section the beam is extruded from.
    pub cross_section_area: f64,
    /// The area of the faces along the extrusion.
    pub outer_surface_area: f64,
    /// The area of all its faces: the faces along it and its two ends.
    pub surface_area: f64,
    /// The volume of the solid.
    pub volume: f64,
}

/// The base quantities of a column read from the mesh of its body (RE-424),
/// feet, square and cubic feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ColumnQuantities {
    /// The extent of the solid along the vertical: the depth of the extrusion.
    pub length: f64,
    /// The area of the section the column is extruded from.
    pub cross_section_area: f64,
    /// The area of the faces along the extrusion.
    pub outer_surface_area: f64,
    /// The volume of the solid.
    pub volume: f64,
}

/// The distinct directions of the edges of `triangles`, unit vectors with a
/// direction and its opposite one: the axes a prism may run along, since its
/// lateral edges run along it. None where a vertex is not there or there are
/// more than [`MAX_AXIS_CANDIDATES`].
fn edge_directions(vertices: &[[f64; 3]], triangles: &[BrepTriangle]) -> Option<Vec<[f64; 3]>> {
    let mut found: Vec<[f64; 3]> = Vec::new();
    for BrepTriangle(a, b, c) in triangles {
        let (Some(&p), Some(&q), Some(&r)) = (
            vertices.get(*a as usize),
            vertices.get(*b as usize),
            vertices.get(*c as usize),
        ) else {
            return None;
        };
        for (from, to) in [(p, q), (q, r), (r, p)] {
            let edge = sub(to, from);
            let length = dot(edge, edge).sqrt();
            if !(length.is_finite() && length > 0.0) {
                continue;
            }
            let unit = edge.map(|component| component / length);
            if found
                .iter()
                .any(|known| dot(*known, unit).abs() > 1.0 - EDGE_COSINE)
            {
                continue;
            }
            if found.len() == MAX_AXIS_CANDIDATES {
                return None;
            }
            found.push(unit);
        }
    }
    Some(found)
}

/// A mesh read as the prism of one section: the axis it runs along, the prism
/// and the end area and the volume that make it one (RE-423, RE-424).
struct FlatPrism {
    axis: [f64; 3],
    prism: Prism,
    end_area: f64,
    volume: f64,
}

/// The longest axis a mesh is a prism along with two flat ends of one area, where
/// the mesh is one closed solid with no hole through it and its volume is the end
/// area times the extent; None where it is not, or where two axes of one extent
/// are (a cube).
fn longest_flat_prism(vertices: &[[f64; 3]], triangles: &[BrepTriangle]) -> Option<FlatPrism> {
    let closed = closed_mesh(vertices, triangles)?;
    // One piece, and a solid with no hole through it has a sphere's Euler
    // characteristic, 2: vertices minus edges plus faces, and a closed triangle
    // mesh has one and a half edges to a face.
    if closed.components != 1 || 2 * closed.vertices as i64 - closed.triangles as i64 != 4 {
        return None;
    }
    let mut best: Option<([f64; 3], Prism)> = None;
    let mut tied = false;
    for axis in edge_directions(vertices, triangles)? {
        let Some(prism) = prism_along(vertices, triangles, axis) else {
            continue;
        };
        let [a, b] = prism.ends;
        let same = (a.area - b.area).abs() <= END_AREA_TOLERANCE * a.area.max(b.area);
        if !(a.square && b.square && same && prism.extent > 0.0) {
            continue;
        }
        if let Some((held_axis, held)) = &best {
            if (prism.extent - held.extent).abs() <= EXTENT_TIE * prism.extent.max(held.extent) {
                tied |= dot(*held_axis, axis).abs() < 0.999;
                continue;
            }
            if prism.extent < held.extent {
                continue;
            }
        }
        best = Some((axis, prism));
        tied = false;
    }
    let (axis, prism) = best?;
    if tied {
        return None;
    }
    let end_area = (prism.ends[0].area + prism.ends[1].area) / 2.0;
    if (closed.volume - end_area * prism.extent).abs() > PRISM_VOLUME_TOLERANCE * closed.volume {
        return None;
    }
    Some(FlatPrism {
        axis,
        prism,
        end_area,
        volume: closed.volume,
    })
}

/// The base quantities of a beam from the mesh of its body (RE-423): the
/// section, the area along it and the volume of the prism the solid is, where it
/// is one. Revit's export draws a beam as an extrusion of its profile and takes
/// the quantities from it (revit-ifc's `CreateBeamColumnBaseQuantities`,
/// CLEANROOM.md source 5): `CrossSectionArea` is the profile's area,
/// `OuterSurfaceArea` its perimeter times the depth, `GrossSurfaceArea` and
/// `NetSurfaceArea` that and the two ends, `GrossVolume` and `NetVolume` the
/// solid's. The extrusion runs along the longest axis the mesh is a prism along
/// with two flat ends: on all 142 beams of the BIM4LCA housing and office files
/// the scorer reads, Revit's extrusion is that axis, which is the beam's line
/// where its ends are square and across it where they are cut.
///
/// None where the mesh is not one closed solid with no hole through it (a
/// hollow section's surfaces are outer and inner, and the outer perimeter is not
/// told from the mesh), where no axis makes it a prism with two flat ends of one
/// area, where two axes of one extent do (a cube), or where its volume is not the
/// end area times the extent.
pub(super) fn beam_quantities(
    vertices: &[[f64; 3]],
    triangles: &[BrepTriangle],
) -> Option<BeamQuantities> {
    let found = longest_flat_prism(vertices, triangles)?;
    Some(BeamQuantities {
        cross_section_area: found.end_area,
        outer_surface_area: found.prism.lateral,
        surface_area: found.prism.lateral + 2.0 * found.end_area,
        volume: found.volume,
    })
}

/// The base quantities of a column from the mesh of its body (RE-424): the
/// depth of the extrusion, the section, the area along it and the volume, where
/// the mesh is the prism rule's (see [`beam_quantities`]) and its axis is the
/// vertical. Revit's export draws a column as an extrusion of its profile and
/// takes the same quantities from it as a beam's (revit-ifc's
/// `CreateBeamColumnBaseQuantities`, CLEANROOM.md source 5), unless a slab cuts
/// the column where it meets it: then it draws a cut mesh, which has a
/// `CrossSectionArea` that is half its surface and a `NetVolume`, and no
/// `Length`, `OuterSurfaceArea` or `GrossVolume` (RE-291, RE-353). On the
/// BIM4LCA office, housing and timber office, the longest axis the mesh of an
/// extruded column is a prism along is the vertical on every one, and the
/// longest such axis of 14 of the 18 cut columns is a horizontal one, the other
/// 4 being no prism at all, so a vertical axis is what tells the two apart
/// where a slab's cut is not square across the column. A column lying down, a
/// stub column wider than it is tall and one with two axes of one extent have
/// another axis for their longest and get none.
pub(super) fn column_quantities(
    vertices: &[[f64; 3]],
    triangles: &[BrepTriangle],
) -> Option<ColumnQuantities> {
    let found = longest_flat_prism(vertices, triangles)?;
    if found.axis[2].abs() < SQUARE_COSINE {
        return None;
    }
    Some(ColumnQuantities {
        length: found.prism.extent,
        cross_section_area: found.end_area,
        outer_surface_area: found.prism.lateral,
        volume: found.volume,
    })
}

/// Where a curtain mullion stands, for the test of whether another continues
/// it (RE-404): the unit axis it runs along, the centres of its two end
/// faces, in one frame for all of them, feet, and the areas of those faces,
/// square feet (RE-436).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placed {
    pub axis: [f64; 3],
    pub ends: [[f64; 3]; 2],
    pub areas: [f64; 2],
}

/// How close, feet, an end must be to another mullion's end for the other to be
/// its partner (about 1 cm): the two meet end to end.
const PARTNER_FEET: f64 = 0.033;

/// How far, as a fraction of the larger, the areas of the two end faces that meet
/// may differ and the ends still be of one section (RE-436): a 50 x 50 mm
/// mullion's face is a third of a 150 x 50 mm one's, and a mitre the two pieces
/// share changes both alike.
const SAME_SECTION_AREA: f64 = 1e-3;

/// The cosine of the angle between two axes over which the mullions coincide
/// (about a quarter of a degree, RE-432): a continuation bent off the axis by
/// two degrees leaves the end face counted in Revit's value, on all 159 of the
/// BIM4LCA office's mullions that have one, while every partner that hides an
/// end is within 0.01 degrees.
const PARTNER_COSINE: f64 = 0.99999;

/// For each mullion of `placed`, at each of its two ends, whether another
/// mullion continues it there (RE-404): one that runs the same way, has an end
/// within [`PARTNER_FEET`] of the end, and whose body lies beyond it. Revit's
/// solid hides an end that meets its continuation's end face (RE-268), and the
/// count of kept ends RE-268 reads from the boxes of one curtain wall misses a
/// continuation on another wall: on the BIM4LCA timber office at permit it
/// counted both ends of four vertical mullions that a piece of the same section
/// stands on, and Revit's value counts one.
pub(super) fn partner_ends(placed: &[Placed]) -> Vec<[bool; 2]> {
    const CELL: f64 = 0.1;
    // A curtain grid node holds a few mullion ends in one cell; a crafted file
    // put every end in one, and each end then tested all of them, n² (#1845).
    // A cell holding more is no evidence: its ends are continued by none.
    const MAX_ENDS_PER_CELL: usize = 64;
    let cell = |point: [f64; 3]| point.map(|coordinate| (coordinate / CELL).floor() as i64);
    let mut grid: HashMap<[i64; 3], Vec<(usize, usize)>> = HashMap::new();
    for (index, mullion) in placed.iter().enumerate() {
        for (end, point) in mullion.ends.iter().enumerate() {
            grid.entry(cell(*point)).or_default().push((index, end));
        }
    }
    if grid.values().any(|ends| ends.len() > MAX_ENDS_PER_CELL) {
        crate::budget::stopped("mullion_end_cells");
    }
    placed
        .iter()
        .enumerate()
        .map(|(index, mullion)| {
            [0, 1].map(|end| {
                let point = mullion.ends[end];
                let away = sub(point, mullion.ends[1 - end]);
                let norm = dot(away, away).sqrt();
                if !(norm.is_finite() && norm > 0.0) {
                    return false;
                }
                let outward = away.map(|component| component / norm);
                let [cx, cy, cz] = cell(point);
                (-1..=1).any(|dx| {
                    (-1..=1).any(|dy| {
                        (-1..=1).any(|dz| {
                            grid.get(&[
                                cx.saturating_add(dx),
                                cy.saturating_add(dy),
                                cz.saturating_add(dz),
                            ])
                            .filter(|ends| ends.len() <= MAX_ENDS_PER_CELL)
                            .into_iter()
                            .flatten()
                            .any(|&(other, other_end)| {
                                let neighbour = &placed[other];
                                let near = sub(neighbour.ends[other_end], point);
                                let beyond = sub(neighbour.ends[1 - other_end], point);
                                other != index
                                    && dot(mullion.axis, neighbour.axis).abs() > PARTNER_COSINE
                                    && (mullion.areas[end] - neighbour.areas[other_end]).abs()
                                        <= SAME_SECTION_AREA
                                            * mullion.areas[end].max(neighbour.areas[other_end])
                                    && dot(near, near).sqrt() < PARTNER_FEET
                                    && dot(beyond, outward) > PARTNER_FEET
                            })
                        })
                    })
                })
            })
        })
        .collect()
}

/// How far apart, along its axis, a mullion's two ends must be, feet, to be told
/// apart by the side of the axis each lies on.
const MIN_SPAN_FEET: f64 = 1e-6;

/// How many of the ends of a mullion that have a partner in `partners`
/// ([`partner_ends`]) are not an end its curtain wall's boxes kept with a partner
/// they saw (`seen`: the end at the `-axis` side of the box, then the one at its
/// `+axis` side, RE-439): each is a continuation the boxes missed, and the count
/// of kept ends they read is wrong at it. The mesh's two ends come in the order
/// its planes were found, not along the axis, so each is told by which side of the
/// other it lies along `placed.axis`, the axis the boxes' ends are told by; two
/// ends with no length between them are not told apart, and each partner is
/// counted as missed.
pub(super) fn partners_the_boxes_missed(
    placed: &Placed,
    partners: [bool; 2],
    seen: [bool; 2],
) -> usize {
    let along = dot(sub(placed.ends[1], placed.ends[0]), placed.axis);
    [0usize, 1]
        .into_iter()
        .filter(|&end| {
            let side = if along > MIN_SPAN_FEET {
                Some(end)
            } else if along < -MIN_SPAN_FEET {
                Some(1 - end)
            } else {
                None
            };
            partners[end] && !side.is_some_and(|side| seen[side])
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The unit cube's corners, bottom (z 0) then top, and its twelve
    /// triangles wound counter-clockwise seen from outside.
    fn cube() -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let faces = [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let triangles = faces
            .iter()
            .map(|[a, b, c]| BrepTriangle(*a, *b, *c))
            .collect();
        (vertices, triangles)
    }

    #[test]
    fn a_cube_has_its_volume() {
        let (vertices, triangles) = cube();
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        assert!((volume - 1.0).abs() < 1e-12, "{volume}");
    }

    #[test]
    fn a_cube_wound_inwards_has_the_same_volume() {
        let (vertices, mut triangles) = cube();
        for triangle in &mut triangles {
            *triangle = BrepTriangle(triangle.0, triangle.2, triangle.1);
        }
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        assert!((volume - 1.0).abs() < 1e-12, "{volume}");
    }

    #[test]
    fn faces_that_each_hold_their_own_vertices_are_welded() {
        let (corners, triangles) = cube();
        let mut vertices = Vec::new();
        let mut separate = Vec::new();
        for BrepTriangle(a, b, c) in &triangles {
            let base = vertices.len() as u32;
            vertices.extend([
                corners[*a as usize],
                corners[*b as usize],
                corners[*c as usize],
            ]);
            separate.push(BrepTriangle(base, base + 1, base + 2));
        }
        let volume = closed_mesh_volume(&vertices, &separate).unwrap();
        assert!((volume - 1.0).abs() < 1e-12, "{volume}");
    }

    #[test]
    fn a_missing_triangle_leaves_no_volume() {
        let (vertices, mut triangles) = cube();
        triangles.pop();
        assert_eq!(closed_mesh_volume(&vertices, &triangles), None);
    }

    #[test]
    fn one_triangle_wound_the_other_way_leaves_no_volume() {
        let (vertices, mut triangles) = cube();
        let first = triangles[0];
        triangles[0] = BrepTriangle(first.0, first.2, first.1);
        assert_eq!(closed_mesh_volume(&vertices, &triangles), None);
    }

    #[test]
    fn a_repeated_triangle_leaves_no_volume() {
        let (vertices, mut triangles) = cube();
        triangles.push(triangles[0]);
        assert_eq!(closed_mesh_volume(&vertices, &triangles), None);
    }

    #[test]
    fn a_vertex_that_is_not_there_or_not_finite_leaves_no_volume() {
        let (mut vertices, mut triangles) = cube();
        triangles.push(BrepTriangle(0, 1, 99));
        assert_eq!(closed_mesh_volume(&vertices, &triangles), None);
        triangles.pop();
        vertices[3][2] = f64::NAN;
        assert_eq!(closed_mesh_volume(&vertices, &triangles), None);
    }

    #[test]
    fn nothing_leaves_no_volume() {
        assert_eq!(closed_mesh_volume(&[], &[]), None);
        let (vertices, _) = cube();
        assert_eq!(closed_mesh_volume(&vertices, &[]), None);
    }

    fn face(normals: Vec<[f64; 3]>) -> Primitive {
        Primitive {
            source_owner_id: None,
            object_index: 0,
            face_tag: 0,
            render_style_id: 0,
            material_id: None,
            vertices: vec![[0.0; 3]; normals.len()],
            normals,
            triangles: Vec::new(),
            hidden: false,
            filtered: false,
        }
    }

    #[test]
    fn a_face_is_flat_only_with_one_normal_on_every_vertex() {
        let up = [0.0, 0.0, 1.0];
        assert!(all_faces_flat(&[
            face(vec![up; 4]),
            face(vec![[1.0, 0.0, 0.0]; 3])
        ]));
        assert!(!all_faces_flat(&[face(vec![up, up, [0.0, 0.1, 0.995]])]));
        assert!(!all_faces_flat(&[face(vec![up; 4]), face(Vec::new())]));
        assert!(!all_faces_flat(&[]));
    }

    /// The twelve triangles of a box with corner (x0, y0, z0) and sides dx, dy, dz.
    fn box_mesh(origin: [f64; 3], size: [f64; 3]) -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let (mut vertices, triangles) = cube();
        for vertex in &mut vertices {
            for (coordinate, (start, extent)) in vertex.iter_mut().zip(origin.iter().zip(&size)) {
                *coordinate = start + *coordinate * extent;
            }
        }
        (vertices, triangles)
    }

    #[test]
    fn a_vertical_box_has_its_longer_horizontal_side_for_width() {
        let (vertices, triangles) = box_mesh([10.0, 20.0, 5.0], [0.082, 4.9, 7.9]);
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        let width = box_plan_width(&vertices, &triangles, volume).unwrap();
        assert!((width - 4.9).abs() < 1e-9, "{width}");
        let (vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [7.9, 0.082, 4.9]);
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        let width = box_plan_width(&vertices, &triangles, volume).unwrap();
        assert!((width - 7.9).abs() < 1e-9, "{width}");
    }

    #[test]
    fn a_flat_box_has_no_width() {
        let (vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [4.9, 7.9, 0.082]);
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        assert_eq!(box_plan_width(&vertices, &triangles, volume), None);
    }

    #[test]
    fn a_mesh_that_is_not_a_box_has_no_width() {
        // One top corner of a vertical box pulled in: a closed mesh about 1% short of its box.
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [0.082, 4.9, 7.9]);
        vertices[6] = [0.082, 4.9 - 0.4, 7.9];
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        assert_eq!(box_plan_width(&vertices, &triangles, volume), None);
    }

    #[test]
    fn a_point_no_triangle_uses_is_not_part_of_the_box() {
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [0.082, 4.9, 7.9]);
        let volume = closed_mesh_volume(&vertices, &triangles).unwrap();
        vertices.push([500.0, 500.0, 500.0]);
        let width = box_plan_width(&vertices, &triangles, volume).unwrap();
        assert!((width - 4.9).abs() < 1e-9, "{width}");
    }

    #[test]
    fn a_triangle_that_points_outside_the_vertices_has_no_width() {
        let (vertices, mut triangles) = box_mesh([0.0, 0.0, 0.0], [0.082, 4.9, 7.9]);
        triangles.push(BrepTriangle(0, 1, 99));
        assert_eq!(box_plan_width(&vertices, &triangles, 1.0), None);
    }

    /// A rectangular section 1 by 2 ft along z, 10 ft long at x 0 and cut at
    /// 45 degrees at the top, so 11 ft long at x 1.
    fn mitred_prism() -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [1.0, 2.0, 10.0]);
        // The cube's top corners 5 (1, 0) and 6 (1, 1) in x, y rise by the x.
        vertices[5][2] = 11.0;
        vertices[6][2] = 11.0;
        (vertices, triangles)
    }

    /// The triangles of a closed polygon `corners` of one plane, as a fan.
    fn fan(corners: &[u32]) -> Vec<BrepTriangle> {
        (1..corners.len() - 1)
            .map(|i| BrepTriangle(corners[0], corners[i], corners[i + 1]))
            .collect()
    }

    /// A 0.15 by 0.05 ft section along z, 0.19 ft long at its two sides and
    /// 0.2 ft along its ridge: a flat bottom and a gabled top, which is two
    /// planes (a mullion piece cut across by a pair of mitres). It is a prism
    /// along x, as its section's long side, and along no other axis.
    fn gabled_piece() -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [0.15, 0.0, 0.0],
            [0.15, 0.05, 0.0],
            [0.0, 0.05, 0.0],
            [0.0, 0.0, 0.19],
            [0.15, 0.0, 0.19],
            [0.15, 0.025, 0.2],
            [0.0, 0.025, 0.2],
            [0.0, 0.05, 0.19],
            [0.15, 0.05, 0.19],
        ];
        let mut triangles = Vec::new();
        for face in [
            &[0, 3, 2, 1][..],
            &[0, 1, 5, 4],
            &[3, 8, 9, 2],
            &[0, 4, 7, 8, 3],
            &[1, 2, 9, 6, 5],
            &[4, 5, 6, 7],
            &[7, 6, 9, 8],
        ] {
            triangles.extend(fan(face));
        }
        (vertices, triangles)
    }

    /// The extent of the prism a mesh is along an axis, where it is one.
    fn extent_of(vertices: &[[f64; 3]], triangles: &[BrepTriangle], axis: [f64; 3]) -> Option<f64> {
        prism_along(vertices, triangles, axis).map(|prism| prism.extent)
    }

    fn assert_close(found: f64, wanted: f64) {
        assert!((found - wanted).abs() < 1e-9, "{found} not {wanted}");
    }

    #[test]
    fn a_box_is_a_prism_along_each_of_its_three_axes() {
        let (vertices, triangles) = box_mesh([10.0, 20.0, 5.0], [0.05, 0.1, 3.0]);
        let along = |axis| extent_of(&vertices, &triangles, axis).unwrap();
        assert_close(along([1.0, 0.0, 0.0]), 0.05);
        assert_close(along([0.0, 1.0, 0.0]), 0.1);
        assert_close(along([0.0, 0.0, 1.0]), 3.0);
        // The direction's sign and length do not matter.
        assert_close(along([0.0, 0.0, -2.0]), 3.0);
    }

    #[test]
    fn a_mitred_end_counts_at_its_longer_side() {
        let (vertices, triangles) = mitred_prism();
        assert_close(
            extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).unwrap(),
            11.0,
        );
        // It is a prism along y as well, and along no side of its section in x.
        assert_close(
            extent_of(&vertices, &triangles, [0.0, 1.0, 0.0]).unwrap(),
            2.0,
        );
        assert!(extent_of(&vertices, &triangles, [1.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn a_gabled_end_is_no_prism_along_the_length_but_is_along_a_side_of_the_section() {
        // The mullion of a Revit sample whose written length was taken from its
        // section's side where the axis was found from the mesh: along its own axis
        // three planes are not parallel to it (the bottom and the two of the gable).
        let (vertices, triangles) = gabled_piece();
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).is_none());
        assert_close(
            extent_of(&vertices, &triangles, [1.0, 0.0, 0.0]).unwrap(),
            0.15,
        );
        assert!(extent_of(&vertices, &triangles, [0.0, 1.0, 0.0]).is_none());
    }

    /// A prism of `sides` sides of radius 0.1 ft and 3 ft long along z.
    fn polygon_prism(sides: usize) -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let mut vertices = Vec::new();
        for z in [0.0, 3.0] {
            for k in 0..sides {
                let angle = std::f64::consts::TAU * k as f64 / sides as f64;
                vertices.push([0.1 * angle.cos(), 0.1 * angle.sin(), z]);
            }
        }
        let mut triangles = Vec::new();
        for k in 0..sides {
            let next = (k + 1) % sides;
            let corners = [k, next, sides + next, sides + k].map(|i| i as u32);
            triangles.extend(fan(&corners));
        }
        triangles.extend(fan(&(0..sides as u32).rev().collect::<Vec<_>>()));
        triangles.extend(fan(&(sides as u32..2 * sides as u32).collect::<Vec<_>>()));
        (vertices, triangles)
    }

    #[test]
    fn a_mesh_with_more_planes_than_a_section_has_is_not_read() {
        // A 40-sided prism, as a bent member's many planes are: the planes are
        // counted as the triangles are read, and past the cap nothing is made.
        let (vertices, triangles) = polygon_prism(40);
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).is_none());
        // One of eleven sides, the most a measured mullion has, is read.
        let (vertices, triangles) = polygon_prism(11);
        assert_close(
            extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).unwrap(),
            3.0,
        );
    }

    #[test]
    fn no_axis_a_missing_vertex_or_no_triangle_leaves_no_extent() {
        let (mut vertices, mut triangles) = box_mesh([0.0, 0.0, 0.0], [1.0, 2.0, 10.0]);
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, 0.0]).is_none());
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, f64::NAN]).is_none());
        assert!(extent_of(&vertices, &[], [0.0, 0.0, 1.0]).is_none());
        assert!(extent_of(&[], &[], [0.0, 0.0, 1.0]).is_none());
        triangles.push(BrepTriangle(0, 1, 99));
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).is_none());
        triangles.pop();
        vertices[3][2] = f64::NAN;
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).is_none());
    }

    #[test]
    fn a_triangle_with_no_area_adds_no_plane_and_no_corner() {
        let (mut vertices, mut triangles) = box_mesh([0.0, 0.0, 0.0], [0.05, 0.1, 3.0]);
        vertices.push([40.0, 40.0, 40.0]);
        vertices.push([41.0, 41.0, 41.0]);
        vertices.push([42.0, 42.0, 42.0]);
        triangles.push(BrepTriangle(8, 9, 10));
        assert_close(
            extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).unwrap(),
            3.0,
        );
    }

    #[test]
    fn corners_off_by_single_precision_noise_still_make_one_plane_a_face() {
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [0.05, 0.1, 3.0]);
        for (index, vertex) in vertices.iter_mut().enumerate() {
            for (axis, coordinate) in vertex.iter_mut().enumerate() {
                *coordinate += 1e-6 * (((index * 7 + axis * 3) % 5) as f64 - 2.0);
            }
        }
        // The axis is the record's exact one, so the extent is read to the noise.
        let found = extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).unwrap();
        assert!((found - 3.0).abs() < 1e-5, "{found}");
    }

    #[test]
    fn a_top_corner_pulled_in_leaves_the_height_no_prism() {
        // One top corner of the box pulled in 0.4 ft: the top and the side it
        // sits on are no longer planes, so more than two planes are off the axis.
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [0.082, 4.9, 7.9]);
        vertices[6] = [0.082, 4.9 - 0.4, 7.9];
        assert!(extent_of(&vertices, &triangles, [0.0, 0.0, 1.0]).is_none());
    }

    #[test]
    fn a_prism_reports_the_areas_and_the_centres_of_its_faces() {
        let (vertices, triangles) = box_mesh([10.0, 20.0, 5.0], [0.05, 0.1, 3.0]);
        let z = prism_along(&vertices, &triangles, [0.0, 0.0, 1.0]).unwrap();
        assert_close(z.lateral, 2.0 * (0.05 * 3.0 + 0.1 * 3.0));
        let mut heights = Vec::new();
        for end in z.ends {
            assert!(end.square);
            assert_close(end.area, 0.05 * 0.1);
            assert_close(end.centre[0], 10.025);
            assert_close(end.centre[1], 20.05);
            heights.push(end.centre[2]);
        }
        heights.sort_by(f64::total_cmp);
        assert_close(heights[0], 5.0);
        assert_close(heights[1], 8.0);
    }

    #[test]
    fn a_mitred_prism_reports_its_mitre_face_apart_from_its_flat_end() {
        let (vertices, triangles) = mitred_prism();
        let z = prism_along(&vertices, &triangles, [0.0, 0.0, 1.0]).unwrap();
        // Two sides of 10 and 11 ft by 2 ft, and two trapezoids of 10.5 ft by 1 ft.
        assert_close(z.lateral, 2.0 * 10.0 + 2.0 * 11.0 + 2.0 * 10.5);
        let flat: Vec<_> = z.ends.iter().filter(|end| end.square).collect();
        let mitre: Vec<_> = z.ends.iter().filter(|end| !end.square).collect();
        assert_eq!((flat.len(), mitre.len()), (1, 1));
        assert_close(flat[0].area, 2.0);
        assert_close(mitre[0].area, 2.0 * 2.0_f64.sqrt());
    }

    fn prism(lateral: f64, ends: [(bool, f64); 2]) -> Prism {
        Prism {
            extent: 1.0,
            lateral,
            ends: ends.map(|(square, area)| PrismEnd {
                square,
                area,
                centre: [0.0; 3],
            }),
        }
    }

    #[test]
    fn two_flat_ends_count_as_many_sections_as_are_kept() {
        let flat = prism(0.9, [(true, 0.005), (true, 0.005)]);
        assert_close(mullion_surface_area(&flat, 0).unwrap(), 0.9);
        assert_close(mullion_surface_area(&flat, 1).unwrap(), 0.905);
        assert_close(mullion_surface_area(&flat, 2).unwrap(), 0.91);
        assert!(mullion_surface_area(&flat, 3).is_none());
    }

    #[test]
    fn two_flat_ends_of_different_areas_are_not_one_section() {
        let uneven = prism(0.9, [(true, 0.005), (true, 0.0051)]);
        assert!(mullion_surface_area(&uneven, 1).is_none());
    }

    #[test]
    fn a_mitred_end_always_counts_and_the_flat_end_where_both_are_kept() {
        let mitred = prism(0.2, [(true, 0.005), (false, 0.007)]);
        assert!(mullion_surface_area(&mitred, 0).is_none());
        assert_close(mullion_surface_area(&mitred, 1).unwrap(), 0.207);
        assert_close(mullion_surface_area(&mitred, 2).unwrap(), 0.212);
        let swapped = prism(0.2, [(false, 0.007), (true, 0.005)]);
        assert_close(mullion_surface_area(&swapped, 2).unwrap(), 0.212);
    }

    #[test]
    fn two_mitred_ends_count_where_both_are_kept_and_are_not_read_otherwise() {
        let two_mitres = prism(0.2, [(false, 0.007), (false, 0.0071)]);
        // A mitred end always counts, so a mullion with two of them and an end
        // hidden contradicts it.
        assert!(mullion_surface_area(&two_mitres, 0).is_none());
        assert!(mullion_surface_area(&two_mitres, 1).is_none());
        assert_close(mullion_surface_area(&two_mitres, 2).unwrap(), 0.2141);
        // A flat end and a mitred one with no end kept is none, as before.
        let one_flat = prism(0.2, [(true, 0.005), (false, 0.007)]);
        assert!(mullion_surface_area(&one_flat, 0).is_none());
    }

    fn placed(axis: [f64; 3], from: [f64; 3], to: [f64; 3]) -> Placed {
        Placed {
            axis,
            ends: [from, to],
            areas: [0.0025; 2],
        }
    }

    #[test]
    fn a_piece_standing_end_to_end_on_a_mullion_is_its_partner_at_that_end() {
        let lower = placed([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 2.1]);
        let upper = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.0, 0.0, 2.5]);
        assert_eq!(
            partner_ends(&[lower, upper]),
            vec![[false, true], [true, false]]
        );
        // The order of the pair's ends and of the pieces does not matter.
        let upper = placed([0.0, 0.0, -1.0], [0.0, 0.0, 2.5], [0.0, 0.0, 2.1]);
        assert_eq!(
            partner_ends(&[upper, lower]),
            vec![[false, true], [false, true]]
        );
    }

    #[test]
    fn pieces_that_overlap_run_across_or_leave_a_gap_are_no_partners() {
        let lower = placed([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 2.1]);
        // Meets the end but lies back over the mullion: overlapped, not continued.
        let over = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.0, 0.0, 1.0]);
        assert_eq!(partner_ends(&[lower, over]), vec![[false, false]; 2]);
        // Meets the end across it.
        let across = placed([1.0, 0.0, 0.0], [0.0, 0.0, 2.1], [0.4, 0.0, 2.1]);
        assert_eq!(partner_ends(&[lower, across]), vec![[false, false]; 2]);
        // Stands 10 cm above the end.
        let gap = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.2], [0.0, 0.0, 2.6]);
        assert_eq!(partner_ends(&[lower, gap]), vec![[false, false]; 2]);
    }

    #[test]
    fn a_continuation_bent_off_the_axis_is_no_partner_but_a_hair_off_it_is() {
        let lower = placed([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 2.1]);
        let continuation = |degrees: f64| {
            let angle = degrees.to_radians();
            let axis = [angle.sin(), 0.0, angle.cos()];
            placed(
                axis,
                [0.0, 0.0, 2.1],
                [0.4 * axis[0], 0.0, 2.1 + 0.4 * axis[2]],
            )
        };
        // Two degrees off the axis (cosine 0.9994): it covers none of the end face.
        assert_eq!(
            partner_ends(&[lower, continuation(2.0)]),
            vec![[false, false]; 2]
        );
        // A hundredth of a degree off it still meets the end face to face.
        assert_eq!(
            partner_ends(&[lower, continuation(0.01)]),
            vec![[false, true], [true, false]]
        );
    }

    #[test]
    fn a_continuation_of_another_section_is_no_partner_but_one_of_the_same_is() {
        let lower = placed([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 2.1]);
        let upper = |area: f64| Placed {
            areas: [area; 2],
            ..placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.0, 0.0, 2.5])
        };
        // A 150 x 50 mm face (0.0075) on a 50 x 50 mm one (0.0025): not the same end face.
        assert_eq!(
            partner_ends(&[lower, upper(0.0075)]),
            vec![[false, false]; 2]
        );
        assert_eq!(
            partner_ends(&[lower, upper(0.0025)]),
            vec![[false, true], [true, false]]
        );
        // The same section with a face a hundredth of a percent different (a shared mitre).
        assert_eq!(
            partner_ends(&[lower, upper(0.002_500_2)]),
            vec![[false, true], [true, false]]
        );
    }

    // How reading a partner as one the boxes saw can go wrong (RE-439): a partner at
    // an end the boxes kept with none is taken for theirs because another end has
    // one (a count cannot tell), or the mesh's ends are matched to the boxes' by
    // the order they come in, which is the order their planes were found.
    #[test]
    fn a_partner_at_an_end_the_boxes_kept_with_one_is_no_continuation_they_missed() {
        let lower = placed([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 2.1]);
        // The boxes kept the upper end with a partner: the mesh's there is theirs.
        assert_eq!(
            partners_the_boxes_missed(&lower, [false, true], [false, true]),
            0
        );
        // A partner at the end they kept with none is one they missed, though they
        // saw one at the other end: the counts of partners alone are equal.
        assert_eq!(
            partners_the_boxes_missed(&lower, [true, false], [false, true]),
            1
        );
        // Both ends have one in the mesh and the boxes saw the upper only.
        assert_eq!(
            partners_the_boxes_missed(&lower, [true, true], [false, true]),
            1
        );
        // None in the mesh, none missed, whatever the boxes saw.
        assert_eq!(
            partners_the_boxes_missed(&lower, [false, false], [true, true]),
            0
        );
    }

    #[test]
    fn the_meshs_ends_are_told_by_the_axis_and_not_by_the_order_they_come_in() {
        // The mesh lists the upper end first.
        let reversed = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.0, 0.0, 0.0]);
        // Its partner is at the upper end, which the boxes kept with one (their
        // `+axis` end), and then at the lower end, which they did not.
        assert_eq!(
            partners_the_boxes_missed(&reversed, [true, false], [false, true]),
            0
        );
        assert_eq!(
            partners_the_boxes_missed(&reversed, [true, false], [true, false]),
            1
        );
        // An axis that runs the other way turns the boxes' sides over.
        let flipped = placed([0.0, 0.0, -1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 2.1]);
        assert_eq!(
            partners_the_boxes_missed(&flipped, [false, true], [true, false]),
            0
        );
        assert_eq!(
            partners_the_boxes_missed(&flipped, [false, true], [false, true]),
            1
        );
    }

    #[test]
    fn ends_with_no_length_between_them_are_not_told_apart_and_a_partner_is_missed() {
        let point = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.0, 0.0, 2.1]);
        assert_eq!(
            partners_the_boxes_missed(&point, [true, false], [true, true]),
            1
        );
        // Ends across the axis, none along it, are no better.
        let across = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.4, 0.0, 2.1]);
        assert_eq!(
            partners_the_boxes_missed(&across, [false, true], [true, true]),
            1
        );
    }

    #[test]
    fn a_partner_is_found_across_a_cell_boundary_and_a_mullion_of_no_length_has_none() {
        let lower = placed([0.0, 0.0, 1.0], [0.0, 0.0, -1.0], [0.0, 0.0, 0.0999]);
        let upper = placed([0.0, 0.0, 1.0], [0.0, 0.0, 0.1005], [0.0, 0.0, 1.0]);
        assert_eq!(
            partner_ends(&[lower, upper]),
            vec![[false, true], [true, false]]
        );
        let point = placed([0.0, 0.0, 1.0], [0.0, 0.0, 2.1], [0.0, 0.0, 2.1]);
        assert_eq!(partner_ends(&[point, upper]), vec![[false, false]; 2]);
        assert!(partner_ends(&[]).is_empty());
    }

    /// A parallelogram side profile 4 ft along x and 0.2 ft high, its ends slanted
    /// by 1 ft, extruded 0.05 ft along y: a beam modelled as the side of its
    /// section, with cut ends. It is a prism along y with two flat ends and along
    /// x with two slanted ones.
    fn slanted_beam() -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [5.0, 0.0, 0.2],
            [1.0, 0.0, 0.2],
            [0.0, 0.05, 0.0],
            [4.0, 0.05, 0.0],
            [5.0, 0.05, 0.2],
            [1.0, 0.05, 0.2],
        ];
        let mut triangles = Vec::new();
        for face in [
            &[0, 1, 2, 3][..],
            &[7, 6, 5, 4],
            &[0, 4, 5, 1],
            &[3, 2, 6, 7],
            &[1, 5, 6, 2],
            &[0, 3, 7, 4],
        ] {
            triangles.extend(fan(face));
        }
        (vertices, triangles)
    }

    #[test]
    fn a_box_beam_takes_the_sums_of_its_longest_axis_whichever_way_it_lies() {
        for size in [[0.1, 0.2, 3.0], [3.0, 0.1, 0.2], [0.2, 3.0, 0.1]] {
            let (vertices, triangles) = box_mesh([1.0, 2.0, 3.0], size);
            let found = beam_quantities(&vertices, &triangles).unwrap();
            assert_close(found.cross_section_area, 0.02);
            assert_close(found.outer_surface_area, 2.0 * (0.1 + 0.2) * 3.0);
            assert_close(found.surface_area, 2.0 * (0.1 + 0.2) * 3.0 + 0.04);
            assert_close(found.volume, 0.06);
        }
    }

    #[test]
    fn a_beam_with_slanted_ends_is_read_across_them() {
        let (vertices, triangles) = slanted_beam();
        let found = beam_quantities(&vertices, &triangles).unwrap();
        let perimeter = 8.0 + 2.0 * 1.04_f64.sqrt();
        assert_close(found.cross_section_area, 0.8);
        assert_close(found.outer_surface_area, perimeter * 0.05);
        assert_close(found.surface_area, perimeter * 0.05 + 1.6);
        assert_close(found.volume, 0.04);
    }

    #[test]
    fn a_cube_has_three_axes_of_one_extent_and_is_no_beam() {
        let (vertices, triangles) = cube();
        assert_eq!(beam_quantities(&vertices, &triangles), None);
    }

    #[test]
    fn a_mesh_that_is_not_closed_is_no_beam() {
        let (vertices, mut triangles) = box_mesh([0.0; 3], [0.1, 0.2, 3.0]);
        triangles.pop();
        assert_eq!(beam_quantities(&vertices, &triangles), None);
    }

    #[test]
    fn two_solids_in_one_mesh_are_no_beam() {
        let (mut vertices, mut triangles) = box_mesh([0.0; 3], [0.1, 0.2, 3.0]);
        let (more, extra) = box_mesh([5.0, 0.0, 0.0], [0.1, 0.2, 3.0]);
        let shift = u32::try_from(vertices.len()).unwrap();
        vertices.extend(more);
        triangles.extend(
            extra
                .into_iter()
                .map(|BrepTriangle(a, b, c)| BrepTriangle(a + shift, b + shift, c + shift)),
        );
        assert_eq!(beam_quantities(&vertices, &triangles), None);
    }

    /// A square ring of flat faces, 3 ft high, 0.4 ft across outside and 0.2 inside,
    /// centred on (cx, 0): a surface of Euler characteristic 0.
    fn ring_mesh(cx: f64) -> (Vec<[f64; 3]>, Vec<BrepTriangle>) {
        let mut vertices = Vec::new();
        for (half, z) in [(0.2, 0.0), (0.2, 3.0), (0.1, 0.0), (0.1, 3.0)] {
            for (x, y) in [(-half, -half), (half, -half), (half, half), (-half, half)] {
                vertices.push([cx + x, y, z]);
            }
        }
        // Outer bottom 0 to 3, outer top 4 to 7, inner bottom 8 to 11, inner top
        // 12 to 15, each corner ring counter-clockwise from above.
        let mut triangles = Vec::new();
        for i in 0..4u32 {
            let j = (i + 1) % 4;
            triangles.extend(fan(&[i, j, 4 + j, 4 + i]));
            triangles.extend(fan(&[8 + j, 8 + i, 12 + i, 12 + j]));
            triangles.extend(fan(&[4 + i, 4 + j, 12 + j, 12 + i]));
            triangles.extend(fan(&[i, 8 + i, 8 + j, j]));
        }
        (vertices, triangles)
    }

    #[test]
    fn a_ring_alone_or_beside_a_box_is_no_beam() {
        let (vertices, triangles) = ring_mesh(0.0);
        assert_eq!(beam_quantities(&vertices, &triangles), None);
        // The box is a sphere (characteristic 2) and the ring a torus (0): together
        // 2, the characteristic of one solid with no hole, and both prisms run along z
        // with ends in the same two planes. Only the count of pieces tells them.
        let (mut vertices, mut triangles) = box_mesh([0.0; 3], [0.2, 0.3, 3.0]);
        let (more, extra) = ring_mesh(2.0);
        let shift = u32::try_from(vertices.len()).unwrap();
        vertices.extend(more);
        triangles.extend(
            extra
                .into_iter()
                .map(|BrepTriangle(a, b, c)| BrepTriangle(a + shift, b + shift, c + shift)),
        );
        assert_eq!(
            2 * vertices.len() as i64 - triangles.len() as i64,
            4,
            "the pair has a single solid's characteristic"
        );
        assert_eq!(beam_quantities(&vertices, &triangles), None);
    }

    #[test]
    fn a_round_section_with_many_planes_is_no_beam() {
        let (vertices, triangles) = polygon_prism(40);
        assert_eq!(beam_quantities(&vertices, &triangles), None);
    }

    #[test]
    fn a_column_standing_on_its_end_takes_its_height_its_section_and_its_sides() {
        let (vertices, triangles) = box_mesh([1.0, 2.0, 3.0], [0.3, 0.4, 3.0]);
        let found = column_quantities(&vertices, &triangles).unwrap();
        assert_close(found.length, 3.0);
        assert_close(found.cross_section_area, 0.12);
        assert_close(found.outer_surface_area, 2.0 * (0.3 + 0.4) * 3.0);
        assert_close(found.volume, 0.36);
    }

    #[test]
    fn a_box_lying_down_or_wider_than_it_is_tall_is_no_column() {
        // The longest axis it is a prism along is a horizontal one (or two tie).
        for size in [
            [3.0, 0.1, 0.2],
            [0.2, 3.0, 0.1],
            [0.4, 0.3, 0.2],
            [0.4, 0.4, 0.3],
        ] {
            let (vertices, triangles) = box_mesh([0.0; 3], size);
            assert_eq!(column_quantities(&vertices, &triangles), None, "{size:?}");
        }
    }

    #[test]
    fn a_column_whose_longest_flat_prism_axis_is_not_vertical_is_none() {
        // Flat ends along y only: a column cut across its top reads this way.
        let (vertices, triangles) = slanted_beam();
        assert!(beam_quantities(&vertices, &triangles).is_some());
        assert_eq!(column_quantities(&vertices, &triangles), None);
    }

    #[test]
    fn a_column_that_is_not_one_closed_solid_is_none() {
        let (vertices, mut triangles) = box_mesh([0.0; 3], [0.3, 0.4, 3.0]);
        triangles.pop();
        assert_eq!(column_quantities(&vertices, &triangles), None);
        let (vertices, triangles) = ring_mesh(0.0);
        assert_eq!(column_quantities(&vertices, &triangles), None);
        let (mut vertices, mut triangles) = box_mesh([0.0; 3], [0.3, 0.4, 3.0]);
        let (more, extra) = box_mesh([5.0, 0.0, 0.0], [0.3, 0.4, 3.0]);
        let shift = u32::try_from(vertices.len()).unwrap();
        vertices.extend(more);
        triangles.extend(
            extra
                .into_iter()
                .map(|BrepTriangle(a, b, c)| BrepTriangle(a + shift, b + shift, c + shift)),
        );
        assert_eq!(column_quantities(&vertices, &triangles), None);
    }

    #[test]
    fn a_round_column_of_many_planes_and_no_triangles_are_none() {
        let (vertices, triangles) = polygon_prism(40);
        assert_eq!(column_quantities(&vertices, &triangles), None);
        assert_eq!(column_quantities(&[], &[]), None);
    }

    #[test]
    fn no_triangles_and_a_vertex_that_is_not_there_are_no_beam() {
        assert_eq!(beam_quantities(&[], &[]), None);
        let (vertices, mut triangles) = box_mesh([0.0; 3], [0.1, 0.2, 3.0]);
        triangles.push(BrepTriangle(0, 1, 99));
        assert_eq!(beam_quantities(&vertices, &triangles), None);
    }
}
