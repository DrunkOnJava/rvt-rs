//! Pipes as their cylinders, from their connector entries (RE-131, #96).
//!
//! A pipe's element record holds its bounding box and nothing that says how a
//! pipe sits in it. Its two ends are in the partition's connector data, one
//! entry per end:
//!
//! ```text
//! -16  u64           the element at that end, or the pipe itself
//! -8   u32           the connector's index on that element (0 to 11 measured)
//! -4   u32 1
//! +0   f64 × 3       the end's point, model feet
//! ```
//!
//! jakobhirn-bit (Discussion #112) read a pipe's centreline from these on
//! Autodesk's Snowdon Towers plumbing sample. On the RE1 Mechanical and
//! Plumbing models (Revit 2025, MIT) the entries hold the two points at which
//! Revit's own IFC starts and ends each pipe's extrusion: 69 of 69 pipes, to
//! 0.00001 ft. At a connected end the entry's element is the one connected
//! there (pipe 442026's two ends are keyed by 442040 and 442191, fittings
//! Revit's export places 0.1413 ft beyond them on its axis); pipes 441731 and
//! 441849 have both entries keyed by the pipe itself. So an entry is found by
//! where its point falls, never by whose it is, and other elements'
//! connectors lie at the same points.
//!
//! The record box then gives the outside radius, which no field stores. A
//! cylinder of radius `r` and length `L` along the unit vector `u` has the
//! half extent `|L·u_q|/2 + r·sqrt(1 − u_q²)` on axis `q`, so each axis that
//! is not nearly the pipe's own gives `r`, and they must agree. [`pipe_body`]
//! also requires the box rebuilt from the cylinder to be the record box on all
//! six sides. A pair of points that does not pass both is not a pipe's ends:
//! the pipe keeps its box.

use crate::le_bytes;
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, HashMap};

/// Releases where this layout is measured: the RE1 models (2025), and the
/// BIM4LCA office (2024), whose 12 pipes and their 24 ports it places as
/// Revit's export does (#869).
pub const PIPE_AXIS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// Whether `revit_version` is a release this layout is measured on.
pub fn supports_revit_version(revit_version: u32) -> bool {
    PIPE_AXIS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

/// The `u32` 1 that precedes an entry's point.
const ENTRY_MARKER: [u8; 4] = 1u32.to_le_bytes();

/// Largest connector index an entry is taken to have; those measured are 0
/// to 11.
const MAX_CONNECTOR_INDEX: u32 = 255;

/// How far outside a pipe's box an entry's point may fall, feet. Measured
/// ends lie in it, on its faces, to 0.00001 ft; this is slack for float noise.
pub const POINT_TOLERANCE_FEET: f64 = 1e-3;

/// The radius each well-conditioned axis gives must agree to this, feet.
pub const RADIUS_TOLERANCE_FEET: f64 = 2e-3;

/// The box rebuilt from the cylinder must be the record box to this on every
/// side, feet.
pub const BOX_TOLERANCE_FEET: f64 = 2e-3;

/// The shortest pipe drawn, feet.
pub const MIN_LENGTH_FEET: f64 = 1e-3;

/// An axis whose share of the pipe's direction leaves `sqrt(1 − u²)` at or
/// below this is nearly the pipe's own, and gives no radius.
const CONDITIONED_SHARE: f64 = 0.3;

/// Two solutions whose lengths differ by less than this are the same length.
const LENGTH_TIE_FEET: f64 = 1e-3;

/// Two ends closer than this are one point, feet.
const SAME_POINT_FEET: f64 = 1e-4;

/// Most points kept for one pipe.
const MAX_POINTS_PER_PIPE: usize = 64;

/// Edge of the cells pipes' boxes are indexed in, feet.
const CELL_FEET: f64 = 1.0;

/// Most cells one box may cover; a box spanning more is not indexed.
const MAX_CELLS_PER_PIPE: i64 = 4096;

/// A pipe's cylinder: its axis and outside radius, model feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PipeBody {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub radius_feet: f64,
}

impl PipeBody {
    /// The length of the axis, feet.
    pub fn length_feet(&self) -> f64 {
        distance(self.start, self.end)
    }
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// The cylinder from `start` to `end` whose box is `bbox` (`[min x, min y,
/// min z, max x, max y, max z]`), or `None` where no cylinder along that line
/// has it: the radii the axes give disagree, or the rebuilt box is not the
/// record box.
pub fn pipe_body(bbox: [f64; 6], start: [f64; 3], end: [f64; 3]) -> Option<PipeBody> {
    let length = distance(start, end);
    if !length.is_finite() || length < MIN_LENGTH_FEET {
        return None;
    }
    let unit = [
        (end[0] - start[0]) / length,
        (end[1] - start[1]) / length,
        (end[2] - start[2]) / length,
    ];
    let across = unit.map(|u| (1.0 - u * u).max(0.0).sqrt());
    let mut sum = 0.0;
    let mut count = 0u32;
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for axis in 0..3 {
        if across[axis] <= CONDITIONED_SHARE {
            continue;
        }
        let half = (bbox[axis + 3] - bbox[axis]) / 2.0;
        let radius = (half - (length * unit[axis]).abs() / 2.0) / across[axis];
        low = low.min(radius);
        high = high.max(radius);
        sum += radius;
        count += 1;
    }
    if count == 0 {
        return None;
    }
    let radius = sum / f64::from(count);
    if !(radius.is_finite() && radius > 0.0) || high - low > RADIUS_TOLERANCE_FEET {
        return None;
    }
    for axis in 0..3 {
        let centre = (start[axis] + end[axis]) / 2.0;
        let half = (length * unit[axis]).abs() / 2.0 + radius * across[axis];
        if (centre - half - bbox[axis]).abs() > BOX_TOLERANCE_FEET
            || (centre + half - bbox[axis + 3]).abs() > BOX_TOLERANCE_FEET
        {
            return None;
        }
    }
    Some(PipeBody {
        start,
        end,
        radius_feet: radius,
    })
}

fn same_ends(a: &PipeBody, b: &PipeBody) -> bool {
    let close = |p: [f64; 3], q: [f64; 3]| distance(p, q) <= SAME_POINT_FEET;
    (close(a.start, b.start) && close(a.end, b.end))
        || (close(a.start, b.end) && close(a.end, b.start))
}

/// The pipe's cylinder among the points found in its box: the pair furthest
/// apart that [`pipe_body`] accepts. `None` where no pair is accepted, or
/// where two different cylinders of the same length are.
pub fn best_pipe_body(bbox: [f64; 6], points: &[[f64; 3]]) -> Option<PipeBody> {
    let mut best: Option<PipeBody> = None;
    let mut tied = false;
    for (index, a) in points.iter().enumerate() {
        for b in &points[index + 1..] {
            let Some(body) = pipe_body(bbox, *a, *b) else {
                continue;
            };
            match best {
                None => best = Some(body),
                Some(current) => {
                    let longer = body.length_feet() - current.length_feet();
                    if longer > LENGTH_TIE_FEET {
                        best = Some(body);
                        tied = false;
                    } else if longer >= -LENGTH_TIE_FEET && !same_ends(&current, &body) {
                        tied = true;
                    }
                }
            }
        }
    }
    if tied { None } else { best }
}

fn cell(value: f64) -> i64 {
    (value / CELL_FEET).floor() as i64
}

/// Whether `point` is in `bbox` grown by [`POINT_TOLERANCE_FEET`].
pub(crate) fn holds(bbox: &[f64; 6], point: [f64; 3]) -> bool {
    (0..3).all(|axis| {
        point[axis] >= bbox[axis] - POINT_TOLERANCE_FEET
            && point[axis] <= bbox[axis + 3] + POINT_TOLERANCE_FEET
    })
}

/// Boxes indexed by the cells of [`CELL_FEET`] they cover once grown by
/// [`POINT_TOLERANCE_FEET`], so a point is tested against the boxes of its
/// own cell rather than all of them. A box covering more than
/// [`MAX_CELLS_PER_PIPE`] cells, or with a coordinate that is not finite, is
/// not indexed but listed in `oversized`.
pub(crate) struct BoxGrid {
    cells: HashMap<[i64; 3], Vec<usize>>,
    pub(crate) oversized: Vec<usize>,
}

impl BoxGrid {
    pub(crate) fn new<'a>(boxes: impl IntoIterator<Item = &'a [f64; 6]>) -> Self {
        let mut grid = BoxGrid {
            cells: HashMap::new(),
            oversized: Vec::new(),
        };
        for (index, bbox) in boxes.into_iter().enumerate() {
            let first = [0, 1, 2].map(|axis| cell(bbox[axis] - POINT_TOLERANCE_FEET));
            let last = [0, 1, 2].map(|axis| cell(bbox[axis + 3] + POINT_TOLERANCE_FEET));
            if !bbox.iter().all(|value| value.is_finite()) {
                grid.oversized.push(index);
                continue;
            }
            // A finite box inverted on any axis by more than the tolerance
            // holds no point, and its cell count is no bound on the fill below.
            if (0..3).any(|axis| last[axis] < first[axis]) {
                continue;
            }
            let span = |axis: usize| last[axis].saturating_sub(first[axis]).saturating_add(1);
            let cells = span(0).saturating_mul(span(1)).saturating_mul(span(2));
            if cells > MAX_CELLS_PER_PIPE {
                grid.oversized.push(index);
                continue;
            }
            for x in first[0]..=last[0] {
                for y in first[1]..=last[1] {
                    for z in first[2]..=last[2] {
                        grid.cells.entry([x, y, z]).or_default().push(index);
                    }
                }
            }
        }
        grid
    }

    /// The indexed boxes whose cells include `point`'s, in index order: every
    /// indexed box that [`holds`] it is among them.
    pub(crate) fn near(&self, point: [f64; 3]) -> &[usize] {
        self.cells
            .get(&point.map(cell))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// The point of the connector entry whose marker `u32` 1 starts at `at`.
fn entry_point(buf: &[u8], at: usize) -> Option<[f64; 3]> {
    let index = le_bytes::u32_at(buf, at.checked_sub(4)?)?;
    if index > MAX_CONNECTOR_INDEX {
        return None;
    }
    let mut point = [0.0f64; 3];
    for (axis, slot) in point.iter_mut().enumerate() {
        let from = at.checked_add(4)?.checked_add(8 * axis)?;
        *slot = le_bytes::f64_at(buf, from)?;
        if !slot.is_finite() {
            return None;
        }
    }
    Some(point)
}

/// For each of `boxes`, the connector entries' points inside it, found in one
/// pass over `buf`. Points are kept in the order they are met, without
/// repeats.
fn connector_points(buf: &[u8], boxes: &[(u32, [f64; 6])]) -> Vec<Vec<[f64; 3]>> {
    let mut found: Vec<Vec<[f64; 3]>> = vec![Vec::new(); boxes.len()];
    // A pipe whose box is not indexed gets no points.
    let grid = BoxGrid::new(boxes.iter().map(|(_, bbox)| bbox));
    if grid.cells.is_empty() {
        return found;
    }
    let marker = ENTRY_MARKER;
    for at in memchr::memmem::find_iter(buf, marker.as_slice()) {
        let Some(point) = entry_point(buf, at) else {
            continue;
        };
        for &index in grid.near(point) {
            let inside = holds(&boxes[index].1, point);
            let points = &mut found[index];
            if inside
                && points.len() < MAX_POINTS_PER_PIPE
                && !points.iter().any(|kept| distance(*kept, point) < 1e-7)
            {
                points.push(point);
            }
        }
    }
    found
}

/// A connector entry's key: the element at that end (the pipe itself, or the
/// element connected there) and the connector's index on it (RE-131).
pub type EntryKey = (u64, u32);

/// Two-ended elements by the partition stream their records are in: each
/// one's ElementId and its two ends, model feet (#847).
pub type CurveEnds = BTreeMap<String, Vec<(u32, [[f64; 3]; 2])>>;

/// The ends that fall in one cell: ElementId, which end, and the point.
type EndCell = Vec<(u32, usize, [f64; 3])>;

/// The keys of the connector entries at each end of each pipe (#847), by
/// ElementId: `[start, end]`, for the pipes of `pipes` (their ends, by the
/// partition stream their records are in). An entry is at an end when its
/// point is that end to 0.0001 ft on every axis.
///
/// On the RE1 models (Measure run 39370306743) every end with a port in
/// Revit's export is named by its keys: an entry keyed by the pipe itself
/// holds the connector Revit places there (57 of 57 keys), and one keyed by
/// another element holds the connector Revit's export joins the pipe's port
/// there to (168 of 168).
pub fn scan_pipe_end_keys(
    rf: &mut RevitFile,
    revit_version: u32,
    pipes: &CurveEnds,
) -> Result<BTreeMap<u32, [Vec<EntryKey>; 2]>> {
    let mut out: BTreeMap<u32, [Vec<EntryKey>; 2]> = BTreeMap::new();
    if !supports_revit_version(revit_version) {
        return Ok(out);
    }
    for (stream, ends) in pipes {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let mut grid: HashMap<[i64; 3], EndCell> = HashMap::new();
        for (id, points) in ends {
            for (slot, point) in points.iter().enumerate() {
                let key = [cell(point[0]), cell(point[1]), cell(point[2])];
                grid.entry(key).or_default().push((*id, slot, *point));
            }
        }
        for at in memchr::memmem::find_iter(buf, ENTRY_MARKER.as_slice()) {
            let Some(point) = entry_point(buf, at) else {
                continue;
            };
            // The cells the point could share with an end, 0.0001 ft either
            // way on each axis.
            let mut cells: Vec<[i64; 3]> = Vec::with_capacity(8);
            for dx in [-SAME_POINT_FEET, SAME_POINT_FEET] {
                for dy in [-SAME_POINT_FEET, SAME_POINT_FEET] {
                    for dz in [-SAME_POINT_FEET, SAME_POINT_FEET] {
                        let key = [
                            cell(point[0] + dx),
                            cell(point[1] + dy),
                            cell(point[2] + dz),
                        ];
                        if !cells.contains(&key) {
                            cells.push(key);
                        }
                    }
                }
            }
            let candidates: Vec<&(u32, usize, [f64; 3])> = cells
                .iter()
                .filter_map(|key| grid.get(key))
                .flatten()
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let Some(element) = at
                .checked_sub(12)
                .and_then(|from| le_bytes::u64_at(buf, from))
            else {
                continue;
            };
            let Some(index) = at
                .checked_sub(4)
                .and_then(|from| le_bytes::u32_at(buf, from))
            else {
                continue;
            };
            for &&(id, slot, end) in &candidates {
                if (0..3).all(|axis| (point[axis] - end[axis]).abs() <= SAME_POINT_FEET) {
                    let keys = &mut out.entry(id).or_default()[slot];
                    if !keys.contains(&(element, index)) {
                        keys.push((element, index));
                    }
                }
            }
        }
    }
    Ok(out)
}

/// The cylinder of each pipe, by ElementId, for the pipes of `pipes` (their
/// record boxes, by the partition stream their records are in).
///
/// The connector entries of a pipe are in the partition of its record. A pipe
/// whose ends are not found, or found and not proven, is absent; so is one
/// whose ElementId gives different cylinders in two streams. The map is empty
/// for a release this layout is not measured on.
pub fn scan_pipe_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    pipes: &BTreeMap<String, Vec<(u32, [f64; 6])>>,
) -> Result<BTreeMap<u32, PipeBody>> {
    let mut bodies: BTreeMap<u32, Option<PipeBody>> = BTreeMap::new();
    if !supports_revit_version(revit_version) {
        return Ok(BTreeMap::new());
    }
    for (stream, boxes) in pipes {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let points = connector_points(inflated.bytes(), boxes);
        for ((id, bbox), points) in boxes.iter().zip(points) {
            let Some(body) = best_pipe_body(*bbox, &points) else {
                continue;
            };
            let existing = bodies.get(id).copied();
            match existing {
                Some(Some(kept)) if !same_ends(&kept, &body) => {
                    bodies.insert(*id, None);
                }
                Some(_) => {}
                None => {
                    bodies.insert(*id, Some(body));
                }
            }
        }
    }
    Ok(bodies
        .into_iter()
        .filter_map(|(id, body)| body.map(|body| (id, body)))
        .collect())
}
