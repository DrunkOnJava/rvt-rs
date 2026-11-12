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
//!   that have one; RE-436: nor does one of another section, whose end face
//!   is not the mullion's, and Revit's value counts the end of both pieces
//!   where a 50 x 50 mm and a 150 x 50 mm mullion meet);
//! - across it, with `p` on its axis (centre) or off its axis and off its side
//!   faces (inside), where it covers `p`; or across it with `p` on a side face,
//!   where the end only butts against it;
//! - lying over it (the same way, its end at `p` and its body on the same
//!   side, or its box holding `p` elsewhere): the end is overlapped, and the
//!   mullion that lies over it counts for nothing at `p` (RE-400), when its
//!   box is its solid, which a box with a section that most boxes of its type
//!   do not have is not (RE-403);
//! - neither along nor across it (oblique).
//!
//! An end with a partner is hidden unless another mullion lies at `p`, one
//! oblique to it or one across that holds `p` inside its box, which makes it
//! kept; a mullion across centred on `p` leaves it hidden (RE-438). An end
//! with no partner is hidden when a mullion across is centred on `p`, and kept
//! otherwise, an oblique mullion at it changing nothing (RE-438: on the
//! BIM4LCA concrete office as built Revit counts all 50 ends that have only an
//! oblique neighbour, all 76 that have a partner and one, and all 52 that have
//! a partner and a mullion across inside). A mullion whose axis is not known,
//! or one of whose ends has no partner and a mullion across that holds `p`
//! inside its box (Revit counts 14 such ends and hides 5 that read alike), or
//! an oblique mullion where one across holds `p`, is left out. So is a mullion
//! with an end that meets, within `NEAR_FEET`, a mullion that is left out,
//! unless it is overlapped itself, and a mullion with another lying over one
//! of its ends whose section is not the usual one of its type: on the BIM4LCA
//! timber office at permit 16 of the 22 overlapped mullions that are wrong
//! have such a mullion over an end, and 1 of the 148 that are right does
//! (RE-403). A mullion beside an overlapped mullion is read like any other
//! (RE-441: Revit's count is the rule's on 312 of the 314 such mullions of the
//! BIM4LCA concrete office as built, the other 2 having no value, and on 272 of
//! the 280 of the timber office), but for two kinds: one with a mullion across
//! centred on an end, where Revit keeps an end the rule hides on 3 of the 28 of
//! the timber office, and one with an end that has no partner where an
//! overlapped mullion's axis lies on the plane of the end, a vertical standing
//! on the face of a bar, where Revit hides the end on all 4 of the timber
//! office, none of the 216 chain ends without such a mullion being wrong. A
//! mullion shorter than its section is read like any other and is not a reason
//! to leave its neighbours out (RE-400).

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

/// Whether the section of `o` is that of `m` (RE-436): `o`'s extent along each of
/// `m`'s two sides across the mullion is `m`'s half extent there, to
/// `TOLERANCE_FEET`. The two run along the one axis, so only the sides across
/// count.
fn same_section(m: &MullionBox, along: usize, o: &MullionBox, o_along: usize) -> bool {
    (0..3).filter(|side| *side != along).all(|side| {
        let reach: f64 = (0..3)
            .filter(|other| *other != o_along)
            .map(|other| dot(m.axes[side], o.axes[other]).abs() * o.half[other])
            .sum();
        (reach - m.half[side]).abs() <= TOLERANCE_FEET
    })
}

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
    /// box and the end has no partner (Revit counts such an end on some
    /// mullions and hides it on others that read alike, RE-438), or a mullion
    /// oblique to it lies where one across holds `p` (no end of the BIM4LCA
    /// offices has both).
    fn unread(&self) -> bool {
        (self.inside && !self.partner) || (self.oblique && (self.centre || self.inside))
    }

    /// An end with a mullion lying over it, which counts for nothing at `p`.
    fn overlapped(&self) -> bool {
        self.same || self.parallel_in
    }

    /// Whether the end is hidden: its partner hides it unless another mullion
    /// lies at `p` that is not the partner's, an oblique one or one across that
    /// holds `p` inside its box (RE-438: Revit counts all 128 such ends of the
    /// BIM4LCA concrete office as built, and hides the 4 of the timber office at
    /// permit where the mullion across is centred on `p`); with no partner, a
    /// mullion across centred on `p` hides it.
    fn hidden(&self) -> bool {
        if self.partner {
            !(self.inside || self.oblique)
        } else {
            self.centre
        }
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

/// Whether the axis of `o`, a mullion across `m` near its end `p`, lies on the
/// plane of that end: `o` is centred on `p` along `m`'s axis though its box may not
/// hold `p`, as a vertical that stands on the face of a bar does at the bar's end
/// (RE-441: Revit hides that end).
fn centred_on_end_plane(m: &MullionBox, along: usize, o: &MullionBox, p: [f64; 3]) -> bool {
    let Some(o_along) = o.along.filter(|a| *a < 3) else {
        return false;
    };
    dot(o.axes[o_along], m.axes[along]).abs() < ACROSS
        && dot(sub(o.centre, p), m.axes[along]).abs() < TOLERANCE_FEET
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
                        // end face, and one of another section is not the end
                        // face: nothing hides it (RE-432, RE-436).
                        found.partner |= cosine > COLLINEAR && same_section(m, along, o, o_along);
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

/// How many of a curtain mullion's two end faces its Revit solid keeps, and which
/// of those ends have a partner: one that an oblique mullion or a mullion across
/// holding its point inside keeps (RE-438). The boxes of the wall saw that
/// partner, so the pass that draws the mullion from its saved mesh does not take
/// it for a continuation on another curtain wall that they missed (RE-439).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeptEnds {
    /// 0, 1 or 2.
    pub kept: u8,
    /// Whether the end of the box at its `-axis` side, then the end at its `+axis`
    /// side (the order of `end_points`), is kept and has a partner.
    pub partnered: [bool; 2],
}

/// For each mullion of `group`, the mullions of one curtain wall, how many of
/// its two end faces its Revit solid keeps (0, 1 or 2); a mullion left out has
/// none.
pub fn kept_ends(group: &[MullionBox]) -> BTreeMap<u32, u8> {
    kept_ends_with_partners(group)
        .into_iter()
        .map(|(id, ends)| (id, ends.kept))
        .collect()
}

/// [`kept_ends`], with which of the kept ends of each mullion have a partner
/// (RE-439).
pub fn kept_ends_with_partners(group: &[MullionBox]) -> BTreeMap<u32, KeptEnds> {
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
    let mut kept: BTreeMap<u32, KeptEnds> = BTreeMap::new();
    for m in group {
        let Some((along, ends)) = found.get(&m.id) else {
            continue;
        };
        if left_out.contains(&m.id) {
            continue;
        }
        // An overlapped mullion is read on its own where what lies over it is
        // of its type: its count is Revit's on all 159 of the office's. A
        // neighbour of a mullion left out is not read (RE-400). A neighbour of an
        // overlapped one is read like any other (RE-441), but for two kinds
        // whose count Revit does not give as the rule does: one with a mullion
        // across centred on an end, and one with an end that has no partner where
        // an overlapped mullion's axis lies on the end's plane.
        let points = end_points(m, *along);
        let beside = |ids: &BTreeSet<u32>| {
            points.iter().any(|(p, _)| {
                group.iter().any(|o| {
                    o.id != m.id && ids.contains(&o.id) && distance_to_box(o, *p) < NEAR_FEET
                })
            })
        };
        let doubtful = || {
            ends.iter().any(|end| end.centre)
                || points.iter().zip(ends).any(|((p, _), end)| {
                    !end.partner
                        && group.iter().any(|o| {
                            overlapped.contains(&o.id)
                                && distance_to_box(o, *p) < NEAR_FEET
                                && centred_on_end_plane(m, *along, o, *p)
                        })
                })
        };
        let meets_one_not_read = !overlapped.contains(&m.id)
            && (beside(&left_out) || (beside(&overlapped) && doubtful()));
        if meets_one_not_read {
            continue;
        }
        let count = ends.iter().filter(|end| !end.hidden()).count();
        let partnered = [0, 1].map(|end| !ends[end].hidden() && ends[end].partner);
        if let Ok(count) = u8::try_from(count) {
            kept.insert(
                m.id,
                KeptEnds {
                    kept: count,
                    partnered,
                },
            );
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vertical mullion's box whose bottom end is at `from`, `length` feet long,
    /// leaning `degrees` off the vertical in the x-z plane, with a square section
    /// of half side `side`.
    fn mullion(id: u32, from: [f64; 3], length: f64, degrees: f64, side: f64) -> MullionBox {
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
            half: [0.5 * length, side, side],
            along: Some(0),
            odd: false,
        }
    }

    #[test]
    fn a_continuation_hides_the_end_it_coincides_with_and_a_bent_one_does_not() {
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let kept = |degrees: f64| {
            kept_ends(&[
                lower.clone(),
                mullion(2, [0.0, 0.0, 2.0], 2.0, degrees, 0.08),
            ])
        };
        // Face to face: each loses the end it meets.
        assert_eq!(kept(0.0), BTreeMap::from([(1, 1), (2, 1)]));
        assert_eq!(kept(0.01), BTreeMap::from([(1, 1), (2, 1)]));
        // Bent by two degrees (the BIM4LCA office's, RE-432): both ends stay.
        assert_eq!(kept(2.0), BTreeMap::from([(1, 2), (2, 2)]));
    }

    /// A horizontal mullion along y, `length` feet long and centred on `centre`,
    /// with a square section of half side `side`.
    fn across(id: u32, centre: [f64; 3], length: f64, side: f64) -> MullionBox {
        MullionBox {
            id,
            kind: 1,
            centre,
            axes: [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            half: [0.5 * length, side, side],
            along: Some(0),
            odd: false,
        }
    }

    /// A mullion whose end is at `from`, `length` feet long, leaning `degrees` off
    /// the vertical towards +y in the y-z plane.
    fn leaning(id: u32, from: [f64; 3], length: f64, degrees: f64, side: f64) -> MullionBox {
        let angle = degrees.to_radians();
        let axis = [0.0, angle.sin(), angle.cos()];
        MullionBox {
            id,
            kind: 1,
            centre: [
                from[0],
                from[1] + 0.5 * length * axis[1],
                from[2] + 0.5 * length * axis[2],
            ],
            axes: [axis, [1.0, 0.0, 0.0], [0.0, angle.cos(), -angle.sin()]],
            half: [0.5 * length, side, side],
            along: Some(0),
            odd: false,
        }
    }

    // How reading the ends beside an oblique mullion or a mullion across can go wrong
    // (RE-438): a joint centred under a bar counted whole although Revit hides it, a
    // partner end kept or left out for what lies beside it, and an end read where
    // Revit's count is not told by what the boxes show.
    #[test]
    fn a_partner_end_with_a_mullion_across_centred_on_it_is_hidden() {
        // Two pieces of one line meet at the middle of a bar that crosses them: the
        // chain of 50 x 50 mm mullions of the BIM4LCA timber office at permit.
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let upper = mullion(2, [0.0, 0.0, 2.0], 2.0, 0.0, 0.08);
        let bar = across(3, [0.0, 0.0, 2.0], 4.0, 0.08);
        assert_eq!(
            kept_ends(&[lower, upper, bar]),
            BTreeMap::from([(1, 1), (2, 1), (3, 2)])
        );
    }

    #[test]
    fn an_oblique_mullion_keeps_a_partner_end_and_does_not_leave_the_mullion_out() {
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let upper = mullion(2, [0.0, 0.0, 2.0], 2.0, 0.0, 0.08);
        let brace = leaning(3, [0.0, 0.0, 2.0], 2.0, 45.0, 0.08);
        // The joint's end of each piece counts, and the brace, which has only oblique
        // neighbours, is read like a mullion with none.
        assert_eq!(
            kept_ends(&[lower, upper, brace]),
            BTreeMap::from([(1, 2), (2, 2), (3, 2)])
        );
    }

    #[test]
    fn a_mullion_across_inside_leaves_an_unpartnered_end_unread_and_a_partnered_one_kept() {
        // A bar whose face holds the joint 0.04 ft off its middle.
        let bar = across(3, [0.0, 0.0, 2.04], 4.0, 0.08);
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        assert!(!kept_ends(&[lower.clone(), bar.clone()]).contains_key(&1));
        let upper = mullion(2, [0.0, 0.0, 2.0], 2.0, 0.0, 0.08);
        assert_eq!(
            kept_ends(&[lower, upper, bar]),
            BTreeMap::from([(1, 2), (2, 2), (3, 2)])
        );
    }

    #[test]
    fn an_oblique_mullion_where_one_across_is_centred_is_not_read() {
        // Revit's count of such an end is not told by the boxes (no end of the
        // BIM4LCA offices has both): the mullion is left out as before.
        let post = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let bar = across(2, [0.0, 0.0, 2.0], 4.0, 0.08);
        let brace = leaning(3, [0.0, 0.0, 2.0], 2.0, 45.0, 0.08);
        assert!(!kept_ends(&[post, bar, brace]).contains_key(&1));
    }

    // How telling the kept ends that have a partner can go wrong (RE-439): a hidden
    // joint is told as a kept end with a partner, the end an oblique mullion keeps
    // is told as one with none (the pass that draws the mullion from its saved mesh
    // then takes it for a continuation the boxes missed), or the two ends are told
    // the wrong way round (the pass maps the mesh's ends to them by the side of the
    // axis each lies on, the `-axis` end first).
    #[test]
    fn the_kept_ends_with_a_partner_are_the_joints_an_oblique_mullion_keeps() {
        let with_partners = |group: &[MullionBox]| {
            kept_ends_with_partners(group)
                .into_iter()
                .map(|(id, ends)| (id, (ends.kept, ends.partnered)))
                .collect::<BTreeMap<_, _>>()
        };
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let upper = mullion(2, [0.0, 0.0, 2.0], 2.0, 0.0, 0.08);
        // A joint nothing else lies at is hidden: no kept end has a partner.
        assert_eq!(
            with_partners(&[lower.clone(), upper.clone()]),
            BTreeMap::from([(1, (1, [false, false])), (2, (1, [false, false]))])
        );
        // An oblique brace at the joint keeps it: the end of each piece at the joint
        // (the `+axis` end of the lower piece, the `-axis` end of the upper one) is
        // kept and has a partner, and the brace has none.
        let brace = leaning(3, [0.0, 0.0, 2.0], 2.0, 45.0, 0.08);
        assert_eq!(
            with_partners(&[lower, upper, brace]),
            BTreeMap::from([
                (1, (2, [false, true])),
                (2, (2, [true, false])),
                (3, (2, [false, false])),
            ])
        );
    }

    #[test]
    fn a_continuation_of_another_section_hides_no_end() {
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let kept =
            |side: f64| kept_ends(&[lower.clone(), mullion(2, [0.0, 0.0, 2.0], 2.0, 0.0, side)]);
        assert_eq!(kept(0.08), BTreeMap::from([(1, 1), (2, 1)]));
        // A section 0.17 ft wider on a side (the 50 x 50 and 150 x 50 mm pieces of
        // the BIM4LCA timber office, RE-436): its end face is not the mullion's.
        assert_eq!(kept(0.25), BTreeMap::from([(1, 2), (2, 2)]));
        // A section that differs by less than the 3 mm the boxes are read to is one.
        assert_eq!(kept(0.085), BTreeMap::from([(1, 1), (2, 1)]));
    }

    /// Two bars of one line along y that each hold an end of the other, so that both
    /// are overlapped, `x` feet from the vertical axis and `z` feet up: the pair
    /// of verticals that stand on a bar of the BIM4LCA timber office, seen end on.
    fn overlapped_pair(first_id: u32, x: f64, z: f64) -> [MullionBox; 2] {
        [
            across(first_id, [x, -0.5, z], 3.0, 0.08),
            across(first_id + 1, [x, 0.5, z], 3.0, 0.08),
        ]
    }

    // How reading the mullions beside an overlapped mullion can go wrong (RE-441): not
    // read at all, as before; or read where a bar that stands on the mullion's face is
    // centred on the plane of its free end, which Revit hides and the end's point
    // does not show; or read where a mullion across is centred on an end; or the
    // neighbours of a mullion left out read with them.
    #[test]
    fn a_mullion_beside_an_overlapped_pair_is_read_unless_a_bar_is_centred_on_its_free_end() {
        // A 0.3 ft stub whose top end meets its continuation and whose bottom end is free.
        let stub = mullion(1, [0.0, 0.0, 0.0], 0.3, 0.0, 0.08);
        let above = mullion(2, [0.0, 0.0, 0.3], 2.0, 0.0, 0.08);
        let with_pair = |z: f64| {
            let [first, second] = overlapped_pair(3, 0.16, z);
            kept_ends(&[stub.clone(), above.clone(), first, second])
        };
        // The pair's axes lie 0.08 ft from the plane of the free end, at the stub's side
        // face: the partner end is hidden and the free end kept.
        assert_eq!(with_pair(0.08).get(&1), Some(&1));
        // A pair centred on that plane: Revit hides the end, the boxes do not show it.
        assert_eq!(with_pair(0.0).get(&1), None);
    }

    #[test]
    fn a_mullion_with_a_bar_centred_on_an_end_is_not_read_beside_an_overlapped_pair() {
        let stub = mullion(1, [0.0, 0.0, 0.0], 0.3, 0.0, 0.08);
        let above = mullion(2, [0.0, 0.0, 0.3], 2.0, 0.0, 0.08);
        // A bar through the free end, centred on it: both ends of the stub are hidden.
        let bar = across(5, [0.0, 0.0, 0.0], 4.0, 0.08);
        assert_eq!(
            kept_ends(&[stub.clone(), above.clone(), bar.clone()]).get(&1),
            Some(&0)
        );
        let [first, second] = overlapped_pair(3, 0.16, 0.08);
        assert_eq!(kept_ends(&[stub, above, bar, first, second]).get(&1), None);
    }

    #[test]
    fn a_mullion_beside_one_left_out_is_still_not_read() {
        // The mullion of the oldest test above, left out for a bar across inside its
        // unpartnered end, with a mullion 0.12 ft from its bottom end.
        let lower = mullion(1, [0.0, 0.0, 0.0], 2.0, 0.0, 0.08);
        let bar = across(3, [0.0, 0.0, 2.04], 4.0, 0.08);
        let beside = mullion(4, [0.2, 0.0, 0.0], 1.0, 0.0, 0.08);
        assert!(!kept_ends(&[lower.clone(), bar.clone()]).contains_key(&1));
        assert!(!kept_ends(&[lower, bar, beside.clone()]).contains_key(&4));
        assert_eq!(kept_ends(&[beside]).get(&4), Some(&2));
    }
}
