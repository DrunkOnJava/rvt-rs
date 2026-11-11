//! Which end faces of a curtain mullion Revit's solid keeps, from where the
//! boxes of the mullions of its curtain wall meet (#1046, RE-268, RE-400).
//!
//! A curtain mullion's `Qto_MemberBaseQuantities.NetSurfaceArea` is its
//! section's perimeter times its length plus 0, 1 or 2 times its section's
//! area, the end faces its Revit solid keeps (RE-248). The boxes rvt-rs draws
//! are those solids' boxes, so which ends are kept follows from where they
//! meet. At an end's point `p`, outward along the mullion's axis, each other
//! mullion of the same curtain wall whose box holds `p` (within
//! `TOLERANCE_FEET`) is one of:
//!
//! - its partner: it runs the same way with an end at `p` and its body on the
//!   far side, the mullion's continuation, coinciding with it (RE-432: a
//!   continuation bent off the axis by two degrees does not hide the end, and
//!   Revit's value counts the end of all 159 of the BIM4LCA office's mullions
//!   that have one);
//! - across it, with `p` on its axis (centre) or off its axis and off its side
//!   faces (inside), where it covers `p`; or across it with `p` on a side face,
//!   where the end only butts against it;
//! - lying over it (the same way, its end at `p` and its body on the same
//!   side, or its box holding `p` elsewhere): the end is overlapped, and the
//!   mullion that lies over it counts for nothing at `p` (RE-400), when its
//!   box is its solid, which a box with a section that most boxes of its type
//!   do not have is not (RE-403);
//! - neither along nor across it (oblique): the end's join is not read.
//!
//! An end is hidden when it has a partner and nothing across covers `p`, or
//! when a mullion across covers `p` and it has no partner. A mullion whose
//! axis is not known, or one of whose ends has a mullion across it that holds
//! `p` inside its box or one that is oblique to it, is left out. So is a mullion
//! with an end that meets, within `NEAR_FEET`, a mullion that is left out or
//! overlapped, unless it is overlapped itself, and a mullion with another
//! lying over one of its ends whose section is not the usual one of its type:
//! on the BIM4LCA timber office at permit 16 of the 22 overlapped mullions
//! that are wrong have such a mullion over an end, and 1 of the 148 that are
//! right does (RE-403). A mullion shorter than its section is read like any
//! other and is not a reason to leave its neighbours out (RE-400).

use std::collections::{BTreeMap, BTreeSet};

/// Two points this close, about 3 mm, meet.
const TOLERANCE_FEET: f64 = 0.01;
/// A mullion within this distance of an end, about 6 cm, is its neighbour.
const NEAR_FEET: f64 = 0.2;
/// Two axes whose cosine is above this run the same way.
const PARALLEL: f64 = 0.999;
/// Two axes whose cosine is above this coincide (about a quarter of a degree),
/// so that the end of one meets the end of the other face to face (RE-432):
/// every partner that hides an end of the BIM4LCA office's mullions is within
/// 0.01 degrees, and the ones that do not are bent by 1.8 degrees or more.
const COLLINEAR: f64 = 0.99999;
/// Two axes whose cosine is below this run across each other.
const ACROSS: f64 = 0.01;
/// A curtain wall with more mullions than this is left unread: the ends of each
/// are compared with every other mullion of the wall.
const MAX_GROUP: usize = 4096;

/// The box of a curtain mullion: its centre, three unit axes with the
/// half-extent along each (model feet), and the index of the axis it runs
/// along, where that is known.
#[derive(Debug, Clone, PartialEq)]
pub struct MullionBox {
    pub id: u32,
    /// The mullion's type, 0 where it is not known.
    pub kind: u64,
    pub centre: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub half: [f64; 3],
    pub along: Option<usize>,
    /// Whether the box's section is not the one most boxes of its type have, so
    /// that the box is not Revit's solid for it (RE-403).
    pub odd: bool,
}

/// What lies at an end's point.
#[derive(Debug, Clone, Copy, Default)]
struct Neighbours {
    partner: bool,
    same: bool,
    centre: bool,
    inside: bool,
    parallel_in: bool,
    oblique: bool,
    /// A mullion lies over the end whose box is not its solid (RE-403).
    unreliable: bool,
}

impl Neighbours {
    /// An end whose join is not read: a mullion across it holds `p` inside its
    /// box, or one runs oblique to it.
    fn unread(&self) -> bool {
        self.inside || self.oblique
    }

    /// An end with a mullion lying over it, which counts for nothing at `p`.
    fn overlapped(&self) -> bool {
        self.same || self.parallel_in
    }

    fn hidden(&self) -> bool {
        self.partner != (self.centre || self.inside)
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn distance_to_box(b: &MullionBox, p: [f64; 3]) -> f64 {
    let v = sub(p, b.centre);
    b.axes
        .iter()
        .zip(b.half)
        .map(|(axis, half)| (dot(v, *axis).abs() - half).max(0.0).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// The two ends of `b` along its axis `along`: each point and the direction
/// out of the mullion there.
fn end_points(b: &MullionBox, along: usize) -> [([f64; 3], [f64; 3]); 2] {
    let axis = b.axes[along];
    let reach = b.half[along];
    let at = |sign: f64| {
        [
            b.centre[0] + sign * reach * axis[0],
            b.centre[1] + sign * reach * axis[1],
            b.centre[2] + sign * reach * axis[2],
        ]
    };
    [(at(-1.0), axis.map(|c| -c)), (at(1.0), axis)]
}

/// Tenths of a millimetre in a foot.
const TENTH_MM_PER_FOOT: f64 = 3048.0;

/// A box's two sides across the mullion, in tenths of a millimetre, the smaller
/// first, or none where its axis is not known.
fn section_of(b: &MullionBox) -> Option<(i64, i64)> {
    let along = b.along.filter(|axis| *axis < 3)?;
    let mut sides = (0..3)
        .filter(|axis| *axis != along)
        .map(|axis| (2.0 * b.half[axis] * TENTH_MM_PER_FOOT).round() as i64);
    let (first, second) = (sides.next()?, sides.next()?);
    Some((first.min(second), first.max(second)))
}

/// The section more than half of the boxes of each type have, in tenths of a
/// millimetre, for the types that have one (RE-403): a type has one section, so
/// a box with another is not its mullion's solid.
pub fn usual_sections(boxes: &[&MullionBox]) -> BTreeMap<u64, (i64, i64)> {
    let mut counts: BTreeMap<u64, BTreeMap<(i64, i64), usize>> = BTreeMap::new();
    for b in boxes.iter().filter(|b| b.kind != 0) {
        if let Some(section) = section_of(b) {
            *counts
                .entry(b.kind)
                .or_default()
                .entry(section)
                .or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(kind, sections)| {
            let total: usize = sections.values().sum();
            let (section, most) = sections.into_iter().max_by_key(|(_, count)| *count)?;
            (2 * most > total).then_some((kind, section))
        })
        .collect()
}

/// Whether `b`'s section is not the usual one of its type in `usual`
/// ([`usual_sections`]): a type with no usual section has no odd box.
pub fn has_unusual_section(b: &MullionBox, usual: &BTreeMap<u64, (i64, i64)>) -> bool {
    matches!(
        (section_of(b), usual.get(&b.kind)),
        (Some(section), Some(common)) if section != *common
    )
}

fn neighbours(
    m: &MullionBox,
    along: usize,
    p: [f64; 3],
    out: [f64; 3],
    group: &[MullionBox],
) -> Neighbours {
    let axis = m.axes[along];
    let mut found = Neighbours::default();
    for o in group {
        let Some(o_along) = o.along.filter(|a| *a < 3) else {
            continue;
        };
        if o.id == m.id || distance_to_box(o, p) > TOLERANCE_FEET {
            continue;
        }
        let cosine = dot(o.axes[o_along], axis).abs();
        if cosine > PARALLEL {
            let ends = end_points(o, o_along);
            let at_p = ends.iter().position(|(q, _)| {
                let d = sub(*q, p);
                dot(d, d).sqrt() < TOLERANCE_FEET
            });
            match at_p {
                Some(index) => {
                    let far = ends[1 - index].0;
                    if dot(sub(far, p), out) > TOLERANCE_FEET {
                        // A continuation bent off the axis covers none of the
                        // end face: nothing hides it (RE-432).
                        found.partner |= cosine > COLLINEAR;
                    } else {
                        found.same = true;
                        found.unreliable |= o.odd;
                    }
                }
                None => {
                    found.parallel_in = true;
                    found.unreliable |= o.odd;
                }
            }
        } else if cosine < ACROSS {
            let offset = dot(sub(p, o.centre), axis).abs();
            let reach: f64 = (0..3)
                .filter(|i| *i != o_along)
                .map(|i| dot(axis, o.axes[i]).abs() * o.half[i])
                .sum();
            if offset < TOLERANCE_FEET {
                found.centre = true;
            } else if (offset - reach).abs() >= TOLERANCE_FEET {
                found.inside = true;
            }
        } else {
            found.oblique = true;
        }
    }
    found
}

/// For each mullion of `group`, the mullions of one curtain wall, how many of
/// its two end faces its Revit solid keeps (0, 1 or 2); a mullion left out has
/// none.
pub fn kept_ends(group: &[MullionBox]) -> BTreeMap<u32, u8> {
    if group.len() > MAX_GROUP {
        return BTreeMap::new();
    }
    let mut found: BTreeMap<u32, (usize, [Neighbours; 2])> = BTreeMap::new();
    let mut left_out: BTreeSet<u32> = BTreeSet::new();
    let mut overlapped: BTreeSet<u32> = BTreeSet::new();
    for m in group {
        let Some(along) = m.along.filter(|a| *a < 3) else {
            left_out.insert(m.id);
            continue;
        };
        let ends = end_points(m, along).map(|(p, out)| neighbours(m, along, p, out, group));
        if ends.iter().any(|end| end.unread()) {
            left_out.insert(m.id);
        } else if ends.iter().any(|end| end.overlapped()) {
            if ends.iter().any(|end| end.unreliable) {
                left_out.insert(m.id);
            } else {
                overlapped.insert(m.id);
            }
        }
        found.insert(m.id, (along, ends));
    }
    let mut kept = BTreeMap::new();
    for m in group {
        let Some((along, ends)) = found.get(&m.id) else {
            continue;
        };
        if left_out.contains(&m.id) {
            continue;
        }
        // An overlapped mullion is read on its own where what lies over it is
        // of its type: its count is Revit's on all 159 of the office's. A
        // neighbour of one, or of a mullion left out, is not read (RE-400).
        let meets_one_not_read = !overlapped.contains(&m.id)
            && end_points(m, *along).iter().any(|(p, _)| {
                group.iter().any(|o| {
                    o.id != m.id
                        && (left_out.contains(&o.id) || overlapped.contains(&o.id))
                        && distance_to_box(o, *p) < NEAR_FEET
                })
            });
        if meets_one_not_read {
            continue;
        }
        let count = ends.iter().filter(|end| !end.hidden()).count();
        if let Ok(count) = u8::try_from(count) {
            kept.insert(m.id, count);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vertical mullion's box whose bottom end is at `from`, `length` feet long,
    /// leaning `degrees` off the vertical in the x-z plane.
    fn mullion(id: u32, from: [f64; 3], length: f64, degrees: f64) -> MullionBox {
        let angle = degrees.to_radians();
        let axis = [angle.sin(), 0.0, angle.cos()];
        MullionBox {
            id,
            kind: 1,
            centre: [
                from[0] + 0.5 * length * axis[0],
                from[1],
                from[2] + 0.5 * length * axis[2],
            ],
            axes: [axis, [angle.cos(), 0.0, -angle.sin()], [0.0, 1.0, 0.0]],
            half: [0.5 * length, 0.08, 0.08],
            along: Some(0),
            odd: false,
        }
    }

    #[test]
    fn a_continuation_hides_the_end_it_coincides_with_and_a_bent_one_does_not() {
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0);
        let kept =
            |degrees: f64| kept_ends(&[lower.clone(), mullion(2, [0.0, 0.0, 2.0], 2.0, degrees)]);
        // Face to face: each loses the end it meets.
        assert_eq!(kept(0.0), BTreeMap::from([(1, 1), (2, 1)]));
        assert_eq!(kept(0.01), BTreeMap::from([(1, 1), (2, 1)]));
        // Bent by two degrees (the BIM4LCA office's, RE-432): both ends stay.
        assert_eq!(kept(2.0), BTreeMap::from([(1, 2), (2, 2)]));
    }
}
