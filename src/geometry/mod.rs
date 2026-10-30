//! Geometry primitives, and the recovery of element geometry from typed views.
//!
//! Primitive types ([`Point3`], [`Curve`], [`CurveLoop`]) are what
//! [`recovery`] returns. It turns typed Wall / Floor / Door / Window /
//! Level views (and ArcWall partition records) into location curves,
//! boundary loops, host relationships, and elevations — fail-closed
//! when data is absent.
//!
//! Primitives are intentionally format-agnostic: no Revit-specific
//! type IDs or wire-level detail. Coordinates are just `f64` triples;
//! curves describe themselves parametrically.
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

pub mod recovery;

pub use recovery::{
    FloorBoundaryLoop, FloorBoundarySource, GeometryDiagnostic, LevelElevation,
    OpeningHostRelationship, OpeningKind, RecoveryOutcome, WallLocationCurve, WallLocationSource,
    recover_door_host, recover_floor_boundary, recover_floor_boundary_from_floor,
    recover_level_elevation, recover_level_elevations, recover_wall_location_curve,
    recover_wall_location_curve_from_arc_wall, recover_wall_location_curve_from_wall,
    recover_window_host,
};

/// 3D point in project coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
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
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
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
#[non_exhaustive]
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
