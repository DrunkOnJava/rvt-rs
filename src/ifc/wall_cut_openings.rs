//! The openings Revit's export writes in a wall for a rectangular wall
//! opening, which nothing fills (RE-321, #1165).
//!
//! A wall's native record names each element that cuts it, in the order
//! revit-ifc numbers the wall's openings (CLEANROOM.md source 5,
//! `OpeningUtil.AddOpeningsToElement`). A `WallRectOpeningGStep` names a
//! rectangular opening of the wall, whose own record saves the rectangle in
//! the plane of the wall (`RectOpeningData.m_aPoint`). Revit's export
//! draws the opening as that rectangle extruded through the whole wall,
//! tags it with the opening's ElementId, names it for the wall, and gives it
//! the wall's type name as its `Reference`.
//!
//! The other kinds #1165 names are left out: Revit's export writes an
//! opening for a curtain wall embedded in a wall, for a fire hose cabinet
//! and for a door's further cut in a second wall, and none of them can be
//! told from the cutters it writes no opening for (RE-321 §4).

use super::entities::{IfcEntity, ProfileDef, PropertySet, PropertyValue};
use crate::RevitFile;
use crate::native_wall_cuts::{self, WallCutKind};
use std::collections::{BTreeMap, BTreeSet};

/// The least extent of any side of an opening, feet.
const MIN_EXTENT_FEET: f64 = 0.01;
/// How far an opening's plane may lie outside its wall's body, feet.
const PLANE_TOLERANCE_FEET: f64 = 0.02;
/// How far a wall's turn may be from a multiple of a right angle for the
/// axis-aligned corners of its openings to lie across it.
const AXIS_TOLERANCE: f64 = 1e-6;

/// A wall the openings are cut in: its entity, name and frame, as the
/// writer places its body.
struct Host {
    index: usize,
    name: String,
    location: [f64; 3],
    local_turn: f64,
    axis: [f64; 2],
    left: [f64; 2],
    /// The body's extent along the wall's axis and across it, to the left
    /// of the placement's origin, feet.
    along: [f64; 2],
    band: [f64; 2],
}

impl Host {
    /// The extent of a world box along the wall's axis, across it (to the
    /// left) and up, relative to the wall's placement, feet.
    fn ranges(&self, world: &[f64; 6]) -> ([f64; 2], [f64; 2], [f64; 2]) {
        let mut along = [f64::INFINITY, f64::NEG_INFINITY];
        let mut left = [f64::INFINITY, f64::NEG_INFINITY];
        for x in [world[0], world[3]] {
            for y in [world[1], world[4]] {
                let (dx, dy) = (x - self.location[0], y - self.location[1]);
                let a = dx * self.axis[0] + dy * self.axis[1];
                let l = dx * self.left[0] + dy * self.left[1];
                along = [along[0].min(a), along[1].max(a)];
                left = [left[0].min(l), left[1].max(l)];
            }
        }
        (
            along,
            left,
            [world[2] - self.location[2], world[5] - self.location[2]],
        )
    }
}

/// Whether a wall's body is RE-320's stepped one: its `extrusion` is still
/// the whole prism, and an opening voids every piece of the body.
fn is_stepped(set: Option<&PropertySet>) -> bool {
    set.is_some_and(|set| {
        set.properties.iter().any(|property| {
            property.name == "BodyHeightSource"
                && matches!(
                    &property.value,
                    PropertyValue::Text(source)
                        if source == super::export_content::WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE
                )
        })
    })
}

fn host_of(index: usize, entity: &IfcEntity) -> Option<(u64, Host)> {
    let IfcEntity::BuildingElement {
        ifc_type,
        name,
        type_guid: Some(tag),
        location_feet: Some(location),
        rotation_radians,
        extrusion: Some(body),
        solid_shape,
        property_set,
        ..
    } = entity
    else {
        return None;
    };
    if ifc_type != "IFCWALL" || (solid_shape.is_some() && !is_stepped(property_set.as_ref())) {
        return None;
    }
    let id: u64 = tag.parse().ok()?;
    let (x0, x1, y0, y1) = match &body.profile_override {
        None => (
            -body.width_feet / 2.0,
            body.width_feet / 2.0,
            -body.depth_feet / 2.0,
            body.depth_feet / 2.0,
        ),
        Some(ProfileDef::Rectangle {
            width_feet,
            depth_feet,
        }) => (
            -width_feet / 2.0,
            width_feet / 2.0,
            -depth_feet / 2.0,
            depth_feet / 2.0,
        ),
        Some(ProfileDef::ArbitraryClosed { points }) => {
            let mut bounds = (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            );
            for (x, y) in points {
                bounds = (
                    bounds.0.min(*x),
                    bounds.1.max(*x),
                    bounds.2.min(*y),
                    bounds.3.max(*y),
                );
            }
            bounds
        }
        Some(_) => return None,
    };
    let (width, depth) = (x1 - x0, y1 - y0);
    if !(width.is_finite() && depth.is_finite() && width > 0.0 && depth > 0.0) {
        return None;
    }
    let rotation = rotation_radians.unwrap_or(0.0);
    let local_turn = if width >= depth {
        0.0
    } else {
        std::f64::consts::FRAC_PI_2
    };
    let turn = rotation + local_turn;
    if (2.0 * turn).sin().abs() > AXIS_TOLERANCE {
        return None;
    }
    let axis = [turn.cos(), turn.sin()];
    let left = [-turn.sin(), turn.cos()];
    let (sin, cos) = rotation.sin_cos();
    let mut along = [f64::INFINITY, f64::NEG_INFINITY];
    let mut band = [f64::INFINITY, f64::NEG_INFINITY];
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        let (wx, wy) = (x * cos - y * sin, x * sin + y * cos);
        let a = wx * axis[0] + wy * axis[1];
        let l = wx * left[0] + wy * left[1];
        along = [along[0].min(a), along[1].max(a)];
        band = [band[0].min(l), band[1].max(l)];
    }
    Some((
        id,
        Host {
            index,
            name: name.clone(),
            location: *location,
            local_turn,
            axis,
            left,
            along,
            band,
        },
    ))
}

/// Write each opening the rectangular wall openings the native records of
/// the walls in `entities` name, which Revit's export writes with nothing
/// to fill it (RE-321). A wall, opening or rectangle that is not read, or
/// does not lie in its wall's body, is left out.
pub(super) fn attach_wall_cut_openings(rf: &mut RevitFile, entities: &mut Vec<IfcEntity>) {
    let mut hosts: BTreeMap<u64, Host> = BTreeMap::new();
    let mut repeated: BTreeSet<u64> = BTreeSet::new();
    for (index, entity) in entities.iter().enumerate() {
        if let Some((id, host)) = host_of(index, entity) {
            if hosts.insert(id, host).is_some() {
                repeated.insert(id);
            }
        }
    }
    for id in &repeated {
        hosts.remove(id);
    }
    if hosts.is_empty() {
        return;
    }
    let walls: BTreeSet<u64> = hosts.keys().copied().collect();
    let Ok(steps) = native_wall_cuts::wall_cut_steps(rf, &walls) else {
        return;
    };
    let rects: BTreeSet<u64> = steps
        .values()
        .flatten()
        .filter(|step| step.kind == WallCutKind::RectOpening)
        .map(|step| step.cutter)
        .collect();
    if rects.is_empty() {
        return;
    }
    let corners = native_wall_cuts::rect_opening_boxes(rf, &rects).unwrap_or_default();
    let global_ids = crate::revit_global_ids::revit_global_ids(rf).unwrap_or_default();
    let mut openings = Vec::new();
    for (wall, list) in &steps {
        let Some(host) = hosts.get(wall) else {
            continue;
        };
        let mut seen: BTreeSet<u64> = BTreeSet::new();
        for (position, step) in list.iter().enumerate() {
            if step.kind != WallCutKind::RectOpening {
                continue;
            }
            let Some(rectangle) = corners.get(&step.cutter) else {
                continue;
            };
            let (along, plane, up) = host.ranges(rectangle);
            // The rectangle lies in the wall's plane, so its corners' spread
            // across the wall is nil; a wall whose axis was not read fails it.
            let in_the_wall = plane[0] >= host.band[0] - PLANE_TOLERANCE_FEET
                && plane[1] <= host.band[1] + PLANE_TOLERANCE_FEET
                && plane[1] - plane[0] <= PLANE_TOLERANCE_FEET;
            let extents = [
                along[1] - along[0],
                host.band[1] - host.band[0],
                up[1] - up[0],
            ];
            if !in_the_wall
                || along[1].min(host.along[1]) - along[0].max(host.along[0]) <= MIN_EXTENT_FEET
                || extents
                    .iter()
                    .any(|extent| !extent.is_finite() || *extent <= MIN_EXTENT_FEET)
                || !seen.insert(step.cutter)
            {
                continue;
            }
            let cutter_global_id = u32::try_from(step.cutter)
                .ok()
                .and_then(|id| global_ids.get(&id))
                .cloned();
            openings.push(IfcEntity::WallCutOpening {
                host: host.index,
                number: (position + 1) as u32,
                tag: step.cutter.to_string(),
                name: host.name.clone(),
                cutter_global_id,
                outline_feet: vec![
                    (along[0], up[0]),
                    (along[1], up[0]),
                    (along[1], up[1]),
                    (along[0], up[1]),
                ],
                axis_turn_radians: host.local_turn,
                plane_offset_feet: (host.band[0] + host.band[1]) / 2.0,
                thickness_feet: extents[1],
            });
        }
    }
    entities.extend(openings);
}
