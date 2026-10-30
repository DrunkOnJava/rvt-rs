//! A wall's top cut at the underside of the floor that covers it (#1067,
//! RE-267).
//!
//! Revit cuts a wall's top at the underside of a floor, roof or footing that
//! its top join (`JoinToRoofGStep` with `m_baseOrTop` 1) or one of its
//! `CutPairGStep`s names, over the part of the wall's plan the element
//! covers. This pass lowers the top of the extrusion a wall is drawn as to
//! that underside when all of these hold, measured on rvt-rs's own bodies:
//!
//! - the element is a slab, roof or footing drawn as one plain extrusion (or
//!   several pieces at one height) with its sketch's openings, so that its
//!   underside is level across the wall. A roof drawn from its slopes is a
//!   solid and is left out, which keeps the R2025 roofs off walls that sit
//!   under them;
//! - the underside lies between the wall's base and top;
//! - the element's plan, less the openings that void it, covers the whole of
//!   the wall's plan: the wall's plan outside it measures under 1e-6 ft².
//!
//! On the BIM4LCA office the rule cuts 64 of the 65 tops Revit cuts as built
//! and 70 of 70 at building permit, and no wall Revit leaves whole; it cuts
//! none of Core Interior's, RE1 Architecture's or the R2025 models' walls.
//! A wall the floor covers in part is left at its full height, where Revit
//! steps the top; so is the one wall as built whose plan reaches 0.085 ft past
//! the floor's edge and which Revit cuts all the same. Several elements may
//! qualify; the wall is cut at the lowest underside, since a wall cannot rise
//! through a floor above one that has cut it.
//!
//! The same body, a lower top: the placement and `BoundingBoxHeight` stay,
//! and the wall carries a `BodyHeightSource` and the floor's ElementId.
//! Areas are worked out on triangles: the wall's plan is clipped against
//! each triangle of the element's triangulated plan (Sutherland-Hodgman,
//! against a convex clip), and the pieces add to the area the element
//! covers. A plan that does not triangulate to its own area, or triangles
//! that overlap, leave the wall as it was.

use std::collections::{HashMap, HashSet};

use super::{ELEMENT_RECORD_PROPERTY_SET, WALL_TOP_COVER_BODY_HEIGHT_SOURCE};
use crate::ifc::body_geometry::{
    Body, Placement, Point2, Ring, extrusion_rings, plan_outline, section,
};
use crate::ifc::entities::{IfcEntity, Property, PropertyValue};

/// How much of a wall's plan may lie outside its floor's and still be
/// covered, square feet.
const COVER_TOLERANCE_SQUARE_FEET: f64 = 1e-3;

/// How far a floor's triangulated plan may differ in area from its outline,
/// square feet.
const TRIANGULATION_TOLERANCE_SQUARE_FEET: f64 = 1e-6;

/// How far a floor's underside must lie inside the wall's height, and how
/// far its pieces may differ in height, feet.
const UNDERSIDE_TOLERANCE_FEET: f64 = 1e-3;

/// The entities a wall's top may be cut at (the scorer's `IfcSlab`,
/// `IfcRoof` and `IfcFooting`).
const CUTTER_TYPES: [&str; 3] = ["IFCSLAB", "IFCROOF", "IFCFOOTING"];

type Triangle = [Point2; 3];

/// A wall drawn as one extrusion: its base and top and its plan, counter-
/// clockwise, in model feet.
struct Wall {
    base: f64,
    top: f64,
    plan: Ring,
    area: f64,
    bounds: [f64; 4],
}

/// A floor, roof or footing: its underside, and the triangles of its plan
/// and of the openings that void it, in model feet.
struct Cutter {
    underside: f64,
    triangles: Vec<Triangle>,
    voids: Vec<Triangle>,
}

/// The signed area of `ring`, positive when it runs counter-clockwise.
fn area(ring: &[Point2]) -> f64 {
    let Some(&(ox, oy)) = ring.first() else {
        return 0.0;
    };
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| (a.0 - ox) * (b.1 - oy) - (b.0 - ox) * (a.1 - oy))
        .sum::<f64>()
        / 2.0
}

/// The bounds of `ring`: least x and y, then greatest x and y.
fn bounds(ring: &[Point2]) -> [f64; 4] {
    ring.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)],
    )
}

/// The area of `subject` inside `triangle` (Sutherland-Hodgman: the subject
/// may be concave, the clip is convex).
fn clipped_area(subject: &[Point2], triangle: &Triangle) -> f64 {
    let mut corners = *triangle;
    if area(&corners) < 0.0 {
        corners.swap(1, 2);
    }
    let mut ring: Ring = subject.to_vec();
    for (a, b) in [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[0]),
    ] {
        let side = |p: Point2| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        let mut next: Ring = Vec::with_capacity(ring.len() + 2);
        for (&p, &q) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                next.push((p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)));
            }
        }
        if next.len() < 3 {
            return 0.0;
        }
        ring = next;
    }
    area(&ring).max(0.0)
}

fn overlap(wall: &Wall, triangle: &Triangle) -> f64 {
    let b = bounds(triangle);
    if b[2] < wall.bounds[0]
        || b[0] > wall.bounds[2]
        || b[3] < wall.bounds[1]
        || b[1] > wall.bounds[3]
    {
        return 0.0;
    }
    clipped_area(&wall.plan, triangle)
}

/// Whether the plan of `cutter`, less its openings, covers the whole of the
/// wall's: the wall's plan outside it under [`COVER_TOLERANCE_SQUARE_FEET`],
/// and no more of it inside than the wall has (triangles that overlap).
fn covers(cutter: &Cutter, wall: &Wall) -> bool {
    let inside: f64 = cutter.triangles.iter().map(|t| overlap(wall, t)).sum();
    if (wall.area - inside).abs() > COVER_TOLERANCE_SQUARE_FEET {
        return false;
    }
    let voided: f64 = cutter.voids.iter().map(|t| overlap(wall, t)).sum();
    voided <= COVER_TOLERANCE_SQUARE_FEET
}

/// The wall `entity` is, where it is one plain extrusion with a plan and no
/// holes.
fn build_wall(entity: &IfcEntity) -> Option<Wall> {
    let IfcEntity::BuildingElement {
        ifc_type,
        extrusion: Some(body),
        solid_shape: None,
        representation_map_index: None,
        location_feet: Some(location),
        rotation_radians,
        ..
    } = entity
    else {
        return None;
    };
    if !matches!(ifc_type.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE")
        || !(body.height_feet.is_finite() && body.height_feet > 0.0)
        || !location.iter().all(|v| v.is_finite())
    {
        return None;
    }
    let placement = Placement::new(Some(*location), *rotation_radians);
    let (mut plan, holes) = plan_outline(Body::Extrusion(body), &placement)?;
    if !holes.is_empty() {
        return None;
    }
    if area(&plan) < 0.0 {
        plan.reverse();
    }
    let size = area(&plan);
    if !(size.is_finite() && size > TRIANGULATION_TOLERANCE_SQUARE_FEET) {
        return None;
    }
    Some(Wall {
        base: location[2],
        top: location[2] + body.height_feet,
        bounds: bounds(&plan),
        plan,
        area: size,
    })
}

/// The triangles of `outer` less `holes`, in model feet through `placement`,
/// or `None` where the triangulation does not hold the area between them.
fn plan_triangles(
    outer: &[Point2],
    holes: &[Ring],
    placement: &Placement,
) -> Option<Vec<Triangle>> {
    let expected = area(outer).abs() - holes.iter().map(|hole| area(hole).abs()).sum::<f64>();
    let cap = section(outer, holes)?;
    let mut triangles = Vec::with_capacity(cap.cap_triangles.len());
    let mut drawn = 0.0;
    for corners in &cap.cap_triangles {
        let mut triangle: Triangle = [(0.0, 0.0); 3];
        for (slot, &corner) in triangle.iter_mut().zip(corners) {
            let (x, y) = *cap.cap_points.get(usize::try_from(corner).ok()?)?;
            let at = placement.apply([x, y, 0.0]);
            *slot = (at[0], at[1]);
        }
        drawn += area(&triangle).abs();
        triangles.push(triangle);
    }
    ((drawn - expected).abs() <= TRIANGULATION_TOLERANCE_SQUARE_FEET).then_some(triangles)
}

/// The slab, roof or footing made of the entities `pieces` (a floor of
/// separate loops is several under one Tag), where each is one plain
/// extrusion and they stand at one height.
fn build_cutter(
    entities: &[IfcEntity],
    pieces: &[usize],
    voids: &HashMap<usize, Vec<&[Point2]>>,
) -> Option<Cutter> {
    let mut heights: Option<(f64, f64)> = None;
    let mut triangles = Vec::new();
    let mut void_triangles = Vec::new();
    for &index in pieces {
        let IfcEntity::BuildingElement {
            extrusion: Some(body),
            solid_shape: None,
            representation_map_index: None,
            location_feet: Some(location),
            rotation_radians,
            ..
        } = entities.get(index)?
        else {
            return None;
        };
        if !(body.height_feet.is_finite() && body.height_feet > 0.0)
            || !location.iter().all(|v| v.is_finite())
        {
            return None;
        }
        let (least, greatest) = heights.unwrap_or((location[2], location[2]));
        heights = Some((least.min(location[2]), greatest.max(location[2])));
        let placement = Placement::new(Some(*location), *rotation_radians);
        let (outer, holes) = extrusion_rings(body)?;
        triangles.extend(plan_triangles(&outer, &holes, &placement)?);
        for outline in voids.get(&index).into_iter().flatten() {
            void_triangles.extend(plan_triangles(outline, &[], &placement)?);
        }
    }
    let (least, greatest) = heights?;
    (greatest - least <= UNDERSIDE_TOLERANCE_FEET).then_some(Cutter {
        underside: least,
        triangles,
        voids: void_triangles,
    })
}

/// For each of `pending` (a wall's entity index and the ElementIds its top
/// join and cut pairs name) that is cut: its index, the underside its top is
/// cut at, and the ElementId of the element that gives it.
fn decide(entities: &[IfcEntity], pending: &[(usize, Vec<u32>)]) -> Vec<(usize, f64, u32)> {
    let mut pieces: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut voids: HashMap<usize, Vec<&[Point2]>> = HashMap::new();
    let mut profiled: HashSet<usize> = HashSet::new();
    for (index, entity) in entities.iter().enumerate() {
        match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(guid),
                ..
            } if CUTTER_TYPES.contains(&ifc_type.as_str()) => {
                if let Ok(id) = guid.parse::<u32>() {
                    pieces.entry(id).or_default().push(index);
                }
            }
            IfcEntity::VoidOpening {
                host, outline_feet, ..
            } => voids
                .entry(*host)
                .or_default()
                .push(outline_feet.as_slice()),
            IfcEntity::WallProfileOpening { host, .. } => {
                profiled.insert(*host);
            }
            _ => {}
        }
    }
    let mut cutters: HashMap<u32, Option<Cutter>> = HashMap::new();
    let mut cuts = Vec::new();
    for (index, named) in pending {
        let wall_id = match entities.get(*index) {
            Some(IfcEntity::BuildingElement { type_guid, .. }) => {
                type_guid.clone().unwrap_or_default()
            }
            _ => String::new(),
        };
        if profiled.contains(index) {
            eprintln!("wall_tops: wall {wall_id}: profiled");
            continue;
        }
        let Some(wall) = entities.get(*index).and_then(build_wall) else {
            eprintln!("wall_tops: wall {wall_id}: not a plain extrusion wall");
            continue;
        };
        let mut lowest: Option<(f64, u32)> = None;
        for &id in named {
            let Some(found) = pieces.get(&id) else {
                continue;
            };
            let Some(cutter) = cutters
                .entry(id)
                .or_insert_with(|| build_cutter(entities, found, &voids))
                .as_ref()
            else {
                eprintln!(
                    "wall_tops: wall {wall_id}: cutter {id} not usable ({} pieces)",
                    found.len()
                );
                continue;
            };
            let level = cutter.underside;
            if level > wall.base + UNDERSIDE_TOLERANCE_FEET
                && level < wall.top - UNDERSIDE_TOLERANCE_FEET
            {
                let inside: f64 = cutter.triangles.iter().map(|t| overlap(&wall, t)).sum();
                let voided: f64 = cutter.voids.iter().map(|t| overlap(&wall, t)).sum();
                eprintln!(
                    "wall_tops: wall {wall_id}: cutter {id} level {level:.4} base {:.4} top {:.4} area {:.9} inside {:.9} outside {:.9} voided {:.9} pieces {} triangles {}",
                    wall.base,
                    wall.top,
                    wall.area,
                    inside,
                    wall.area - inside,
                    voided,
                    found.len(),
                    cutter.triangles.len()
                );
            } else {
                eprintln!(
                    "wall_tops: wall {wall_id}: cutter {id} level {level:.4} outside base {:.4} top {:.4}",
                    wall.base, wall.top
                );
            }
            if level > wall.base + UNDERSIDE_TOLERANCE_FEET
                && level < wall.top - UNDERSIDE_TOLERANCE_FEET
                && lowest.is_none_or(|(held, _)| level < held)
                && covers(cutter, &wall)
            {
                lowest = Some((level, id));
            }
        }
        if let Some((level, id)) = lowest {
            cuts.push((*index, level, id));
        }
    }
    cuts
}

/// Cut the top of each wall of `pending` at the underside of the floor that
/// covers it (see the module), setting its extrusion's height to the
/// underside less its base.
pub(super) fn cut_wall_tops(entities: &mut [IfcEntity], pending: &[(usize, Vec<u32>)]) {
    if pending.is_empty() {
        return;
    }
    let cuts = decide(entities, pending);
    for (index, level, floor) in cuts {
        let Some(IfcEntity::BuildingElement {
            extrusion: Some(body),
            location_feet: Some(location),
            property_set,
            ..
        }) = entities.get_mut(index)
        else {
            continue;
        };
        body.height_feet = level - location[2];
        if let Some(set) = property_set
            .as_mut()
            .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
        {
            set.properties.push(Property {
                name: "BodyHeightSource".into(),
                value: PropertyValue::Text(WALL_TOP_COVER_BODY_HEIGHT_SOURCE.into()),
            });
            set.properties.push(Property {
                name: "TopCoverElementId".into(),
                value: PropertyValue::Integer(i64::from(floor)),
            });
        }
    }
}
