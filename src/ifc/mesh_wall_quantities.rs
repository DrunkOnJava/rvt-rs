//! A wall's quantities from the mesh Revit saved for it (#1046, RE-388).
//!
//! The mesh Revit saves for a wall is its solid as Revit last drew it, joined
//! to the walls beside it and cut by what it hosts, and revit-ifc's
//! `PropertyUtil.CreateWallBaseQuantities` (CLEANROOM.md source 5) writes the
//! wall's quantities from that solid: `NetVolume` its volume, `NetSideArea`
//! the area of its largest vertical face, less that face's holes,
//! `GrossSideArea` the area inside the outer loop of such a face, and
//! `GrossVolume` the wall type's width times the `GrossSideArea`.
//! [`wall_mesh_quantities`] reads them from the mesh where it can be trusted to
//! be the solid: every face a plane, the faces one closed surface wound one
//! way, all in one piece. Revit's export writes the volume of every such wall
//! and the other three only of some (on the BIM4LCA office as built, 26 of 581
//! walls with quantities have a `NetVolume` and nothing else), and nothing in
//! the mesh tells them apart but that none of those 26 is a plain prism: each
//! has five or more vertical faces. So the areas are given only of a wall with
//! no more than four (its two sides and its two ends), and a gross area only
//! where the two largest vertical faces, the sides, face opposite ways and
//! agree on it, the one case in which Revit's choice between them cannot
//! matter.

use crate::native_saved_mesh::Primitive;
use crate::vector_math::{cross, dot, sub};
use std::collections::{HashMap, HashSet};

/// Points this close, feet, are one vertex.
const WELD_FEET: f64 = 1e-6;

/// The cosine of the angle between the normals of two triangles of one face
/// must be at least this for the face to be a plane.
const FLAT_COSINE: f64 = 1.0 - 1e-9;

/// A face whose normal is this close to horizontal, in the component along
/// the vertical, is vertical.
const VERTICAL_TOLERANCE: f64 = 1e-6;

/// How far apart, relative, the outer-loop areas of a wall's two sides may be
/// and still be one area.
const SIDES_AGREE: f64 = 1e-5;

/// More vertical faces than this and the wall is given no area: Revit's export
/// writes the areas and the gross volume of a wall of a plain prism's plan and of
/// others, and the walls it writes only a volume of all have five or more.
const MAX_VERTICAL_FACES: usize = 4;

/// The quantities a wall's mesh gives, in feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct WallMeshQuantities {
    /// The volume of the mesh, cubic feet.
    pub net_volume: f64,
    /// The areas of its sides, where it has no more than four vertical faces.
    pub sides: Option<WallSideAreas>,
    /// The vertical span of the mesh, from its lowest corner to its highest,
    /// feet. It is the `Height` Revit's export writes, to the single
    /// precision of the saved corners, which is too coarse to write (RE-388),
    /// so it only confirms a height read elsewhere (RE-405).
    pub height_feet: f64,
}

/// The areas of a wall's sides, square feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct WallSideAreas {
    /// The area of the largest vertical face, its holes taken out.
    pub net: f64,
    /// The area inside the outer loop of the wall's sides, where both agree.
    pub gross: Option<f64>,
}

type Triangle = ([f64; 3], [f64; 3], [f64; 3]);

/// One flat vertical face: its area, the area inside its outer loop (nan where
/// its boundary does not close into loops one way each) and its horizontal
/// unit normal.
#[derive(Debug, Clone, Copy)]
struct Side {
    net: f64,
    gross: f64,
    normal: [f64; 2],
}

/// The triangles of `primitive` wound as its normals point, with none that has
/// no area; none where a corner is not there or not finite.
fn oriented(primitive: &Primitive) -> Option<Vec<Triangle>> {
    let mut out = Vec::with_capacity(primitive.triangles.len());
    for triangle in &primitive.triangles {
        let corners = triangle.map(|i| primitive.vertices.get(i as usize).copied());
        let [Some(a), Some(mut b), Some(mut c)] = corners else {
            return None;
        };
        if !a.iter().chain(&b).chain(&c).all(|x| x.is_finite()) {
            return None;
        }
        let normal = cross(sub(b, a), sub(c, a));
        if dot(normal, normal).sqrt() < 1e-12 {
            continue;
        }
        if primitive.normals.len() == primitive.vertices.len() {
            let outward = triangle.iter().fold([0.0; 3], |sum, i| {
                let n = primitive
                    .normals
                    .get(*i as usize)
                    .copied()
                    .unwrap_or([0.0; 3]);
                [sum[0] + n[0], sum[1] + n[1], sum[2] + n[2]]
            });
            if dot(normal, outward) < 0.0 {
                std::mem::swap(&mut b, &mut c);
            }
        }
        out.push((a, b, c));
    }
    Some(out)
}

/// The unit normal of a face made of `triangles` where they all have it, or
/// none where the face is not a plane (or has no triangle).
fn flat_normal(triangles: &[Triangle]) -> Option<[f64; 3]> {
    let mut first: Option<[f64; 3]> = None;
    for (a, b, c) in triangles {
        let normal = cross(sub(*b, *a), sub(*c, *a));
        let length = dot(normal, normal).sqrt();
        let unit = [normal[0] / length, normal[1] / length, normal[2] / length];
        match first {
            None => first = Some(unit),
            Some(seen) if dot(seen, unit) < FLAT_COSINE => return None,
            Some(_) => {}
        }
    }
    first
}

fn area(triangle: &Triangle) -> f64 {
    let (a, b, c) = triangle;
    let n = cross(sub(*b, *a), sub(*c, *a));
    dot(n, n).sqrt() / 2.0
}

/// The area inside the outer loop of the boundary of one face's `triangles`
/// (corners welded), seen along the horizontal unit normal `n`, or nan where
/// the boundary is not made of loops, each vertex left once.
fn outer_loop_area(triangles: &[Triangle], n: [f64; 2]) -> f64 {
    let mut ids: HashMap<[i64; 3], u32> = HashMap::new();
    let mut points: Vec<[f64; 3]> = Vec::new();
    let mut edges: HashSet<(u32, u32)> = HashSet::new();
    for (a, b, c) in triangles {
        let mut corner = |p: &[f64; 3]| {
            let key = (*p).map(|x| (x / WELD_FEET).round() as i64);
            *ids.entry(key).or_insert_with(|| {
                points.push(*p);
                (points.len() - 1) as u32
            })
        };
        let (ia, ib, ic) = (corner(a), corner(b), corner(c));
        for edge in [(ia, ib), (ib, ic), (ic, ia)] {
            edges.insert(edge);
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    for &(from, to) in &edges {
        if !edges.contains(&(to, from)) && next.insert(from, to).is_some() {
            return f64::NAN;
        }
    }
    let along = [-n[1], n[0]];
    let plane = |i: u32| {
        let p = points[i as usize];
        (p[0] * along[0] + p[1] * along[1], p[2])
    };
    let mut seen: HashSet<u32> = HashSet::new();
    let mut outer = 0.0f64;
    let starts: Vec<u32> = next.keys().copied().collect();
    for start in starts {
        if seen.contains(&start) {
            continue;
        }
        let mut sum = 0.0;
        let mut at = start;
        loop {
            seen.insert(at);
            let Some(&to) = next.get(&at) else {
                return f64::NAN;
            };
            let ((u0, v0), (u1, v1)) = (plane(at), plane(to));
            sum += u0 * v1 - u1 * v0;
            at = to;
            if at == start {
                break;
            }
            if seen.contains(&at) {
                return f64::NAN;
            }
        }
        outer = outer.max(sum.abs() / 2.0);
    }
    if outer == 0.0 { f64::NAN } else { outer }
}

/// The volume, cubic feet, of the closed surface `triangles` make, wound one
/// way with their corners welded and in one piece, or none: an edge no other
/// triangle shares, one three share, two triangles that wind the same way
/// along an edge, two or more pieces, or no volume at all.
fn one_piece_volume(triangles: &[Triangle]) -> Option<f64> {
    let mut ids: HashMap<[i64; 3], usize> = HashMap::new();
    let mut centre = [0.0; 3];
    let mut count = 0.0;
    let mut corners: Vec<[usize; 3]> = Vec::with_capacity(triangles.len());
    for (a, b, c) in triangles {
        let mut id = [0usize; 3];
        for (slot, p) in id.iter_mut().zip([a, b, c]) {
            let key = (*p).map(|x| (x / WELD_FEET).round() as i64);
            let next = ids.len();
            *slot = *ids.entry(key).or_insert(next);
            for (total, x) in centre.iter_mut().zip(p) {
                *total += x;
            }
            count += 1.0;
        }
        corners.push(id);
    }
    if triangles.is_empty() {
        return None;
    }
    centre = centre.map(|sum| sum / count);
    let mut parent: Vec<usize> = (0..ids.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut edges: HashMap<(usize, usize), u32> = HashMap::new();
    let mut sum = 0.0;
    for ((a, b, c), [ia, ib, ic]) in triangles.iter().zip(&corners) {
        if ia == ib || ib == ic || ic == ia {
            continue;
        }
        for edge in [(*ia, *ib), (*ib, *ic), (*ic, *ia)] {
            *edges.entry(edge).or_insert(0) += 1;
            let (ra, rb) = (find(&mut parent, edge.0), find(&mut parent, edge.1));
            if ra != rb {
                parent[ra] = rb;
            }
        }
        sum += dot(sub(*a, centre), cross(sub(*b, centre), sub(*c, centre)));
    }
    if edges.is_empty()
        || edges
            .iter()
            .any(|(&(from, to), &seen)| seen != 1 || edges.get(&(to, from)) != Some(&1))
    {
        return None;
    }
    let used: HashSet<usize> = edges.keys().flat_map(|&(a, b)| [a, b]).collect();
    let roots: HashSet<usize> = used.iter().map(|&i| find(&mut parent, i)).collect();
    if roots.len() != 1 {
        return None;
    }
    let volume = sum.abs() / 6.0;
    (volume.is_finite() && volume > 0.0).then_some(volume)
}

/// The gross side area of the wall whose two largest vertical faces are
/// `first` and `second`: the area inside their outer loops where they face
/// opposite ways and agree on it.
fn agreeing_gross(first: &Side, second: &Side) -> Option<f64> {
    let opposite = first.normal[0] * second.normal[0] + first.normal[1] * second.normal[1] < -0.999;
    let (a, b) = (first.gross, second.gross);
    (opposite && a.is_finite() && b.is_finite() && (a - b).abs() <= SIDES_AGREE * a.max(b))
        .then_some(a.max(b))
}

/// The areas of the wall whose vertical faces are `sides`, or none where it has
/// more than [`MAX_VERTICAL_FACES`] or none.
fn side_areas(sides: &mut [Side]) -> Option<WallSideAreas> {
    if sides.len() > MAX_VERTICAL_FACES {
        return None;
    }
    sides.sort_by(|x, y| y.net.total_cmp(&x.net));
    let largest = sides.first()?;
    Some(WallSideAreas {
        net: largest.net,
        gross: sides
            .get(1)
            .and_then(|second| agreeing_gross(largest, second)),
    })
}

/// The quantities the saved mesh of a wall gives, or none where its faces are
/// not all planes or they are not one closed piece wound one way.
pub(super) fn wall_mesh_quantities(primitives: &[Primitive]) -> Option<WallMeshQuantities> {
    let mut every: Vec<Triangle> = Vec::new();
    let mut sides: Vec<Side> = Vec::new();
    for primitive in primitives {
        let triangles = oriented(primitive)?;
        if triangles.is_empty() {
            continue;
        }
        let unit = flat_normal(&triangles)?;
        if unit[2].abs() <= VERTICAL_TOLERANCE {
            let horizontal = unit[0].hypot(unit[1]);
            let normal = [unit[0] / horizontal, unit[1] / horizontal];
            sides.push(Side {
                net: triangles.iter().map(area).sum(),
                gross: outer_loop_area(&triangles, normal),
                normal,
            });
        }
        every.extend(triangles);
    }
    let net_volume = one_piece_volume(&every)?;
    let (low, high) = every
        .iter()
        .flat_map(|(a, b, c)| [a[2], b[2], c[2]])
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), z| {
            (low.min(z), high.max(z))
        });
    Some(WallMeshQuantities {
        net_volume,
        sides: side_areas(&mut sides),
        height_feet: high - low,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat face of four corners, its normal on every vertex.
    fn quad(corners: [[f64; 3]; 4], normal: [f64; 3]) -> Primitive {
        Primitive {
            source_owner_id: None,
            object_index: 0,
            face_tag: 0,
            render_style_id: 0,
            material_id: None,
            vertices: corners.to_vec(),
            normals: vec![normal; 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            hidden: false,
            filtered: false,
        }
    }

    /// The six faces of a wall `length` long on x, `thickness` on y and `height`
    /// on z with a corner at `origin`, each with its outward normal.
    fn wall(origin: [f64; 3], length: f64, thickness: f64, height: f64) -> Vec<Primitive> {
        let [x0, y0, z0] = origin;
        let (x1, y1, z1) = (x0 + length, y0 + thickness, z0 + height);
        vec![
            quad(
                [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
                [0.0, 0.0, -1.0],
            ),
            quad(
                [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                [0.0, 0.0, 1.0],
            ),
            quad(
                [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                [0.0, -1.0, 0.0],
            ),
            quad(
                [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
                [0.0, 1.0, 0.0],
            ),
            quad(
                [[x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]],
                [-1.0, 0.0, 0.0],
            ),
            quad(
                [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
                [1.0, 0.0, 0.0],
            ),
        ]
    }

    #[test]
    fn a_plain_wall_gives_its_volume_and_its_sides() {
        let found = wall_mesh_quantities(&wall([100.0, 200.0, 190.0], 10.0, 1.0, 8.0)).unwrap();
        assert!((found.net_volume - 80.0).abs() < 1e-9);
        assert!((found.height_feet - 8.0).abs() < 1e-9);
        let sides = found.sides.unwrap();
        assert!((sides.net - 80.0).abs() < 1e-9);
        assert!((sides.gross.unwrap() - 80.0).abs() < 1e-9);
    }

    #[test]
    fn a_wall_missing_a_face_gives_nothing() {
        let mut faces = wall([0.0, 0.0, 0.0], 10.0, 1.0, 8.0);
        faces.remove(1);
        assert_eq!(wall_mesh_quantities(&faces), None);
    }

    #[test]
    fn two_walls_in_one_mesh_give_nothing() {
        let mut faces = wall([0.0, 0.0, 0.0], 10.0, 1.0, 8.0);
        faces.extend(wall([50.0, 0.0, 0.0], 10.0, 1.0, 8.0));
        assert_eq!(wall_mesh_quantities(&faces), None);
    }

    #[test]
    fn a_face_that_is_not_a_plane_gives_nothing() {
        let mut faces = wall([0.0, 0.0, 0.0], 10.0, 1.0, 8.0);
        // One corner of the front face lifted out of its plane.
        faces[2].vertices[2] = [10.0, -0.3, 8.0];
        assert_eq!(wall_mesh_quantities(&faces), None);
    }

    #[test]
    fn nothing_at_all_gives_nothing() {
        assert_eq!(wall_mesh_quantities(&[]), None);
    }

    #[test]
    fn a_face_with_a_hole_has_a_gross_area_beyond_its_net() {
        // 10 by 8 with a 2 by 2 hole, in the plane y = 0, wound as one face.
        let at = |u: f64, v: f64| [u, 0.0, v];
        let vertices = vec![
            at(0.0, 0.0),
            at(10.0, 0.0),
            at(10.0, 8.0),
            at(0.0, 8.0),
            at(4.0, 3.0),
            at(6.0, 3.0),
            at(6.0, 5.0),
            at(4.0, 5.0),
        ];
        let triangles = vec![
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let face = Primitive {
            source_owner_id: None,
            object_index: 0,
            face_tag: 0,
            render_style_id: 0,
            material_id: None,
            normals: vec![[0.0, -1.0, 0.0]; vertices.len()],
            vertices,
            triangles,
            hidden: false,
            filtered: false,
        };
        let triangles = oriented(&face).unwrap();
        let net: f64 = triangles.iter().map(area).sum();
        assert!((net - 76.0).abs() < 1e-9);
        assert!((outer_loop_area(&triangles, [0.0, -1.0]) - 80.0).abs() < 1e-9);
    }

    #[test]
    fn a_wall_with_more_than_four_vertical_faces_gives_no_areas() {
        let side = |normal: [f64; 2]| Side {
            net: 80.0,
            gross: 80.0,
            normal,
        };
        let mut four = vec![
            side([0.0, -1.0]),
            side([0.0, 1.0]),
            side([-1.0, 0.0]),
            side([1.0, 0.0]),
        ];
        assert!(side_areas(&mut four).is_some());
        let mut five = four.clone();
        five.push(side([1.0, 0.0]));
        assert_eq!(side_areas(&mut five), None);
        assert_eq!(side_areas(&mut []), None);
    }

    #[test]
    fn sides_agree_only_where_they_face_apart_and_are_equal() {
        let side = |gross: f64, normal: [f64; 2]| Side {
            net: gross,
            gross,
            normal,
        };
        let front = side(80.0, [0.0, -1.0]);
        assert_eq!(agreeing_gross(&front, &side(80.0, [0.0, 1.0])), Some(80.0));
        assert_eq!(agreeing_gross(&front, &side(80.0, [0.0, -1.0])), None);
        assert_eq!(agreeing_gross(&front, &side(70.0, [0.0, 1.0])), None);
        assert_eq!(agreeing_gross(&front, &side(f64::NAN, [0.0, 1.0])), None);
    }
}
