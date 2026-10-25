//! A wall's location curve and the walls joined at its ends, from its
//! native `SWall` record (RE-222).
//!
//! The record holds the curve the wall is drawn along, a `GLine`:
//! `m_origin`, the unit `m_dirVec` and `m_endParams`, its start and end
//! along it. Where the partition data gives the wall a line (RE-53) it is
//! the same line, and the record's `m_isFlipped` and `m_wallCrossSection`
//! are the partition data's flip and orientation word (RE-54), on every
//! wall of Core Interior, RE1 Architecture and both BIM4LCA office files.
//! The record is the wall's current copy (RE-216). Where the partition data
//! gives a wall a line but no orientation, because none is found or its
//! copies disagree (13 of the office's 17 US-1 exterior walls), the record's
//! word and flip stand in, so the wall is drawn from its centreline and its
//! joins read; every such wall's layer set and layer centres are then
//! Revit's (RE-222 §8). The partition data drops a wall whose line copies
//! disagree (RE-216); the record's curve is not used for those, as the
//! current copy's line misses 7 of Core Interior's walls by 0.333 ft that
//! their record box draws exactly.
//!
//! The record's `VWallDriver.m_controlJoinsSet` lists the joins at the
//! wall's ends. Each `m_JoinInfo.m_info` lists the walls in one join, this
//! one included, each with the end it joins by (`m_joinedEnd`: 0 its start,
//! 1 its end, 2 or 3 its middle, a T joint), in an order the record keeps.
//! Where another wall meets this wall's end with its middle, Revit's body of
//! this wall stops half that wall's thickness short of its curve's end
//! ([`tee_reach`]), where the wall is one layer. A wall type's
//! `CompoundStructure` lists its layers exterior first (`m_layers`,
//! `m_layerWidth`) and how many lie outside its core on its exterior and its
//! interior side (`m_numShellLayersExt`, `m_numShellLayersInt`): a layered
//! wall's core runs on through the other wall's layers outside its core
//! (RE-223, [`crate::element_record_wall_joins::tee_join`]).

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
    /// How thick the layers outside its type's core are on its exterior and
    /// its interior side, feet; `None` where its type is not a basic wall
    /// type with a compound structure.
    pub shells: Option<[f64; 2]>,
}

/// The widths of a `CompoundStructure`'s layers outside its core, on its
/// exterior and its interior side.
fn shell_widths(structure: &Value) -> Option<[f64; 2]> {
    let widths: Vec<f64> = structure
        .get("m_layers")?
        .as_array()?
        .iter()
        .map(|layer| layer.get("m_layerWidth")?.as_f64())
        .collect::<Option<_>>()?;
    let count = |name: &str| usize::try_from(structure.get(name)?.as_u64()?).ok();
    let (exterior, interior) = (count("m_numShellLayersExt")?, count("m_numShellLayersInt")?);
    if exterior + interior > widths.len() || widths.iter().any(|w| !w.is_finite() || *w < 0.0) {
        return None;
    }
    Some([
        widths[..exterior].iter().sum(),
        widths[widths.len() - interior..].iter().sum(),
    ])
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

/// The curve, word, flip, end joins and type shells of each of `walls` from
/// its native `SWall` record and its type's `BasicWallType` record, read in
/// one pass: the current copy, the one the document routes to (RE-216),
/// whatever the partition data's copies say. A wall whose record has not
/// exactly one curve, or whose curve is not a level line, is left out.
pub fn native_walls(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, NativeWall>> {
    Ok(native_walls_and_top_joins(rf, walls)?.0)
}

/// [`native_walls`], and for every one of `walls` whose `SWall` or `ArcWall`
/// record is read, whether its top is joined to a roof or floor: one of its
/// `JoinToRoofGStep` objects has `m_baseOrTop` 1 (a wall attached at its
/// base only has 0). That is Revit's `Pset_WallCommon.ExtendToStructure`,
/// which revit-ifc's `ExtendToStructureCalculator` takes from Revit's
/// `IsWallJoinedToTop` (CLEANROOM.md source 5): on all 607 walls of the
/// BIM4LCA office as built and 589 at building permit (RE-228).
pub fn native_walls_and_top_joins(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<(BTreeMap<u64, NativeWall>, BTreeMap<u64, bool>)> {
    if walls.is_empty() {
        return Ok((BTreeMap::new(), BTreeMap::new()));
    }
    let mut top_joined: BTreeMap<u64, bool> = BTreeMap::new();
    let options = Options {
        selected_ids: walls.clone(),
        selected_classes: BTreeSet::from(["BasicWallType".to_string()]),
        ..Options::default()
    };
    let mut found: BTreeMap<u64, (NativeWall, Option<u64>)> = BTreeMap::new();
    let mut types: BTreeMap<u64, [f64; 2]> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if class == "BasicWallType" {
            if let Some(shells) = graph
                .objects
                .iter()
                .find(|object| object.class_name == "CompoundStructure")
                .and_then(|structure| shell_widths(&structure.fields))
            {
                types.insert(id, shells);
            }
            return Ok(());
        }
        if !matches!(class, "SWall" | "ArcWall") || !walls.contains(&id) {
            return Ok(());
        }
        top_joined.insert(
            id,
            graph.objects.iter().any(|object| {
                object.class_name == "JoinToRoofGStep"
                    && object.fields.get("m_baseOrTop").and_then(Value::as_i64) == Some(1)
            }),
        );
        if class != "SWall" {
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
            let wall = NativeWall {
                start,
                end,
                flipped: root.and_then(|fields| fields.get("m_isFlipped")?.as_bool()),
                cross_section: root.and_then(|fields| fields.get("m_wallCrossSection")?.as_i64()),
                joins: [0, 1].map(|end| end_join(id, &controls, end)),
                shells: None,
            };
            let type_id = root.and_then(|fields| element_id(fields.get("m_WallAttributesId")?));
            (wall, type_id)
        });
        if let Some(read) = read {
            found.insert(id, read);
        }
        Ok(())
    })?;
    let walls = found
        .into_iter()
        .map(|(id, (mut wall, type_id))| {
            wall.shells = type_id.and_then(|type_id| types.get(&type_id).copied());
            (id, wall)
        })
        .collect();
    Ok((walls, top_joined))
}

/// How far past the `end` (0 its start, 1 its end) of wall `id`'s curve its
/// body reaches where the join there is a T joint: another wall, the host,
/// meets that end with its middle, and Revit's body stops half the host's
/// thickness short of the curve's end. Against Revit's bodies this is
/// Revit's end on 315 of the 396 such ends of the BIM4LCA office as built,
/// where the join lists' reach (RE-70, RE-73) gave 289, and on all 269 of
/// Core Interior's (RE-222). `None` where the join is not a T joint with
/// exactly one host, or the host's thickness is not known.
pub fn tee_reach(
    id: u64,
    end: usize,
    walls: &BTreeMap<u64, NativeWall>,
    thickness: impl Fn(u64) -> Option<f64>,
) -> Option<f64> {
    let join = walls.get(&id)?.joins.get(end)?.as_ref()?;
    let tees: Vec<u64> = join
        .iter()
        .filter(|(other, joined)| *other != id && matches!(joined, 2 | 3))
        .map(|(other, _)| *other)
        .collect();
    let [host] = tees.as_slice() else {
        return None;
    };
    thickness(*host)
        .filter(|t| t.is_finite() && *t > 0.0)
        .map(|t| -t / 2.0)
}
