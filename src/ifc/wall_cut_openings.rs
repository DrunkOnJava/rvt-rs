//! The openings Revit's export writes in a wall that nothing fills: a
//! rectangular wall opening's, a fire hose cabinet's, a door's in a second
//! wall and a curtain wall's (RE-321, RE-338, #1165).
//!
//! A wall's native record names each element that cuts it, in the order
//! revit-ifc numbers the wall's openings (CLEANROOM.md source 5,
//! `OpeningUtil.AddOpeningsToElement`): the step's kind says which of the
//! four it is. Each opening is a rectangle in the wall's vertical plane
//! extruded across the wall, as Revit's export draws it:
//!
//! - a rectangular opening (`WallRectOpeningGStep`): the rectangle its own
//!   record saves, across the wall's whole thickness;
//! - a cabinet (`WallCutoutGStep` naming an `IfcFireSuppressionTerminal`):
//!   the box of the solid its symbol cuts, as its instance places it,
//!   written where it reaches into the wall; it is a recess where its depth
//!   is less than the wall's thickness;
//! - a door's further wall (`WallCutoutGStep` naming an `IfcDoor`, with
//!   `m_bCutSomething` 0): the door's own opening box, where the wall runs
//!   across the door's wall;
//! - a curtain wall (`WallEmbedWallGStep`): its box along the wall and up,
//!   and across the wall the wall's thickness and half a foot beyond each
//!   face, written only where nothing trims it: Revit cuts the opening back
//!   where the curtain wall reaches the wall's end or another wall lies
//!   beside its box, which the body rvt-rs draws does not say to the
//!   foot.
//!
//! Every step that is not written is counted with its reason.

use super::entities::{IfcEntity, OpeningCut, ProfileDef, Property, PropertySet, PropertyValue};
use crate::RevitFile;
use crate::native_wall_cuts::{self, WallCutKind};
use std::collections::{BTreeMap, BTreeSet};

/// How far a curtain wall's opening runs past each face of its host, feet:
/// Revit extends the cut half a foot either side (RE-321).
const CURTAIN_OVERRUN_FEET: f64 = 0.5;
/// The least a curtain wall's box must lie inside its wall's body at each
/// end for the opening to be its box, feet.
const CURTAIN_INSET_FEET: f64 = 0.5;
/// The least a cabinet's box must reach into the wall for an opening, feet.
const MIN_REACH_FEET: f64 = 0.05;
/// The least extent of any side of an opening, feet.
const MIN_EXTENT_FEET: f64 = 0.01;
/// How much two boxes must overlap on each axis to touch, feet.
const OVERLAP_FEET: f64 = 0.001;
/// How far an opening's plane may lie outside its wall's body, feet.
const PLANE_TOLERANCE_FEET: f64 = 0.02;
/// How far a wall's turn may be from a multiple of a right angle for the
/// axis-aligned corners of its openings to lie across it.
const AXIS_TOLERANCE: f64 = 1e-6;
/// How much longer than thick a door's opening is, at the least, for the
/// door's wall to run along its long side.
const DOOR_SLENDERNESS: f64 = 2.0;
/// How far a door's opening box may lie from a second wall's body, on either
/// axis, for the wall to be the one it reaches, feet: Revit's export writes
/// the opening where the box only touches the wall's face or end, and on the
/// office where it lies up to 0.164 ft off.

/// Why a step of a wall's record writes no opening, as the diagnostics
/// count it (`exported.wall_cut_openings_left_out`).
pub(super) mod reason {
    pub const RECTANGLE_NOT_READ: &str = "rectangular_opening_not_read";
    pub const RECTANGLE_NOT_IN_WALL: &str = "rectangular_opening_not_in_wall";
    pub const CURTAIN_BOX_NOT_READ: &str = "curtain_wall_box_not_read";
    pub const CURTAIN_NOT_IN_WALL: &str = "curtain_wall_not_in_wall";
    pub const CURTAIN_AT_WALL_END: &str = "curtain_wall_within_half_foot_of_wall_end";
    pub const CURTAIN_BESIDE_WALL: &str = "curtain_wall_box_beside_another_wall";
    pub const CABINET_SOLID_NOT_READ: &str = "cabinet_cut_solid_not_read";
    pub const CABINET_NOT_IN_WALL: &str = "cabinet_not_reaching_wall";
    pub const DOOR_FLAG_NOT_READ: &str = "door_cut_flag_not_read";
    pub const DOOR_OPENING_NOT_READ: &str = "door_opening_box_not_read";
    pub const DOOR_WALL_AXIS_NOT_READ: &str = "door_wall_axis_not_read";
    pub const DOOR_WALL_PARALLEL: &str = "door_second_wall_parallel_to_door_wall";
    pub const DOOR_NOT_IN_WALL: &str = "door_opening_not_in_second_wall";
    pub const DEGENERATE: &str = "opening_extent_not_positive";
}

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
    /// The body's height, feet.
    height: f64,
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

    /// The world box of an extent along the wall's axis, across it and up
    /// (relative to the wall's placement), feet.
    fn world_box(&self, along: [f64; 2], left: [f64; 2], up: [f64; 2]) -> [f64; 6] {
        let mut low = [f64::INFINITY; 2];
        let mut high = [f64::NEG_INFINITY; 2];
        for a in along {
            for l in left {
                let x = self.location[0] + a * self.axis[0] + l * self.left[0];
                let y = self.location[1] + a * self.axis[1] + l * self.left[1];
                low = [low[0].min(x), low[1].min(y)];
                high = [high[0].max(x), high[1].max(y)];
            }
        }
        [
            low[0],
            low[1],
            self.location[2] + up[0],
            high[0],
            high[1],
            self.location[2] + up[1],
        ]
    }
}

/// Whether two world boxes share volume.
fn boxes_touch(a: &[f64; 6], b: &[f64; 6]) -> bool {
    (0..3).all(|axis| a[axis + 3].min(b[axis + 3]) - a[axis].max(b[axis]) > OVERLAP_FEET)
}

/// Whether a wall's body is RE-320's stepped one or #1384's, cut at the top
/// of the floor it stands on: its `extrusion` is still the whole prism, and an
/// opening voids every piece of the body.
fn is_stepped(set: Option<&PropertySet>) -> bool {
    set.is_some_and(|set| {
        set.properties
            .iter()
            .any(|property| match (property.name.as_str(), &property.value) {
                ("BodyHeightSource", PropertyValue::Text(source)) => {
                    source == super::export_content::WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE
                }
                (name, PropertyValue::Text(_)) => {
                    name == super::export_content::WALL_BASE_SOURCE_PROPERTY
                }
                _ => false,
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
            height: body.height_feet,
        },
    ))
}

/// An opening to write: its extent in the wall's frame and what it is.
struct Cut {
    along: [f64; 2],
    left: [f64; 2],
    up: [f64; 2],
    recess: bool,
    reference_from: usize,
    quantities: bool,
}

fn overlap(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[1].min(b[1]) - a[0].max(b[0])
}

/// A rectangular wall opening of the wall: its record's rectangle, whose
/// corners lie in the wall's plane, across the wall's whole thickness.
fn rect_cut(host: &Host, rectangle: &[f64; 6]) -> Result<Cut, &'static str> {
    let (along, plane, up) = host.ranges(rectangle);
    // The rectangle lies in the wall's plane, so its corners' spread across
    // the wall is nil; a wall whose axis was not read fails it.
    let in_the_wall = plane[0] >= host.band[0] - PLANE_TOLERANCE_FEET
        && plane[1] <= host.band[1] + PLANE_TOLERANCE_FEET
        && plane[1] - plane[0] <= PLANE_TOLERANCE_FEET
        && overlap(along, host.along) > MIN_EXTENT_FEET;
    if !in_the_wall {
        return Err(reason::RECTANGLE_NOT_IN_WALL);
    }
    Ok(Cut {
        along,
        left: host.band,
        up,
        recess: false,
        reference_from: host.index,
        quantities: true,
    })
}

/// A cabinet whose cut box reaches into the wall: the box itself, a recess
/// where it is not as deep as the wall is thick (revit-ifc's `OpeningUtil`
/// compares the opening's length with the host's width).
fn cabinet_cut(host: &Host, world: &[f64; 6], cabinet: usize) -> Result<Cut, &'static str> {
    let (along, left, up) = host.ranges(world);
    if overlap(left, host.band) < MIN_REACH_FEET || overlap(along, host.along) <= MIN_EXTENT_FEET {
        return Err(reason::CABINET_NOT_IN_WALL);
    }
    let recess = left[1] - left[0] < host.band[1] - host.band[0] - 1e-6;
    Ok(Cut {
        along,
        left,
        up,
        recess,
        reference_from: cabinet,
        quantities: true,
    })
}

/// A curtain wall embedded in the wall: its box along the wall and up, and
/// across the wall the wall's band and half a foot more on each side, where
/// the box lies at least half a foot inside the wall's body at both ends and
/// no other wall's box touches the opening.
fn curtain_cut(
    host: &Host,
    world: &[f64; 6],
    curtain: usize,
    others: &[[f64; 6]],
) -> Result<Cut, &'static str> {
    let (along, left, up) = host.ranges(world);
    if overlap(left, host.band) <= 0.0 {
        return Err(reason::CURTAIN_NOT_IN_WALL);
    }
    if along[0] < host.along[0] + CURTAIN_INSET_FEET
        || along[1] > host.along[1] - CURTAIN_INSET_FEET
    {
        return Err(reason::CURTAIN_AT_WALL_END);
    }
    let left = [
        host.band[0] - CURTAIN_OVERRUN_FEET,
        host.band[1] + CURTAIN_OVERRUN_FEET,
    ];
    let opening = host.world_box(along, left, up);
    if others.iter().any(|other| boxes_touch(&opening, other)) {
        return Err(reason::CURTAIN_BESIDE_WALL);
    }
    Ok(Cut {
        along,
        left,
        up,
        recess: false,
        reference_from: curtain,
        quantities: true,
    })
}

/// The world box of the opening a door cuts in its own wall, from its
/// placement and the cut rvt-rs writes it (`[min_x, min_y, min_z, max_x,
/// max_y, max_z]`, feet), where the door stands on a quarter turn.
fn door_box(door: &IfcEntity, cut: &OpeningCut) -> Option<[f64; 6]> {
    let IfcEntity::BuildingElement {
        location_feet: Some(location),
        rotation_radians,
        ..
    } = door
    else {
        return None;
    };
    let [z, height] = cut.z_range_feet?;
    let angle = rotation_radians.unwrap_or(0.0);
    if (2.0 * angle).sin().abs() > AXIS_TOLERANCE {
        return None;
    }
    let (sin, cos) = angle.sin_cos();
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            let x = cut.centre_feet[0] + sx * cut.x_dim_feet / 2.0;
            let y = cut.centre_feet[1] + sy * cut.y_dim_feet / 2.0;
            let wx = location[0] + x * cos - y * sin;
            let wy = location[1] + x * sin + y * cos;
            low = [low[0].min(wx), low[1].min(wy)];
            high = [high[0].max(wx), high[1].max(wy)];
        }
    }
    let world = [
        low[0],
        low[1],
        location[2] + z,
        high[0],
        high[1],
        location[2] + z + height,
    ];
    world.iter().all(|value| value.is_finite()).then_some(world)
}

/// A door that also cuts a second wall: its own opening box, where the
/// wall runs across the door's wall and the box reaches it. The door's wall runs along the long
/// side of the box; a second wall along it is not written, for the opening
/// then reaches five feet either side of the plane its symbol cuts in,
/// which is not read.
fn door_cut(host: &Host, world: &[f64; 6], door: usize) -> Result<Cut, &'static str> {
    let (dx, dy) = (world[3] - world[0], world[4] - world[1]);
    let (long, thin) = if dx >= dy { (dx, dy) } else { (dy, dx) };
    if !(thin > MIN_EXTENT_FEET && long >= DOOR_SLENDERNESS * thin) {
        return Err(reason::DOOR_WALL_AXIS_NOT_READ);
    }
    let wall_runs_along_x = host.axis[0].abs() > 0.5;
    let door_wall_runs_along_x = dx >= dy;
    if wall_runs_along_x == door_wall_runs_along_x {
        return Err(reason::DOOR_WALL_PARALLEL);
    }
    let (along, left, up) = host.ranges(world);
    if overlap(left, host.band) < -DOOR_REACH_FEET || overlap(along, host.along) < -DOOR_REACH_FEET
    {
        return Err(reason::DOOR_NOT_IN_WALL);
    }
    Ok(Cut {
        along,
        left,
        up,
        recess: false,
        reference_from: door,
        quantities: false,
    })
}

/// Write each opening the native records of the walls in `entities` name
/// that Revit's export writes with nothing to fill it (RE-321). A wall,
/// cutter or box that is not read, or is not as the rules above need it, is
/// left out and counted by its reason in the result.
pub(super) fn attach_wall_cut_openings(
    rf: &mut RevitFile,
    entities: &mut Vec<IfcEntity>,
    record_handoffs: &BTreeMap<usize, Vec<Property>>,
) -> BTreeMap<String, usize> {
    let mut left_out: BTreeMap<String, usize> = BTreeMap::new();
    let mut hosts: BTreeMap<u64, Host> = BTreeMap::new();
    let mut curtains: BTreeMap<u64, usize> = BTreeMap::new();
    let mut cabinets: BTreeMap<u64, usize> = BTreeMap::new();
    let mut doors: BTreeMap<u64, usize> = BTreeMap::new();
    let mut walls: BTreeSet<u64> = BTreeSet::new();
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
        let table = if ifc_type.eq_ignore_ascii_case("IFCCURTAINWALL") {
            &mut curtains
        } else if ifc_type.eq_ignore_ascii_case("IFCFIRESUPPRESSIONTERMINAL") {
            &mut cabinets
        } else if ifc_type.eq_ignore_ascii_case("IFCDOOR") {
            &mut doors
        } else {
            if ifc_type.eq_ignore_ascii_case("IFCWALL") {
                walls.insert(id);
            }
            continue;
        };
        if table.insert(id, index).is_some() {
            repeated.insert(id);
        }
    }
    for id in &repeated {
        hosts.remove(id);
        curtains.remove(id);
        cabinets.remove(id);
        doors.remove(id);
    }
    if hosts.is_empty() {
        return left_out;
    }
    let wall_ids: BTreeSet<u64> = hosts.keys().copied().collect();
    let Ok(steps) = native_wall_cuts::wall_cut_steps(rf, &wall_ids) else {
        return left_out;
    };
    let mut framed: BTreeSet<u64> = BTreeSet::new();
    let mut rects: BTreeSet<u64> = BTreeSet::new();
    let mut placed: BTreeSet<u64> = BTreeSet::new();
    let mut further_doors = false;
    for step in steps.values().flatten() {
        match step.kind {
            WallCutKind::Embed if curtains.contains_key(&step.cutter) => {
                framed.insert(step.cutter);
            }
            WallCutKind::RectOpening => {
                rects.insert(step.cutter);
            }
            WallCutKind::Cutout if cabinets.contains_key(&step.cutter) => {
                placed.insert(step.cutter);
            }
            WallCutKind::Cutout if doors.contains_key(&step.cutter) => {
                further_doors |= step.cuts_something == Some(false);
            }
            _ => {}
        }
    }
    // The boxes a curtain wall's opening is tested against: every other
    // wall's body where rvt-rs draws one, else its record's box.
    let neighbours: Vec<(u64, [f64; 6])> = if framed.is_empty() {
        Vec::new()
    } else {
        let unwritten: BTreeSet<u64> = walls
            .iter()
            .filter(|id| !hosts.contains_key(*id))
            .copied()
            .collect();
        let mut boxes: Vec<(u64, [f64; 6])> = hosts
            .iter()
            .map(|(id, host)| {
                (
                    *id,
                    host.world_box(host.along, host.band, [0.0, host.height]),
                )
            })
            .collect();
        boxes.extend(native_wall_cuts::frame_boxes(rf, &unwritten));
        boxes
    };
    let frames = if framed.is_empty() {
        BTreeMap::new()
    } else {
        native_wall_cuts::frame_boxes(rf, &framed)
    };
    let rectangles = native_wall_cuts::rect_opening_boxes(rf, &rects).unwrap_or_default();
    let boxes = native_wall_cuts::instance_cut_boxes(rf, &placed).unwrap_or_default();
    let door_cuts = if further_doors {
        super::opening_cuts_through_hosts(entities.as_slice(), record_handoffs)
    } else {
        BTreeMap::new()
    };
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
        let near: Vec<[f64; 6]> = neighbours
            .iter()
            .filter(|(id, _)| id != wall)
            .map(|(_, world)| *world)
            .collect();
        let mut seen: BTreeSet<u64> = BTreeSet::new();
        for (position, step) in list.iter().enumerate() {
            let cut = match step.kind {
                WallCutKind::Embed => match curtains.get(&step.cutter) {
                    Some(curtain) => match frames.get(&step.cutter) {
                        Some(world) => curtain_cut(host, world, *curtain, &near),
                        None => Err(reason::CURTAIN_BOX_NOT_READ),
                    },
                    None => continue,
                },
                WallCutKind::Cutout => {
                    if let Some(cabinet) = cabinets.get(&step.cutter) {
                        match boxes.get(&step.cutter) {
                            Some(world) => cabinet_cut(host, world, *cabinet),
                            None => Err(reason::CABINET_SOLID_NOT_READ),
                        }
                    } else if let Some(door) = doors.get(&step.cutter) {
                        match step.cuts_something {
                            Some(true) => continue,
                            None => Err(reason::DOOR_FLAG_NOT_READ),
                            Some(false) => {
                                let world = door_cuts
                                    .get(door)
                                    .and_then(|cut| door_box(&entities[*door], cut));
                                match world {
                                    Some(world) => door_cut(host, &world, *door),
                                    None => Err(reason::DOOR_OPENING_NOT_READ),
                                }
                            }
                        }
                    } else {
                        continue;
                    }
                }
                WallCutKind::RectOpening => match rectangles.get(&step.cutter) {
                    Some(rectangle) => rect_cut(host, rectangle),
                    None => Err(reason::RECTANGLE_NOT_READ),
                },
                WallCutKind::ShadowCutout => continue,
            };
            let cut = match cut {
                Ok(cut) => cut,
                Err(why) => {
                    *left_out.entry(why.to_string()).or_insert(0) += 1;
                    continue;
                }
            };
            let extents = [
                cut.along[1] - cut.along[0],
                cut.left[1] - cut.left[0],
                cut.up[1] - cut.up[0],
            ];
            if extents
                .iter()
                .any(|extent| !extent.is_finite() || *extent <= MIN_EXTENT_FEET)
            {
                *left_out.entry(reason::DEGENERATE.to_string()).or_insert(0) += 1;
                continue;
            }
            if !seen.insert(step.cutter) {
                continue;
            }
            openings.push(IfcEntity::WallCutOpening {
                host: host.index,
                number: (position + 1) as u32,
                tag: step.cutter.to_string(),
                name: host.name.clone(),
                cutter_global_id: global_id(step.cutter),
                reference_from: cut.reference_from,
                outline_feet: vec![
                    (cut.along[0], cut.up[0]),
                    (cut.along[1], cut.up[0]),
                    (cut.along[1], cut.up[1]),
                    (cut.along[0], cut.up[1]),
                ],
                axis_turn_radians: host.local_turn,
                plane_offset_feet: (cut.left[0] + cut.left[1]) / 2.0,
                thickness_feet: extents[1],
                recess: cut.recess,
                quantities: cut.quantities,
            });
        }
    }
    entities.extend(openings);
    left_out
}
