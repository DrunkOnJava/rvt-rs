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
use std::collections::HashMap;

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
    (volume.is_finite() && volume > 0.0).then_some(volume)
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
/// under this (0.06 degrees): the axis is the cross product of two noisy
/// normals, which adds the two errors.
const PARALLEL_COSINE: f64 = 1e-3;

/// Two planes give an axis where the sine of the angle between their normals is
/// over this (0.6 degrees); nearer parallel than that, their cross product is
/// the noise in their normals.
const AXIS_SINE: f64 = 1e-2;

/// The extents, feet, of the mesh `triangles` make of `vertices` along each
/// axis it is a prism along, smallest first (RE-418): the planes of its
/// triangles, all but at most two of them (its ends) parallel to the axis. A
/// rectangular mullion with a flat or a mitred end is one along its own axis
/// and, being a box in section, along no other unless it is a box; a box is one
/// along each of its three. The extent is the distance between the first and
/// the last of the mesh's corners along the axis, so a mitred end counts at its
/// longer side.
///
/// None where a vertex is not there or not finite, where no triangle has an
/// area, and where no axis leaves the mesh at two end planes, which a pyramid
/// or a mesh cut in three directions does not.
pub(super) fn prism_extents(vertices: &[[f64; 3]], triangles: &[BrepTriangle]) -> Vec<f64> {
    let mut planes: Vec<([f64; 3], f64)> = Vec::new();
    let mut corners: Vec<[f64; 3]> = Vec::new();
    for BrepTriangle(a, b, c) in triangles {
        let (Some(&p), Some(&q), Some(&r)) = (
            vertices.get(*a as usize),
            vertices.get(*b as usize),
            vertices.get(*c as usize),
        ) else {
            return Vec::new();
        };
        if ![p, q, r]
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
        {
            return Vec::new();
        }
        let normal = cross(sub(q, p), sub(r, p));
        let twice_area = dot(normal, normal).sqrt();
        if twice_area < MIN_TWICE_AREA {
            continue;
        }
        let normal = normal.map(|component| component / twice_area);
        let offset = dot(normal, p);
        let known = planes.iter().any(|&(other, other_offset)| {
            let cosine = dot(normal, other);
            cosine.abs() > 1.0 - PLANE_COSINE
                && (offset - cosine.signum() * other_offset).abs() < PLANE_FEET
        });
        if !known {
            planes.push((normal, offset));
        }
        corners.extend([p, q, r]);
    }
    let mut axes: Vec<[f64; 3]> = Vec::new();
    for (position, (first, _)) in planes.iter().enumerate() {
        for (second, _) in &planes[position + 1..] {
            let direction = cross(*first, *second);
            let length = dot(direction, direction).sqrt();
            if length < AXIS_SINE {
                continue;
            }
            let axis = direction.map(|component| component / length);
            if axes
                .iter()
                .all(|known| dot(axis, *known).abs() < 1.0 - 1e-6)
            {
                axes.push(axis);
            }
        }
    }
    let mut extents: Vec<f64> = axes
        .iter()
        .filter(|axis| {
            planes
                .iter()
                .filter(|(normal, _)| dot(*normal, **axis).abs() > PARALLEL_COSINE)
                .count()
                <= 2
        })
        .map(|axis| {
            let (low, high) =
                corners
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), corner| {
                        let along = dot(*corner, *axis);
                        (low.min(along), high.max(along))
                    });
            high - low
        })
        .collect();
    extents.sort_by(f64::total_cmp);
    extents
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

    fn assert_extents(found: Vec<f64>, wanted: &[f64]) {
        assert_eq!(found.len(), wanted.len(), "{found:?}");
        for (found, wanted) in found.iter().zip(wanted) {
            assert!((found - wanted).abs() < 1e-9, "{found} not {wanted}");
        }
    }

    #[test]
    fn a_box_is_a_prism_along_each_of_its_three_axes() {
        let (vertices, triangles) = box_mesh([10.0, 20.0, 5.0], [0.082, 4.9, 7.9]);
        assert_extents(prism_extents(&vertices, &triangles), &[0.082, 4.9, 7.9]);
    }

    #[test]
    fn a_mitred_end_counts_at_its_longer_side() {
        let (vertices, triangles) = mitred_prism();
        let extents = prism_extents(&vertices, &triangles);
        assert_extents(extents, &[2.0, 11.0]);
    }

    #[test]
    fn a_mesh_cut_in_more_than_two_directions_is_no_prism() {
        // An octahedron: along every direction two of its planes meet in,
        // four more are not parallel to it.
        let vertices = vec![
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        let mut triangles = Vec::new();
        for x in [0, 1] {
            for y in [2, 3] {
                for z in [4, 5] {
                    triangles.push(BrepTriangle(x, y, z));
                }
            }
        }
        assert!(prism_extents(&vertices, &triangles).is_empty());
    }

    #[test]
    fn a_vertex_that_is_not_there_or_not_finite_leaves_no_extent() {
        let (mut vertices, mut triangles) = box_mesh([0.0, 0.0, 0.0], [1.0, 2.0, 10.0]);
        triangles.push(BrepTriangle(0, 1, 99));
        assert!(prism_extents(&vertices, &triangles).is_empty());
        triangles.pop();
        vertices[3][2] = f64::NAN;
        assert!(prism_extents(&vertices, &triangles).is_empty());
    }

    #[test]
    fn nothing_leaves_no_extent() {
        assert!(prism_extents(&[], &[]).is_empty());
        let (vertices, _) = box_mesh([0.0, 0.0, 0.0], [1.0, 2.0, 10.0]);
        assert!(prism_extents(&vertices, &[]).is_empty());
    }

    #[test]
    fn a_triangle_with_no_area_adds_no_plane_and_no_corner() {
        let (mut vertices, mut triangles) = box_mesh([0.0, 0.0, 0.0], [0.05, 0.1, 3.0]);
        vertices.push([40.0, 40.0, 40.0]);
        vertices.push([41.0, 41.0, 41.0]);
        vertices.push([42.0, 42.0, 42.0]);
        triangles.push(BrepTriangle(8, 9, 10));
        assert_extents(prism_extents(&vertices, &triangles), &[0.05, 0.1, 3.0]);
    }

    #[test]
    fn corners_off_by_single_precision_noise_still_make_one_plane_a_face() {
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [0.05, 0.1, 3.0]);
        for (index, vertex) in vertices.iter_mut().enumerate() {
            for (axis, coordinate) in vertex.iter_mut().enumerate() {
                *coordinate += 1e-6 * (((index * 7 + axis * 3) % 5) as f64 - 2.0);
            }
        }
        // The long axis is read to the noise; the short ones are tilted by it
        // toward the long side and are read to a hundredth of the section.
        let extents = prism_extents(&vertices, &triangles);
        assert_eq!(extents.len(), 3, "{extents:?}");
        assert!((extents[2] - 3.0).abs() < 1e-5, "{extents:?}");
        assert!((extents[0] - 0.05).abs() < 1e-3, "{extents:?}");
    }

    #[test]
    fn a_top_corner_pulled_in_leaves_the_height_no_prism_extent() {
        // One top corner of the box pulled in 0.4 ft: the top and the side it
        // sits on are no longer planes, so more than two planes are off the
        // height's axis.
        let (mut vertices, triangles) = box_mesh([0.0, 0.0, 0.0], [0.082, 4.9, 7.9]);
        vertices[6] = [0.082, 4.9 - 0.4, 7.9];
        let extents = prism_extents(&vertices, &triangles);
        assert!(
            extents.iter().all(|extent| (extent - 7.9).abs() > 1e-6),
            "{extents:?}"
        );
    }
}
