//! A multistory stair is written once per storey (#1977, RE-463).
//!
//! Revit's export writes each stair of a multistory group on its own Level and
//! again, as a copy, on each Level above that the group's sets leave uncovered:
//! on Autodesk's Snowdon Towers architectural sample the three groups hold one
//! such span each, L4 to L5, and the copy of the stair below it is the stair
//! again, with its own parts, 4.064 m higher. Revit's copy differs from its
//! original in a few ways and in no other (RE-463, section 2): the stair and
//! each part but a railing take the name of the original with `:2` appended,
//! all keep the original's `Tag`, the copy of the stair, its flights and its
//! landing has no type object (its stringers keep theirs), its flights'
//! `PredefinedType` is `NOTDEFINED`, and every `GlobalId` is its own.
//!
//! The sets of a group come from [`crate::native_multistory_stairs`], the
//! Levels from the partition Level records. A span between two consecutive
//! Levels of the sets' stairs that no set covers gets the copy of the set whose
//! stair ends where the span begins, when the span is as high as that stair.
//! Nothing in the file says how far a stair repeats (RE-463, section 3), so a
//! span the rule does not reach is counted, not copied.
//!
//! A copy shares its original's `Tag`, so the writer gives it a `GlobalId` of
//! its own by the rule that serves the further pieces of an element (#1840): it
//! is not Revit's, and the copy says so with [`MULTISTORY_COPY_PROPERTY`].

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use super::IfcModel;
use super::entities::{IfcEntity, Property, PropertySet, PropertyValue};
use crate::RevitFile;
use crate::native_multistory_stairs::{MULTISTORY_REVIT_VERSIONS, MultistoryGroup, StairSet};

/// Property of a copy's record set: the ElementId of the stair it repeats.
pub(crate) const MULTISTORY_COPY_PROPERTY: &str = "MultistoryCopyOf";

/// Whether the element of `ifc_type` with the record set `set` is a copy of a
/// multistory stair that Revit's export gives no type object: the stair, its
/// flights and its landing. Its stringers keep their originals' type (30 of 30
/// on Snowdon Towers architectural, RE-463) and its railings have none either way.
pub(crate) fn copy_without_type_object(ifc_type: &str, set: Option<&PropertySet>) -> bool {
    !ifc_type.eq_ignore_ascii_case("IFCMEMBER")
        && set.is_some_and(|set| {
            set.properties
                .iter()
                .any(|property| property.name == MULTISTORY_COPY_PROPERTY)
        })
}

/// How far two heights may differ for a span to be as high as a stair, feet.
const HEIGHT_TOLERANCE_FEET: f64 = 0.01;

/// How far two elevations may differ for a Level to be a storey's, feet.
const ELEVATION_TOLERANCE_FEET: f64 = 1e-6;

/// What the pass did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct MultistoryReport {
    /// Multistory groups in the file.
    pub(crate) groups: usize,
    /// Copies written.
    pub(crate) copies: usize,
    /// Entities those copies added to the model.
    pub(crate) entities: usize,
    /// Why a span or a stair was left uncopied, by reason, with how many.
    pub(crate) left_out: BTreeMap<String, usize>,
}

/// The reasons [`MultistoryReport::left_out`] counts.
pub(crate) mod reason {
    /// A span no set covers that no set's stair ends below, or whose height is
    /// not that stair's.
    pub(crate) const SPAN_NOT_REPEATED: &str = "span_not_the_height_of_the_stair_below";
    /// A Level of a set that the Level records do not give.
    pub(crate) const LEVEL_NOT_READ: &str = "level_not_read";
    /// The stair of the set is not exported as a stair carried by parts.
    pub(crate) const STAIR_NOT_EXPORTED: &str = "stair_not_exported_with_parts";
    /// The Level the copy starts on is no storey of the export.
    pub(crate) const STOREY_NOT_FOUND: &str = "copy_level_is_no_storey";
    /// A part cannot be moved: it has no location, or the model keeps
    /// something about it by index that a copy would not carry, or it has no
    /// record set to carry the copy's mark where the writer needs the mark.
    pub(crate) const NOT_MOVABLE: &str = "stair_part_not_movable";
}

/// The Level and elevation of one end of a set's stair.
#[derive(Debug, Clone, Copy)]
struct Floor {
    id: u64,
    elevation_feet: f64,
}

/// The parts of each aggregate of a model, by the index of its whole.
type PartsOf = BTreeMap<usize, Vec<usize>>;

/// The index of each stair of a model that parts carry, by ElementId.
type StairIndexes = BTreeMap<u64, usize>;

/// The aggregates of `model` and, among their wholes, the stairs.
fn stairs_carried_by_parts(model: &IfcModel) -> (PartsOf, StairIndexes) {
    let parts_of: PartsOf = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, parts } => Some((*whole, parts.clone())),
            _ => None,
        })
        .collect();
    let mut stairs: StairIndexes = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        if ifc_type != "IFCSTAIR" || !parts_of.contains_key(&index) {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            stairs.entry(id).or_insert(index);
        }
    }
    (parts_of, stairs)
}

/// What the file says of its multistory stairs: the groups, and the Levels
/// their sets span.
#[derive(Debug)]
pub(crate) struct MultistoryRead {
    groups: Vec<MultistoryGroup>,
    levels: BTreeMap<u64, (String, f64)>,
}

/// Read the multistory groups of `rf` for [`attach`]. `None`, and nothing read
/// from the file, where the model holds no stair carried by parts or the
/// release is not one the records are measured on; `None` too where the file
/// holds no group.
///
/// The export makes this read before the saved-graphics pass, while the handle
/// still holds the partitions' inflated copies: that pass lets them go, and a
/// read after it inflates the whole file again for these records alone (+408
/// MiB of peak and +1 s on Snowdon Towers architectural, Measure run
/// 43566139741 of the first version of this pass, #1977).
pub(crate) fn read(rf: &mut RevitFile, model: &IfcModel) -> Result<Option<MultistoryRead>> {
    let release = rf.layout_release()?;
    if !MULTISTORY_REVIT_VERSIONS.contains(&release) {
        return Ok(None);
    }
    if stairs_carried_by_parts(model).1.is_empty() {
        return Ok(None);
    }
    let groups = crate::native_multistory_stairs::multistory_groups(rf)?;
    if groups.is_empty() {
        return Ok(None);
    }
    let levels: BTreeMap<u64, (String, f64)> =
        crate::partition_level_records::recover_partition_levels(rf, release)?
            .into_iter()
            .map(|level| {
                (
                    u64::from(level.element_id),
                    (level.name, level.elevation_feet),
                )
            })
            .collect();
    Ok(Some(MultistoryRead { groups, levels }))
}

/// Write the copies of the multistory stairs `read` found into `model`, as it
/// stands after the passes that give its stairs their bodies.
pub(crate) fn attach(model: &mut IfcModel, read: &MultistoryRead) -> MultistoryReport {
    let mut report = MultistoryReport {
        groups: read.groups.len(),
        ..MultistoryReport::default()
    };
    let (parts_of, stairs) = stairs_carried_by_parts(model);
    for group in &read.groups {
        for plan in plan_copies(group, &read.levels, &stairs, &mut report.left_out) {
            let Some(storey) = model.building_storeys.iter().position(|storey| {
                storey.name == plan.level_name
                    && (storey.elevation_feet - plan.level_elevation_feet).abs()
                        <= ELEVATION_TOLERANCE_FEET
            }) else {
                *report
                    .left_out
                    .entry(reason::STOREY_NOT_FOUND.to_string())
                    .or_insert(0) += 1;
                continue;
            };
            let Some(&whole) = stairs.get(&plan.stair) else {
                continue;
            };
            let parts = parts_of.get(&whole).cloned().unwrap_or_default();
            match copy_stair(model, whole, &parts, plan.shift_feet, storey, plan.stair) {
                Some(added) => {
                    report.copies += 1;
                    report.entities += added;
                }
                None => {
                    *report
                        .left_out
                        .entry(reason::NOT_MOVABLE.to_string())
                        .or_insert(0) += 1;
                }
            }
        }
    }
    report
}

/// One copy to write: the stair to repeat, how far up, and where it starts.
#[derive(Debug, Clone, PartialEq)]
struct CopyPlan {
    stair: u64,
    shift_feet: f64,
    level_name: String,
    level_elevation_feet: f64,
}

/// The copies a group asks for: for each span between consecutive Levels of
/// the sets' stairs that no set covers, the set whose stair ends where the
/// span begins and is as high as the span. Spans the rule does not reach are
/// counted in `left_out`.
fn plan_copies(
    group: &MultistoryGroup,
    levels: &BTreeMap<u64, (String, f64)>,
    stairs: &BTreeMap<u64, usize>,
    left_out: &mut BTreeMap<String, usize>,
) -> Vec<CopyPlan> {
    let mut count = |why: &str| *left_out.entry(why.to_string()).or_insert(0) += 1;
    let floor = |id: u64| {
        levels.get(&id).map(|(_, elevation_feet)| Floor {
            id,
            elevation_feet: *elevation_feet,
        })
    };
    // Each set with its two ends; one whose Levels are not read ends the group.
    let mut ends: Vec<(&StairSet, Floor, Floor)> = Vec::new();
    for set in &group.sets {
        let (Some(base), Some(top)) = (floor(set.base_level), floor(set.top_level)) else {
            count(reason::LEVEL_NOT_READ);
            return Vec::new();
        };
        ends.push((set, base, top));
    }
    // The Levels the sets' stairs start and end on, lowest first.
    let mut points: Vec<Floor> = ends
        .iter()
        .flat_map(|(_, base, top)| [*base, *top])
        .collect();
    points.sort_by(|a, b| {
        a.elevation_feet
            .total_cmp(&b.elevation_feet)
            .then(a.id.cmp(&b.id))
    });
    points.dedup_by_key(|point| point.id);
    let mut plans = Vec::new();
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for pair in points.windows(2) {
        let (low, high) = (pair[0], pair[1]);
        if high.elevation_feet - low.elevation_feet <= ELEVATION_TOLERANCE_FEET {
            continue;
        }
        let covered = ends.iter().any(|(_, base, top)| {
            base.elevation_feet <= low.elevation_feet + ELEVATION_TOLERANCE_FEET
                && top.elevation_feet >= high.elevation_feet - ELEVATION_TOLERANCE_FEET
        });
        if covered {
            continue;
        }
        let span = high.elevation_feet - low.elevation_feet;
        // The set whose stair ends where the span starts, and is as high.
        let below = ends.iter().find(|(set, base, top)| {
            top.id == low.id
                && stairs.contains_key(&set.stair)
                && ((top.elevation_feet - base.elevation_feet) - span).abs()
                    <= HEIGHT_TOLERANCE_FEET
        });
        match below {
            Some((set, base, _)) => {
                // A stair repeats once: a second span above it is not reached.
                if !seen.insert(set.stair) {
                    count(reason::SPAN_NOT_REPEATED);
                    continue;
                }
                let Some((name, elevation_feet)) = levels.get(&low.id) else {
                    count(reason::LEVEL_NOT_READ);
                    continue;
                };
                plans.push(CopyPlan {
                    stair: set.stair,
                    shift_feet: low.elevation_feet - base.elevation_feet,
                    level_name: name.clone(),
                    level_elevation_feet: *elevation_feet,
                });
            }
            None => {
                let stair_below = ends
                    .iter()
                    .any(|(set, _, top)| top.id == low.id && !stairs.contains_key(&set.stair));
                count(if stair_below {
                    reason::STAIR_NOT_EXPORTED
                } else {
                    reason::SPAN_NOT_REPEATED
                });
            }
        }
    }
    plans
}

/// Append the copy of the stair at `whole` and its `parts`, `shift_feet` up
/// and in the storey `storey`, with the property sets of each, and the
/// aggregate that joins them. `original` is the ElementId the copy repeats.
/// Returns how many entities were added, or `None` where a part cannot be
/// moved (the model is then left as it was).
fn copy_stair(
    model: &mut IfcModel,
    whole: usize,
    parts: &[usize],
    shift_feet: f64,
    storey: usize,
    original: u64,
) -> Option<usize> {
    let originals: Vec<usize> = std::iter::once(whole)
        .chain(parts.iter().copied())
        .collect();
    // Something the model keeps by entity index, which a copy would not carry.
    if originals.iter().any(|index| {
        model.body_pieces.contains_key(index)
            || model
                .body_pieces
                .values()
                .any(|pieces| pieces.contains(index))
            || model.material_layer_usages.contains_key(index)
            || model.opening_cuts.contains_key(index)
            || model.filler_sizes.contains_key(index)
    }) {
        return None;
    }
    let mut moved: Vec<IfcEntity> = Vec::with_capacity(originals.len());
    for &index in &originals {
        let mut copy = model.entities.get(index)?.clone();
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            predefined_type,
            storey_index,
            location_feet,
            property_set,
            ..
        } = &mut copy
        else {
            return None;
        };
        let location = location_feet.as_mut()?;
        location[2] += shift_feet;
        *storey_index = Some(storey);
        // Revit names the stair and each part but a railing after the original
        // with `:2`, and writes the copy's flights with no predefined type.
        if ifc_type.as_str() != "IFCRAILING" {
            name.push_str(":2");
        }
        if ifc_type.as_str() == "IFCSTAIRFLIGHT" {
            *predefined_type = Some("NOTDEFINED".to_string());
        }
        // The mark is what keeps the writer from giving the copy of a stair, a
        // flight, a landing or a railing the type object Revit's export gives
        // it none; a stringer's copy keeps its original's, so it needs none.
        match property_set {
            Some(set) => set.properties.push(Property {
                name: MULTISTORY_COPY_PROPERTY.to_string(),
                value: PropertyValue::Integer(i64::try_from(original).ok()?),
            }),
            None if ifc_type.as_str() != "IFCMEMBER" => return None,
            None => {}
        }
        moved.push(copy);
    }
    let base = model.entities.len();
    let new_of: BTreeMap<usize, usize> = originals
        .iter()
        .enumerate()
        .map(|(offset, index)| (*index, base + offset))
        .collect();
    // The property sets the originals carry, for the copies.
    let sets: Vec<(usize, PropertySet)> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::ElementPropertySet { element, set } => {
                new_of.get(element).map(|new| (*new, set.clone()))
            }
            _ => None,
        })
        .collect();
    let added = moved.len() + 1 + sets.len();
    model.entities.extend(moved);
    model.entities.push(IfcEntity::Aggregate {
        whole: base,
        parts: (1..originals.len()).map(|offset| base + offset).collect(),
    });
    model.entities.extend(
        sets.into_iter()
            .map(|(element, set)| IfcEntity::ElementPropertySet { element, set }),
    );
    Some(added)
}
