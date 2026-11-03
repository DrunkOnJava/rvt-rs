//! The openings Revit's export writes in a wall that nothing fills: a
//! curtain wall's cut, a fire hose cabinet's and a rectangular wall
//! opening's (RE-321, #1165).
//!
//! A wall's native record names each element that cuts it, in the order
//! revit-ifc numbers the wall's openings (CLEANROOM.md source 5,
//! `OpeningUtil.AddOpeningsToElement`): the step's kind says which of the
//! three it is. Each opening is a rectangle in the wall's vertical plane
//! extruded across the wall, as Revit's export draws it:
//!
//! - a rectangular opening (`WallRectOpeningGStep`): the box of its own
//!   element record, which reaches through the wall;
//! - a curtain wall (`WallEmbedWallGStep`): its box along the wall and up,
//!   cut back to the ends of the wall's body, and across the wall the
//!   wall's thickness and half a foot beyond each face;
//! - a cabinet (`WallCutoutGStep` naming an `IfcFireSuppressionTerminal`):
//!   the box of the solid its symbol cuts, as its instance places it, which
//!   Revit's export writes only where it reaches into the wall; it is a
//!   recess where its depth is less than the wall's thickness.
//!
//! A door's further cut in a second wall is not written: its depth across
//! the wall is the door's own wall's thickness on some and ten feet on
//! others, and nothing read says which.

use super::entities::{IfcEntity, ProfileDef};
use crate::RevitFile;
use crate::native_wall_cuts::{self, WallCutKind};
use std::collections::{BTreeMap, BTreeSet};

/// How far a curtain wall's opening runs past each face of its host, feet:
/// Revit extends the cut half a foot either side (RE-321).
const CURTAIN_OVERRUN_FEET: f64 = 0.5;
/// The least a cabinet's box must reach into the wall for an opening, feet.
const MIN_REACH_FEET: f64 = 0.05;
/// The least extent of any side of an opening, feet.
const MIN_EXTENT_FEET: f64 = 0.01;
/// How far a rectangular opening's thickness may differ from its wall's, feet.
const THICKNESS_TOLERANCE_FEET: f64 = 0.02;
/// How far a wall's turn may be from a multiple of a right angle for the
/// boxes of its cutters, which are axis-aligned, to lie across it.
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

fn host_of(index: usize, entity: &IfcEntity) -> Option<(u64, Host)> {
    let IfcEntity::BuildingElement {
        ifc_type,
        name,
        type_guid: Some(tag),
        location_feet: Some(location),
        rotation_radians,
        extrusion: Some(body),
        solid_shape: None,
        ..
    } = entity
    else {
        return None;
    };
    if ifc_type != "IFCWALL" {
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

/// What an opening is, as ranges relative to its wall's placement: along
/// the wall, across it and up, whether it stops short of the far face, and
/// the entity whose type name is its `Reference`.
struct Cut {
    along: [f64; 2],
    left: [f64; 2],
    up: [f64; 2],
    recess: bool,
    reference_from: usize,
}

fn overlap(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[1].min(b[1]) - a[0].max(b[0])
}

/// A rectangular wall opening of the wall: its record's box, which must
/// reach across the wall.
fn rect_cut(host: &Host, world: &[f64; 6]) -> Option<Cut> {
    let (along, left, up) = host.ranges(world);
    let across_the_wall = (left[0] - host.band[0]).abs() <= THICKNESS_TOLERANCE_FEET
        && (left[1] - host.band[1]).abs() <= THICKNESS_TOLERANCE_FEET;
    (across_the_wall && overlap(along, host.along) > MIN_EXTENT_FEET).then_some(Cut {
        along,
        left,
        up,
        recess: false,
        reference_from: host.index,
    })
}

/// A curtain wall embedded in the wall: its box along the wall and up, cut
/// back to the wall's body, and across the wall the wall's band and half a
/// foot more on each side.
fn curtain_cut(host: &Host, world: &[f64; 6], curtain: usize) -> Option<Cut> {
    let (along, left, up) = host.ranges(world);
    if overlap(left, host.band) <= 0.0 {
        return None;
    }
    let along = [along[0].max(host.along[0]), along[1].min(host.along[1])];
    (along[1] - along[0] > MIN_EXTENT_FEET).then_some(Cut {
        along,
        left: [
            host.band[0] - CURTAIN_OVERRUN_FEET,
            host.band[1] + CURTAIN_OVERRUN_FEET,
        ],
        up,
        recess: false,
        reference_from: curtain,
    })
}

/// A cabinet whose cut box reaches into the wall: the box itself, a recess
/// where it is not as deep as the wall is thick (revit-ifc's `OpeningUtil`
/// compares the opening's length with the host's width).
fn cabinet_cut(host: &Host, world: &[f64; 6], cabinet: usize) -> Option<Cut> {
    let (along, left, up) = host.ranges(world);
    if overlap(left, host.band) < MIN_REACH_FEET || overlap(along, host.along) <= MIN_EXTENT_FEET {
        return None;
    }
    let recess = left[1] - left[0] < host.band[1] - host.band[0] - 1e-6;
    Some(Cut {
        along,
        left,
        up,
        recess,
        reference_from: cabinet,
    })
}

/// Write each opening the native records of the walls in `entities` name
/// that Revit's export writes with nothing to fill it (RE-321). A wall,
/// cutter or box that is not read, or is not as the rules above need it, is
/// left out.
pub(super) fn attach_wall_cut_openings(rf: &mut RevitFile, entities: &mut Vec<IfcEntity>) {
    let mut hosts: BTreeMap<u64, Host> = BTreeMap::new();
    let mut curtains: BTreeMap<u64, usize> = BTreeMap::new();
    let mut cabinets: BTreeMap<u64, usize> = BTreeMap::new();
    let mut repeated: BTreeSet<u64> = BTreeSet::new();
    for (index, entity) in entities.iter().enumerate() {
        if let Some((id, host)) = host_of(index, entity) {
            if hosts.insert(id, host).is_some() {
                repeated.insert(id);
            }
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        let table = match ifc_type.as_str() {
            "IFCCURTAINWALL" => &mut curtains,
            "IFCFIRESUPPRESSIONTERMINAL" => &mut cabinets,
            _ => continue,
        };
        if table.insert(id, index).is_some() {
            repeated.insert(id);
        }
    }
    for id in &repeated {
        hosts.remove(id);
        curtains.remove(id);
        cabinets.remove(id);
    }
    if hosts.is_empty() {
        return;
    }
    let walls: BTreeSet<u64> = hosts.keys().copied().collect();
    let Ok(steps) = native_wall_cuts::wall_cut_steps(rf, &walls) else {
        return;
    };
    let mut framed: BTreeSet<u64> = BTreeSet::new();
    let mut placed: BTreeSet<u64> = BTreeSet::new();
    for step in steps.values().flatten() {
        match step.kind {
            WallCutKind::Embed if curtains.contains_key(&step.cutter) => {
                framed.insert(step.cutter);
            }
            WallCutKind::RectOpening => {
                framed.insert(step.cutter);
            }
            WallCutKind::Cutout if cabinets.contains_key(&step.cutter) => {
                placed.insert(step.cutter);
            }
            _ => {}
        }
    }
    if framed.is_empty() && placed.is_empty() {
        return;
    }
    let frames = native_wall_cuts::frame_boxes(rf, &framed);
    let boxes = native_wall_cuts::instance_cut_boxes(rf, &placed).unwrap_or_default();
    let global_ids = crate::revit_global_ids::revit_global_ids(rf).unwrap_or_default();
    let global_id = |id: u64| {
        u32::try_from(id)
            .ok()
            .and_then(|id| global_ids.get(&id))
            .cloned()
    };
    let mut openings = Vec::new();
    for (wall, list) in &steps {
        let Some(host) = hosts.get(wall) else {
            continue;
        };
        let mut seen: BTreeSet<u64> = BTreeSet::new();
        for (position, step) in list.iter().enumerate() {
            let cut = match step.kind {
                WallCutKind::Embed => curtains
                    .get(&step.cutter)
                    .zip(frames.get(&step.cutter))
                    .and_then(|(curtain, world)| curtain_cut(host, world, *curtain)),
                WallCutKind::Cutout => cabinets
                    .get(&step.cutter)
                    .zip(boxes.get(&step.cutter))
                    .and_then(|(cabinet, world)| cabinet_cut(host, world, *cabinet)),
                WallCutKind::RectOpening => frames
                    .get(&step.cutter)
                    .and_then(|world| rect_cut(host, world)),
                WallCutKind::ShadowCutout => None,
            };
            let Some(cut) = cut else {
                continue;
            };
            let extents = [
                cut.along[1] - cut.along[0],
                cut.left[1] - cut.left[0],
                cut.up[1] - cut.up[0],
            ];
            if !seen.insert(step.cutter)
                || extents
                    .iter()
                    .any(|extent| !extent.is_finite() || *extent <= MIN_EXTENT_FEET)
            {
                continue;
            }
            let outline_feet = vec![
                (cut.along[0], cut.up[0]),
                (cut.along[1], cut.up[0]),
                (cut.along[1], cut.up[1]),
                (cut.along[0], cut.up[1]),
            ];
            openings.push(IfcEntity::WallCutOpening {
                host: host.index,
                number: (position + 1) as u32,
                tag: step.cutter.to_string(),
                name: host.name.clone(),
                cutter_global_id: global_id(step.cutter),
                reference_from: cut.reference_from,
                outline_feet,
                axis_turn_radians: host.local_turn,
                plane_offset_feet: (cut.left[0] + cut.left[1]) / 2.0,
                thickness_feet: extents[1],
                recess: cut.recess,
            });
        }
    }
    entities.extend(openings);
}
