//! A wall's location curve and the walls joined at its ends, from its
//! native `SWall` record (W6).
//!
//! The record holds the curve the wall is drawn along, a `GLine`:
//! `m_origin`, the unit `m_dirVec` and `m_endParams`, its start and end
//! along it. Where the partition data gives the wall a line (RE-53) it is
//! the same line, and the record's `m_isFlipped` and `m_wallCrossSection`
//! are the partition data's flip and orientation word (RE-54), on every
//! wall of Core Interior, RE1 Architecture and both BIM4LCA office files.
//! Where the partition data gives a wall a line but no orientation (13 of
//! the office's 17 US-1 exterior walls), the record's word and flip stand
//! in, so the wall is drawn from its centreline and its joins read. The
//! partition data drops a wall whose copies disagree (RE-216); the record's
//! curve is not used for those, as the current copy's line misses 7 of Core
//! Interior's walls by 0.333 ft that their record box draws exactly.
//!
//! The record's `VWallDriver.m_controlJoinsSet` lists the joins at the
//! wall's ends. Each `m_JoinInfo.m_info` lists the walls in one join, this
//! one included, each with the end it joins by (`m_joinedEnd`: 0 its start,
//! 1 its end, 2 or 3 its middle, a T joint), in an order the record keeps.
//! Where another wall meets this wall's end with its middle, Revit's body of
//! this wall stops half that wall's thickness short of its curve's end
//! ([`tee_reach`]).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::RevitFile;
use crate::native_document::{Options, extract_graphs};

/// A wall's curve and the joins at its ends, from its native record.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeWall {
    /// The curve's start, plan model feet.
    pub start: [f64; 2],
    /// The curve's end, plan model feet.
    pub end: [f64; 2],
    /// `m_isFlipped`: set puts the wall's exterior to the right of the
    /// curve's direction, as the partition data's flip does (RE-53).
    pub flipped: Option<bool>,
    /// `m_wallCrossSection`, the partition data's orientation word (RE-54).
    pub cross_section: Option<i64>,
    /// The walls in the join at the curve's start and at its end, in the
    /// record's order and this wall included, each with the end it joins
    /// by. Empty where no join lists that end; `None` where several do.
    pub joins: [Option<Vec<(u64, u32)>>; 2],
}

fn element_id(value: &Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(Value::as_i64)
        .and_then(|n| u64::try_from(n).ok())
}

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    match value?.as_array()?.as_slice() {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

/// The plan start and end of a horizontal `GLine`.
fn line_ends(fields: &Value) -> Option<([f64; 2], [f64; 2])> {
    let origin = triple(fields.get("m_origin"))?;
    let direction = triple(fields.get("m_dirVec"))?;
    let [a, b] = match fields.get("m_endParams")?.as_array()?.as_slice() {
        [a, b] => [a.as_f64()?, b.as_f64()?],
        _ => return None,
    };
    let at = |t: f64| [origin[0] + direction[0] * t, origin[1] + direction[1] * t];
    let (start, end) = (at(a), at(b));
    let finite = start.iter().chain(&end).all(|v| v.is_finite());
    (finite && direction[2].abs() <= 1e-9 && (end[0] - start[0]).hypot(end[1] - start[1]) > 1e-9)
        .then_some((start, end))
}

/// The join listing `wall`'s `end`, as (member, end it joins by) pairs.
fn end_join(wall: u64, controls: &[Value], end: u32) -> Option<Vec<(u64, u32)>> {
    let mut found = Vec::new();
    for control in controls {
        let members: Vec<(u64, u32)> = control
            .pointer("/m_JoinInfo/m_info")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|member| {
                let id = element_id(member.get("m_elemId")?)?;
                let joined = u32::try_from(member.get("m_joinedEnd")?.as_u64()?).ok()?;
                Some((id, joined))
            })
            .collect();
        if members.contains(&(wall, end)) {
            found.push(members);
        }
    }
    match found.len() {
        0 => Some(Vec::new()),
        1 => found.pop(),
        _ => None,
    }
}

/// The curve and end joins of each of `walls` from its native `SWall`
/// record. A wall whose record has not exactly one curve, or whose curve is
/// not a level line, is left out, and so is one whose records in several
/// partitions disagree.
pub fn native_walls(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, NativeWall>> {
    if walls.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = Options {
        selected_ids: walls.clone(),
        ..Options::default()
    };
    let mut found: BTreeMap<u64, Option<NativeWall>> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let (Some("SWall"), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if !walls.contains(&id) {
            return Ok(());
        }
        let curves: Vec<_> = graph
            .objects
            .iter()
            .filter(|object| matches!(object.class_name.as_str(), "GLine" | "GArc"))
            .collect();
        let read = match curves.as_slice() {
            [curve] if curve.class_name == "GLine" => line_ends(&curve.fields),
            _ => None,
        }
        .map(|(start, end)| {
            let controls: Vec<Value> = graph
                .objects
                .iter()
                .find(|object| object.class_name == "VWallDriver")
                .and_then(|driver| driver.fields.get("m_controlJoinsSet"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let root = graph.objects.first().map(|object| &object.fields);
            NativeWall {
                start,
                end,
                flipped: root.and_then(|fields| fields.get("m_isFlipped")?.as_bool()),
                cross_section: root.and_then(|fields| fields.get("m_wallCrossSection")?.as_i64()),
                joins: [0, 1].map(|end| end_join(id, &controls, end)),
            }
        });
        match found.get(&id) {
            None => {
                found.insert(id, read);
            }
            Some(seen) if *seen != read => {
                found.insert(id, None);
            }
            Some(_) => {}
        }
        Ok(())
    })?;
    Ok(found
        .into_iter()
        .filter_map(|(id, wall)| Some((id, wall?)))
        .collect())
}

/// How far past `wall`'s end its body reaches where the join there is a T
/// joint: another wall meets that end with its middle, and Revit's body
/// stops half that wall's thickness short of the curve's end. Against
/// Revit's bodies this is Revit's end on 293 of the 374 such ends of the
/// BIM4LCA office as built, where the join lists' reach (RE-70, RE-73) gave
/// 268, and on all 137 of Core Interior's (W6). `None` where the join is not
/// a T joint with exactly one such wall, or that wall's thickness is not
/// known.
pub fn tee_reach(
    wall: u64,
    join: &[(u64, u32)],
    thickness: impl Fn(u64) -> Option<f64>,
) -> Option<f64> {
    let tees: Vec<u64> = join
        .iter()
        .filter(|(id, joined)| *id != wall && matches!(joined, 2 | 3))
        .map(|(id, _)| *id)
        .collect();
    let [tee] = tees.as_slice() else {
        return None;
    };
    thickness(*tee)
        .filter(|t| t.is_finite() && *t > 0.0)
        .map(|t| -t / 2.0)
}
