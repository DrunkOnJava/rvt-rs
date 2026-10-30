//! Which end faces of a curtain mullion Revit's solid keeps, from where the
//! boxes of the mullions of its curtain wall meet (#1046, RE-268).
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
//!   far side, the mullion's continuation;
//! - across it, with `p` on its axis (centre) or off its axis and off its side
//!   faces (inside), where it covers `p`; or across it with `p` on a side face,
//!   where the end only butts against it;
//! - lying over it (the same way, its end at `p` and its body on the same
//!   side, or its box holding `p` elsewhere), or neither along nor across it
//!   (oblique): the end's join is not read.
//!
//! An end is hidden when it has a partner and nothing across covers `p`, or
//! when a mullion across covers `p` and it has no partner. A mullion whose
//! axis is not known, or one of whose ends is not read, is left out, and so
//! is one with an end that meets, within `NEAR_FEET`, a mullion that is left
//! out or is shorter than its own section.

use std::collections::{BTreeMap, BTreeSet};

/// Two points this close, about 3 mm, meet.
const TOLERANCE_FEET: f64 = 0.01;
/// A mullion within this distance of an end, about 6 cm, is its neighbour.
const NEAR_FEET: f64 = 0.2;
/// Two axes whose cosine is above this run the same way.
const PARALLEL: f64 = 0.999;
/// Two axes whose cosine is below this run across each other.
const ACROSS: f64 = 0.01;
/// A mullion shorter than its section by more than this is shorter than it.
const SHORT_MARGIN_FEET: f64 = 0.001;
/// A curtain wall with more mullions than this is left unread: the ends of each
/// are compared with every other mullion of the wall.
const MAX_GROUP: usize = 4096;

/// The box of a curtain mullion: its centre, three unit axes with the
/// half-extent along each (model feet), and the index of the axis it runs
/// along, where that is known.
#[derive(Debug, Clone, PartialEq)]
pub struct MullionBox {
    pub id: u32,
    pub centre: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub half: [f64; 3],
    pub along: Option<usize>,
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
}

impl Neighbours {
    fn unsure(&self) -> bool {
        self.same || self.inside || self.parallel_in || self.oblique
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
                        found.partner = true;
                    } else {
                        found.same = true;
                    }
                }
                None => found.parallel_in = true,
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
/// its two end faces its Revit solid keeps (0, 1 or 2); a mullion left out
/// has none.
pub fn kept_ends(group: &[MullionBox]) -> BTreeMap<u32, u8> {
    if group.len() > MAX_GROUP {
        return BTreeMap::new();
    }
    let mut found: BTreeMap<u32, (usize, [Neighbours; 2])> = BTreeMap::new();
    let mut left_out: BTreeSet<u32> = BTreeSet::new();
    let mut sources: BTreeSet<u32> = BTreeSet::new();
    for m in group {
        let Some(along) = m.along.filter(|a| *a < 3) else {
            left_out.insert(m.id);
            sources.insert(m.id);
            continue;
        };
        let ends = end_points(m, along).map(|(p, out)| neighbours(m, along, p, out, group));
        let section = 2.0 * m.half[(along + 1) % 3].min(m.half[(along + 2) % 3]);
        if ends.iter().any(|end| end.unsure()) {
            left_out.insert(m.id);
            sources.insert(m.id);
        } else if 2.0 * m.half[along] < section - SHORT_MARGIN_FEET {
            sources.insert(m.id);
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
        let meets_one_left_out = end_points(m, *along).iter().any(|(p, _)| {
            group.iter().any(|o| {
                o.id != m.id && sources.contains(&o.id) && distance_to_box(o, *p) < NEAR_FEET
            })
        });
        if meets_one_left_out {
            continue;
        }
        let count = ends.iter().filter(|end| !end.hidden()).count();
        if let Ok(count) = u8::try_from(count) {
            kept.insert(m.id, count);
        }
    }
    kept
}
