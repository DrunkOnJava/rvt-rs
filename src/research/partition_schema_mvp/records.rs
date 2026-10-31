//! Category record selection, instance selection, host binding, and the design-option and nesting filters.

use super::*;

/// Placed instances of every
/// [`crate::partition_element_records::PRODUCT_RECORD_CATEGORIES`]
/// category, each decoded under its class name (RE-33).
///
/// The selection is the #211 instance rule, the same one that reproduces
/// the exported wall, door, window and column sets; one sweep of the
/// partitions serves all thirteen categories. Bodies are the record
/// bounding box, and a Level binds only when the record's reference list
/// names exactly one.
pub fn product_instances_from_partition_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    use crate::partition_element_records::PRODUCT_RECORD_CATEGORIES;
    if !crate::partition_element_records::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let categories: Vec<i64> = PRODUCT_RECORD_CATEGORIES.iter().map(|(c, _)| *c).collect();
    let records = crate::partition_element_records::scan_category_records_multi(
        rf,
        revit_version,
        &categories,
        &declared,
    )?;
    let records = without_non_primary_options(rf, records);
    let mut by_category: BTreeMap<i64, Vec<_>> = BTreeMap::new();
    for record in records {
        by_category
            .entry(record.builtin_category)
            .or_default()
            .push(record);
    }
    let mut out = Vec::new();
    for (category, class) in PRODUCT_RECORD_CATEGORIES {
        if let Some(records) = by_category.remove(&category) {
            out.extend(instances_from_records(records, class, level_ids));
        }
    }
    Ok(out)
}

/// The `OST_Columns` and `OST_Walls` records of one file, from a
/// single partition sweep.
///
/// The two categories travel together because the column body is the
/// column's prism minus the prisms of the walls it names
/// ([`crate::element_record_column_cuts`]), and
/// [`crate::partition_element_records::scan_category_records_multi`]
/// reads both out of one pass over the inflated streams.
pub(super) fn column_and_wall_records(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<(
    Vec<crate::partition_element_records::PartitionElementRecord>,
    Vec<crate::partition_element_records::PartitionElementRecord>,
)> {
    use crate::partition_element_records as per;

    if !per::supports_revit_version(revit_version) {
        return Ok((Vec::new(), Vec::new()));
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok((Vec::new(), Vec::new())),
    };
    if declared.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let records = per::scan_category_records_multi(
        rf,
        revit_version,
        &[per::OST_COLUMNS, per::OST_WALLS],
        &declared,
    )?;
    let records = without_non_primary_options(rf, records);
    Ok(records
        .into_iter()
        .partition(|record| record.builtin_category == per::OST_COLUMNS))
}

/// Every partition element record of one `BuiltInCategory`, or `None`
/// on a release / file where the shape is not proven.
pub(super) fn category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
) -> Result<Option<Vec<crate::partition_element_records::PartitionElementRecord>>> {
    if !crate::partition_element_records::supports_revit_version(revit_version) {
        return Ok(None);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(None),
    };
    if declared.is_empty() {
        return Ok(None);
    }
    let records = crate::partition_element_records::scan_category_records(
        rf,
        revit_version,
        builtin_category,
        &declared,
    )?;
    Ok(Some(without_non_primary_options(rf, records)))
}

/// Recover placed element instances of one `BuiltInCategory` from
/// partition element records (#211).
///
/// Fail-closed pipeline, each step justified in
/// [`crate::partition_element_records`]:
///
/// 1. Every candidate record's leading `u64` must be an ElementId
///    declared in `Global/ElemTable`, and the record must carry the
///    fixed bbox marker — a random byte match cannot become an
///    element.
/// 2. The record must be a standalone placed instance:
///    [`crate::partition_element_records::PartitionElementRecord::is_exported_instance`] — no container
///    reference at `+0x32`, placement kind at `+0x42` equal to
///    [`crate::partition_element_records::PLACEMENT_KIND_INSTANCE`].
///
/// Nothing here invents an ElementId, a level binding, or a profile
/// shape: the emitted geometry is exactly the recorded bounding box.
pub fn instances_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
    class: &str,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    if !crate::partition_element_records::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let records = crate::partition_element_records::scan_category_records(
        rf,
        revit_version,
        builtin_category,
        &declared,
    )?;
    let records = without_non_primary_options(rf, records);
    Ok(instances_from_records(records, class, level_ids))
}

/// Instance selection over already-decoded category records.
///
/// The selector is the direct test
/// [`crate::partition_element_records::PartitionElementRecord::is_exported_instance`]. It supersedes
/// the family-local bbox proxy plus highest-id-per-footprint collapse
/// that #204 shipped for columns: both reproduced the 256 exported
/// columns, only the direct test also reproduces the exact exported
/// id sets for walls, doors and windows (#211).
pub fn instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    class: &str,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    select_instance_records(records)
        .values()
        .map(|record| element_record_decoded(record, class, level_ids))
        .collect()
}

/// Door / window instance selection with the host-wall binding
/// attached (#222, RE-23, #439).
///
/// The host is a wall the record's counted list at `+0x88` names: one of
/// `host_candidates`, the ElementIds already selected as exported wall
/// instances by the same rule. Wall-set membership is what makes a
/// wrong read fail closed rather than invent a host. The list is in
/// ascending ElementId order, so RE-23's slot immediately before the
/// record's own id is only the nearest smaller id; a host with a larger
/// id than its door comes after it, and families that list their type
/// (Door-Opening, Schematic Opening Cut) put it between (#439, RE-85).
/// When the list names several walls, the nearest below the record's
/// own id is taken, then the nearest above. The partition pass then
/// skips curtain walls.
///
/// Measured against Revit's own export (`IfcRelVoidsElement` ∘
/// `IfcRelFillsElement`, read with IfcOpenShell): on
/// `2024_Core_Interior.rvt` 138 of 138 `(host wall, filling element)`
/// pairs, as with RE-23's slot. On Snowdon Towers (local only) every
/// door and window record naming exactly one wall names Revit's host
/// (235), and each of the 6 naming several takes a wall Revit cuts for
/// it, 3 of them the one it fills.
pub fn opening_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    class: &str,
    host_candidates: &BTreeSet<u32>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    select_instance_records(records)
        .values()
        .map(|record| {
            let mut decoded = element_record_decoded(record, class, level_ids);
            attach_host_candidates(record, host_candidates, &mut decoded);
            decoded
        })
        .collect()
}

/// Record on `decoded` the walls in `host_candidates` that `record`'s
/// reference list names, nearest to its own ElementId first, and take the
/// nearest as its host until [`bind_opening_hosts`] has seen curtain walls
/// (#439, RE-85). The list is in ascending ElementId order: the nearest
/// wall below the record's own id comes first, then the nearest above.
pub(super) fn attach_host_candidates(
    record: &crate::partition_element_records::PartitionElementRecord,
    host_candidates: &BTreeSet<u32>,
    decoded: &mut DecodedElement,
) {
    let walls: Vec<u32> = record
        .references
        .iter()
        .filter_map(|id| u32::try_from(*id).ok())
        .filter(|id| *id != record.element_id && host_candidates.contains(id))
        .collect();
    let (below, above): (Vec<u32>, Vec<u32>) =
        walls.iter().partition(|id| **id < record.element_id);
    let candidates: Vec<u32> = below.into_iter().rev().chain(above).collect();
    let Some(&host) = candidates.first() else {
        return;
    };
    decoded.fields.push((
        OPENING_HOST_CANDIDATES_FIELD.into(),
        InstanceField::Vector(
            candidates
                .iter()
                .map(|&id| InstanceField::ElementId { tag: 0, id })
                .collect(),
        ),
    ));
    decoded.fields.push((
        OPENING_HOST_FIELD.into(),
        InstanceField::ElementId { tag: 0, id: host },
    ));
    decoded.fields.push((
        OPENING_HOST_PROVENANCE_FIELD.into(),
        InstanceField::String(OPENING_HOST_PROVENANCE.into()),
    ));
}

/// Bind each door or window to the first of its host candidates that is
/// not a curtain wall, and drop the candidate list (#439). Revit's export
/// cuts no opening for a door or window whose only candidate is a
/// curtain wall; such an element keeps no host.
pub(super) fn bind_opening_hosts(walls: &[DecodedElement], openings: &mut [DecodedElement]) {
    let curtain_walls: BTreeSet<u32> = walls
        .iter()
        .filter(|wall| wall.class == CURTAIN_WALL_CLASS)
        .filter_map(|wall| wall.id)
        .collect();
    for element in openings.iter_mut() {
        let candidates = element_ids_field(element, OPENING_HOST_CANDIDATES_FIELD);
        if candidates.is_empty() {
            continue;
        }
        // A curtain wall's door or window (B64) is set in no wall.
        let part_of_curtain_wall = element
            .fields
            .iter()
            .any(|(name, _)| name == AGGREGATE_WHOLE_FIELD);
        let host = candidates
            .into_iter()
            .find(|id| !curtain_walls.contains(id))
            .filter(|_| !part_of_curtain_wall);
        element.fields.retain(|(name, _)| {
            name != OPENING_HOST_CANDIDATES_FIELD
                && name != OPENING_HOST_FIELD
                && name != OPENING_HOST_PROVENANCE_FIELD
        });
        if let Some(host) = host {
            element.fields.push((
                OPENING_HOST_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: host },
            ));
            element.fields.push((
                OPENING_HOST_PROVENANCE_FIELD.into(),
                InstanceField::String(OPENING_HOST_PROVENANCE.into()),
            ));
        }
    }
}

/// Recover door / window instances of one `BuiltInCategory` and bind
/// each to its host wall (#222, RE-23).
pub fn openings_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
    class: &str,
    host_candidates: &BTreeSet<u32>,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    if !crate::partition_element_records::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let records = crate::partition_element_records::scan_category_records(
        rf,
        revit_version,
        builtin_category,
        &declared,
    )?;
    let records = without_non_primary_options(rf, records);
    Ok(opening_instances_from_records(
        records,
        class,
        host_candidates,
        level_ids,
    ))
}

/// `records` without those in a non-primary design option: Revit's own
/// export writes the main model and each option set's primary option, and
/// leaves the other options out (#319, RE-40). Elements of an option whose
/// set is unresolved are kept.
pub(super) fn without_non_primary_options(
    rf: &mut RevitFile,
    mut records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> Vec<crate::partition_element_records::PartitionElementRecord> {
    let options = rf.memoised_design_options();
    // Elements not in the export phase are left out too, as Revit's export
    // leaves them out, except rooms (#328, RE-184).
    let phases = rf.memoised_phase_filter();
    records.retain(|record| !options.excludes(record) && !phases.excludes(record));
    records
}

/// ElementIds of the instances in `selected` nested in a door or window
/// (RE-81): a family instance (class tag `family_instance`) whose
/// reference list names a placed door or window that names it back. Revit's export writes some of these and leaves out
/// others, by a rule not decoded (on a 2023 project it keeps a window's
/// frame and sill cut and drops its mullion pattern), so all are left out
/// rather than guessed.
pub(super) fn nested_in_openings(
    selected: &BTreeMap<u32, crate::partition_element_records::PartitionElementRecord>,
    family_instance: u16,
) -> BTreeSet<u32> {
    use crate::partition_element_records::{OST_DOORS, OST_WINDOWS};
    let is_opening = |category: i64| category == OST_DOORS || category == OST_WINDOWS;
    selected
        .values()
        .filter(|child| child.class_tag == family_instance && !is_opening(child.builtin_category))
        .filter(|child| {
            child.references.iter().any(|&reference| {
                u32::try_from(reference)
                    .ok()
                    .and_then(|parent| selected.get(&parent))
                    .is_some_and(|parent| {
                        is_opening(parent.builtin_category)
                            && parent.references.contains(&u64::from(child.element_id))
                    })
            })
        })
        .map(|child| child.element_id)
        .collect()
}

/// Instance selection proper: one record per exported ElementId.
pub(super) fn select_instance_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> BTreeMap<u32, crate::partition_element_records::PartitionElementRecord> {
    use crate::partition_element_records::PartitionElementRecord;
    use std::collections::BTreeMap;

    // One record per ElementId: a single element can be framed more
    // than once across partitions, and the frames can disagree on the
    // vertical extent. Keep the greatest z-extent — that is the
    // element's full body, and it is measured: against the reference
    // export's `IfcExtrudedAreaSolid.Depth`, first-by-(stream, offset)
    // agrees on 67 of 80 slabs and greatest-extent on 79 of 80 (the
    // 80th is exported as two stacked solids whose depths sum to the
    // recorded extent) (#212, RE-22).
    //
    // Ties go to the **newest** frame — the greatest (stream, offset)
    // — because a Revit 2024 project rewrites an edited element into a
    // higher-numbered `Partitions/NN` stream and leaves the earlier
    // copy in place. On `2024_Core_Interior.rvt` 171 of 360 walls and
    // 96 of 132 doors carry two frames that disagree about the plan
    // box, and the newest one is the current state (RE-26 §3):
    //
    // - the newest frame's thin plan extent equals the nominal
    //   thickness of the `IfcWallType` Revit's export assigns on
    //   **360 of 360** walls; the oldest frame's on 201;
    // - the newest frame's type slot in the counted reference list
    //   at `+0x88` equals that `IfcWallType.Tag` on **356 of 360**;
    //   the oldest frame's on 183;
    // - the `u32` at `+12` of the element's `Global/ElemTable` record
    //   (from `0x06`, #152; `+0x1c` in the older `0x1E` framing), its
    //   last-modification counter, ranks the same way — 36 on every one of the 480
    //   instances whose newest frame is `Partitions/59`, 35 on the 12
    //   whose newest is `/55`, 33 on the 18 whose newest is `/51`,
    //   and at most 30 on the 344 framed only in `/46`.
    //
    // No frame of any instance on that file disagrees about the box's
    // `z`, so this cannot move an element between storeys — checked
    // over all 976 multi-framed instances.
    let mut by_id: BTreeMap<u32, PartitionElementRecord> = BTreeMap::new();
    for record in records {
        // A placed instance with no volume has no 3D body, and Revit's
        // export leaves it out (#309); see `has_volume`.
        if !record.is_exported_instance() || !record.has_volume() {
            continue;
        }
        let better = match by_id.get(&record.element_id) {
            None => true,
            Some(existing) => {
                let candidate = z_extent_key(&record);
                let held = z_extent_key(existing);
                candidate > held
                    || (candidate == held
                        && (record.stream.as_str(), record.offset)
                            > (existing.stream.as_str(), existing.offset))
            }
        };
        if better {
            by_id.insert(record.element_id, record);
        }
    }
    without_nested_components(by_id)
}

/// `by_id` without the shared nested components of MEP devices (#96): a
/// nested device and its parent, in the same
/// [`crate::partition_element_records::NESTED_COMPONENT_CATEGORIES`]
/// category, each list the other among their references, and the nested
/// one, created after its parent, has the larger ElementId. On RE1
/// Electrical each receptacle, switch and junction-box symbol nested in a
/// device is such a pair, and Revit's export leaves the nested one out
/// (21 of 21). Other categories are left alone: joined walls and connected
/// fittings reference each other too.
pub(super) fn without_nested_components(
    mut by_id: BTreeMap<u32, crate::partition_element_records::PartitionElementRecord>,
) -> BTreeMap<u32, crate::partition_element_records::PartitionElementRecord> {
    use crate::partition_element_records::NESTED_COMPONENT_CATEGORIES;
    let nested: Vec<u32> = by_id
        .values()
        .filter(|child| NESTED_COMPONENT_CATEGORIES.contains(&child.builtin_category))
        .filter(|child| {
            child.references.iter().any(|&reference| {
                u32::try_from(reference)
                    .ok()
                    .filter(|&parent| parent < child.element_id)
                    .and_then(|parent| by_id.get(&parent))
                    .is_some_and(|parent| {
                        parent.builtin_category == child.builtin_category
                            && parent.references.contains(&u64::from(child.element_id))
                    })
            })
        })
        .map(|child| child.element_id)
        .collect();
    for id in nested {
        by_id.remove(&id);
    }
    by_id
}

/// Vertical extent of a record's bounding box, quantised to 1e-4 ft
/// so two frames of the same element compare exactly.
pub(super) fn z_extent_key(
    record: &crate::partition_element_records::PartitionElementRecord,
) -> i64 {
    let (_, _, dz) = record.extents_feet();
    if !dz.is_finite() {
        return i64::MIN;
    }
    (dz * 10_000.0).round() as i64
}
