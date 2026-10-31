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
//!
//! Revit's export writes a wall's `Qto_WallBaseQuantities.Length` as either
//! its curve's length or the length of its curve cut back where its body
//! meets the walls joined at its ends (revit-ifc's `LengthCalculator` and
//! `WallExporter`, CLEANROOM.md source 5), one or the other by how the
//! export draws the wall. [`trimmed_ends`] reads where the cut-back curve
//! ends from the record: the curve's `GLine` ends on a joined wall's middle,
//! the record's outermost two `Plane` objects along the curve are its faces,
//! and an end whose `JoinEndGStep.m_exposedFaceMap` names a wall runs to that
//! wall's far face, any other joined end to the nearest face of the walls
//! joined there (RE-252).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::RevitFile;
use crate::native_document::{Options, extract_graphs};
use crate::native_json::{element_id, element_id_i64};

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
    /// The signed distances, to the left of the curve's direction, of the
    /// record's outermost two upright `Plane` objects along it, its faces,
    /// feet; `None` where it has none.
    pub faces: Option<[f64; 2]>,
    /// The elements whose faces the `JoinEndGStep.m_exposedFaceMap` of the
    /// curve's start and of its end names, in its order.
    pub exposed: [Vec<u64>; 2],
    /// The walls whose end meets this wall's middle, from its `CDJoinInfo`
    /// objects, each with that end (0 or 1).
    pub side_joins: Vec<(u64, u32)>,
    /// The other element of each `CleanPairGStep` and `CutPairGStep`
    /// (`m_otherId`) and `WallEmbedWallGStep` (`m_instId`).
    pub partners: Vec<u64>,
    /// `m_WallAttributesId`, the type the record names.
    pub type_id: Option<u64>,
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

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    match value?.as_array()?.as_slice() {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

/// The signed distances to the left of the line from `start` along the unit
/// `direction` of the outermost two of `planes` that stand upright along it.
fn face_offsets(planes: &[&Value], start: [f64; 2], direction: [f64; 2]) -> Option<[f64; 2]> {
    let offsets: Vec<f64> = planes
        .iter()
        .filter_map(|plane| {
            let origin = triple(plane.get("m_origin"))?;
            let x = triple(plane.get("m_xVec"))?;
            let y = triple(plane.get("m_yVec"))?;
            let along =
                (direction[0] * x[1] - direction[1] * x[0]).abs() <= 1e-6 && x[2].abs() <= 1e-6;
            let upright = (y[2].abs() - 1.0).abs() <= 1e-6;
            (along && upright).then(|| {
                (origin[0] - start[0]) * -direction[1] + (origin[1] - start[1]) * direction[0]
            })
        })
        .filter(|offset| offset.is_finite())
        .collect();
    let low = offsets.iter().copied().reduce(f64::min)?;
    let high = offsets.iter().copied().reduce(f64::max)?;
    Some([low, high])
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

/// The ElementIds named by a wall's top join (every `JoinToRoofGStep` with
/// `m_baseOrTop` 1, its `m_ids`) and by its `CutPairGStep`s (`m_otherId`),
/// from the `(class, fields)` of the objects of its record; a null reference
/// (an id of 0 or less) names nothing. `None` where a step names its
/// elements in a shape that is not read, so that no step is left out unseen.
fn named_by_steps<'a>(steps: impl Iterator<Item = (&'a str, &'a Value)>) -> Option<BTreeSet<u64>> {
    let mut named = BTreeSet::new();
    for (class, fields) in steps {
        let references: Vec<&Value> = match class {
            "JoinToRoofGStep" if fields.get("m_baseOrTop").and_then(Value::as_i64) == Some(1) => {
                fields.get("m_ids")?.as_array()?.iter().collect()
            }
            "CutPairGStep" => vec![fields.get("m_otherId")?],
            _ => continue,
        };
        for reference in references {
            let id = element_id_i64(reference)?;
            if id > 0 {
                named.insert(u64::try_from(id).ok()?);
            }
        }
    }
    Some(named)
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
    Ok(native_walls_and_top_joins(rf, walls)?.walls)
}

/// The name of each curtain wall type in `types` whose native record is a
/// `NewCurtainWallType`: its `SymbolInfo.m_name`. A curtain wall type's
/// serialised data gives no name where a basic wall type's does (#322), and
/// on the BIM4LCA office these names are the type half of Revit's name for
/// all 220 curtain walls (RE-230).
pub fn curtain_wall_type_names(
    rf: &mut RevitFile,
    types: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, String>> {
    let mut out = BTreeMap::new();
    if types.is_empty() {
        return Ok(out);
    }
    let options = Options {
        selected_ids: types.clone(),
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        if record.class_name.as_deref() != Some("NewCurtainWallType") {
            return Ok(());
        }
        if let Some(name) = record.graph.as_ref().and_then(|graph| {
            graph
                .objects
                .iter()
                .find(|o| o.class_name == "SymbolInfo")?
                .fields
                .get("m_name")?
                .as_str()
        }) {
            out.insert(record.identity.element_id, name.to_string());
        }
        Ok(())
    })?;
    Ok(out)
}

/// The step classes of a wall's record that leave its solid as its joins and
/// its type's layers make it (RE-270): every other class ending in `Step` may
/// cut, join, embed or sweep it.
const HARMLESS_STEPS: [&str; 6] = [
    "BaseWallGStep",
    "JoinEndGStep",
    "VCWSplitFaceGStep",
    "WallRefPlanesGStep",
    "WallJoinTweakGStep",
    "VerticalExtensionOfLayersGStep",
];

/// The step classes besides `HARMLESS_STEPS` that a wall joined to a roof or
/// floor may hold and still be drawn as its joins and layers make it, when
/// the elements it is joined to are accounted for (RE-270).
const BOUNDED_STEPS: [&str; 2] = ["JoinToRoofGStep", "CleanPairGStep"];

/// What [`native_walls_and_top_joins`] reads from walls' native records.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WallRecords {
    /// [`native_walls`].
    pub walls: BTreeMap<u64, NativeWall>,
    /// Whether each wall's top is joined to a roof or floor.
    pub top_joined: BTreeMap<u64, bool>,
    /// The walls nothing is joined to or cuts.
    pub uncut: BTreeSet<u64>,
    /// The ElementIds each wall's top could be cut at (RE-267): those its
    /// top join (`JoinToRoofGStep` with `m_baseOrTop` 1) and its
    /// `CutPairGStep`s name, in ascending order. A wall that names none, or
    /// one of whose steps is not readable, is left out.
    pub top_cutters: BTreeMap<u64, Vec<u64>>,
    /// The walls whose record holds no step but the harmless ones (a wall's
    /// base, ends, split faces, reference planes, join tweaks and layer
    /// extensions):
    /// nothing cuts, embeds or sweeps the solid its joins and layers make
    /// (RE-270).
    pub whole: BTreeSet<u64>,
    /// The walls whose record holds no step but the harmless ones (a wall's
    /// base, ends, split faces, reference planes, join tweaks and layer
    /// extensions) and a roof or floor join or a clean join, with the ElementIds their
    /// roof and floor joins name (at the top and at the base, in ascending
    /// order; none where they hold none). A wall whose join names its
    /// elements in a shape that is not read is left out (RE-270).
    pub bounded: BTreeMap<u64, Vec<u64>>,
}

/// Whether a wall's top is joined to a roof or floor: one of the
/// `JoinToRoofGStep` objects of its record has `m_baseOrTop` 1.
fn is_top_joined(graph: &crate::native_parameters::ObjectGraph) -> bool {
    graph.objects.iter().any(|object| {
        object.class_name == "JoinToRoofGStep"
            && object.fields.get("m_baseOrTop").and_then(Value::as_i64) == Some(1)
    })
}

/// Whether the top of each of `walls` whose `SWall` or `ArcWall` record is
/// read is joined to a roof or floor ([`WallRecords::top_joined`]), without
/// the rest of [`native_walls_and_top_joins`]. Read on Revit 2015 to 2027
/// (RE-284). Errs where the native record path does not read the file's
/// release.
pub fn wall_top_joins(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, bool>> {
    let mut top_joined: BTreeMap<u64, bool> = BTreeMap::new();
    if walls.is_empty() {
        return Ok(top_joined);
    }
    let options = Options {
        selected_ids: walls.clone(),
        releases: crate::native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if matches!(class, "SWall" | "ArcWall") && walls.contains(&id) {
            top_joined.insert(id, is_top_joined(graph));
        }
        Ok(())
    })?;
    Ok(top_joined)
}

/// [`native_walls`], and for every one of `walls` whose `SWall` or `ArcWall`
/// record is read, whether its top is joined to a roof or floor: one of its
/// `JoinToRoofGStep` objects has `m_baseOrTop` 1 (a wall attached at its
/// base only has 0). That is Revit's `Pset_WallCommon.ExtendToStructure`,
/// which revit-ifc's `ExtendToStructureCalculator` takes from Revit's
/// `IsWallJoinedToTop` (CLEANROOM.md source 5): on all 607 walls of the
/// BIM4LCA office as built and 589 at building permit (RE-228).
///
/// Also the walls whose record holds no `JoinToRoofGStep`, no
/// `CutPairGStep` and no `WallCutoutGStep`: no roof or floor joined to it,
/// no element cutting it, no hosted instance's cutout. The record box of
/// each is as tall as Revit's body, and Revit's export writes its `Height`
/// on every one: 81 of the BIM4LCA office's walls as built, 70 at building
/// permit and 93 of puzzbobb's R2025 walls (RE-266).
///
/// And the elements each wall's top join and cut pairs name, from which the
/// export cuts the top at a floor (RE-267, [`WallRecords::top_cutters`]).
pub fn native_walls_and_top_joins(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<WallRecords> {
    if walls.is_empty() {
        return Ok(WallRecords::default());
    }
    let mut top_joined: BTreeMap<u64, bool> = BTreeMap::new();
    let mut uncut: BTreeSet<u64> = BTreeSet::new();
    let mut top_cutters: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut whole: BTreeSet<u64> = BTreeSet::new();
    let mut bounded: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
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
        top_joined.insert(id, is_top_joined(graph));
        if !graph.objects.iter().any(|object| {
            matches!(
                object.class_name.as_str(),
                "JoinToRoofGStep" | "CutPairGStep" | "WallCutoutGStep"
            )
        }) {
            uncut.insert(id);
        }
        let steps = graph
            .objects
            .iter()
            .map(|object| (object.class_name.as_str(), &object.fields));
        if let Some(named) = named_by_steps(steps).filter(|named| !named.is_empty()) {
            top_cutters.insert(id, named.into_iter().collect());
        }
        if graph.objects.iter().all(|object| {
            !object.class_name.ends_with("Step")
                || HARMLESS_STEPS.contains(&object.class_name.as_str())
        }) {
            whole.insert(id);
        }
        if graph.objects.iter().all(|object| {
            !object.class_name.ends_with("Step")
                || HARMLESS_STEPS.contains(&object.class_name.as_str())
                || BOUNDED_STEPS.contains(&object.class_name.as_str())
        }) {
            let named: Option<BTreeSet<u64>> = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "JoinToRoofGStep")
                .map(|object| object.fields.get("m_ids")?.as_array())
                .collect::<Option<Vec<_>>>()
                .and_then(|lists| {
                    let mut named = BTreeSet::new();
                    for reference in lists.into_iter().flatten() {
                        let id = element_id_i64(reference)?;
                        if id > 0 {
                            named.insert(u64::try_from(id).ok()?);
                        }
                    }
                    Some(named)
                });
            if let Some(named) = named {
                bounded.insert(id, named.into_iter().collect());
            }
        }
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
            let length = (end[0] - start[0]).hypot(end[1] - start[1]);
            let direction = [(end[0] - start[0]) / length, (end[1] - start[1]) / length];
            let planes: Vec<&Value> = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "Plane")
                .map(|object| &object.fields)
                .collect();
            let mut exposed = [Vec::new(), Vec::new()];
            let mut stepped = [false, false];
            for step in graph
                .objects
                .iter()
                .filter(|object| object.class_name == "JoinEndGStep")
                .map(|object| &object.fields)
            {
                let Some(slot) = step
                    .get("m_end")
                    .and_then(Value::as_u64)
                    .and_then(|end| usize::try_from(end).ok())
                    .filter(|end| *end < 2)
                else {
                    continue;
                };
                if !std::mem::replace(&mut stepped[slot], true) {
                    exposed[slot] = step
                        .get("m_exposedFaceMap")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|face| element_id(face.pointer("/second/m_elemId")?))
                        .collect();
                }
            }
            let side_joins = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "CDJoinInfo")
                .flat_map(|join| {
                    join.fields
                        .get("m_info")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter_map(|member| {
                    let other = element_id(member.get("m_elemId")?)?;
                    let joined = u32::try_from(member.get("m_joinedEnd")?.as_u64()?).ok()?;
                    (other != id && joined < 2).then_some((other, joined))
                })
                .collect();
            let partners = graph
                .objects
                .iter()
                .filter_map(|object| {
                    let field = match object.class_name.as_str() {
                        "CleanPairGStep" | "CutPairGStep" => "m_otherId",
                        "WallEmbedWallGStep" => "m_instId",
                        _ => return None,
                    };
                    Some(object.fields.get(field).and_then(element_id))
                })
                .map(|other| other.unwrap_or(u64::MAX))
                .collect();
            let wall = NativeWall {
                start,
                end,
                flipped: root.and_then(|fields| fields.get("m_isFlipped")?.as_bool()),
                cross_section: root.and_then(|fields| fields.get("m_wallCrossSection")?.as_i64()),
                joins: [0, 1].map(|end| end_join(id, &controls, end)),
                shells: None,
                faces: face_offsets(&planes, start, direction),
                exposed,
                side_joins,
                partners,
                type_id: None,
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
            wall.type_id = type_id;
            (id, wall)
        })
        .collect();
    Ok(WallRecords {
        walls,
        top_joined,
        uncut,
        top_cutters,
        whole,
        bounded,
    })
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

/// The walls joined at each end of each of `walls`: those its record's join
/// lists name there, and those whose `CDJoinInfo` pairs that end with their
/// middle.
pub fn joined_at_ends(walls: &BTreeMap<u64, NativeWall>) -> BTreeMap<(u64, u32), BTreeSet<u64>> {
    let mut out: BTreeMap<(u64, u32), BTreeSet<u64>> = BTreeMap::new();
    for (&id, wall) in walls {
        for (end, join) in (0u32..).zip(&wall.joins) {
            for &(other, _) in join.iter().flatten() {
                if other != id {
                    out.entry((id, end)).or_default().insert(other);
                }
            }
        }
        for &(other, end) in &wall.side_joins {
            out.entry((other, end)).or_default().insert(id);
        }
    }
    out
}

/// Where wall `id`'s curve, cut back where its body meets the walls joined
/// at its ends, starts and ends, as distances along it from its start, feet
/// (RE-252). Each end is read twice: from the record, as the curve's end
/// where no wall is joined there, the far face of the wall its end step's
/// exposed face map names, or else the nearest face of the walls joined
/// there; and from `reaches`, how far past that end the body reaches (RE-70,
/// RE-71, RE-73, RE-222). An end is taken where the two agree, or where no
/// wall is joined, no reach is given and nothing cuts the wall. `None`
/// where an end is not taken, or an element other than one of `walls` cuts
/// or joins the wall. Against Revit's exports of the BIM4LCA office (both
/// stages) and puzzbobb's Revit 2025 models, every wall whose cut-back curve
/// this gives is as long as its curve has that length as Revit's `Length`.
pub fn trimmed_ends(
    id: u64,
    walls: &BTreeMap<u64, NativeWall>,
    joined: &BTreeMap<(u64, u32), BTreeSet<u64>>,
    reaches: [Option<f64>; 2],
) -> Option<[f64; 2]> {
    const AGREE_FEET: f64 = 1e-4;
    let wall = walls.get(&id)?;
    if wall.partners.iter().any(|other| !walls.contains_key(other))
        || wall.joins.iter().any(Option::is_none)
    {
        return None;
    }
    let length = (wall.end[0] - wall.start[0]).hypot(wall.end[1] - wall.start[1]);
    let direction = [
        (wall.end[0] - wall.start[0]) / length,
        (wall.end[1] - wall.start[1]) / length,
    ];
    // Where this wall's curve crosses the other wall's face at `offset`.
    let crossing = |other: &NativeWall, offset: f64| {
        let span = (other.end[0] - other.start[0]).hypot(other.end[1] - other.start[1]);
        let along = [
            (other.end[0] - other.start[0]) / span,
            (other.end[1] - other.start[1]) / span,
        ];
        let point = [
            other.start[0] - along[1] * offset,
            other.start[1] + along[0] * offset,
        ];
        let cross = direction[0] * along[1] - direction[1] * along[0];
        (cross.abs() >= 1e-9).then(|| {
            ((point[0] - wall.start[0]) * along[1] - (point[1] - wall.start[1]) * along[0]) / cross
        })
    };
    let faces = |other: u64| -> Vec<f64> {
        walls
            .get(&other)
            .and_then(|o| Some((o, o.faces?)))
            .map(|(o, faces)| faces.iter().filter_map(|&f| crossing(o, f)).collect())
            .unwrap_or_default()
    };
    fn nearest(to: f64) -> impl Fn(&f64, &f64) -> std::cmp::Ordering {
        move |a, b| (a - to).abs().total_cmp(&(b - to).abs())
    }
    let end_of = |end: u32| -> Option<f64> {
        let (here, there) = if end == 0 {
            (0.0, length)
        } else {
            (length, 0.0)
        };
        let reach = reaches[end as usize].map(|r| if end == 0 { here - r } else { here + r });
        let read = match joined.get(&(id, end)) {
            None if reach.is_none() && wall.partners.is_empty() => return Some(here),
            None => Some(here),
            Some(named) => match wall.exposed[end as usize]
                .iter()
                .find(|other| walls.contains_key(other))
            {
                Some(&other) => faces(other).into_iter().max_by(nearest(there)),
                None => named
                    .iter()
                    .filter_map(|&other| faces(other).into_iter().min_by(nearest(there)))
                    .min_by(nearest(there)),
            },
        };
        let (read, reach) = (read?, reach?);
        ((read - reach).abs() <= AGREE_FEET).then_some(read)
    };
    Some([end_of(0)?, end_of(1)?])
}
