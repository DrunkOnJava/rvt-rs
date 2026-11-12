//! Areas' regions from Revit's own plan topology (#1261, RE-333).
//!
//! Revit keeps, for each area scheme on each Level, a `LevelRoomPlan`
//! element whose `PlanTopology` is the arrangement of that scheme's area
//! boundary lines as Revit computed it:
//!
//! - `m_segments`: each line cut where others meet it, keyed by the line's
//!   ElementId, a major index (0 for an area boundary line) and a sub index,
//!   with the piece's parameter range on the line (`m_param0`, `m_param1`,
//!   0 to 1 along the line) and its points;
//! - `m_circuits` (`PlanCircuit`): each closed loop of segments, with its
//!   area (`m_area`, square feet, the loop alone, its holes not taken out),
//!   its id and its sides, each a segment key whose sub index is `-k - 1`
//!   where the loop runs segment `k` backwards;
//! - `m_components`: which circuit is the outer loop of a connected set of
//!   lines, and which circuit, if any, encloses that set (the holes);
//! - `m_rooms`: the Areas the plan holds.
//!
//! Each Area (`RoomElem`) names its loop, `m_cachedCircuitId`. A line's
//! segments are the line itself; the Area's edge is the line moved to its
//! right by the area rule's offset over each part of the line
//! (`AreaMeasureCurveData.m_schemeSegments`, `m_offset`, feet: half a
//! wall's thickness, say, where the scheme measures to a wall's face), and
//! consecutive edges meet where their curves cross. That is why lines that
//! do not meet in plan (an offset line beside one that has none) still close
//! a region, which the arrangement of their boxes (RE-317) does not.
//!
//! Measured: on every save of Autodesk's `rac_advanced` sample from 2015 to
//! 2027 the region rebuilt this way has the area of its circuit to 1e-9 on
//! all 25 Areas, an arc's among them, and holds the Area's point; on the
//! BIM4LCA office (as built and at building permit, Revit 2024) on all 30,
//! and the region less its holes has the area of Revit's own export of the
//! Area to 1e-9. Every region read here is checked against the circuit's
//! area and the Area's point, and an Area whose region fails a check has
//! none (fail closed).

use crate::RevitFile;
use crate::graph_value::Fv;
use crate::ifc::element_provenance::SourcePart;
use crate::native_json::identifier_i64;
use crate::research::native_document;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

/// The largest angle one chord of an arc spans where an outline is tested
/// for a point (2°, which keeps the chord within 0.02% of the radius of the
/// arc). Only the test uses chords: a region's outline is written with its
/// arcs as arcs.
const POINT_TEST_ARC_STEP_RADIANS: f64 = PI / 90.0;

/// How far an arc's middle may lie from its chord, feet, for the arc to be
/// written as a line: the IFC writer rounds coordinates to a micrometre
/// (3.3e-6 ft), and a middle that close to the chord can round onto it, which
/// leaves three collinear points for an arc (as `family_extrusions` has it).
pub const FLAT_ARC_SAGITTA_FEET: f64 = 1.6e-5;

/// The relative difference allowed between a rebuilt loop's area and the
/// area Revit saves for its circuit.
pub const CIRCUIT_AREA_TOLERANCE: f64 = 1e-6;

/// How close, along a line from 0 to 1, a change of offset may lie to a
/// segment's end and still cut it.
pub const PARAMETER_TOLERANCE: f64 = 1e-9;

/// How far, feet, the curve of a segment may lie from the points its plan
/// topology saves for it.
pub const SEGMENT_POINT_TOLERANCE_FEET: f64 = 1e-6;

/// The most sides a circuit may have to be read: a loop of a room is never
/// near this many. Also the most edges the sides of one Area's region, its
/// outer loop and its holes together, may make once cut where their area
/// rule's offset changes: a region over it is refused before it is built, so
/// that a crafted topology cannot ask for sides times parts edges.
pub const MAX_CIRCUIT_SIDES: usize = 10_000;

/// The most offset parts the lines of one Area's region may have, summed over
/// the sides of its outer loop and its holes (a line run twice counts twice).
/// Cutting a side is quadratic in its parts, so this bounds the work of a
/// region.
pub const MAX_CIRCUIT_OFFSET_PARTS: usize = 20_000;

/// The most holes a region may have to be read. A component enclosed by the
/// circuit is a hole once, however often a plan topology lists it.
pub const MAX_REGION_HOLES: usize = 1_000;

/// How close, feet, two segments' ends must be to be the same node of a plan
/// topology (RE-317's node tolerance).
pub const NODE_TOLERANCE_FEET: f64 = 1e-5;

/// The side, feet, of the cells the plan's nodes are kept in to be found by
/// position: much wider than [`NODE_TOLERANCE_FEET`].
const NODE_CELL_FEET: f64 = 1e-3;

/// The most parts a line's area rule may split it into: `offset_at` is
/// searched for every cut.
pub const MAX_OFFSET_PARTS: usize = 1_000;

/// The Area is in no plan topology, or in more than one.
pub const REFUSED_NO_PLAN: &str = "area_not_in_one_plan_topology";
/// The circuit the Area names is not one of its plan's.
pub const REFUSED_NO_CIRCUIT: &str = "area_circuit_not_found";
/// A side of the circuit has no segment, no area boundary line, or a curve
/// that is not a line or an arc, or does not lie on the segment's points.
pub const REFUSED_SIDE: &str = "area_circuit_side_not_read";
/// The rebuilt loop does not have the circuit's area, whether as the edges
/// are rebuilt or as the outline is written, arcs as arcs.
pub const REFUSED_AREA: &str = "area_circuit_area_differs";
/// An arc of the rebuilt loop cannot be written: its centre, radius or sweep
/// is not finite, or it turns more than once.
pub const REFUSED_ARC: &str = "area_arc_not_finite";
/// The Area's point is not inside the rebuilt region.
pub const REFUSED_POINT: &str = "area_point_outside_circuit";
/// The region is over the bounds of [`MAX_CIRCUIT_SIDES`],
/// [`MAX_CIRCUIT_OFFSET_PARTS`] or [`MAX_REGION_HOLES`], so it is not built.
pub const REFUSED_TOO_LARGE: &str = "area_circuit_too_large";

/// An Area's region: its outer loop and holes in plan feet, and the
/// ElementIds of the lines that bound it.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaRegion {
    /// The outer loop's vertices, counter-clockwise, without a repeated
    /// closing vertex. The ends of an arc are vertices; so is the point where
    /// an arc of more than a half turn is cut in two.
    pub outer: Vec<(f64, f64)>,
    /// For each edge of the outer loop, from a vertex to the next (the last
    /// to the first), the middle point of the arc it is, or `None` for a line.
    pub outer_arcs: Vec<Option<(f64, f64)>>,
    /// The loops directly inside it, clockwise.
    pub holes: Vec<Vec<(f64, f64)>>,
    /// The arcs of each hole, as `outer_arcs` has them.
    pub hole_arcs: Vec<Vec<Option<(f64, f64)>>>,
    /// The ElementIds of the lines that bound the region, its holes' too.
    pub line_ids: BTreeSet<u32>,
    /// The native records the region was read from, kept only while the handle
    /// records value sources (#1368): the Area's `RoomElem` (its circuit and
    /// point), the `LevelRoomPlan` that holds the circuit and each bounding
    /// line's `CurveElem` (its curve and its area rule's offsets), each with
    /// the partition and offset of its current copy. Empty where the handle
    /// does not record them or one of the records has no place in the
    /// partition's own bytes: a list that left a record out would not be the
    /// records the region was read from.
    pub records: Vec<SourcePart>,
}

impl AreaRegion {
    /// The area of the outline less its holes, as the loops are written (an
    /// arc is an arc, not its chord), or `None` where an arc is degenerate.
    pub fn net_area_feet2(&self) -> Option<f64> {
        let mut area = written_area(&self.outer, &self.outer_arcs)?;
        for (hole, arcs) in self.holes.iter().zip(&self.hole_arcs) {
            area += written_area(hole, arcs)?;
        }
        Some(area)
    }
}

/// A circular arc through three points, found from them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreePointArc {
    /// The circle's centre.
    pub center: (f64, f64),
    /// The circle's radius.
    pub radius: f64,
    /// The angle of the first point about the centre.
    pub start: f64,
    /// The signed angle the arc turns from the first point through the
    /// second to the third: positive counter-clockwise, at most a full turn.
    pub sweep: f64,
}

/// The arc from `p` through `m` to `q`, or `None` where the three points are
/// collinear, coincide or are not finite.
pub fn three_point_arc(p: (f64, f64), m: (f64, f64), q: (f64, f64)) -> Option<ThreePointArc> {
    let (bx, by) = (m.0 - p.0, m.1 - p.1);
    let (cx, cy) = (q.0 - p.0, q.1 - p.1);
    let d = 2.0 * (bx * cy - by * cx);
    if !d.is_finite() || d == 0.0 {
        return None;
    }
    let (b2, c2) = (bx * bx + by * by, cx * cx + cy * cy);
    let center = (p.0 + (cy * b2 - by * c2) / d, p.1 + (bx * c2 - cx * b2) / d);
    let radius = distance(center, p);
    if !(center.0.is_finite() && center.1.is_finite() && radius.is_finite() && radius > 0.0) {
        return None;
    }
    let counter_clockwise = d > 0.0;
    let start = (p.1 - center.1).atan2(p.0 - center.0);
    let end = (q.1 - center.1).atan2(q.0 - center.0);
    let turn = (end - start).rem_euclid(2.0 * PI);
    let sweep = if counter_clockwise {
        turn
    } else {
        turn - 2.0 * PI
    };
    Some(ThreePointArc {
        center,
        radius,
        start,
        sweep,
    })
}

/// The box `[min_x, min_y, max_x, max_y]` of the arc from `p` through `m` to
/// `q`, its extreme points included, or `None` where it is degenerate.
pub fn arc_bounds(p: (f64, f64), m: (f64, f64), q: (f64, f64)) -> Option<[f64; 4]> {
    let arc = three_point_arc(p, m, q)?;
    let mut bounds = [p.0.min(q.0), p.1.min(q.1), p.0.max(q.0), p.1.max(q.1)];
    for quarter in 0..4 {
        let angle = quarter as f64 * PI / 2.0;
        let along = if arc.sweep >= 0.0 {
            (angle - arc.start).rem_euclid(2.0 * PI)
        } else {
            (arc.start - angle).rem_euclid(2.0 * PI)
        };
        if along <= arc.sweep.abs() {
            let (x, y) = (
                arc.center.0 + arc.radius * angle.cos(),
                arc.center.1 + arc.radius * angle.sin(),
            );
            bounds = [
                bounds[0].min(x),
                bounds[1].min(y),
                bounds[2].max(x),
                bounds[3].max(y),
            ];
        }
    }
    Some(bounds)
}

/// The signed area of a loop as it is written: its vertices, and for each edge
/// the middle of its arc or `None` for a line, by Green's theorem, so an arc
/// counts as itself and not its chord. `None` where an arc is degenerate.
fn written_area(points: &[(f64, f64)], arcs: &[Option<(f64, f64)>]) -> Option<f64> {
    let mut area = 0.0;
    for (index, &p) in points.iter().enumerate() {
        let q = points[(index + 1) % points.len()];
        area += match arcs.get(index).copied().flatten() {
            None => (p.0 * q.1 - q.0 * p.1) / 2.0,
            Some(m) => {
                let arc = three_point_arc(p, m, q)?;
                (arc.radius * arc.radius * arc.sweep + arc.center.0 * (q.1 - p.1)
                    - arc.center.1 * (q.0 - p.0))
                    / 2.0
            }
        };
    }
    area.is_finite().then_some(area)
}

/// A loop turned round: its vertices in the other order, each edge's arc kept
/// (an arc is named by its middle, which does not depend on its direction).
fn reversed_loop(written: WrittenLoop) -> WrittenLoop {
    let n = written.points.len();
    if n == 0 {
        return written;
    }
    let points: Vec<(f64, f64)> = written.points.iter().rev().copied().collect();
    // Edge `j` of the turned loop joins turned[j] = old[n - 1 - j] to
    // turned[j + 1] = old[n - 2 - j]: the old edge `n - 2 - j`, taken mod n.
    let arcs = (0..n)
        .map(|j| written.arcs.get((2 * n - 2 - j) % n).copied().flatten())
        .collect();
    WrittenLoop { points, arcs }
}

/// A segment key: the line's ElementId, the major and the sub index.
type SegmentKey = (i64, i64, i64);

/// A segment of a plan topology: its parameter range and its two ends.
struct Segment {
    params: (f64, f64),
    ends: ((f64, f64), (f64, f64)),
}

/// One circuit: its saved area and its sides.
struct Circuit {
    area: f64,
    sides: Vec<SegmentKey>,
}

/// One plan topology: its segments, circuits, the circuits enclosed by each
/// circuit (holes), and its Areas.
#[derive(Default)]
struct Plan {
    /// The `LevelRoomPlan` record the topology was read from.
    record: Option<SourcePart>,
    segments: BTreeMap<SegmentKey, Segment>,
    circuits: BTreeMap<i64, Circuit>,
    holes: BTreeMap<i64, Vec<i64>>,
    areas: BTreeSet<i64>,
    /// The ends of the segments that bound a region (a segment no circuit
    /// runs only as a spur), by cell, to count the segments that meet at a
    /// node.
    nodes: BTreeMap<NodeCell, Vec<(f64, f64)>>,
}

/// A cell of the grid the plan's nodes are kept in.
type NodeCell = (i64, i64);

fn node_cell(point: (f64, f64)) -> NodeCell {
    (
        (point.0 / NODE_CELL_FEET).floor() as i64,
        (point.1 / NODE_CELL_FEET).floor() as i64,
    )
}

impl Plan {
    /// The number of ends of segments that bound a region at `point`: 2 where
    /// a line runs on through it or two lines meet end to end, more where a
    /// third meets them (RE-317: that point is a vertex of an outline).
    fn degree_at(&self, point: (f64, f64)) -> usize {
        let (cx, cy) = node_cell(point);
        let mut count = 0;
        for x in cx.saturating_sub(1)..=cx.saturating_add(1) {
            for y in cy.saturating_sub(1)..=cy.saturating_add(1) {
                count += self.nodes.get(&(x, y)).map_or(0, |ends| {
                    ends.iter()
                        .filter(|end| distance(**end, point) <= NODE_TOLERANCE_FEET)
                        .count()
                });
            }
        }
        count
    }
}

/// What an Area saves of its place: its circuit and its point.
struct AreaPlace {
    circuit: i64,
    point: (f64, f64),
    /// The Area's `RoomElem` record.
    record: Option<SourcePart>,
}

/// An area boundary line's curve in plan.
#[derive(Clone, Copy)]
enum Curve {
    /// `origin + t · dir` for `t` in `ends`.
    Line {
        origin: (f64, f64),
        dir: (f64, f64),
        ends: (f64, f64),
    },
    /// `center + radius · (cos t · x + sin t · y)` for `t` in `ends`.
    Arc {
        center: (f64, f64),
        radius: f64,
        x: (f64, f64),
        y: (f64, f64),
        ends: (f64, f64),
    },
}

/// An area boundary line: its curve and its offsets, `(from, to, offset)`
/// over its parameter, 0 to 1.
struct BoundaryCurve {
    curve: Curve,
    offsets: Vec<(f64, f64, f64)>,
    /// The line's `CurveElem` record.
    record: Option<SourcePart>,
}

impl Curve {
    /// The point at `u` (0 to 1 along the curve) and the curve's direction
    /// there, not normalised.
    fn at(&self, u: f64) -> ((f64, f64), (f64, f64)) {
        match *self {
            Curve::Line { origin, dir, ends } => {
                let t = ends.0 + u * (ends.1 - ends.0);
                let span = ends.1 - ends.0;
                (
                    (origin.0 + t * dir.0, origin.1 + t * dir.1),
                    (dir.0 * span, dir.1 * span),
                )
            }
            Curve::Arc {
                center,
                radius,
                x,
                y,
                ends,
            } => {
                let t = ends.0 + u * (ends.1 - ends.0);
                let span = ends.1 - ends.0;
                let (c, s) = (t.cos(), t.sin());
                (
                    (
                        center.0 + radius * (c * x.0 + s * y.0),
                        center.1 + radius * (c * x.1 + s * y.1),
                    ),
                    (
                        radius * span * (-s * x.0 + c * y.0),
                        radius * span * (-s * x.1 + c * y.1),
                    ),
                )
            }
        }
    }
}

/// One edge of a rebuilt loop, already moved by its offset: a line from
/// `from` to `to`, or an arc about `center` of `radius` from angle `start`
/// through `sweep` (counter-clockwise where positive).
#[derive(Clone, Copy)]
enum Edge {
    Line {
        from: (f64, f64),
        to: (f64, f64),
    },
    Arc {
        center: (f64, f64),
        radius: f64,
        start: f64,
        sweep: f64,
    },
}

impl Edge {
    fn start(&self) -> (f64, f64) {
        match *self {
            Edge::Line { from, .. } => from,
            Edge::Arc {
                center,
                radius,
                start,
                ..
            } => (
                center.0 + radius * start.cos(),
                center.1 + radius * start.sin(),
            ),
        }
    }

    fn end(&self) -> (f64, f64) {
        match *self {
            Edge::Line { to, .. } => to,
            Edge::Arc {
                center,
                radius,
                start,
                sweep,
            } => (
                center.0 + radius * (start + sweep).cos(),
                center.1 + radius * (start + sweep).sin(),
            ),
        }
    }

    /// The edge moved so that it starts at `from`, its curve kept.
    fn starting_at(self, from: (f64, f64)) -> Edge {
        match self {
            Edge::Line { to, .. } => Edge::Line { from, to },
            Edge::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let new = unwrap_near((from.1 - center.1).atan2(from.0 - center.0), start);
                Edge::Arc {
                    center,
                    radius,
                    start: new,
                    sweep: start + sweep - new,
                }
            }
        }
    }

    /// The edge moved so that it ends at `to`, its curve kept.
    fn ending_at(self, to: (f64, f64)) -> Edge {
        match self {
            Edge::Line { from, .. } => Edge::Line { from, to },
            Edge::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let new = (to.1 - center.1).atan2(to.0 - center.0);
                Edge::Arc {
                    center,
                    radius,
                    start,
                    sweep: unwrap_near(new, start + sweep) - start,
                }
            }
        }
    }

    /// `x dy - y dx` integrated along the edge, halved: the edge's share of
    /// its loop's signed area (Green's theorem), exact for an arc.
    fn area_share(&self) -> f64 {
        let (a, b) = (self.start(), self.end());
        match *self {
            Edge::Line { .. } => (a.0 * b.1 - b.0 * a.1) / 2.0,
            Edge::Arc {
                center,
                radius,
                sweep,
                ..
            } => (radius * radius * sweep + center.0 * (b.1 - a.1) - center.1 * (b.0 - a.0)) / 2.0,
        }
    }
}

/// `angle` plus a whole number of turns, as close to `near` as it gets.
fn unwrap_near(angle: f64, near: f64) -> f64 {
    angle + ((near - angle) / (2.0 * PI)).round() * 2.0 * PI
}

/// The plan point of a two- or three-number array.
fn plan_point(value: &Fv) -> Option<(f64, f64)> {
    let items = value.as_array()?;
    let x = items.first()?.as_f64()?;
    let y = items.get(1)?.as_f64()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// A segment key as a plan topology saves it.
fn segment_key(value: &Fv) -> Option<SegmentKey> {
    let item = value.get("m_item")?;
    Some((
        identifier_i64(item.get("m_linkInstOrHostId")?)?,
        value.get("m_majorIndex")?.as_i64()?,
        value.get("m_subIndex")?.as_i64()?,
    ))
}

/// A circuit id, `{"m_id": n}`.
fn circuit_id(value: &Fv) -> Option<i64> {
    value.get("m_id")?.as_i64()
}

/// The parts of a `PlanTopology` read here.
fn plan(fields: &Fv) -> Option<Plan> {
    let mut out = Plan::default();
    for segment in fields.get("m_segments")?.as_array()? {
        let key = segment_key(segment.get("m_key")?)?;
        let info = segment.get("m_info")?;
        let points = info.get("m_points")?.as_array()?;
        let ends = (plan_point(points.first()?)?, plan_point(points.last()?)?);
        let params = (
            info.get("m_param0")?.as_f64()?,
            info.get("m_param1")?.as_f64()?,
        );
        out.segments.insert(key, Segment { params, ends });
    }
    for circuit in fields.get("m_circuits")?.as_array()? {
        let id = circuit_id(circuit.get("m_id")?)?;
        let sides = circuit
            .get("m_sides")?
            .as_array()?
            .iter()
            .map(segment_key)
            .collect::<Option<Vec<_>>>()?;
        let area = circuit.get("m_area")?.as_f64()?;
        out.circuits.insert(id, Circuit { area, sides });
    }
    // A hole is listed once, in the order its plan lists it, however often a
    // crafted plan repeats the component.
    let mut listed: BTreeSet<(i64, i64)> = BTreeSet::new();
    for component in fields.get("m_components")?.as_array()? {
        let enclosing = circuit_id(component.get("m_enclosingCircuit")?)?;
        let outer = circuit_id(component.get("m_outerCircuit")?)?;
        if enclosing >= 0 && listed.insert((enclosing, outer)) {
            out.holes.entry(enclosing).or_default().push(outer);
        }
    }
    for room in fields.get("m_rooms")?.as_array()? {
        out.areas.insert(identifier_i64(room)?);
    }
    let mut live: BTreeSet<SegmentKey> = BTreeSet::new();
    for circuit in out.circuits.values() {
        if circuit.sides.len() <= MAX_CIRCUIT_SIDES {
            live.extend(without_spurs(&circuit.sides).into_iter().map(segment_of));
        }
    }
    for key in live {
        if let Some(segment) = out.segments.get(&key) {
            for end in [segment.ends.0, segment.ends.1] {
                out.nodes.entry(node_cell(end)).or_default().push(end);
            }
        }
    }
    Some(out)
}

/// An area boundary line's curve and offsets, from its record's objects.
fn boundary_curve(objects: &[crate::native_parameters::GraphObject]) -> Option<BoundaryCurve> {
    let element = objects.iter().find(|o| o.class_name == "CurveElem")?;
    if element.fields.get("m_isAreaMeasurementScheme") != Some(&Fv::Bool(true)) {
        return None;
    }
    let ends = |fields: &Fv| -> Option<(f64, f64)> {
        let ends = fields.get("m_endParams")?.as_array()?;
        Some((ends.first()?.as_f64()?, ends.get(1)?.as_f64()?))
    };
    // An arc's record also holds a line of no length at its centre.
    let arcs: Vec<_> = objects.iter().filter(|o| o.class_name == "GArc").collect();
    let curve = match arcs.as_slice() {
        [arc] => Curve::Arc {
            center: plan_point(arc.fields.get("m_center")?)?,
            radius: arc.fields.get("m_radius")?.as_f64()?,
            x: plan_point(arc.fields.get("m_xVec")?)?,
            y: plan_point(arc.fields.get("m_yVec")?)?,
            ends: ends(&arc.fields)?,
        },
        [] => {
            let lines: Vec<_> = objects.iter().filter(|o| o.class_name == "GLine").collect();
            let [line] = lines.as_slice() else {
                return None;
            };
            Curve::Line {
                origin: plan_point(line.fields.get("m_origin")?)?,
                dir: plan_point(line.fields.get("m_dirVec")?)?,
                ends: ends(&line.fields)?,
            }
        }
        _ => return None,
    };
    let mut offsets = Vec::new();
    if let Some(data) = objects
        .iter()
        .find(|o| o.class_name == "AreaMeasureCurveData")
    {
        let parts = data.fields.get("m_schemeSegments")?.as_array()?;
        if parts.len() > MAX_OFFSET_PARTS {
            return None;
        }
        for part in parts {
            offsets.push((
                part.get("m_startParam")?.as_f64()?,
                part.get("m_endParam")?.as_f64()?,
                part.get("m_offset")?.as_f64()?,
            ));
        }
    }
    Some(BoundaryCurve {
        curve,
        offsets,
        record: None,
    })
}

impl BoundaryCurve {
    /// The area rule's offset at `u`, 0 where none is saved.
    fn offset_at(&self, u: f64) -> f64 {
        self.offsets
            .iter()
            .find(|(from, to, _)| *from <= u && u <= *to)
            .map_or(0.0, |part| part.2)
    }

    /// The edges of the curve from `u0` to `u1`, each moved to the curve's
    /// right by its offset, cut where the offset changes. Parts of the line
    /// with the same offset are one edge: a cut where nothing changes, at a
    /// segment's end, would leave an edge too short to have a direction.
    fn edges(&self, u0: f64, u1: f64) -> Option<Vec<Edge>> {
        let mut cuts = vec![u0, u1];
        for &(from, to, _) in &self.offsets {
            cuts.extend([from, to].into_iter().filter(|&u| {
                u - u0 > PARAMETER_TOLERANCE
                    && u1 - u > PARAMETER_TOLERANCE
                    && self.offset_at(u - PARAMETER_TOLERANCE)
                        != self.offset_at(u + PARAMETER_TOLERANCE)
            }));
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let mut out = Vec::new();
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let offset = self.offset_at((a + b) / 2.0);
            let moved = |u: f64| {
                let (p, d) = self.curve.at(u);
                let length = d.0.hypot(d.1);
                (length > 0.0).then(|| (p.0 + offset * d.1 / length, p.1 - offset * d.0 / length))
            };
            let edge = match self.curve {
                Curve::Line { .. } => Edge::Line {
                    from: moved(a)?,
                    to: moved(b)?,
                },
                Curve::Arc { center, .. } => {
                    let (from, to) = (moved(a)?, moved(b)?);
                    let radius = (from.0 - center.0).hypot(from.1 - center.1);
                    let start = (from.1 - center.1).atan2(from.0 - center.0);
                    let end = (to.1 - center.1).atan2(to.0 - center.0);
                    // The direction the arc runs, from its tangent at `a`.
                    let (_, d) = self.curve.at(a);
                    let ccw = (from.0 - center.0) * d.1 - (from.1 - center.1) * d.0 > 0.0;
                    let mut sweep = end - start;
                    if ccw && sweep <= 0.0 {
                        sweep += 2.0 * PI;
                    }
                    if !ccw && sweep >= 0.0 {
                        sweep -= 2.0 * PI;
                    }
                    if radius.is_nan() || radius <= 0.0 {
                        return None;
                    }
                    Edge::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    }
                }
            };
            out.push(edge);
        }
        Some(out)
    }
}

/// The edge reversed.
fn reversed(edge: Edge) -> Edge {
    match edge {
        Edge::Line { from, to } => Edge::Line { from: to, to: from },
        Edge::Arc {
            center,
            radius,
            start,
            sweep,
        } => Edge::Arc {
            center,
            radius,
            start: start + sweep,
            sweep: -sweep,
        },
    }
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The points where the curves of `a` and `b` (extended) cross.
fn crossings(a: &Edge, b: &Edge) -> Vec<(f64, f64)> {
    fn line_circle(p: (f64, f64), q: (f64, f64), c: (f64, f64), r: f64) -> Vec<(f64, f64)> {
        let d = (q.0 - p.0, q.1 - p.1);
        let f = (p.0 - c.0, p.1 - c.1);
        let (aa, bb, cc) = (
            d.0 * d.0 + d.1 * d.1,
            2.0 * (f.0 * d.0 + f.1 * d.1),
            f.0 * f.0 + f.1 * f.1 - r * r,
        );
        let disc = bb * bb - 4.0 * aa * cc;
        if aa == 0.0 || disc < 0.0 {
            return Vec::new();
        }
        let root = disc.sqrt();
        [(-bb - root) / (2.0 * aa), (-bb + root) / (2.0 * aa)]
            .into_iter()
            .map(|t| (p.0 + t * d.0, p.1 + t * d.1))
            .collect()
    }
    match (*a, *b) {
        (Edge::Line { from: p, to: q }, Edge::Line { from: s, to: t }) => {
            let d = (q.0 - p.0, q.1 - p.1);
            let e = (t.0 - s.0, t.1 - s.1);
            let den = d.0 * e.1 - d.1 * e.0;
            if den.abs() <= 1e-12 * d.0.hypot(d.1) * e.0.hypot(e.1) {
                return Vec::new();
            }
            let u = ((s.0 - p.0) * e.1 - (s.1 - p.1) * e.0) / den;
            vec![(p.0 + u * d.0, p.1 + u * d.1)]
        }
        (Edge::Line { from, to }, Edge::Arc { center, radius, .. })
        | (Edge::Arc { center, radius, .. }, Edge::Line { from, to }) => {
            line_circle(from, to, center, radius)
        }
        (
            Edge::Arc {
                center: c0,
                radius: r0,
                ..
            },
            Edge::Arc {
                center: c1,
                radius: r1,
                ..
            },
        ) => {
            let d = distance(c0, c1);
            if d == 0.0 || d > r0 + r1 || d < (r0 - r1).abs() {
                return Vec::new();
            }
            let a = (r0 * r0 - r1 * r1 + d * d) / (2.0 * d);
            let h = (r0 * r0 - a * a).max(0.0).sqrt();
            let m = (c0.0 + a * (c1.0 - c0.0) / d, c0.1 + a * (c1.1 - c0.1) / d);
            let o = (-(c1.1 - c0.1) * h / d, (c1.0 - c0.0) * h / d);
            vec![(m.0 + o.0, m.1 + o.1), (m.0 - o.0, m.1 - o.1)]
        }
    }
}

/// A segment key with its direction left out: the segment a side runs.
fn segment_of(side: SegmentKey) -> SegmentKey {
    let (line, major, sub) = side;
    (line, major, if sub < 0 { !sub } else { sub })
}

/// `sides` without its spurs: a segment run one way and, by the next side,
/// straight back, a line that ends inside the loop. Its two sides enclose
/// nothing, and once moved by its offset it would leave a notch that
/// Revit's area does not have (seen on the Snowdon Towers samples, RE-333). Cancelling inverse
/// neighbours, round the loop, is a stack pass; a circuit that is nothing but
/// spurs comes out empty.
fn without_spurs(sides: &[SegmentKey]) -> Vec<SegmentKey> {
    let inverse =
        |a: SegmentKey, b: SegmentKey| segment_of(a) == segment_of(b) && (a.2 < 0) != (b.2 < 0);
    let mut out: Vec<SegmentKey> = Vec::with_capacity(sides.len());
    for &side in sides {
        match out.last() {
            Some(&last) if inverse(last, side) => {
                out.pop();
            }
            _ => out.push(side),
        }
    }
    // The loop is a ring: its last side may cancel its first.
    let (mut from, mut to) = (0, out.len());
    while to - from >= 2 && inverse(out[from], out[to - 1]) {
        from += 1;
        to -= 1;
    }
    out[from..to].to_vec()
}

/// A loop of edges and, for each, whether its start is kept as a vertex of the
/// outline where the loop runs straight on through it.
struct Loop {
    edges: Vec<Edge>,
    keep: Vec<bool>,
}

/// What the circuits of one Area's region have used of the bounds on a
/// region ([`MAX_CIRCUIT_SIDES`], [`MAX_CIRCUIT_OFFSET_PARTS`]): shared by its
/// outer loop and its holes, so that many holes cannot each ask for a full
/// budget.
#[derive(Default)]
struct Budget {
    edges: usize,
    parts: usize,
    /// Whether a bound stopped a circuit, as opposed to a side not read.
    exceeded: bool,
}

/// The edges of one circuit, each moved by its offset and met by its
/// neighbours where their curves cross, or `None` where a side is not read or
/// `budget` is used up (`budget.exceeded` says which).
fn circuit_edges(
    plan: &Plan,
    circuit: &Circuit,
    curves: &BTreeMap<i64, BoundaryCurve>,
    budget: &mut Budget,
) -> Option<Loop> {
    if circuit.sides.len() > MAX_CIRCUIT_SIDES {
        budget.exceeded = true;
        return None;
    }
    // How many edges a side made, and the end of its unmoved line.
    type Side = (usize, (f64, f64));
    let mut pieces: Vec<Side> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    for (line, major, sub) in without_spurs(&circuit.sides) {
        let backwards = sub < 0;
        let index = if backwards { !sub } else { sub };
        let segment = plan.segments.get(&(line, major, index))?;
        let curve = curves.get(&line)?;
        if major != 0 {
            return None;
        }
        let (u0, u1) = segment.params;
        // The line's curve must lie on the segment's own points.
        let (start, end) = (curve.curve.at(u0).0, curve.curve.at(u1).0);
        if distance(start, segment.ends.0) > SEGMENT_POINT_TOLERANCE_FEET
            || distance(end, segment.ends.1) > SEGMENT_POINT_TOLERANCE_FEET
        {
            return None;
        }
        budget.parts = budget.parts.saturating_add(curve.offsets.len());
        if budget.parts > MAX_CIRCUIT_OFFSET_PARTS {
            budget.exceeded = true;
            return None;
        }
        let mut side_edges = curve.edges(u0, u1)?;
        let mut unmoved_end = end;
        if backwards {
            side_edges = side_edges.into_iter().rev().map(reversed).collect();
            unmoved_end = start;
        }
        budget.edges = budget.edges.saturating_add(side_edges.len());
        if budget.edges > MAX_CIRCUIT_SIDES {
            budget.exceeded = true;
            return None;
        }
        pieces.push((side_edges.len(), unmoved_end));
        edges.extend(side_edges);
    }
    if edges.is_empty() {
        return None;
    }
    // Where each side's last edge meets the next side's first: the crossing
    // of their curves nearest the lines' shared end, or, where the curves do
    // not cross near it (parallel lines with different offsets), a straight
    // step between them.
    let n = edges.len();
    let mut joins: Vec<Option<(f64, f64)>> = vec![None; n];
    // Whether a third segment meets the node a join is at.
    let mut third: Vec<bool> = vec![false; n];
    let mut at = 0;
    for (side_edges, unmoved_end) in &pieces {
        at += side_edges;
        let (prev, next) = ((at + n - 1) % n, at % n);
        let (a, b) = (edges[prev], edges[next]);
        let reach = distance(a.end(), *unmoved_end) + distance(b.start(), *unmoved_end);
        let meet = if distance(a.end(), b.start()) <= SEGMENT_POINT_TOLERANCE_FEET {
            Some(a.end())
        } else {
            crossings(&a, &b)
                .into_iter()
                .filter(|p| distance(*p, *unmoved_end) <= 4.0 * reach)
                .min_by(|p, q| distance(*p, *unmoved_end).total_cmp(&distance(*q, *unmoved_end)))
        };
        joins[next] = meet;
        third[next] = plan.degree_at(*unmoved_end) > 2;
    }
    for k in 0..n {
        let mut edge = edges[k];
        if let Some(p) = joins[k] {
            edge = edge.starting_at(p);
        }
        if let Some(p) = joins[(k + 1) % n] {
            edge = edge.ending_at(p);
        }
        edges[k] = edge;
    }
    let mut out: Vec<Edge> = Vec::new();
    let mut keep: Vec<bool> = Vec::new();
    for k in 0..n {
        let edge = edges[k];
        let next = edges[(k + 1) % n];
        out.push(edge);
        keep.push(third[k]);
        if distance(edge.end(), next.start()) > SEGMENT_POINT_TOLERANCE_FEET {
            out.push(Edge::Line {
                from: edge.end(),
                to: next.start(),
            });
            keep.push(false);
        }
    }
    Some(Loop { edges: out, keep })
}

/// The signed area of a loop of edges, exact for arcs.
fn edges_area(edges: &[Edge]) -> f64 {
    edges.iter().map(Edge::area_share).sum()
}

/// Whether the straight edge `edge` continues `before`, also straight, in the
/// same direction.
fn continues_straight(before: Edge, edge: Edge) -> bool {
    let (Edge::Line { from: a, .. }, Edge::Line { from: b, to: c }) = (before, edge) else {
        return false;
    };
    let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
    let cross = u.0 * v.1 - u.1 * v.0;
    let dot = u.0 * v.0 + u.1 * v.1;
    dot > 0.0 && cross.abs() <= 1e-12 * u.0.hypot(u.1) * v.0.hypot(v.1)
}

/// A loop of edges as it is written: its vertices, without a repeated closing
/// vertex, and for each edge the middle of the arc it is, or `None` for a line.
/// A point where two straight edges continue in a straight line is left out,
/// unless a third line meets it (`keep`): then it is a vertex of the outline,
/// as RE-317 §3 has it and as Revit's export keeps it (RE-333 section 8, the
/// office's Area 1). An arc is written as the arc, in the form Revit's export
/// gives it: a span over a half turn is two arcs, and an arc too flat to
/// survive the writer's micrometre rounding is a line.
fn written_loop(edges: &[Edge], keep: &[bool]) -> Result<WrittenLoop, &'static str> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut arcs: Vec<Option<(f64, f64)>> = Vec::new();
    for (k, edge) in edges.iter().enumerate() {
        let before = edges[(k + edges.len() - 1) % edges.len()];
        if !keep.get(k).copied().unwrap_or(false) && continues_straight(before, *edge) {
            continue;
        }
        match *edge {
            Edge::Line { from, .. } => {
                points.push(from);
                arcs.push(None);
            }
            Edge::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let finite = center.0.is_finite()
                    && center.1.is_finite()
                    && radius.is_finite()
                    && start.is_finite()
                    && sweep.is_finite();
                if !finite || sweep.abs() > 2.0 * PI + 1e-9 {
                    return Err(REFUSED_ARC);
                }
                let pieces: usize = if sweep.abs() > PI + 1e-9 { 2 } else { 1 };
                let half = sweep / pieces as f64;
                let flat = radius * (1.0 - (half.abs() / 2.0).cos()) < FLAT_ARC_SAGITTA_FEET;
                let on_circle = |angle: f64| {
                    (
                        center.0 + radius * angle.cos(),
                        center.1 + radius * angle.sin(),
                    )
                };
                for j in 0..pieces {
                    let from = start + half * j as f64;
                    points.push(on_circle(from));
                    arcs.push((!flat).then(|| on_circle(from + half / 2.0)));
                }
            }
        }
    }
    let finite = points
        .iter()
        .chain(arcs.iter().flatten())
        .all(|p| p.0.is_finite() && p.1.is_finite());
    if !finite {
        return Err(REFUSED_ARC);
    }
    // A circle written as two half arcs has two vertices; anything else needs
    // three.
    let closed = points.len() >= 3 || (points.len() == 2 && arcs.iter().all(Option::is_some));
    if !closed {
        return Err(REFUSED_AREA);
    }
    Ok(WrittenLoop { points, arcs })
}

/// A loop of edges as a polygon whose arcs are chords of at most
/// [`POINT_TEST_ARC_STEP_RADIANS`]: only for testing whether a point lies in
/// it, never written.
fn edges_polyline(edges: &[Edge]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for edge in edges {
        out.push(edge.start());
        if let Edge::Arc {
            center,
            radius,
            start,
            sweep,
        } = *edge
        {
            let steps = (sweep.abs() / POINT_TEST_ARC_STEP_RADIANS)
                .ceil()
                .clamp(1.0, 100_000.0) as usize;
            for k in 1..steps {
                let t = start + sweep * k as f64 / steps as f64;
                out.push((center.0 + radius * t.cos(), center.1 + radius * t.sin()));
            }
        }
    }
    out
}

/// A loop as it is written: its vertices, and for each edge the middle of its
/// arc or `None` for a line.
struct WrittenLoop {
    points: Vec<(f64, f64)>,
    arcs: Vec<Option<(f64, f64)>>,
}

/// Whether `point` lies inside the loop `ring` (even-odd).
fn inside(ring: &[(f64, f64)], point: (f64, f64)) -> bool {
    let mut odd = false;
    for k in 0..ring.len() {
        let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
        if (a.1 > point.1) != (b.1 > point.1)
            && point.0 < a.0 + (point.1 - a.1) * (b.0 - a.0) / (b.1 - a.1)
        {
            odd = !odd;
        }
    }
    odd
}

/// Whether a rebuilt loop's area is the area Revit saves for its circuit.
fn area_agrees(rebuilt: f64, saved: f64) -> bool {
    (rebuilt - saved).abs() <= CIRCUIT_AREA_TOLERANCE * saved.abs().max(1.0)
}

/// The region of one Area, or why it has none.
fn region(
    plan: &Plan,
    place: &AreaPlace,
    curves: &BTreeMap<i64, BoundaryCurve>,
) -> Result<AreaRegion, &'static str> {
    let circuit = plan
        .circuits
        .get(&place.circuit)
        .ok_or(REFUSED_NO_CIRCUIT)?;
    let mut budget = Budget::default();
    let outline = circuit_edges(plan, circuit, curves, &mut budget).ok_or(if budget.exceeded {
        REFUSED_TOO_LARGE
    } else {
        REFUSED_SIDE
    })?;
    let written = written_loop(&outline.edges, &outline.keep)?;
    // The loop as the edges are rebuilt, and as it is written, arcs as arcs,
    // must each have the area Revit saves for the circuit.
    let rebuilt = edges_area(&outline.edges);
    let area = written_area(&written.points, &written.arcs).ok_or(REFUSED_ARC)?;
    if area <= 0.0 || !area_agrees(rebuilt, circuit.area) || !area_agrees(area, circuit.area) {
        return Err(REFUSED_AREA);
    }
    let test_outer = edges_polyline(&outline.edges);
    let (outer, outer_arcs) = (written.points, written.arcs);
    let mut holes = Vec::new();
    let mut hole_arcs = Vec::new();
    let mut test_holes = Vec::new();
    let mut line_ids: BTreeSet<u32> = BTreeSet::new();
    add_line_ids(&mut line_ids, circuit)?;
    let hole_ids = plan
        .holes
        .get(&place.circuit)
        .map_or(&[][..], Vec::as_slice);
    if hole_ids.len() > MAX_REGION_HOLES {
        return Err(REFUSED_TOO_LARGE);
    }
    for hole_id in hole_ids {
        let hole = plan.circuits.get(hole_id).ok_or(REFUSED_NO_CIRCUIT)?;
        let hole_outline =
            circuit_edges(plan, hole, curves, &mut budget).ok_or(if budget.exceeded {
                REFUSED_TOO_LARGE
            } else {
                REFUSED_SIDE
            })?;
        let hole_written = written_loop(&hole_outline.edges, &hole_outline.keep)?;
        let rebuilt = edges_area(&hole_outline.edges);
        let area = written_area(&hole_written.points, &hole_written.arcs).ok_or(REFUSED_ARC)?;
        if !area_agrees(rebuilt.abs(), hole.area.abs()) || !area_agrees(area.abs(), hole.area.abs())
        {
            return Err(REFUSED_AREA);
        }
        let hole_written = if area > 0.0 {
            reversed_loop(hole_written)
        } else {
            hole_written
        };
        holes.push(hole_written.points);
        hole_arcs.push(hole_written.arcs);
        test_holes.push(edges_polyline(&hole_outline.edges));
        add_line_ids(&mut line_ids, hole)?;
    }
    if !inside(&test_outer, place.point) || test_holes.iter().any(|h| inside(h, place.point)) {
        return Err(REFUSED_POINT);
    }
    let records = region_records(plan, place, curves, &line_ids);
    Ok(AreaRegion {
        outer,
        outer_arcs,
        holes,
        hole_arcs,
        line_ids,
        records,
    })
}

/// The ElementIds of the lines `circuit` runs, added to `ids`. A side whose
/// line has no `u32` ElementId refuses the Area: the geometry would use it
/// and the list of lines would not name it.
fn add_line_ids(ids: &mut BTreeSet<u32>, circuit: &Circuit) -> Result<(), &'static str> {
    for side in &circuit.sides {
        ids.insert(u32::try_from(side.0).map_err(|_| REFUSED_SIDE)?);
    }
    Ok(())
}

/// The native records a region was read from, one part each: the Area's
/// record, its plan's and the record of every line that bounds it. Empty
/// where one has no place.
fn region_records(
    plan: &Plan,
    place: &AreaPlace,
    curves: &BTreeMap<i64, BoundaryCurve>,
    line_ids: &BTreeSet<u32>,
) -> Vec<SourcePart> {
    let (Some(area), Some(topology)) = (&place.record, &plan.record) else {
        return Vec::new();
    };
    let mut records = vec![area.clone(), topology.clone()];
    for line in line_ids {
        match curves
            .get(&i64::from(*line))
            .and_then(|curve| curve.record.clone())
        {
            Some(record) => records.push(record),
            None => return Vec::new(),
        }
    }
    records
}

/// Where a native record starts, as a part of a value source: its element, its
/// partition and the offset of its header in that partition inflated. `None`
/// where the record's group is not the partition's own bytes, or its
/// ElementId is not a `u32`.
fn record_place(record: &native_document::Record) -> Option<SourcePart> {
    let offset = record
        .source
        .group
        .inflated_offset?
        .checked_add(record.source.group_record_offset)?;
    Some(SourcePart {
        element_id: u32::try_from(record.identity.element_id).ok()?,
        stream: record.source.stream.clone(),
        offset,
    })
}

/// The region of each of `areas` (Areas' ElementIds) from its plan
/// topology, or why it has none. An Area whose native records are not read
/// is left out; the map is empty where the file's native records are not.
pub fn area_regions(
    rf: &mut RevitFile,
    areas: &BTreeSet<u64>,
) -> BTreeMap<u64, Result<AreaRegion, &'static str>> {
    let mut out = BTreeMap::new();
    if areas.is_empty() {
        return out;
    }
    let recording = rf.recording_value_sources();
    let mut plans: Vec<Plan> = Vec::new();
    let mut places: BTreeMap<u64, AreaPlace> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: areas.clone(),
        selected_classes: BTreeSet::from(["LevelRoomPlan".to_string()]),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    let read = native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let place = if recording {
            record_place(&record)
        } else {
            None
        };
        for object in &graph.objects {
            match object.class_name.as_str() {
                "PlanTopology" => {
                    if let Some(mut read) = plan(&object.fields) {
                        read.record = place.clone();
                        plans.push(read);
                    }
                }
                "RoomElem" if areas.contains(&record.identity.element_id) => {
                    let circuit = object.fields.get("m_cachedCircuitId").and_then(circuit_id);
                    let point = object.fields.get("m_point").and_then(plan_point);
                    if let (Some(circuit), Some(point)) = (circuit, point) {
                        places.insert(
                            record.identity.element_id,
                            AreaPlace {
                                circuit,
                                point,
                                record: place.clone(),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(())
    });
    if read.is_err() {
        return out;
    }
    // Each Area's plan, and the lines its circuit and holes name.
    let mut lines: BTreeSet<u64> = BTreeSet::new();
    let mut plan_of: BTreeMap<u64, Result<usize, &'static str>> = BTreeMap::new();
    for (&id, place) in &places {
        let holding: Vec<usize> = plans
            .iter()
            .enumerate()
            .filter(|(_, plan)| i64::try_from(id).is_ok_and(|id| plan.areas.contains(&id)))
            .map(|(k, _)| k)
            .collect();
        let found = match holding.as_slice() {
            [k] => Ok(*k),
            _ => Err(REFUSED_NO_PLAN),
        };
        if let Ok(k) = found {
            let plan = &plans[k];
            let circuits = std::iter::once(place.circuit).chain(
                plan.holes
                    .get(&place.circuit)
                    .into_iter()
                    .flatten()
                    .copied(),
            );
            for circuit in circuits.filter_map(|c| plan.circuits.get(&c)) {
                lines.extend(
                    circuit
                        .sides
                        .iter()
                        .filter_map(|side| u64::try_from(side.0).ok()),
                );
            }
        }
        plan_of.insert(id, found);
    }
    let mut curves: BTreeMap<i64, BoundaryCurve> = BTreeMap::new();
    if !lines.is_empty() {
        let options = native_document::Options {
            selected_ids: lines,
            releases: native_document::NATIVE_RELEASES,
            ..native_document::Options::default()
        };
        let read = native_document::extract_graphs(rf, &options, |record| {
            if let (Some(graph), Ok(id)) = (
                record.graph.as_ref(),
                i64::try_from(record.identity.element_id),
            ) {
                if let Some(mut curve) = boundary_curve(&graph.objects) {
                    if recording {
                        curve.record = record_place(&record);
                    }
                    curves.insert(id, curve);
                }
            }
            Ok(())
        });
        if read.is_err() {
            return out;
        }
    }
    for (id, found) in plan_of {
        let result = found.and_then(|k| region(&plans[k], &places[&id], &curves));
        out.insert(id, result);
    }
    out
}
