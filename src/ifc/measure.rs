//! Measurement tool (VW1-13) — distance, angle, area primitives for
//! the viewer's "measure" overlay.
//!
//! All functions operate in the caller's native unit (feet by
//! convention for this crate); they're pure math and make no
//! unit-system assumptions. Returned values carry the input's
//! unit — `distance(a, b)` in feet returns feet.

use serde::{Deserialize, Serialize};

/// 3D point — three floats, caller's units.
pub type Point3 = [f64; 3];

/// Euclidean distance between two 3D points (VW1-13).
pub fn distance(a: Point3, b: Point3) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Signed 3D vector from `a` to `b` (VW1-13). Not a distance —
/// keep sign per axis so callers can query direction.
pub fn vector(a: Point3, b: Point3) -> Point3 {
    [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
}

/// Dot product of two vectors.
pub fn dot(u: Point3, v: Point3) -> f64 {
    u[0] * v[0] + u[1] * v[1] + u[2] * v[2]
}

/// Cross product.
pub fn cross(u: Point3, v: Point3) -> Point3 {
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

/// Magnitude of a vector.
pub fn magnitude(v: Point3) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Unit-length copy of a vector (VW1-13). Returns the zero vector
/// when magnitude is zero — callers who care about that case must
/// check before normalising.
pub fn normalize(v: Point3) -> Point3 {
    let m = magnitude(v);
    if m == 0.0 {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / m, v[1] / m, v[2] / m]
    }
}

/// Angle in radians at vertex `b` of the path `a -> b -> c` (VW1-13).
/// Returns 0 for degenerate configurations where either leg has
/// zero length.
///
/// Result is in `[0, π]` (unsigned). Viewers that need a signed
/// angle compute it themselves using a reference axis + cross
/// product sign.
pub fn angle_abc(a: Point3, b: Point3, c: Point3) -> f64 {
    let ba = vector(b, a);
    let bc = vector(b, c);
    let m1 = magnitude(ba);
    let m2 = magnitude(bc);
    if m1 == 0.0 || m2 == 0.0 {
        return 0.0;
    }
    (dot(ba, bc) / (m1 * m2)).clamp(-1.0, 1.0).acos()
}

/// Signed area of a 3D polygon via the shoelace-in-plane variant
/// (VW1-13). Assumes the polygon is (approximately) planar —
/// non-planar polygons return the projected area onto the
/// plane whose normal is the vector sum of per-triangle normals
/// (a common convention; good enough for viewer measurement).
///
/// Returns the unsigned magnitude, so winding doesn't matter.
/// Points should be in order (CW or CCW) — random order produces
/// an arbitrary result.
pub fn polygon_area_3d(points: &[Point3]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    // Summed cross product of triangulated fan — magnitude / 2.
    let mut normal = [0.0_f64; 3];
    for i in 1..points.len() - 1 {
        let u = vector(points[0], points[i]);
        let v = vector(points[0], points[i + 1]);
        let n = cross(u, v);
        normal[0] += n[0];
        normal[1] += n[1];
        normal[2] += n[2];
    }
    magnitude(normal) * 0.5
}

/// Total perimeter of a closed 3D polygon (VW1-13). Sums edge
/// distances including the closing edge (last → first). Polygons
/// with < 2 points return 0.
pub fn polygon_perimeter(points: &[Point3]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let n = points.len();
    let mut sum = 0.0;
    for i in 0..n {
        sum += distance(points[i], points[(i + 1) % n]);
    }
    sum
}

/// Record of a single measurement operation (VW1-13). Matches the
/// shape a viewer's measurement-panel UI binds to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Measurement {
    /// Linear distance between two points.
    Distance { a: Point3, b: Point3, value: f64 },
    /// Angle between three points (at vertex `b`).
    Angle {
        a: Point3,
        b: Point3,
        c: Point3,
        radians: f64,
    },
    /// Polygon area from a vertex list.
    Area { vertices: Vec<Point3>, value: f64 },
}

impl Measurement {
    /// Build a distance measurement between two points.
    pub fn distance(a: Point3, b: Point3) -> Self {
        Self::Distance {
            a,
            b,
            value: distance(a, b),
        }
    }

    /// Build an angle measurement at vertex `b`.
    pub fn angle(a: Point3, b: Point3, c: Point3) -> Self {
        Self::Angle {
            a,
            b,
            c,
            radians: angle_abc(a, b, c),
        }
    }

    /// Build an area measurement from a polygon.
    pub fn area(vertices: Vec<Point3>) -> Self {
        let value = polygon_area_3d(&vertices);
        Self::Area { vertices, value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_3_4_5_triangle() {
        assert!((distance([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn distance_including_z() {
        assert!((distance([0.0, 0.0, 0.0], [2.0, 2.0, 1.0]) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn distance_zero_when_identical() {
        assert_eq!(distance([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]), 0.0);
    }

    #[test]
    fn cross_of_basis_vectors() {
        assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(cross([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn dot_of_orthogonal_is_zero() {
        assert!(dot([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]).abs() < 1e-9);
    }

    #[test]
    fn normalize_unit_vector_unchanged() {
        let n = normalize([1.0, 0.0, 0.0]);
        assert_eq!(n, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn normalize_zero_returns_zero() {
        let n = normalize([0.0, 0.0, 0.0]);
        assert_eq!(n, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn normalize_arbitrary_has_magnitude_one() {
        let n = normalize([3.0, 4.0, 0.0]);
        assert!((magnitude(n) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn angle_right_angle_is_half_pi() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 0.0, 0.0];
        let c = [0.0, 1.0, 0.0];
        let r = angle_abc(a, b, c);
        assert!((r - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn angle_straight_is_pi() {
        let a = [-1.0, 0.0, 0.0];
        let b = [0.0, 0.0, 0.0];
        let c = [1.0, 0.0, 0.0];
        let r = angle_abc(a, b, c);
        assert!((r - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn angle_zero_when_leg_collapses() {
        let r = angle_abc([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn polygon_area_unit_square() {
        let sq = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        assert!((polygon_area_3d(&sq) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_area_is_winding_invariant() {
        let ccw = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 3.0, 0.0],
            [0.0, 3.0, 0.0],
        ];
        let cw: Vec<Point3> = ccw.iter().rev().copied().collect();
        assert!((polygon_area_3d(&ccw) - 6.0).abs() < 1e-9);
        assert!((polygon_area_3d(&cw) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_area_triangle() {
        let tri = [[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 3.0, 0.0]];
        assert!((polygon_area_3d(&tri) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_area_degenerate_returns_zero() {
        assert_eq!(polygon_area_3d(&[]), 0.0);
        assert_eq!(polygon_area_3d(&[[0.0, 0.0, 0.0]]), 0.0);
        assert_eq!(polygon_area_3d(&[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]), 0.0);
    }

    #[test]
    fn polygon_perimeter_unit_square() {
        let sq = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        assert!((polygon_perimeter(&sq) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_perimeter_three_four_five_triangle() {
        let tri = [[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [0.0, 4.0, 0.0]];
        // sides: 3, 4, sqrt(9+16)=5 -> perimeter 12
        assert!((polygon_perimeter(&tri) - 12.0).abs() < 1e-9);
    }

    #[test]
    fn measurement_distance_builder() {
        let m = Measurement::distance([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        match m {
            Measurement::Distance { value, .. } => assert!((value - 1.0).abs() < 1e-9),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn measurement_angle_builder() {
        let m = Measurement::angle([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        match m {
            Measurement::Angle { radians, .. } => {
                assert!((radians - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn measurement_area_builder() {
        let m = Measurement::area(vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
        ]);
        match m {
            Measurement::Area { value, .. } => assert!((value - 4.0).abs() < 1e-9),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn measurement_is_serde_roundtrippable() {
        let m = Measurement::distance([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        let json = serde_json::to_string(&m).unwrap();
        let back: Measurement = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }
}
