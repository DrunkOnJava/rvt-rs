//! Geometry primitives for Revit element extraction.
//!
//! These types are the **output** of the per-element walker
//! (Phase 4 Layer 5b) and the **input** of the IFC exporter's
//! geometry mapper (Phase 6). They are intentionally format-agnostic:
//!
//! - No Revit-specific type IDs or wire-level detail. Coordinates are
//!   just `f64` triples; curves describe themselves parametrically.
//! - No IFC-specific naming either. `Extrusion` is any profile swept
//!   linearly, not just `IfcExtrudedAreaSolid`.
//!
//! The idea is that per-element decoders assemble `Solid` / `Face` /
//! `Curve` values from Revit bytes, and the IFC exporter maps those
//! to the matching IFC4 entities (or serializes them as faceted BREP
//! when the primitive doesn't have a direct IFC analogue).
//!
//! # Coordinate conventions
//!
//! - Right-handed coordinate system.
//! - All distances are in the project's unit system. The exporter is
//!   responsible for mapping to IfcSIUnit / IfcConversionBasedUnit
//!   based on the file's `autodesk.unit.*` identifiers. Geometry
//!   values are never silently converted.
//! - Angles are radians unless noted.

use serde::{Deserialize, Serialize};

/// 3D point in project coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub const ORIGIN: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// 3D unit vector. Callers are responsible for normalization; no
/// runtime check enforces it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const X_AXIS: Self = Self::new(1.0, 0.0, 0.0);
    pub const Y_AXIS: Self = Self::new(0.0, 1.0, 0.0);
    pub const Z_AXIS: Self = Self::new(0.0, 0.0, 1.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// 3D transform (rotation + translation + optional uniform scale).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform3 {
    /// Origin of the local frame in parent coordinates.
    pub origin: Point3,
    /// Local X axis direction (unit vector).
    pub x_axis: Vector3,
    /// Local Y axis direction (unit vector).
    pub y_axis: Vector3,
    /// Local Z axis direction (unit vector).
    pub z_axis: Vector3,
    /// Uniform scale factor applied after rotation. Default 1.0 =
    /// no scaling.
    pub scale: f64,
}

impl Transform3 {
    /// Identity transform: origin at (0,0,0), X+Y+Z axes aligned,
    /// scale 1.0.
    pub const IDENTITY: Self = Self {
        origin: Point3::ORIGIN,
        x_axis: Vector3::X_AXIS,
        y_axis: Vector3::Y_AXIS,
        z_axis: Vector3::Z_AXIS,
        scale: 1.0,
    };
}

// ----------------------------------------------------------------------------
// Curves (Phase 5 GEO-04..10)
// ----------------------------------------------------------------------------

/// Bounded parametric curve.
///
/// Every curve is represented parametrically (not tessellated) so
/// the IFC exporter can emit exact `IfcLine` / `IfcCircle` /
/// `IfcNurbsCurve` without loss. Discretization, if needed, is a
/// downstream concern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Curve {
    /// Straight line segment from `start` to `end`.
    Line { start: Point3, end: Point3 },
    /// Circular arc — `center` + `radius` on the plane whose
    /// normal is `normal`. Parameterized by `start_angle` /
    /// `end_angle` in radians. Full circle: set both to cover
    /// `2π`.
    Arc {
        center: Point3,
        radius: f64,
        normal: Vector3,
        start_angle: f64,
        end_angle: f64,
    },
    /// Full circle (convenience — distinguishable from Arc by the
    /// exporter when emitting IfcCircle vs IfcTrimmedCurve).
    Circle {
        center: Point3,
        radius: f64,
        normal: Vector3,
    },
    /// Elliptical arc / full ellipse.
    Ellipse {
        center: Point3,
        /// Major axis: vector from center to semi-major endpoint.
        major_axis: Vector3,
        /// Minor axis: vector from center to semi-minor endpoint.
        minor_axis: Vector3,
        start_angle: f64,
        end_angle: f64,
    },
    /// Hermite-interpolated cubic spline through control points
    /// with specified tangents.
    HermiteSpline {
        control_points: Vec<Point3>,
        tangents: Vec<Vector3>,
    },
    /// Non-Uniform Rational B-Spline. Most general curve form.
    NurbsCurve {
        degree: u8,
        knots: Vec<f64>,
        /// Control points paired with weights.
        control_points: Vec<(Point3, f64)>,
    },
    /// Cylindrical helix (spiral staircases, thread paths).
    CylindricalHelix {
        center: Point3,
        axis: Vector3,
        radius: f64,
        pitch: f64,
        turns: f64,
    },
}

/// Closed or open loop of curves. Used for planar face boundaries
/// and for sketches (floor/ceiling/roof boundaries).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurveLoop {
    pub curves: Vec<Curve>,
    /// `true` if the last curve's endpoint meets the first curve's
    /// start (closed loop, defines a region). `false` for open
    /// polylines.
    pub closed: bool,
}

// ----------------------------------------------------------------------------
// Faces (Phase 5 GEO-11..17)
// ----------------------------------------------------------------------------

/// Parametric surface patch.
///
/// Matches IFC4's `IfcSurface` subtypes. The walker outputs
/// whichever variant best describes the Revit face without
/// lossy tessellation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Face {
    /// Planar face with outer boundary + optional holes.
    Planar {
        /// Plane defined by an on-plane point + normal.
        origin: Point3,
        normal: Vector3,
        outer_loop: CurveLoop,
        holes: Vec<CurveLoop>,
    },
    /// Cylindrical surface — infinite cylinder parameters; trim
    /// to `u_range` × `v_range` for the actual patch.
    Cylindrical {
        axis_origin: Point3,
        axis_direction: Vector3,
        radius: f64,
        u_range: (f64, f64),
        v_range: (f64, f64),
    },
    /// Conical surface with apex + half-angle.
    Conical {
        apex: Point3,
        axis_direction: Vector3,
        half_angle_radians: f64,
        u_range: (f64, f64),
        v_range: (f64, f64),
    },
    /// Surface of revolution: `generator` curve rotated about
    /// `axis_origin` + `axis_direction` by `angle` radians.
    Revolved {
        generator: Curve,
        axis_origin: Point3,
        axis_direction: Vector3,
        angle_radians: f64,
    },
    /// Ruled surface: straight lines connecting corresponding
    /// points on `curve_a` and `curve_b`.
    Ruled { curve_a: Curve, curve_b: Curve },
    /// Hermite-interpolated surface patch grid.
    HermitePatch {
        /// 2D grid of control points indexed `[u][v]`.
        control_net: Vec<Vec<Point3>>,
        /// Tangent vectors at each control point, same shape.
        u_tangents: Vec<Vec<Vector3>>,
        v_tangents: Vec<Vec<Vector3>>,
    },
    /// NURBS surface.
    NurbsSurface {
        u_degree: u8,
        v_degree: u8,
        u_knots: Vec<f64>,
        v_knots: Vec<f64>,
        /// 2D grid of (point, weight) indexed `[u][v]`.
        control_net: Vec<Vec<(Point3, f64)>>,
    },
}

// ----------------------------------------------------------------------------
// Solids (Phase 5 GEO-18..26)
// ----------------------------------------------------------------------------

/// Solid geometry primitive.
///
/// Primitives chosen to match IFC4's swept/boolean/brep hierarchy.
/// Most Revit elements assemble into 1–3 `Solid` values (e.g.
/// a Wall becomes a stack of `Extrusion`s, one per layer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Solid {
    /// Profile swept linearly by `distance` along `direction`.
    /// Maps to `IfcExtrudedAreaSolid`.
    Extrusion {
        profile: CurveLoop,
        direction: Vector3,
        distance: f64,
        placement: Transform3,
    },
    /// Two profiles with linear interpolation between them.
    Blend {
        profile_a: CurveLoop,
        profile_b: CurveLoop,
        alignment: Transform3,
    },
    /// Profile rotated about an axis. Maps to
    /// `IfcRevolvedAreaSolid`.
    Revolve {
        profile: CurveLoop,
        axis_origin: Point3,
        axis_direction: Vector3,
        angle_radians: f64,
    },
    /// Profile swept along an arbitrary path curve. Maps to
    /// `IfcSurfaceCurveSweptAreaSolid` (path on surface) or
    /// `IfcFixedReferenceSweptAreaSolid` (free-path).
    Sweep {
        profile: CurveLoop,
        path: Curve,
        placement: Transform3,
    },
    /// Two profiles swept along a path with interpolation (combines
    /// `Blend` + `Sweep`).
    SweptBlend {
        profile_a: CurveLoop,
        profile_b: CurveLoop,
        path: Curve,
    },
    /// Boolean of two solids. `op` determines union / difference /
    /// intersection. Maps to `IfcBooleanResult` (or
    /// `IfcBooleanClippingResult` for voiding operations).
    Boolean {
        op: BooleanOp,
        operand_a: Box<Solid>,
        operand_b: Box<Solid>,
    },
    /// Explicit void solid — a `Boolean::Difference` where the
    /// subtrahend represents the void (used for Door/Window
    /// openings cutting host Walls). Kept as a separate variant so
    /// the exporter can emit `IfcRelVoidsElement` directly.
    Void {
        host: Box<Solid>,
        void_shape: Box<Solid>,
    },
    /// Discretized triangular mesh. Maps to `IfcFacetedBrep`.
    Mesh(Mesh),
}

/// Boolean operation between two solids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

/// Discretized triangular mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<Point3>,
    /// Triangle vertex indices, 3 per triangle.
    pub triangles: Vec<[u32; 3]>,
    /// Per-vertex normals. Optional — empty Vec means auto-compute.
    pub normals: Vec<Vector3>,
}

/// Point cloud (imported laser scans / photogrammetry).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCloud {
    pub points: Vec<Point3>,
}

// ----------------------------------------------------------------------------
// Bounding box (GEO-35)
// ----------------------------------------------------------------------------

/// Axis-aligned bounding box in project coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: Point3,
    pub max: Point3,
}

impl BoundingBox {
    pub fn empty() -> Self {
        Self {
            min: Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    pub fn expand_point(&mut self, p: Point3) {
        if p.x < self.min.x {
            self.min.x = p.x;
        }
        if p.y < self.min.y {
            self.min.y = p.y;
        }
        if p.z < self.min.z {
            self.min.z = p.z;
        }
        if p.x > self.max.x {
            self.max.x = p.x;
        }
        if p.y > self.max.y {
            self.max.y = p.y;
        }
        if p.z > self.max.z {
            self.max.z = p.z;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point3_origin_is_zero() {
        assert_eq!(Point3::ORIGIN.x, 0.0);
        assert_eq!(Point3::ORIGIN.y, 0.0);
        assert_eq!(Point3::ORIGIN.z, 0.0);
    }

    #[test]
    fn transform_identity_has_unit_axes_at_origin() {
        let t = Transform3::IDENTITY;
        assert_eq!(t.origin, Point3::ORIGIN);
        assert_eq!(t.x_axis, Vector3::X_AXIS);
        assert_eq!(t.y_axis, Vector3::Y_AXIS);
        assert_eq!(t.z_axis, Vector3::Z_AXIS);
        assert_eq!(t.scale, 1.0);
    }

    #[test]
    fn curve_line_stores_endpoints() {
        let c = Curve::Line {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 2.0, 3.0),
        };
        match c {
            Curve::Line { start, end } => {
                assert_eq!(start.x, 0.0);
                assert_eq!(end.z, 3.0);
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn solid_extrusion_builds() {
        let profile = CurveLoop {
            curves: vec![Curve::Line {
                start: Point3::ORIGIN,
                end: Point3::new(10.0, 0.0, 0.0),
            }],
            closed: false,
        };
        let s = Solid::Extrusion {
            profile,
            direction: Vector3::Z_AXIS,
            distance: 5.0,
            placement: Transform3::IDENTITY,
        };
        match s {
            Solid::Extrusion { distance, .. } => assert_eq!(distance, 5.0),
            _ => panic!("expected Extrusion"),
        }
    }

    #[test]
    fn boolean_op_variants_are_distinct() {
        assert_ne!(BooleanOp::Union, BooleanOp::Difference);
        assert_ne!(BooleanOp::Difference, BooleanOp::Intersection);
    }

    #[test]
    fn bbox_empty_starts_inverted() {
        let b = BoundingBox::empty();
        assert!(b.is_empty());
    }

    #[test]
    fn bbox_expand_tracks_extent() {
        let mut b = BoundingBox::empty();
        b.expand_point(Point3::new(1.0, 2.0, 3.0));
        b.expand_point(Point3::new(-5.0, 10.0, 0.0));
        assert!(!b.is_empty());
        assert_eq!(b.min.x, -5.0);
        assert_eq!(b.max.y, 10.0);
    }

    #[test]
    fn mesh_and_pointcloud_serialize() {
        // Sanity: serde serializes the nested types without panic.
        let m = Mesh {
            vertices: vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)],
            triangles: vec![[0, 1, 0]],
            normals: vec![],
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"triangles\""));

        let pc = PointCloud {
            points: vec![Point3::ORIGIN],
        };
        let json = serde_json::to_string(&pc).unwrap();
        assert!(json.contains("\"points\""));
    }
}
