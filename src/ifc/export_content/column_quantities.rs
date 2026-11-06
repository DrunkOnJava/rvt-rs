//! The base quantities of a column joined to slabs and walls (#1046, RE-291,
//! #1203, RE-353).
//!
//! Revit's export draws a column as an extrusion, with its
//! `Qto_ColumnBaseQuantities` from the extrusion's profile and length, unless
//! the column is cut where it meets another element: on the BIM4LCA office
//! (Revit 2024), a slab whose body lies across one end of a column (its
//! `CutPairGStep` names the slab) makes Revit's export draw a cut mesh, which
//! has a `CrossSectionArea` and a `NetVolume` that are not the box's and no
//! `Length`, `OuterSurfaceArea` or `GrossVolume`. A wall that cuts it
//! (`CutPairGStep`) changes its section. Of the office's 345 columns at
//! building permit, 18 are drawn as a mesh and every one has a cut slab that
//! straddles its top; no other column has (RE-291).
//!
//! This pass writes a column's `CrossSectionArea`, `OuterSurfaceArea`,
//! `GrossVolume` and `NetVolume` as its record box's, and, where a clean pair
//! joins it, its `Length` as the box's height (#1203: Revit writes it on every
//! such column, 186 of 186 as built and 250 of 250 at building permit), on a
//! column drawn as
//! that box whose type is a plain rectangle (its length parameters are
//! exactly `b`, or `b` and `h`, the box's sides: a round column is drawn as a
//! box too, and its type holds a `Diameter`), when every pair step of its
//! record is read (a column whose steps are not all read is left out) and
//!
//! - every element its cut pairs name is an exported slab lying wholly above
//!   or wholly below the column;
//! - every element its clean pairs name is an exported slab lying wholly
//!   above or wholly below it, or an exported wall, covering or column.
//!
//! A column no `CleanPairGStep` joins is checked the same way
//! (`add_body_quantities` gives its `Length`, as Revit draws it as an
//! extrusion on every one of them): a Cut Geometry command can leave a
//! `CutPairGStep` with no `CleanPairGStep`, and a slab across its end would
//! make Revit draw it as a mesh all the same. Only these partners are accepted
//! because they are the only ones the office's box-drawn columns hold
//! (RE-291 §2). Any other pair (a beam, roof or footing, say), a wall or
//! covering that cuts it, or an element rvt-rs does not export, leaves the
//! column out.
//!
//! A column that one slab's `CutPairGStep` names, across the top of the
//! column, and a clean pair joins to the same slab, is the box less the part
//! of the slab inside it, from the slab's base up (RE-353). revit-ifc's column
//! quantities (CLEANROOM.md source 5, `PropertyUtil.CreateBeamColumnBaseQuantities`,
//! read and cited, no code copied or translated) take `Length`,
//! `OuterSurfaceArea` and `GrossVolume` from the extrusion Revit's exporter
//! finds, and where there is none, `CrossSectionArea` from the element's
//! computed area parameter and `NetVolume` from its computed volume. Measured
//! on the 18 meshed columns of the office at building permit, the volume is
//! the cut mesh's and the area is half the surface of the mesh, so the pass
//! writes those two and no other, from the slab's plan outline clipped to the
//! column's section. Three limits, each leaving a column with fewer
//! quantities or none:
//!
//! - The slab must be an exported extrusion whose outline was read (a
//!   rectangle, a polyline or one with voids), flat as the wall pass knows it
//!   ([`SlabTop::Flat`]: its record was read and no shape edit moves its top),
//!   and one piece. A sketch that was not read is the record box, and a
//!   sloped or shape-edited floor's underside is not its base.
//! - Half the surface is measured on meshes whose every face is parallel to
//!   a side of the column's section, which cannot tell it from any other
//!   one-sided area where a cut face is oblique. `CrossSectionArea` is written
//!   only where every edge of the slab inside the section runs parallel to a
//!   side of it; `NetVolume`, exact geometry, is written either way.
//! - The cut columns and the round column are measured on Revit 2024 alone
//!   (the BIM4LCA office), so the two rules read that release only
//!   ([`NEW_RULE_REVIT_VERSIONS`]); a column RE-178 trimmed, whose body is
//!   not its record box though its source says so, is left out.
//!
//! A round column (its type's named lengths are `Diameter`, equal to both
//! sides of its record box, and at most the family's two zero offsets) that no
//! pair joins gets its `OuterSurfaceArea`, the circumference times the box's
//! height, which is Revit's exactly (RE-353). Its `CrossSectionArea`,
//! `GrossVolume` and `NetVolume` are not the circle's and stay out: that is one
//! column on each of the two office files, the same column, and it is taken as
//! round from the caption `Diameter` alone.

use std::collections::HashMap;

use super::wall_tops::SlabTop;
use super::{RECORD_BBOX_BODY_SOURCE, add_element_set};
use crate::ifc::body_geometry::{
    Body, Placement, Point2 as Point, Ring, clip_ring_to_band, plan_outline, signed_area,
};
use crate::ifc::entities::{
    Extrusion, IfcEntity, ProfileDef, Property, PropertySet, PropertyValue,
};

/// The Revit releases the cut-column and round-column rules read: the one the
/// BIM4LCA office is, the only file either is measured on (RE-353). A
/// release's reader is a decision, so a later one is not taken until a file
/// of it is measured.
const NEW_RULE_REVIT_VERSIONS: &[u32] = &[2024];

/// How far an element may reach into the column's span and still lie clear
/// of it, feet.
const EDGE_FEET: f64 = 5e-3;

/// How far the box's sides may differ from the type's, feet.
const SIDE_FEET: f64 = 1e-4;

/// How far a slab's edge may be from a side of the column's section and still
/// lie along it, feet.
const ALONG_FEET: f64 = 1e-4;

/// The least plan area of a slab inside a column's section that counts as a
/// cut, square feet.
const MIN_NOTCH_AREA: f64 = 1e-3;

/// How far inside a side of the section a point is taken to test whether the
/// section just beside the side is inside a slab, feet.
const BESIDE_FEET: f64 = 1e-3;

/// How near to parallel two segments are, as the sine of the angle between
/// them, to be taken as parallel.
const PARALLEL: f64 = 1e-9;

/// How near to parallel to a side of the section an edge of a slab is, as the
/// sine of the angle, to be taken as square to the section.
const SQUARE: f64 = 1e-6;

/// A column whose type is a plain rectangle, waiting for the pass.
pub(super) struct Pending {
    /// The column's entity index.
    pub index: usize,
    /// The ElementIds its cut pairs name.
    pub cut: Vec<u32>,
    /// The ElementIds its clean pairs name.
    pub clean: Vec<u32>,
    /// The sides of its type's rectangle, feet; a round column's diameter
    /// twice.
    pub sides: [f64; 2],
    /// Whether its type is round (RE-353).
    pub round: bool,
    /// Whether its body is not its record box though its source says so,
    /// because a joined slab or roof trimmed it (RE-178).
    pub trimmed: bool,
    /// The Revit release of the file it was read from.
    pub release: Option<u32>,
}

/// Whether `body`'s two plan sides are the rectangle `sides`, in either
/// order.
fn fills_section(body_sides: [f64; 2], sides: [f64; 2]) -> bool {
    let (long, short) = (
        body_sides[0].max(body_sides[1]),
        body_sides[0].min(body_sides[1]),
    );
    let (wanted_long, wanted_short) = (sides[0].max(sides[1]), sides[0].min(sides[1]));
    (long - wanted_long).abs() <= SIDE_FEET && (short - wanted_short).abs() <= SIDE_FEET
}

/// A slab's plan region: its outer outline less its voids.
struct Region {
    outer: Ring,
    voids: Vec<Ring>,
}

impl Region {
    /// Whether `point` is in the outline and in none of the voids.
    fn contains(&self, point: Point) -> bool {
        inside_polygon(point, &self.outer)
            && !self.voids.iter().any(|hole| inside_polygon(point, hole))
    }

    /// The outline and the voids.
    fn loops(&self) -> impl Iterator<Item = &[Point]> {
        std::iter::once(self.outer.as_slice()).chain(self.voids.iter().map(Vec::as_slice))
    }
}

/// An element's entity type, where every piece of it is drawn as one
/// extrusion its lowest and highest point, where it is one piece with a
/// profile that was read its plan region, and whether the wall pass knows its
/// top as flat.
struct Partner {
    kind: String,
    span: Option<(f64, f64)>,
    region: Option<Region>,
    flat: bool,
}

/// The plan region of an extrusion placed at `location` and turned by
/// `rotation`, where its profile is a rectangle, a closed polyline or one
/// with voids that was set (`profile_override`): without one the body is the
/// record box and no outline was read. None for any other profile (a curve
/// with arcs, an I section).
fn region_of(body: &Extrusion, location: [f64; 3], rotation: Option<f64>) -> Option<Region> {
    if !matches!(
        body.profile_override,
        Some(
            ProfileDef::Rectangle { .. }
                | ProfileDef::ArbitraryClosed { .. }
                | ProfileDef::ArbitraryWithVoids { .. }
        )
    ) {
        return None;
    }
    let (outer, voids) = plan_outline(
        Body::Extrusion(body),
        &Placement::new(Some(location), rotation),
    )?;
    Some(Region { outer, voids })
}

/// Each exported element's entity and extent, by ElementId: the union of the
/// spans of its pieces, which is none only when a piece has no span (is not
/// one plain extrusion).
fn partners(entities: &[IfcEntity], slab_shapes: &HashMap<u32, SlabTop>) -> HashMap<u32, Partner> {
    let mut out: HashMap<u32, Partner> = HashMap::new();
    for entity in entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(guid),
            extrusion,
            solid_shape,
            representation_map_index,
            location_feet,
            rotation_radians,
            ..
        } = entity
        else {
            continue;
        };
        let Ok(id) = guid.parse::<u32>() else {
            continue;
        };
        let plain = match (
            extrusion,
            solid_shape,
            representation_map_index,
            location_feet,
        ) {
            (Some(body), None, None, Some(location))
                if body.height_feet.is_finite()
                    && body.height_feet > 0.0
                    && location.iter().all(|value| value.is_finite()) =>
            {
                Some((body, *location))
            }
            _ => None,
        };
        let span = plain.map(|(body, location)| (location[2], location[2] + body.height_feet));
        let kind = ifc_type.to_ascii_uppercase();
        let flat = matches!(slab_shapes.get(&id), Some(SlabTop::Flat));
        let merged = match out.remove(&id) {
            None => Partner {
                kind,
                span,
                region: plain
                    .and_then(|(body, location)| region_of(body, location, *rotation_radians)),
                flat,
            },
            Some(held) => Partner {
                kind,
                span: match (held.span, span) {
                    (Some((low, high)), Some((least, greatest))) => {
                        Some((low.min(least), high.max(greatest)))
                    }
                    _ => None,
                },
                region: None,
                flat,
            },
        };
        out.insert(id, merged);
    }
    out
}

/// Whether a span lies wholly above or wholly below `base..top`.
fn lies_clear(span: Option<(f64, f64)>, base: f64, top: f64) -> bool {
    matches!(span, Some((low, high)) if high <= base + EDGE_FEET || low >= top - EDGE_FEET)
}

/// Whether a span starts inside `base..top` and reaches its top.
fn crosses_top(span: Option<(f64, f64)>, base: f64, top: f64) -> bool {
    matches!(span, Some((low, high))
        if low > base + EDGE_FEET && low < top - EDGE_FEET && high >= top - EDGE_FEET)
}

/// What the elements a column's pairs name do to it.
enum Cut<'a> {
    /// Nothing cuts it or crosses an end of it.
    Clear,
    /// One slab, which a cut pair and a clean pair both name, lies across its
    /// top and nothing else cuts it; a clean pair may also name other slabs
    /// across its top, which cut it where their plan meets its section.
    Across {
        cutter: &'a Partner,
        also: Vec<&'a Partner>,
    },
    /// A pair is of another kind or leaves the column's form unknown.
    Refused,
}

/// Whether nothing the column's pairs name cuts it or crosses an end of it,
/// or one slab across its top does.
fn assess<'a>(
    partners: &'a HashMap<u32, Partner>,
    column: &Pending,
    base: f64,
    top: f64,
) -> Cut<'a> {
    let mut across: Option<u32> = None;
    for id in &column.cut {
        let Some(partner) = partners.get(id) else {
            return Cut::Refused;
        };
        if partner.kind != "IFCSLAB" {
            return Cut::Refused;
        }
        if lies_clear(partner.span, base, top) {
            continue;
        }
        if crosses_top(partner.span, base, top) && across.is_none_or(|seen| seen == *id) {
            across = Some(*id);
            continue;
        }
        return Cut::Refused;
    }
    let mut also: Vec<&Partner> = Vec::new();
    for id in &column.clean {
        let Some(partner) = partners.get(id) else {
            return Cut::Refused;
        };
        match partner.kind.as_str() {
            "IFCSLAB" => {
                if lies_clear(partner.span, base, top) || across == Some(*id) {
                    continue;
                }
                if across.is_some() && crosses_top(partner.span, base, top) {
                    also.push(partner);
                    continue;
                }
                return Cut::Refused;
            }
            "IFCWALL" | "IFCCOVERING" | "IFCCOLUMN" => {}
            _ => return Cut::Refused,
        }
    }
    match across {
        // The slab that cuts the column is one a clean pair joins it to too.
        Some(id) if !column.clean.contains(&id) => Cut::Refused,
        Some(id) => match partners.get(&id) {
            Some(cutter) => Cut::Across { cutter, also },
            None => Cut::Refused,
        },
        None => Cut::Clear,
    }
}

fn distance(p: Point, q: Point) -> f64 {
    (p.0 - q.0).hypot(p.1 - q.1)
}

/// How far a point is from the segment `a..b`.
fn distance_to_segment(point: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_squared = dx * dx + dy * dy;
    if length_squared <= 0.0 {
        return distance(point, a);
    }
    let along = (((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0);
    distance(point, (a.0 + along * dx, a.1 + along * dy))
}

/// Whether `point` is inside `polygon` (even-odd).
fn inside_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    for at in 0..polygon.len() {
        let (p, q) = (polygon[at], polygon[(at + 1) % polygon.len()]);
        if (p.1 > point.1) != (q.1 > point.1)
            && point.0 < p.0 + (point.1 - p.1) * (q.0 - p.0) / (q.1 - p.1)
        {
            inside = !inside;
        }
    }
    inside
}

/// A column's section in plan: its corners, counter-clockwise, and the two
/// axes of its own frame with the bounds of the section along each.
struct Section {
    corners: [Point; 4],
    axes: [[f64; 2]; 2],
    bounds: [(f64, f64); 2],
}

impl Section {
    /// The section of a `width` by `depth` box placed at `location` and turned
    /// by `rotation` about the vertical, as the writer places it.
    fn new(location: [f64; 3], rotation: Option<f64>, width: f64, depth: f64) -> Section {
        let (sin, cos) = rotation
            .filter(|angle| angle.is_finite())
            .unwrap_or(0.0)
            .sin_cos();
        let (u, v) = ([cos, sin], [-sin, cos]);
        let (x, y) = (location[0], location[1]);
        let (half_width, half_depth) = (width / 2.0, depth / 2.0);
        let corner = |a: f64, b: f64| (x + a * u[0] + b * v[0], y + a * u[1] + b * v[1]);
        let (centre_u, centre_v) = (x * u[0] + y * u[1], x * v[0] + y * v[1]);
        Section {
            corners: [
                corner(-half_width, -half_depth),
                corner(half_width, -half_depth),
                corner(half_width, half_depth),
                corner(-half_width, half_depth),
            ],
            axes: [u, v],
            bounds: [
                (centre_u - half_width, centre_u + half_width),
                (centre_v - half_depth, centre_v + half_depth),
            ],
        }
    }

    /// The section's area, square feet.
    fn area(&self) -> f64 {
        (self.bounds[0].1 - self.bounds[0].0) * (self.bounds[1].1 - self.bounds[1].0)
    }

    /// The side `at` of the section, from corner `at` to the next.
    fn side(&self, at: usize) -> (Point, Point) {
        (self.corners[at % 4], self.corners[(at + 1) % 4])
    }

    /// The part of `ring` inside the section.
    fn clip(&self, ring: &[Point]) -> Ring {
        let along_first = clip_ring_to_band(ring, self.axes[0], self.bounds[0].0, self.bounds[0].1);
        clip_ring_to_band(
            &along_first,
            self.axes[1],
            self.bounds[1].0,
            self.bounds[1].1,
        )
    }
}

/// The length of the segment `p..q` inside the section (Cyrus-Beck); a
/// segment along a side, within [`ALONG_FEET`] of it, is on the section's
/// boundary and not inside it.
fn length_inside(p: Point, q: Point, section: &Section) -> f64 {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut from, mut to) = (0.0_f64, 1.0_f64);
    for at in 0..4 {
        let (a, b) = section.side(at);
        let (ex, ey) = (b.0 - a.0, b.1 - a.1);
        let start = ex * (p.1 - a.1) - ey * (p.0 - a.0);
        let rate = ex * dy - ey * dx;
        if rate.abs() <= PARALLEL * ex.hypot(ey) * dx.hypot(dy) {
            if start / ex.hypot(ey) <= ALONG_FEET {
                return 0.0;
            }
            continue;
        }
        let t = -start / rate;
        if rate > 0.0 {
            from = from.max(t);
        } else {
            to = to.min(t);
        }
        if from >= to {
            return 0.0;
        }
    }
    (to - from) * distance(p, q)
}

/// Where along the side `a..b`, from 0 at `a` to 1 at `b`, the segment `p..q`
/// crosses it, and not along it.
fn crossing(a: Point, b: Point, p: Point, q: Point) -> Option<f64> {
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let denominator = ex * dy - ey * dx;
    if denominator.abs() <= PARALLEL * ex.hypot(ey) * dx.hypot(dy) {
        return None;
    }
    let t = ((p.0 - a.0) * dy - (p.1 - a.1) * dx) / denominator;
    let u = ((p.0 - a.0) * ey - (p.1 - a.1) * ex) / denominator;
    (t > 0.0 && t < 1.0 && (0.0..=1.0).contains(&u)).then_some(t)
}

/// Where along the side `a..b` the point `v` lies, from 0 at `a` to 1 at `b`,
/// if it is on the side, within [`ALONG_FEET`], and not at an end of it.
fn on_side(a: Point, b: Point, v: Point) -> Option<f64> {
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let t = ((v.0 - a.0) * ex + (v.1 - a.1) * ey) / (ex * ex + ey * ey);
    (distance_to_segment(v, a, b) < ALONG_FEET && t > 0.0 && t < 1.0).then_some(t)
}

/// Whether the segment `p..q` runs parallel to a side of the section, to
/// [`SQUARE`].
fn runs_along_a_side(p: Point, q: Point, section: &Section) -> bool {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    (0..2).any(|axis| {
        let (ex, ey) = (section.axes[axis][0], section.axes[axis][1]);
        (ex * dy - ey * dx).abs() <= SQUARE * dx.hypot(dy)
    })
}

/// The area of the part of a slab's `region` inside the section, square feet.
fn overlap_area(section: &Section, region: &Region) -> f64 {
    let mut area = signed_area(&section.clip(&region.outer)).abs();
    for hole in &region.voids {
        area -= signed_area(&section.clip(hole)).abs();
    }
    area
}

/// The part of a slab inside a column's section.
struct Notch {
    /// Its plan area, square feet.
    area: f64,
    /// The length of the section's sides inside the slab, feet.
    along_sides: f64,
    /// The length of the slab's edges inside the section, feet.
    inside: f64,
    /// Whether every one of those edges runs parallel to a side of the
    /// section.
    square: bool,
}

/// The part of the slab with plan `region` inside the section, none where the
/// part is no more than a nick or it is the whole section.
fn notch(section: &Section, region: &Region) -> Option<Notch> {
    let area = overlap_area(section, region);
    if area < MIN_NOTCH_AREA || area > section.area() - MIN_NOTCH_AREA {
        return None;
    }
    let (mut inside, mut square) = (0.0, true);
    for polygon in region.loops() {
        for at in 0..polygon.len() {
            let (p, q) = (polygon[at], polygon[(at + 1) % polygon.len()]);
            let length = length_inside(p, q, section);
            if length > 1e-9 {
                inside += length;
                square &= runs_along_a_side(p, q, section);
            }
        }
    }
    // A side is inside the slab where the section's interior just beside it
    // is: tested at the middle of each piece between the points where the
    // slab's outline meets the side.
    let mut along_sides = 0.0;
    for side in 0..4 {
        let (a, b) = section.side(side);
        let length = distance(a, b);
        let inward = (-(b.1 - a.1) / length, (b.0 - a.0) / length);
        let mut cuts = vec![0.0, 1.0];
        for polygon in region.loops() {
            for at in 0..polygon.len() {
                cuts.extend(crossing(
                    a,
                    b,
                    polygon[at],
                    polygon[(at + 1) % polygon.len()],
                ));
                cuts.extend(on_side(a, b, polygon[at]));
            }
        }
        cuts.sort_by(f64::total_cmp);
        for pair in cuts.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            if to - from < 1e-12 {
                continue;
            }
            let middle = (from + to) / 2.0;
            let beside = (
                a.0 + middle * (b.0 - a.0) + inward.0 * BESIDE_FEET,
                a.1 + middle * (b.1 - a.1) + inward.1 * BESIDE_FEET,
            );
            if region.contains(beside) {
                along_sides += (to - from) * length;
            }
        }
    }
    Some(Notch {
        area,
        along_sides,
        inside,
        square,
    })
}

/// Write the quantities of each column of `pending` this pass can vouch for.
pub(super) fn add_joined_column_quantities(
    entities: &mut Vec<IfcEntity>,
    pending: &[Pending],
    slab_shapes: &HashMap<u32, SlabTop>,
) {
    if pending.is_empty() {
        return;
    }
    let partners = partners(entities, slab_shapes);
    let mut found: Vec<(usize, PropertySet)> = Vec::new();
    for column in pending {
        let Some(IfcEntity::BuildingElement {
            ifc_type,
            extrusion: Some(body),
            solid_shape: None,
            representation_map_index: None,
            location_feet: Some(location),
            rotation_radians,
            property_set,
            ..
        }) = entities.get(column.index)
        else {
            continue;
        };
        let boxed = property_set.as_ref().is_some_and(|set| {
            set.properties.iter().any(|property| {
                property.name == "BodySource"
                    && matches!(&property.value, PropertyValue::Text(source)
                        if source == RECORD_BBOX_BODY_SOURCE)
            })
        });
        let sizes = [body.width_feet, body.depth_feet, body.height_feet];
        if !(ifc_type.eq_ignore_ascii_case("IFCCOLUMN")
            && boxed
            && body.profile_override.is_none()
            && location.iter().all(|value| value.is_finite())
            && sizes.iter().all(|size| size.is_finite() && *size > 0.0))
        {
            continue;
        }
        if !fills_section([body.width_feet, body.depth_feet], column.sides) {
            continue;
        }
        let [width, depth, height] = sizes;
        let new_rules = !column.trimmed
            && column
                .release
                .is_some_and(|release| NEW_RULE_REVIT_VERSIONS.contains(&release));
        if column.round {
            if new_rules && column.cut.is_empty() && column.clean.is_empty() {
                found.push((
                    column.index,
                    PropertySet {
                        name: "Qto_ColumnBaseQuantities".into(),
                        properties: vec![Property {
                            name: "OuterSurfaceArea".into(),
                            value: PropertyValue::AreaSquareFeet(
                                std::f64::consts::PI * column.sides[0] * height,
                            ),
                        }],
                    },
                ));
            }
            continue;
        }
        let (base, top) = (location[2], location[2] + body.height_feet);
        let area = width * depth;
        let properties = match assess(&partners, column, base, top) {
            Cut::Refused => continue,
            Cut::Clear => {
                let length = (!column.clean.is_empty()).then(|| Property {
                    name: "Length".into(),
                    value: PropertyValue::PositiveLengthFeet(height),
                });
                length
                    .into_iter()
                    .chain([
                        Property {
                            name: "CrossSectionArea".into(),
                            value: PropertyValue::AreaSquareFeet(area),
                        },
                        Property {
                            name: "OuterSurfaceArea".into(),
                            value: PropertyValue::AreaSquareFeet(2.0 * (width + depth) * height),
                        },
                        Property {
                            name: "GrossVolume".into(),
                            value: PropertyValue::VolumeCubicFeet(area * height),
                        },
                        Property {
                            name: "NetVolume".into(),
                            value: PropertyValue::VolumeCubicFeet(area * height),
                        },
                    ])
                    .collect()
            }
            Cut::Across { cutter, also } => {
                let (Some((slab_base, _)), Some(slab_region), true) = (
                    cutter.span,
                    cutter.region.as_ref(),
                    new_rules && cutter.flat,
                ) else {
                    continue;
                };
                let section = Section::new(*location, *rotation_radians, width, depth);
                if !also.iter().all(|other| {
                    other.flat
                        && other
                            .region
                            .as_ref()
                            .is_some_and(|region| overlap_area(&section, region) < MIN_NOTCH_AREA)
                }) {
                    continue;
                }
                let Some(cut) = notch(&section, slab_region) else {
                    continue;
                };
                let cut_depth = top - slab_base;
                let surface = 2.0 * area
                    + 2.0 * (width + depth) * height
                    + (cut.inside - cut.along_sides) * cut_depth;
                let mut properties = vec![Property {
                    name: "NetVolume".into(),
                    value: PropertyValue::VolumeCubicFeet(area * height - cut.area * cut_depth),
                }];
                if cut.square {
                    properties.insert(
                        0,
                        Property {
                            name: "CrossSectionArea".into(),
                            value: PropertyValue::AreaSquareFeet(surface / 2.0),
                        },
                    );
                }
                properties
            }
        };
        found.push((
            column.index,
            PropertySet {
                name: "Qto_ColumnBaseQuantities".into(),
                properties,
            },
        ));
    }
    for (index, set) in found {
        add_element_set(entities, index, set);
    }
}
