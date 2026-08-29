//! Lane Six — recover wall curves, floor loops, opening hosts, and
//! level elevations from typed element views / decoded fields.
//!
//! # Honesty contract
//!
//! - Never invent coordinates, host ids, or elevations.
//! - Missing or unresolvable data yields [`RecoveryOutcome::Absent`]
//!   with a stable diagnostic code — not a fabricated default.
//! - Wrong / empty inputs must not panic.
//!
//! Synthetic `gen-fixture` / tier1 CFBs are scaffold-oriented and
//! typically lack location curves and floor sketches; those paths
//! correctly return `Absent`. ArcWall partition records (Revit 2023
//! standard) do carry centerline endpoints and recover as
//! [`WallLocationSource::ArcWallPartition`].

use crate::elements::arc_wall::ArcWall;
use crate::elements::floor::Floor;
use crate::elements::level::{Level, normalise_field_name};
use crate::elements::openings::{Door, Window};
use crate::elements::wall::Wall;
use crate::geometry::{Curve, CurveLoop, Point3};
use crate::walker::{DecodedElement, InstanceField};

/// Stable machine-readable reason a recovery path failed closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryDiagnostic {
    pub code: &'static str,
    pub message: String,
}

impl GeometryDiagnostic {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Result of a geometry recovery attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryOutcome<T> {
    Recovered(T),
    Absent { diagnostic: GeometryDiagnostic },
}

impl<T> RecoveryOutcome<T> {
    /// `Some` when recovery succeeded.
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Recovered(v) => Some(v),
            Self::Absent { .. } => None,
        }
    }

    /// Borrow the recovered value when present.
    pub fn as_recovered(&self) -> Option<&T> {
        match self {
            Self::Recovered(v) => Some(v),
            Self::Absent { .. } => None,
        }
    }

    /// Diagnostic when recovery failed closed.
    pub fn diagnostic(&self) -> Option<&GeometryDiagnostic> {
        match self {
            Self::Recovered(_) => None,
            Self::Absent { diagnostic } => Some(diagnostic),
        }
    }

    pub fn is_recovered(&self) -> bool {
        matches!(self, Self::Recovered(_))
    }
}

// ---------------------------------------------------------------------------
// Wall location curves
// ---------------------------------------------------------------------------

/// Where a wall location curve was recovered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallLocationSource {
    /// Standard ArcWall partition record centerline (RE-14.3).
    ArcWallPartition,
    /// Explicit start/end XYZ (or XY) floats on a schema-driven Wall.
    SchemaEndpoints,
}

/// Recovered wall location curve in project coordinates (feet).
#[derive(Debug, Clone, PartialEq)]
pub struct WallLocationCurve {
    pub curve: Curve,
    pub source: WallLocationSource,
    /// Handle into the document curve table when present; geometry may
    /// still be unresolved when only the id was decoded.
    pub location_curve_id: Option<u32>,
}

impl WallLocationCurve {
    /// Plan-view endpoints for straight lines. `None` for arcs / other.
    pub fn line_endpoints_xy(&self) -> Option<([f64; 2], [f64; 2])> {
        match &self.curve {
            Curve::Line { start, end } => Some(([start.x, start.y], [end.x, end.y])),
            _ => None,
        }
    }

    /// Segment length in feet for straight location lines.
    pub fn line_length_feet(&self) -> Option<f64> {
        let (start, end) = self.line_endpoints_xy()?;
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        Some((dx * dx + dy * dy).sqrt())
    }
}

/// Recover a location curve from a typed ArcWall partition decode.
///
/// Always succeeds for a valid [`ArcWall`]: the core record carries
/// start/end XYZ under RE-14.3 H16.
pub fn recover_wall_location_curve_from_arc_wall(wall: &ArcWall) -> WallLocationCurve {
    let (sx, sy, sz) = wall.start_point();
    let (ex, ey, ez) = wall.end_point();
    WallLocationCurve {
        curve: Curve::Line {
            start: Point3::new(sx, sy, sz),
            end: Point3::new(ex, ey, ez),
        },
        source: WallLocationSource::ArcWallPartition,
        location_curve_id: None,
    }
}

/// Recover a wall location curve from schema-driven decoded fields.
///
/// Looks for start/end XYZ (or XY) float fields and optional
/// `location_curve` ElementId. A bare curve id without resolvable
/// endpoints fails closed — we do not invent a line.
pub fn recover_wall_location_curve(decoded: &DecodedElement) -> RecoveryOutcome<WallLocationCurve> {
    let curve_id = find_location_curve_id(decoded);
    if let Some(curve) = endpoints_from_fields(decoded) {
        return RecoveryOutcome::Recovered(WallLocationCurve {
            curve,
            source: WallLocationSource::SchemaEndpoints,
            location_curve_id: curve_id,
        });
    }
    if let Some(id) = curve_id {
        return RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "wall_location_curve_unresolved_handle",
                format!(
                    "Wall carries location_curve_id={id} but no start/end endpoints were decoded"
                ),
            ),
        };
    }
    RecoveryOutcome::Absent {
        diagnostic: GeometryDiagnostic::new(
            "wall_location_curve_missing",
            "no location curve endpoints or curve handle on decoded Wall fields",
        ),
    }
}

/// Prefer typed [`Wall`] endpoint projection, then fall back to raw fields.
pub fn recover_wall_location_curve_from_wall(
    wall: &Wall,
    decoded: &DecodedElement,
) -> RecoveryOutcome<WallLocationCurve> {
    if let (Some(start), Some(end)) = (wall.location_start, wall.location_end) {
        return RecoveryOutcome::Recovered(WallLocationCurve {
            curve: Curve::Line { start, end },
            source: WallLocationSource::SchemaEndpoints,
            location_curve_id: wall.location_curve_id,
        });
    }
    let mut outcome = recover_wall_location_curve(decoded);
    if let RecoveryOutcome::Recovered(ref mut loc) = outcome {
        if loc.location_curve_id.is_none() {
            loc.location_curve_id = wall.location_curve_id;
        }
    } else if let Some(id) = wall.location_curve_id {
        return RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "wall_location_curve_unresolved_handle",
                format!(
                    "Wall carries location_curve_id={id} but no start/end endpoints were decoded"
                ),
            ),
        };
    }
    outcome
}

fn find_location_curve_id(decoded: &DecodedElement) -> Option<u32> {
    for (name, value) in &decoded.fields {
        let n = normalise_field_name(name);
        if matches!(
            n.as_str(),
            "locationcurveid" | "locationcurve" | "curvelineid" | "curveline"
        ) {
            if let InstanceField::ElementId { id, .. } = value {
                if *id != 0 {
                    return Some(*id);
                }
            }
        }
    }
    None
}

fn endpoints_from_fields(decoded: &DecodedElement) -> Option<Curve> {
    let mut start_x = None;
    let mut start_y = None;
    let mut start_z = None;
    let mut end_x = None;
    let mut end_y = None;
    let mut end_z = None;

    for (name, value) in &decoded.fields {
        let n = normalise_field_name(name);
        let InstanceField::Float { value, .. } = value else {
            continue;
        };
        match n.as_str() {
            "startx" | "locationstartx" | "curvestartx" | "endpoint0x" => start_x = Some(*value),
            "starty" | "locationstarty" | "curvestarty" | "endpoint0y" => start_y = Some(*value),
            "startz" | "locationstartz" | "curvestartz" | "endpoint0z" => start_z = Some(*value),
            "endx" | "locationendx" | "curveendx" | "endpoint1x" => end_x = Some(*value),
            "endy" | "locationendy" | "curveendy" | "endpoint1y" => end_y = Some(*value),
            "endz" | "locationendz" | "curveendz" | "endpoint1z" => end_z = Some(*value),
            _ => {}
        }
    }

    let (sx, sy) = (start_x?, start_y?);
    let (ex, ey) = (end_x?, end_y?);
    // Degenerate zero-length lines are not a recovered location curve.
    if (ex - sx).abs() < f64::EPSILON && (ey - sy).abs() < f64::EPSILON {
        return None;
    }
    Some(Curve::Line {
        start: Point3::new(sx, sy, start_z.unwrap_or(0.0)),
        end: Point3::new(ex, ey, end_z.unwrap_or(0.0)),
    })
}

// ---------------------------------------------------------------------------
// Floor boundary loops
// ---------------------------------------------------------------------------

/// Where a floor boundary was recovered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorBoundarySource {
    /// Vector / float sequence on schema-driven Floor fields.
    SchemaSketch,
    /// Closed plan polyline recovered from partition bytes (RE-15-07),
    /// after ArcWall-centerline exclusion.
    PartitionPlanLoop,
}

/// Recovered floor/slab plan boundary (feet).
#[derive(Debug, Clone, PartialEq)]
pub struct FloorBoundaryLoop {
    /// Outer loop vertices in plan `(x, y)`. Not required to repeat
    /// the first point; [`Self::closed`] records closure.
    pub vertices_xy: Vec<(f64, f64)>,
    pub closed: bool,
    pub source: FloorBoundarySource,
}

impl FloorBoundaryLoop {
    /// Unsigned shoelace area in square feet.
    pub fn area_sqft(&self) -> f64 {
        polygon_area_xy(&self.vertices_xy)
    }

    /// Convert to a [`CurveLoop`] of straight segments (Z = 0).
    pub fn to_curve_loop(&self) -> CurveLoop {
        let n = self.vertices_xy.len();
        if n < 2 {
            return CurveLoop {
                curves: vec![],
                closed: self.closed,
            };
        }
        let mut curves = Vec::with_capacity(n);
        for i in 0..n.saturating_sub(1) {
            let (x0, y0) = self.vertices_xy[i];
            let (x1, y1) = self.vertices_xy[i + 1];
            curves.push(Curve::Line {
                start: Point3::new(x0, y0, 0.0),
                end: Point3::new(x1, y1, 0.0),
            });
        }
        if self.closed && n >= 3 {
            let (x0, y0) = self.vertices_xy[n - 1];
            let (x1, y1) = self.vertices_xy[0];
            curves.push(Curve::Line {
                start: Point3::new(x0, y0, 0.0),
                end: Point3::new(x1, y1, 0.0),
            });
        }
        CurveLoop {
            curves,
            closed: self.closed,
        }
    }
}

/// Recover a floor boundary loop from decoded schema fields.
///
/// Accepts:
/// - A `Vector` of floats interpreted as XY or XYZ interleaved points
/// - A `Vector` of nested 2-/3-float point vectors
///
/// Requires ≥ 3 unique vertices and non-zero area. Degenerate or
/// missing sketches fail closed.
pub fn recover_floor_boundary(decoded: &DecodedElement) -> RecoveryOutcome<FloorBoundaryLoop> {
    let mut best: Option<Vec<(f64, f64)>> = None;

    for (name, value) in &decoded.fields {
        let n = normalise_field_name(name);
        let sketch_like = matches!(
            n.as_str(),
            "boundary"
                | "boundaryloop"
                | "sketch"
                | "profile"
                | "outline"
                | "polyline"
                | "loop"
                | "outerloop"
                | "floorboundary"
        ) || n.contains("boundary")
            || n.contains("sketch")
            || n.contains("profile");

        if let Some(pts) = points_xy_from_field(value) {
            if pts.len() >= 3 && (sketch_like || best.is_none()) {
                best = Some(pts);
                if sketch_like {
                    break;
                }
            }
        }
    }

    let Some(vertices_xy) = best else {
        return RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "floor_boundary_missing",
                "no floor boundary / sketch polyline on decoded Floor fields",
            ),
        };
    };

    let unique = unique_plan_vertices(&vertices_xy);
    if unique.len() < 3 {
        return RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "floor_boundary_degenerate",
                format!(
                    "floor boundary has fewer than 3 unique vertices ({})",
                    unique.len()
                ),
            ),
        };
    }
    if polygon_area_xy(&unique) <= f64::EPSILON {
        return RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "floor_boundary_zero_area",
                "floor boundary encloses zero area",
            ),
        };
    }

    let closed = is_closed_loop(&vertices_xy);
    let source = if decoded.fields.iter().any(|(n, v)| {
        normalise_field_name(n) == "source"
            && matches!(v, InstanceField::String(s) if s == "partition_plan_loop")
    }) {
        FloorBoundarySource::PartitionPlanLoop
    } else {
        FloorBoundarySource::SchemaSketch
    };
    RecoveryOutcome::Recovered(FloorBoundaryLoop {
        vertices_xy: unique,
        closed,
        source,
    })
}

/// Typed Floor view does not yet carry sketch geometry; delegates to
/// [`recover_floor_boundary`] on the underlying decoded fields.
pub fn recover_floor_boundary_from_floor(
    _floor: &Floor,
    decoded: &DecodedElement,
) -> RecoveryOutcome<FloorBoundaryLoop> {
    recover_floor_boundary(decoded)
}

fn points_xy_from_field(value: &InstanceField) -> Option<Vec<(f64, f64)>> {
    match value {
        InstanceField::Vector(items) => {
            if let Some(pts) = points_from_nested_vectors(items) {
                return Some(pts);
            }
            points_from_flat_floats(items)
        }
        _ => None,
    }
}

fn points_from_nested_vectors(items: &[InstanceField]) -> Option<Vec<(f64, f64)>> {
    if items.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item {
            InstanceField::Vector(coords) => {
                let floats: Vec<f64> = coords
                    .iter()
                    .filter_map(|c| match c {
                        InstanceField::Float { value, .. } => Some(*value),
                        _ => None,
                    })
                    .collect();
                if floats.len() >= 2 {
                    out.push((floats[0], floats[1]));
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if out.len() >= 3 { Some(out) } else { None }
}

fn points_from_flat_floats(items: &[InstanceField]) -> Option<Vec<(f64, f64)>> {
    let floats: Vec<f64> = items
        .iter()
        .filter_map(|c| match c {
            InstanceField::Float { value, .. } => Some(*value),
            _ => None,
        })
        .collect();
    if floats.len() < 6 {
        return None;
    }
    if floats.len() % 3 == 0 {
        let mut pts = Vec::with_capacity(floats.len() / 3);
        for chunk in floats.chunks_exact(3) {
            pts.push((chunk[0], chunk[1]));
        }
        if pts.len() >= 3 {
            return Some(pts);
        }
    }
    if floats.len() % 2 == 0 {
        let mut pts = Vec::with_capacity(floats.len() / 2);
        for chunk in floats.chunks_exact(2) {
            pts.push((chunk[0], chunk[1]));
        }
        if pts.len() >= 3 {
            return Some(pts);
        }
    }
    None
}

fn unique_plan_vertices(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for &(x, y) in pts {
        let dup = out
            .iter()
            .any(|&(ox, oy)| (ox - x).abs() < 1e-9 && (oy - y).abs() < 1e-9);
        if !dup {
            out.push((x, y));
        }
    }
    if out.len() >= 2 {
        let (fx, fy) = out[0];
        let (lx, ly) = out[out.len() - 1];
        if (fx - lx).abs() < 1e-9 && (fy - ly).abs() < 1e-9 {
            out.pop();
        }
    }
    out
}

fn is_closed_loop(pts: &[(f64, f64)]) -> bool {
    if pts.len() < 2 {
        return false;
    }
    let (fx, fy) = pts[0];
    let (lx, ly) = pts[pts.len() - 1];
    (fx - lx).abs() < 1e-9 && (fy - ly).abs() < 1e-9
}

fn polygon_area_xy(points: &[(f64, f64)]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let n = points.len();
    let mut sum = 0.0;
    for i in 0..n {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % n];
        sum += x0 * y1 - x1 * y0;
    }
    (sum * 0.5).abs()
}

// ---------------------------------------------------------------------------
// Door / window host relationships
// ---------------------------------------------------------------------------

/// Kind of wall-hosted opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningKind {
    Door,
    Window,
}

/// Host relationship for a door or window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningHostRelationship {
    pub opening_kind: OpeningKind,
    /// Host wall (or other host) ElementId. Never zero.
    pub host_element_id: u32,
    pub level_id: Option<u32>,
    pub symbol_id: Option<u32>,
}

/// Recover the host wall id from a typed [`Door`].
///
/// `host_id` missing or `0` fails closed — Revit uses 0 for
/// unhosted / unset.
pub fn recover_door_host(door: &Door) -> RecoveryOutcome<OpeningHostRelationship> {
    recover_opening_host(
        OpeningKind::Door,
        door.host_id,
        door.level_id,
        door.symbol_id,
    )
}

/// Recover the host wall id from a typed [`Window`].
pub fn recover_window_host(window: &Window) -> RecoveryOutcome<OpeningHostRelationship> {
    recover_opening_host(
        OpeningKind::Window,
        window.host_id,
        window.level_id,
        window.symbol_id,
    )
}

fn recover_opening_host(
    kind: OpeningKind,
    host_id: Option<u32>,
    level_id: Option<u32>,
    symbol_id: Option<u32>,
) -> RecoveryOutcome<OpeningHostRelationship> {
    match host_id {
        Some(0) => RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "opening_host_unset",
                format!("{kind:?} host_id is 0 (unhosted / unset)"),
            ),
        },
        Some(id) => RecoveryOutcome::Recovered(OpeningHostRelationship {
            opening_kind: kind,
            host_element_id: id,
            level_id,
            symbol_id,
        }),
        None => RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "opening_host_missing",
                format!("{kind:?} has no host_id field"),
            ),
        },
    }
}

// ---------------------------------------------------------------------------
// Level elevations
// ---------------------------------------------------------------------------

/// Recovered level elevation in project feet.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelElevation {
    pub name: Option<String>,
    pub elevation_feet: f64,
    pub is_building_story: Option<bool>,
    pub level_type_id: Option<u32>,
}

/// Recover elevation from a typed [`Level`].
///
/// Missing elevation fails closed — callers must not assume 0.0.
pub fn recover_level_elevation(level: &Level) -> RecoveryOutcome<LevelElevation> {
    match level.elevation_feet {
        Some(elevation_feet) => RecoveryOutcome::Recovered(LevelElevation {
            name: level.name.clone(),
            elevation_feet,
            is_building_story: level.is_building_story,
            level_type_id: level.level_type_id,
        }),
        None => RecoveryOutcome::Absent {
            diagnostic: GeometryDiagnostic::new(
                "level_elevation_missing",
                "Level has no elevation/height field",
            ),
        },
    }
}

/// Recover elevations for a slice of levels, preserving input order.
///
/// Entries without elevation appear as [`RecoveryOutcome::Absent`].
pub fn recover_level_elevations(levels: &[Level]) -> Vec<RecoveryOutcome<LevelElevation>> {
    levels.iter().map(recover_level_elevation).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arc_wall_record::{
        ARC_WALL_TAG, ARC_WALL_VARIANT_STANDARD, RECORD_TRAILER, SCHEMA_FAMILY_MARKER,
        STANDARD_RECORD_MIN_SIZE,
    };

    fn empty_decoded(class: &str) -> DecodedElement {
        DecodedElement {
            id: None,
            class: class.into(),
            fields: vec![],
            byte_range: 0..0,
            provenance: Default::default(),
        }
    }

    fn synth_arc_wall_bytes() -> Vec<u8> {
        let mut buf = vec![0u8; STANDARD_RECORD_MIN_SIZE];
        buf[0..2].copy_from_slice(&ARC_WALL_TAG.to_le_bytes());
        buf[4..8].copy_from_slice(&SCHEMA_FAMILY_MARKER.to_le_bytes());
        buf[8..12].copy_from_slice(&1u32.to_le_bytes());
        buf[12..16].copy_from_slice(&3u32.to_le_bytes());
        buf[16..18].copy_from_slice(&ARC_WALL_VARIANT_STANDARD.to_le_bytes());
        let coords = [0.0_f64, 0.0, 0.0, 10.0, 0.0, 0.0];
        for (i, v) in coords.iter().enumerate() {
            let off = 0x12 + i * 8;
            buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
        }
        for (i, v) in coords.iter().enumerate() {
            let off = 0x42 + i * 8;
            buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
        }
        buf[0x72] = RECORD_TRAILER;
        buf
    }

    #[test]
    fn wall_curve_absent_on_empty_decoded() {
        let outcome = recover_wall_location_curve(&empty_decoded("Wall"));
        assert!(!outcome.is_recovered());
        assert_eq!(
            outcome.diagnostic().map(|d| d.code),
            Some("wall_location_curve_missing")
        );
    }

    #[test]
    fn wall_curve_from_schema_endpoints() {
        let decoded = DecodedElement {
            id: Some(1),
            class: "Wall".into(),
            fields: vec![
                (
                    "m_start_x".into(),
                    InstanceField::Float {
                        value: 0.0,
                        size: 8,
                    },
                ),
                (
                    "m_start_y".into(),
                    InstanceField::Float {
                        value: 0.0,
                        size: 8,
                    },
                ),
                (
                    "m_end_x".into(),
                    InstanceField::Float {
                        value: 12.0,
                        size: 8,
                    },
                ),
                (
                    "m_end_y".into(),
                    InstanceField::Float {
                        value: 5.0,
                        size: 8,
                    },
                ),
                (
                    "m_location_curve_id".into(),
                    InstanceField::ElementId { tag: 0, id: 99 },
                ),
            ],
            byte_range: 0..0,
            provenance: Default::default(),
        };
        let loc = recover_wall_location_curve(&decoded).ok().unwrap();
        assert_eq!(loc.source, WallLocationSource::SchemaEndpoints);
        assert_eq!(loc.location_curve_id, Some(99));
        assert!((loc.line_length_feet().unwrap() - 13.0).abs() < 1e-9);
    }

    #[test]
    fn wall_curve_unresolved_handle_fails_closed() {
        let decoded = DecodedElement {
            id: None,
            class: "Wall".into(),
            fields: vec![(
                "m_location_curve".into(),
                InstanceField::ElementId { tag: 0, id: 7 },
            )],
            byte_range: 0..0,
            provenance: Default::default(),
        };
        let outcome = recover_wall_location_curve(&decoded);
        assert_eq!(
            outcome.diagnostic().map(|d| d.code),
            Some("wall_location_curve_unresolved_handle")
        );
    }

    #[test]
    fn wall_curve_zero_length_rejected() {
        let decoded = DecodedElement {
            id: None,
            class: "Wall".into(),
            fields: vec![
                (
                    "m_start_x".into(),
                    InstanceField::Float {
                        value: 1.0,
                        size: 8,
                    },
                ),
                (
                    "m_start_y".into(),
                    InstanceField::Float {
                        value: 1.0,
                        size: 8,
                    },
                ),
                (
                    "m_end_x".into(),
                    InstanceField::Float {
                        value: 1.0,
                        size: 8,
                    },
                ),
                (
                    "m_end_y".into(),
                    InstanceField::Float {
                        value: 1.0,
                        size: 8,
                    },
                ),
            ],
            byte_range: 0..0,
            provenance: Default::default(),
        };
        assert!(!recover_wall_location_curve(&decoded).is_recovered());
    }

    #[test]
    fn wall_curve_from_typed_wall_endpoints() {
        let wall = Wall {
            location_start: Some(Point3::new(0.0, 0.0, 0.0)),
            location_end: Some(Point3::new(4.0, 0.0, 0.0)),
            location_curve_id: Some(3),
            ..Default::default()
        };
        let loc = recover_wall_location_curve_from_wall(&wall, &empty_decoded("Wall"))
            .ok()
            .unwrap();
        assert_eq!(loc.line_length_feet(), Some(4.0));
        assert_eq!(loc.location_curve_id, Some(3));
    }

    #[test]
    fn wall_curve_from_arc_wall_partition() {
        let wall =
            crate::elements::arc_wall::decode_at(&synth_arc_wall_bytes(), 0, Some("ArcWall"))
                .unwrap();
        let loc = recover_wall_location_curve_from_arc_wall(&wall);
        assert_eq!(loc.source, WallLocationSource::ArcWallPartition);
        assert_eq!(loc.line_length_feet(), Some(10.0));
        let (s, e) = loc.line_endpoints_xy().unwrap();
        assert_eq!(s, [0.0, 0.0]);
        assert_eq!(e, [10.0, 0.0]);
    }

    #[test]
    fn floor_boundary_absent_on_empty() {
        let outcome = recover_floor_boundary(&empty_decoded("Floor"));
        assert_eq!(
            outcome.diagnostic().map(|d| d.code),
            Some("floor_boundary_missing")
        );
    }

    #[test]
    fn floor_boundary_from_flat_xyz_vector() {
        let mut floats = Vec::new();
        for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)] {
            floats.push(InstanceField::Float { value: x, size: 8 });
            floats.push(InstanceField::Float { value: y, size: 8 });
            floats.push(InstanceField::Float {
                value: 0.0,
                size: 8,
            });
        }
        let decoded = DecodedElement {
            id: None,
            class: "Floor".into(),
            fields: vec![("m_boundary".into(), InstanceField::Vector(floats))],
            byte_range: 0..0,
            provenance: Default::default(),
        };
        let loop_ = recover_floor_boundary(&decoded).ok().unwrap();
        assert_eq!(loop_.vertices_xy.len(), 4);
        assert!((loop_.area_sqft() - 80.0).abs() < 1e-9);
        assert!(loop_.to_curve_loop().curves.len() >= 3);
    }

    #[test]
    fn floor_boundary_degenerate_fails_closed() {
        let floats = vec![
            InstanceField::Float {
                value: 0.0,
                size: 8,
            },
            InstanceField::Float {
                value: 0.0,
                size: 8,
            },
            InstanceField::Float {
                value: 1.0,
                size: 8,
            },
            InstanceField::Float {
                value: 0.0,
                size: 8,
            },
            InstanceField::Float {
                value: 2.0,
                size: 8,
            },
            InstanceField::Float {
                value: 0.0,
                size: 8,
            },
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Floor".into(),
            fields: vec![("m_sketch".into(), InstanceField::Vector(floats))],
            byte_range: 0..0,
            provenance: Default::default(),
        };
        let outcome = recover_floor_boundary(&decoded);
        assert!(!outcome.is_recovered());
        assert_eq!(
            outcome.diagnostic().map(|d| d.code),
            Some("floor_boundary_zero_area")
        );
    }

    #[test]
    fn door_host_recovered() {
        let door = Door {
            host_id: Some(42),
            level_id: Some(1),
            symbol_id: Some(9),
            ..Default::default()
        };
        let rel = recover_door_host(&door).ok().unwrap();
        assert_eq!(rel.host_element_id, 42);
        assert_eq!(rel.opening_kind, OpeningKind::Door);
        assert_eq!(rel.level_id, Some(1));
    }

    #[test]
    fn door_host_zero_fails_closed() {
        let door = Door {
            host_id: Some(0),
            ..Default::default()
        };
        assert_eq!(
            recover_door_host(&door).diagnostic().map(|d| d.code),
            Some("opening_host_unset")
        );
    }

    #[test]
    fn window_host_missing_fails_closed() {
        let window = Window::default();
        assert_eq!(
            recover_window_host(&window).diagnostic().map(|d| d.code),
            Some("opening_host_missing")
        );
    }

    #[test]
    fn level_elevation_recovered() {
        let level = Level {
            name: Some("Level 1".into()),
            elevation_feet: Some(10.0),
            is_building_story: Some(true),
            level_type_id: Some(3),
        };
        let elev = recover_level_elevation(&level).ok().unwrap();
        assert_eq!(elev.elevation_feet, 10.0);
        assert_eq!(elev.name.as_deref(), Some("Level 1"));
    }

    #[test]
    fn level_elevation_missing_fails_closed() {
        let level = Level {
            name: Some("Draft".into()),
            ..Default::default()
        };
        assert_eq!(
            recover_level_elevation(&level).diagnostic().map(|d| d.code),
            Some("level_elevation_missing")
        );
    }

    #[test]
    fn recover_level_elevations_preserves_order() {
        let levels = [
            Level {
                elevation_feet: Some(0.0),
                name: Some("L1".into()),
                ..Default::default()
            },
            Level {
                name: Some("missing".into()),
                ..Default::default()
            },
            Level {
                elevation_feet: Some(12.0),
                name: Some("L2".into()),
                ..Default::default()
            },
        ];
        let out = recover_level_elevations(&levels);
        assert!(out[0].is_recovered());
        assert!(!out[1].is_recovered());
        assert_eq!(out[2].as_recovered().map(|e| e.elevation_feet), Some(12.0));
    }

    #[test]
    fn wrong_inputs_do_not_panic() {
        let _ = recover_wall_location_curve(&empty_decoded(""));
        let _ = recover_floor_boundary(&empty_decoded("NotFloor"));
        let _ = recover_door_host(&Door::default());
        let _ = recover_window_host(&Window::default());
        let _ = recover_level_elevation(&Level::default());
        let _ = recover_wall_location_curve_from_wall(&Wall::default(), &empty_decoded("Wall"));
        let _ = recover_floor_boundary_from_floor(&Floor::default(), &empty_decoded("Floor"));
    }
}
