//! Schema / partition MVP recovers for production `iter_elements`.
//!
//! Extends the ArcWall-only partition merge with fail-closed recovers
//! for Level and Material, and (on Revit 2024) ArcWallRectOpening index
//! rows, plus `OST_Columns` / `OST_Walls` / `OST_Doors` /
//! `OST_Windows` element records
//! ([`instances_from_partition_category_records`]),
//! `OST_Floors` / `OST_BuildingPad` slab instances
//! ([`slabs_from_partition_category_records`], #212 / RE-22) and
//! `OST_Rooms` room instances
//! ([`rooms_from_partition_category_records`], #90 / RE-29) on the
//! releases whose element records decode.
//!
//! Semantic `Door` / `Window` classes are still **not** invented from
//! opening-index rows — those keep surfacing as `ArcWallRectOpening`
//! with related-id provenance only, and RE-19's negative (no
//! discriminator in the opening-index bytes, no schema-field Wall)
//! stands untouched. Typed `Wall` / `Door` / `Window` come from a
//! different carrier: the element record's own `BuiltInCategory`
//! field, which names the category outright (#211). Opening-index
//! related ids are cross-checked against `Global/ElemTable` when
//! present; a hit confirms the id is declared, not that it is a host
//! Wall or a Door/Window family instance.
//!
//! # Version guard
//!
//! Partition byte scans reuse [`crate::partition_scanner`] /
//! [`crate::partition_arc_walls`] / [`crate::rect_opening_index`]
//! version gates. Unsupported releases yield empty lists.
//!
//! # Honesty
//!
//! - Never invent ElementIds, host walls, elevations, or geometry.
//! - Tier1 synthetics have no partition building elements — callers
//!   must observe zero Level/Material/Floor/opening hits there.
//! - Rooms and floors come from element records or not at all. The
//!   name-only rooms and plan-loop floors that once filled in were
//!   removed after they matched nothing in Revit's own exports.

use crate::partition_arc_walls::{self, PartitionArcWall};
use crate::partition_name_candidates::{
    NameBucket, building_storey_name_candidates, classify_name, collect_name_candidates,
};
use crate::rect_opening_index::ArcWallRectOpeningIndex;
use crate::walker::{DecodedElement, ElementProvenance, InstanceField, WalkerLimits};
use crate::{Result, RevitFile};
use std::collections::BTreeSet;

/// Bundle of partition-derived MVP `DecodedElement`s.
#[derive(Debug, Clone, Default)]
pub struct PartitionSchemaMvp {
    pub levels: Vec<DecodedElement>,
    pub materials: Vec<DecodedElement>,
    pub rooms: Vec<DecodedElement>,
    /// 2024 ArcWallRectOpening index rows — not typed Door/Window.
    pub rect_openings: Vec<DecodedElement>,
    /// 2024 `OST_Columns` partition element records (M4-09 / #204).
    pub columns: Vec<DecodedElement>,
    /// 2024 `OST_Walls` partition element records (#211).
    pub walls: Vec<DecodedElement>,
    /// 2024 `OST_Doors` partition element records (#211).
    pub doors: Vec<DecodedElement>,
    /// 2024 `OST_Windows` partition element records (#211).
    pub windows: Vec<DecodedElement>,
    /// `OST_Floors` / `OST_BuildingPad` partition element records
    /// (#212, RE-22), the only source of floors.
    pub slabs: Vec<DecodedElement>,
    /// Furniture, casework, fixtures, curtain-wall parts, railings, wall
    /// sweeps, ducts and pipes from partition element records
    /// ([`crate::partition_element_records::PRODUCT_RECORD_CATEGORIES`],
    /// RE-33).
    pub products: Vec<DecodedElement>,
}

impl PartitionSchemaMvp {
    /// Flatten in a stable order for merging into `iter_elements`.
    pub fn into_elements(self) -> Vec<DecodedElement> {
        let mut out = Vec::with_capacity(
            self.levels.len()
                + self.materials.len()
                + self.rooms.len()
                + self.rect_openings.len()
                + self.columns.len()
                + self.walls.len()
                + self.doors.len()
                + self.windows.len()
                + self.slabs.len()
                + self.products.len(),
        );
        out.extend(self.levels);
        out.extend(self.materials);
        out.extend(self.rooms);
        out.extend(self.rect_openings);
        out.extend(self.columns);
        out.extend(self.walls);
        out.extend(self.doors);
        out.extend(self.windows);
        out.extend(self.slabs);
        out.extend(self.products);
        out
    }
}

/// Recover partition MVP elements for a file (version-gated, fail-closed).
pub fn recover_partition_schema_mvp(
    rf: &mut RevitFile,
    revit_version: u32,
    limits: WalkerLimits,
) -> Result<PartitionSchemaMvp> {
    let mut out = PartitionSchemaMvp::default();

    // --- Levels + Materials from partition strings / ArcWall elev ---
    let strings = rf.partition_string_records().unwrap_or_default();
    let string_values: Vec<&str> = strings.iter().map(|r| r.value.as_str()).collect();

    let level_names = building_storey_name_candidates(string_values.iter().copied());
    let name_set = collect_name_candidates(string_values.iter().copied());

    let walls = match partition_arc_walls::scan_partition_arc_walls_with_limits(
        rf,
        revit_version,
        limits,
    ) {
        Ok(scan) => scan.walls,
        Err(_) => Vec::new(),
    };

    out.levels = levels_from_storeys_and_names(&walls, &level_names);
    out.materials = materials_from_names(&name_set);

    // Rooms and floors come from partition element records only. The
    // name-only rooms (space-like display strings) and ArcWall-excluded
    // plan-loop floors that used to fill in matched nothing in Revit's own
    // exports: every family file gained a room named "Office Equipment",
    // the Revit 2025 RE1 MEP models 38 slabs and 71 spaces where their
    // exports hold none, and no 2023 plan loop (mostly triangles) came
    // within 10% of the plan area of a slab in two paired exports.

    // --- 2024 opening index (not Door/Window) ---
    if ArcWallRectOpeningIndex::supports_revit_version(revit_version) {
        out.rect_openings = rect_openings_from_partitions(rf, revit_version, limits)?;
    }

    // --- 2024 partition element records (#204 columns, #211 the rest) ---
    //
    // The `Level` ElementId set costs one partition sweep and is read
    // once here for every category below: each record-backed element
    // binds to the Level its counted reference list names, when it
    // names exactly one (#219, RE-27).
    let level_ids = level_element_ids(rf, revit_version)?;
    // Columns and walls come out of one sweep: Revit cuts a column
    // with the walls it is joined to, so the column body needs the
    // wall boxes (#239, RE-29 §5), and inflating the partitions twice
    // to get them would dominate the cost.
    let (column_records, wall_records) = column_and_wall_records(rf, revit_version)?;
    let wall_instances: Vec<crate::partition_element_records::PartitionElementRecord> =
        select_instance_records(wall_records.clone())
            .into_values()
            .collect();
    out.walls = wall_instances_from_records(wall_records, &level_ids);
    out.columns = column_instances_from_records(column_records, &level_ids, &wall_instances);
    // Doors and windows bind to a host wall (#222, RE-23). The
    // candidate set is exactly the wall instances recovered above —
    // a recovered host that is not itself an exported wall is
    // discarded rather than emitted, so the join is self-checking.
    let wall_ids: BTreeSet<u32> = out.walls.iter().filter_map(|wall| wall.id).collect();
    out.doors = openings_from_partition_category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_DOORS,
        "Door",
        &wall_ids,
        &level_ids,
    )?;
    out.windows = openings_from_partition_category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_WINDOWS,
        "Window",
        &wall_ids,
        &level_ids,
    )?;

    // --- Slab instances from element records (#212, RE-22) ---
    //
    // Record-backed slabs carry an ElementId, a model bounding box,
    // a measured thickness and a storey.
    out.slabs = slabs_from_partition_category_records(rf, revit_version, &level_ids)?;

    // --- Room instances from element records (#90, RE-29) ---
    //
    // A record-backed room carries an ElementId, a model bounding box
    // that is exactly the reference export's plan envelope and
    // floor-to-ceiling extent, a number, a name and a host Level, none
    // of which a partition *string* can supply.
    out.rooms = rooms_from_partition_category_records(rf, revit_version, &level_ids)?;

    // --- Other product categories from element records (RE-33) ---
    out.products = product_instances_from_partition_records(rf, revit_version, &level_ids)?;

    Ok(out)
}

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
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
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
    let mut by_category: std::collections::BTreeMap<i64, Vec<_>> =
        std::collections::BTreeMap::new();
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

/// Recover room instances from partition element records (#90, RE-29).
///
/// `OST_Rooms` under the #211 instance rule reproduces the exported
/// room ElementId set exactly; the number, name and host Level come
/// from the room's own parameter block
/// ([`crate::partition_room_parameters`]), joined by ElementId only.
///
/// The parameter join is attached, never required: a room whose block
/// does not decode, or whose blocks disagree, still emits with its
/// record body and keeps the identity the record gives it.
pub fn rooms_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    let Some(records) = category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_ROOMS,
    )?
    else {
        return Ok(Vec::new());
    };
    let room_ids: BTreeSet<u32> = records
        .iter()
        .filter(|record| record.is_exported_instance())
        .map(|record| record.element_id)
        .collect();
    let parameters =
        crate::partition_room_parameters::scan_room_parameters(rf, revit_version, &room_ids)
            .unwrap_or_default();
    Ok(room_instances_from_records(records, &parameters, level_ids))
}

/// Field carrying a recovered room number.
pub const ROOM_NUMBER_FIELD: &str = "m_number";
/// Field recording where the room number / name came from.
pub const ROOM_PARAMETER_SOURCE_FIELD: &str = "m_room_parameter_source";

/// Room instance selection with the number / name / Level join
/// attached (#90, RE-29).
///
/// The Level the parameter block names is used only when the record's
/// own counted reference list did **not** already name one, so the
/// RE-27 carrier stays the primary. On `2024_Core_Interior.rvt` both
/// carriers answer for all 116 rooms and they agree 116 of 116.
pub fn room_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    parameters: &std::collections::BTreeMap<u32, crate::partition_room_parameters::RoomParameters>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    select_instance_records(records)
        .values()
        .map(|record| {
            let mut decoded = element_record_decoded(record, "Room", level_ids);
            if let Some(block) = parameters.get(&record.element_id) {
                decoded
                    .fields
                    .push(("m_name".into(), InstanceField::String(block.name.clone())));
                decoded.fields.push((
                    ROOM_NUMBER_FIELD.into(),
                    InstanceField::String(block.number.clone()),
                ));
                decoded.fields.push((
                    ROOM_PARAMETER_SOURCE_FIELD.into(),
                    InstanceField::String(
                        crate::partition_room_parameters::ROOM_PARAMETER_SOURCE.into(),
                    ),
                ));
                let already_bound = decoded.fields.iter().any(|(name, _)| {
                    name == crate::element_record_level_refs::LEVEL_REFERENCE_FIELD
                });
                if !already_bound {
                    if let Some(id) = block.level_element_id(level_ids) {
                        decoded.fields.push((
                            crate::element_record_level_refs::LEVEL_REFERENCE_FIELD.into(),
                            InstanceField::ElementId { tag: 0, id },
                        ));
                        if let Some(slot) = decoded
                            .fields
                            .iter_mut()
                            .find(|(name, _)| name == "m_level_bound")
                        {
                            slot.1 = InstanceField::Bool(true);
                        }
                    }
                }
            }
            decoded
        })
        .collect()
}

/// Recover architectural column instances from partition element
/// records (M4-09 / #204, instance rule replaced in #211).
///
/// The `OST_Columns` sweep carries the type-symbol envelopes too, so
/// the family/type join of #215 costs no extra inflate — see
/// [`column_instances_from_records`].
pub fn columns_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    let (column_records, wall_records) = column_and_wall_records(rf, revit_version)?;
    let wall_instances: Vec<crate::partition_element_records::PartitionElementRecord> =
        select_instance_records(wall_records)
            .into_values()
            .collect();
    Ok(column_instances_from_records(
        column_records,
        level_ids,
        &wall_instances,
    ))
}

/// The `OST_Columns` and `OST_Walls` records of one file, from a
/// single partition sweep.
///
/// The two categories travel together because the column body is the
/// column's prism minus the prisms of the walls it names
/// ([`crate::element_record_column_cuts`]), and
/// [`crate::partition_element_records::scan_category_records_multi`]
/// reads both out of one pass over the inflated streams.
fn column_and_wall_records(
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
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
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
    Ok(records
        .into_iter()
        .partition(|record| record.builtin_category == per::OST_COLUMNS))
}

/// Recover wall instances from partition element records (#211), with
/// the join-trimmed body of RE-26.
pub fn walls_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    let Some(records) = category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_WALLS,
    )?
    else {
        return Ok(Vec::new());
    };
    Ok(wall_instances_from_records(records, level_ids))
}

/// The `Level` ElementIds the #219 / RE-27 reference-list join tests
/// against, or an empty set on a release / file with no proven Level
/// framing (the join then resolves nothing and every element keeps the
/// elevation join it already had).
pub fn level_element_ids(rf: &mut RevitFile, revit_version: u32) -> Result<BTreeSet<u32>> {
    if !crate::partition_level_records::supports_revit_version(revit_version) {
        return Ok(BTreeSet::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => return Ok(BTreeSet::new()),
    };
    crate::partition_level_records::scan_partition_level_ids(rf, revit_version, &declared)
}

/// Every partition element record of one `BuiltInCategory`, or `None`
/// on a release / file where the shape is not proven.
fn category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
) -> Result<Option<Vec<crate::partition_element_records::PartitionElementRecord>>> {
    if !crate::partition_element_records::supports_revit_version(revit_version) {
        return Ok(None);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => return Ok(None),
    };
    if declared.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        crate::partition_element_records::scan_category_records(
            rf,
            revit_version,
            builtin_category,
            &declared,
        )?,
    ))
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
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
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
    Ok(instances_from_records(records, class, level_ids))
}

/// Instance selection over already-decoded category records — split
/// out so the rule is unit-testable without a corpus file.
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

/// Field carrying the recovered host ElementId of a door or window.
///
/// The name is the one [`crate::elements::openings::Door`] /
/// [`crate::elements::openings::Window`] already read from
/// schema-field decodes, so the recovered value flows through the
/// existing `recover_door_host` / `recover_window_host` path without
/// a second host concept.
pub const OPENING_HOST_FIELD: &str = "m_hostId";

/// Field recording where [`OPENING_HOST_FIELD`] came from.
pub const OPENING_HOST_PROVENANCE_FIELD: &str = "m_host_provenance";

/// Value of [`OPENING_HOST_PROVENANCE_FIELD`] for the RE-23 carrier.
pub const OPENING_HOST_PROVENANCE: &str = "partition_element_record_reference_list";

/// Door / window instance selection with the host-wall binding
/// attached (#222, RE-23).
///
/// The binding is the record's
/// [`crate::partition_element_records::PartitionElementRecord::preceding_reference`]
/// — the reference slot immediately before the record's own
/// ElementId in the counted list at `+0x88` — accepted only when it
/// is one of `host_candidates`, the ElementIds already selected as
/// exported wall instances by the same rule. Both halves are
/// required: the slot is where the byte says the host is, and the
/// wall-set membership is what makes a wrong read fail closed rather
/// than invent a host.
///
/// Measured on `2024_Core_Interior.rvt` against Revit's own export
/// (`IfcRelVoidsElement` ∘ `IfcRelFillsElement`, read with
/// IfcOpenShell): 138 of 138 `(host wall, filling element)` pairs
/// reproduced — 132 doors, 6 windows — with no wrong host and no
/// missing pair, across all 273 records that frame them.
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
            if let Some(host) = record
                .preceding_reference
                .filter(|id| host_candidates.contains(id))
            {
                decoded.fields.push((
                    OPENING_HOST_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: host },
                ));
                decoded.fields.push((
                    OPENING_HOST_PROVENANCE_FIELD.into(),
                    InstanceField::String(OPENING_HOST_PROVENANCE.into()),
                ));
            }
            decoded
        })
        .collect()
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
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
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
    Ok(opening_instances_from_records(
        records,
        class,
        host_candidates,
        level_ids,
    ))
}

/// Instance selection proper: one record per exported ElementId.
fn select_instance_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> std::collections::BTreeMap<u32, crate::partition_element_records::PartitionElementRecord> {
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
    // - `Global/ElemTable`'s `u32` at `+0x1c` is a monotone version
    //   counter that ranks the same way — 36 on every one of the 480
    //   instances whose newest frame is `Partitions/59`, 35 on the 12
    //   whose newest is `/55`, 33 on the 18 whose newest is `/51`,
    //   and at most 30 on the 344 framed only in `/46`.
    //
    // No frame of any instance on that file disagrees about the box's
    // `z`, so this cannot move an element between storeys — checked
    // over all 976 multi-framed instances.
    let mut by_id: BTreeMap<u32, PartitionElementRecord> = BTreeMap::new();
    for record in records {
        if !record.is_exported_instance() {
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
    by_id
}

/// Vertical extent of a record's bounding box, quantised to 1e-4 ft
/// so two frames of the same element compare exactly.
fn z_extent_key(record: &crate::partition_element_records::PartitionElementRecord) -> i64 {
    let (_, _, dz) = record.extents_feet();
    if !dz.is_finite() {
        return i64::MIN;
    }
    (dz * 10_000.0).round() as i64
}

/// Recover slab instances from partition element records (#212, RE-22).
///
/// Two `BuiltInCategory` ids feed one class, because Revit's own
/// exporter maps both to `IfcSlab`:
/// [`crate::partition_element_records::OST_FLOORS`] (class `Floor`)
/// and [`crate::partition_element_records::OST_BUILDING_PAD`]
/// (class `BuildingPad`). The pad is kept as its own class rather
/// than relabelled a floor — the mapping to `IFCSLAB` happens in
/// [`crate::ifc::category_map`], where it is visible.
///
/// Per-element IFC export-type overrides
/// ([`crate::partition_ifc_export_overrides`]) are attached as the
/// `m_ifc_export_as` field. The decoder does not act on the value;
/// the IFC writer decides which values it is willing to honour.
///
/// The plate's sketched plan profile
/// ([`crate::element_record_plan_profiles`]) is attached in the same
/// pass when it closes — the `OST_SketchLines` records that carry it
/// are read from the same stream sweep, so the profile costs one
/// extra needle search rather than a second inflate (#31, RE-25).
pub fn slabs_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    use crate::partition_element_records as per;

    if !per::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let overrides = crate::partition_ifc_export_overrides::scan_ifc_export_overrides(
        rf,
        revit_version,
        &declared,
    )
    .unwrap_or_default();

    let categories = [
        per::OST_FLOORS,
        per::OST_BUILDING_PAD,
        per::OST_SKETCH_LINES,
    ];
    let scanned = per::scan_category_records_multi(rf, revit_version, &categories, &declared)?;
    let sketch_lines: Vec<per::PartitionElementRecord> = scanned
        .iter()
        .filter(|record| record.builtin_category == per::OST_SKETCH_LINES)
        .cloned()
        .collect();
    let profiles =
        crate::element_record_plan_profiles::plan_profiles_from_sketch_line_records(&sketch_lines);

    let mut out = Vec::new();
    for (category, class) in [
        (per::OST_FLOORS, "Floor"),
        (per::OST_BUILDING_PAD, "BuildingPad"),
    ] {
        let records: Vec<per::PartitionElementRecord> = scanned
            .iter()
            .filter(|record| record.builtin_category == category)
            .cloned()
            .collect();
        for mut decoded in instances_from_records(records, class, level_ids) {
            if let Some(id) = decoded.id {
                if let Some(value) = overrides.get(&id) {
                    decoded.fields.push((
                        "m_ifc_export_as".into(),
                        InstanceField::String(value.clone()),
                    ));
                }
                if let Some(profile) = profiles.get(&id) {
                    decoded.fields.extend(profile.fields());
                }
            }
            out.push(decoded);
        }
    }
    Ok(out)
}

/// Back-compat alias for the #204 entry point.
pub fn columns_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    column_instances_from_records(records, level_ids, &[])
}

/// Field carrying the ElementId of the family/type symbol a placed
/// instance was recovered from (#215, RE-26).
pub const TYPE_SYMBOL_FIELD: &str = "m_type_symbol_id";
/// Field carrying the type symbol's plan width, in feet.
pub const TYPE_PROFILE_WIDTH_FIELD: &str = "m_type_profile_width";
/// Field carrying the type symbol's plan depth, in feet.
pub const TYPE_PROFILE_DEPTH_FIELD: &str = "m_type_profile_depth";
/// Field recording where the section in the two fields above came from.
pub const TYPE_PROFILE_SOURCE_FIELD: &str = "m_type_profile_source";
/// Value of [`TYPE_PROFILE_SOURCE_FIELD`] for the RE-26 carrier.
pub const TYPE_PROFILE_SOURCE: &str = "family_type_symbol_envelope";
/// How far the type section may sit from the instance envelope, in feet.
///
/// The join is accepted only when the symbol's plan extents match the
/// instance's — a symbol that does not is a symbol this decoder has
/// misread, or an instance that is scaled or rotated, and either way
/// the box it would emit is not the one the record states. On
/// `2024_Core_Interior.rvt` all 256 columns pass with a worst
/// disagreement of 8.0e-15 ft.
pub const TYPE_PROFILE_EPS_FEET: f64 = 1e-6;

/// Column instance selection with the family/type symbol join
/// attached (#215, RE-26).
///
/// The same category sweep carries both sides: the placed instances
/// (`is_exported_instance`) and the type-symbol envelopes
/// (`is_type_symbol`, RE-21 §5). Each instance names its symbol in
/// the counted reference list at `+0x88` —
/// [`crate::partition_element_records::PartitionElementRecord::type_symbol_reference`]
/// — and the symbol's own bounding box is the section in family
/// coordinates.
///
/// Measured on `2024_Core_Interior.rvt`: 256 of 256 exported columns
/// join to symbol `5755`, which is the `IfcColumnType.Tag` Revit's
/// own export writes for every one of them, and the section it gives
/// is the 2 ft square the instance envelope already carried — so the
/// join moves no vertex on this file. What it changes is where the
/// profile comes from: the type says the section is a rectangle,
/// instead of a box happening to be square.
pub fn column_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    level_ids: &BTreeSet<u32>,
    wall_records: &[crate::partition_element_records::PartitionElementRecord],
) -> Vec<DecodedElement> {
    use crate::partition_element_records::PartitionElementRecord;
    use std::collections::BTreeMap;

    let symbols: BTreeMap<u32, [f64; 6]> = records
        .iter()
        .filter(|record| record.is_type_symbol())
        .map(|record| (record.element_id, record.bbox_feet))
        .collect();
    let symbol_ids: BTreeSet<u32> = symbols.keys().copied().collect();
    let selected = select_instance_records(records);
    let instances: Vec<PartitionElementRecord> = selected.values().cloned().collect();
    let cuts = crate::element_record_column_cuts::column_cut_boxes(&instances, wall_records);
    selected
        .values()
        .map(|record: &PartitionElementRecord| {
            let mut decoded = element_record_decoded(record, "Column", level_ids);
            if let Some(symbol) = record
                .type_symbol_reference(&symbol_ids)
                .and_then(|id| symbols.get(&id).map(|bbox| (id, *bbox)))
            {
                attach_type_symbol_profile(&mut decoded, record, symbol.0, &symbol.1);
            }
            if let Some(cut) = cuts.get(&record.element_id) {
                apply_column_join_cut(&mut decoded, cut);
            }
            decoded
        })
        .collect()
}

/// Rewrite a column's placement and extents to the cut body, and say
/// where that body came from (#239, RE-29 §5).
///
/// The type section recovered by [`attach_type_symbol_profile`] is
/// the *uncut* family section and stays on the element as the type's
/// own fact; what changes is the emitted solid, which is the record
/// prism minus the walls the record names.
fn apply_column_join_cut(
    decoded: &mut DecodedElement,
    cut: &crate::element_record_column_cuts::ColumnCut,
) {
    use crate::element_record_column_cuts as cuts;

    let bbox = cut.bbox_feet;
    let replacements: [(&str, f64); 6] = [
        ("m_locationX", (bbox[0] + bbox[3]) * 0.5),
        ("m_locationY", (bbox[1] + bbox[4]) * 0.5),
        ("m_locationZ", bbox[2]),
        ("m_bboxWidth", bbox[3] - bbox[0]),
        ("m_bboxDepth", bbox[4] - bbox[1]),
        ("m_bboxHeight", bbox[5] - bbox[2]),
    ];
    for (name, value) in decoded.fields.iter_mut() {
        if let Some((_, replacement)) = replacements
            .iter()
            .find(|(field, _)| *field == name.as_str())
        {
            *value = InstanceField::Float {
                value: *replacement,
                size: 8,
            };
        }
    }
    decoded.fields.push((
        cuts::COLUMN_BODY_SOURCE_FIELD.into(),
        InstanceField::String(cuts::COLUMN_BODY_JOIN_CUT.into()),
    ));
    decoded.fields.push((
        cuts::COLUMN_CUT_WALL_COUNT_FIELD.into(),
        InstanceField::Integer {
            value: cut.wall_count as i64,
            signed: false,
            size: 8,
        },
    ));
}

/// Attach the type symbol's section to a placed instance, when the
/// section agrees with the instance's own plan envelope.
fn attach_type_symbol_profile(
    decoded: &mut DecodedElement,
    record: &crate::partition_element_records::PartitionElementRecord,
    symbol_id: u32,
    symbol_bbox: &[f64; 6],
) {
    let width = symbol_bbox[3] - symbol_bbox[0];
    let depth = symbol_bbox[4] - symbol_bbox[1];
    if !(width.is_finite() && depth.is_finite()) || width <= 0.0 || depth <= 0.0 {
        return;
    }
    let (dx, dy, _) = record.extents_feet();
    if (width - dx).abs() > TYPE_PROFILE_EPS_FEET || (depth - dy).abs() > TYPE_PROFILE_EPS_FEET {
        return;
    }
    decoded.fields.push((
        TYPE_SYMBOL_FIELD.into(),
        InstanceField::ElementId {
            tag: 0,
            id: symbol_id,
        },
    ));
    decoded.fields.push((
        TYPE_PROFILE_WIDTH_FIELD.into(),
        InstanceField::Float {
            value: width,
            size: 8,
        },
    ));
    decoded.fields.push((
        TYPE_PROFILE_DEPTH_FIELD.into(),
        InstanceField::Float {
            value: depth,
            size: 8,
        },
    ));
    decoded.fields.push((
        TYPE_PROFILE_SOURCE_FIELD.into(),
        InstanceField::String(TYPE_PROFILE_SOURCE.into()),
    ));
}

/// Wall instance selection with the join-trimmed body attached
/// (RE-26).
///
/// The record box is the wall's untrimmed prism; Revit's exported
/// body is the same prism after the joins cut it back, and
/// [`crate::element_record_wall_joins`] recovers the cut from the
/// recovered wall set alone. A wall the solver declines keeps its
/// record box and says so in `m_wall_body_source`.
pub fn wall_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    use crate::element_record_wall_joins as joins;

    let selected = select_instance_records(records);
    let boxes: Vec<crate::partition_element_records::PartitionElementRecord> =
        selected.values().cloned().collect();
    let trims = joins::join_trims(&boxes);
    selected
        .values()
        .map(|record| {
            let mut decoded = element_record_decoded(record, "Wall", level_ids);
            if let Some(trim) = trims.get(&record.element_id) {
                apply_wall_join_trim(&mut decoded, record, trim);
            }
            decoded
        })
        .collect()
}

/// Rewrite a wall's plan centre and plan extents to the trimmed body.
fn apply_wall_join_trim(
    decoded: &mut DecodedElement,
    record: &crate::partition_element_records::PartitionElementRecord,
    trim: &crate::element_record_wall_joins::WallJoinTrim,
) {
    use crate::element_record_wall_joins as joins;

    let low = record.bbox_feet[trim.axis] + trim.start_feet;
    let high = record.bbox_feet[trim.axis + 3] - trim.end_feet;
    let (centre_field, extent_field) = if trim.axis == 0 {
        ("m_locationX", "m_bboxWidth")
    } else {
        ("m_locationY", "m_bboxDepth")
    };
    for (name, value) in decoded.fields.iter_mut() {
        if name == centre_field {
            *value = InstanceField::Float {
                value: (low + high) * 0.5,
                size: 8,
            };
        } else if name == extent_field {
            *value = InstanceField::Float {
                value: high - low,
                size: 8,
            };
        }
    }
    decoded.fields.push((
        joins::WALL_BODY_SOURCE_FIELD.into(),
        InstanceField::String(joins::WALL_BODY_JOIN_TRIMMED.into()),
    ));
    decoded.fields.push((
        joins::WALL_THICKNESS_FIELD.into(),
        InstanceField::Float {
            value: trim.thickness_feet,
            size: 8,
        },
    ));
    decoded.fields.push((
        joins::WALL_TRIM_START_FIELD.into(),
        InstanceField::Float {
            value: trim.start_feet,
            size: 8,
        },
    ));
    decoded.fields.push((
        joins::WALL_TRIM_END_FIELD.into(),
        InstanceField::Float {
            value: trim.end_feet,
            size: 8,
        },
    ));
}

fn element_record_decoded(
    record: &crate::partition_element_records::PartitionElementRecord,
    class: &str,
    level_ids: &BTreeSet<u32>,
) -> DecodedElement {
    let (cx, cy) = record.plan_centre_feet();
    let (dx, dy, dz) = record.extents_feet();
    let level_id =
        crate::element_record_level_refs::unique_level_reference(&record.references, level_ids);
    let mut fields = vec![
        (
            "m_locationX".into(),
            InstanceField::Float { value: cx, size: 8 },
        ),
        (
            "m_locationY".into(),
            InstanceField::Float { value: cy, size: 8 },
        ),
        (
            "m_locationZ".into(),
            InstanceField::Float {
                value: record.bbox_feet[2],
                size: 8,
            },
        ),
        (
            "m_bboxWidth".into(),
            InstanceField::Float { value: dx, size: 8 },
        ),
        (
            "m_bboxDepth".into(),
            InstanceField::Float { value: dy, size: 8 },
        ),
        (
            "m_bboxHeight".into(),
            InstanceField::Float { value: dz, size: 8 },
        ),
        (
            "m_builtinCategory".into(),
            InstanceField::Integer {
                value: record.builtin_category,
                signed: true,
                size: 8,
            },
        ),
        (
            "m_source_stream".into(),
            InstanceField::String(record.stream.clone()),
        ),
        (
            "m_source_offset".into(),
            InstanceField::Integer {
                value: record.offset as i64,
                signed: false,
                size: 8,
            },
        ),
        (
            "m_source".into(),
            InstanceField::String("partition_element_record".into()),
        ),
        // The host Level, when the record's counted reference list
        // names exactly one of them (#219, RE-27). The extrusion
        // height below is still the recorded bbox extent, never a
        // level-to-level span.
        (
            "m_level_bound".into(),
            InstanceField::Bool(level_id.is_some()),
        ),
    ];
    if let Some(id) = level_id {
        fields.push((
            crate::element_record_level_refs::LEVEL_REFERENCE_FIELD.into(),
            InstanceField::ElementId { tag: 0, id },
        ));
    }
    // RE-34: the ElementId of a second-prologue frame is inferred from the
    // reference list and the frame order, not read at `+0x00`. Say so.
    if record.id_from_reference_order {
        fields.push((
            ID_FROM_REFERENCE_ORDER_FIELD.into(),
            InstanceField::Bool(true),
        ));
    }
    let mut provenance = ElementProvenance::partition(
        &record.stream,
        record.offset,
        "partition_element_record",
        "partition_schema_mvp::element_category_record",
        0.8,
        Some("level_binding_unresolved"),
    );
    if record.id_from_reference_order {
        provenance
            .warnings
            .push(ID_FROM_REFERENCE_ORDER_WARNING.into());
    }
    DecodedElement {
        id: Some(record.element_id),
        class: class.into(),
        fields,
        byte_range: record.offset
            ..record
                .offset
                .saturating_add(crate::partition_element_records::RECORD_MIN_LEN),
        provenance,
    }
}

/// Field set on an element whose ElementId came from a second-prologue
/// frame's reference list and frame order (RE-34) rather than from `+0x00`.
pub const ID_FROM_REFERENCE_ORDER_FIELD: &str = "m_id_from_reference_order";

/// Provenance warning on the same elements (RE-34).
pub const ID_FROM_REFERENCE_ORDER_WARNING: &str = "element_id_from_reference_order";

fn levels_from_storeys_and_names(
    walls: &[PartitionArcWall],
    level_names: &[String],
) -> Vec<DecodedElement> {
    let recovery = partition_arc_walls::recover_storeys_from_arc_walls(walls, level_names);
    if recovery.storeys.is_empty() {
        // Elevation-less files: still surface named building storeys as
        // Level rows with name only (elevation Absent for geometry).
        return level_names
            .iter()
            .enumerate()
            .map(|(i, name)| level_decoded(name, None, i))
            .collect();
    }
    recovery
        .storeys
        .into_iter()
        .enumerate()
        .map(|(i, s)| level_decoded(&s.name, Some(s.elevation_feet), i))
        .collect()
}

fn level_decoded(name: &str, elevation: Option<f64>, index: usize) -> DecodedElement {
    let mut fields = vec![
        ("m_name".into(), InstanceField::String(name.to_string())),
        ("m_isBuildingStory".into(), InstanceField::Bool(true)),
        (
            "m_source".into(),
            InstanceField::String("partition_schema_mvp".into()),
        ),
    ];
    if let Some(elev) = elevation {
        fields.push((
            "m_elevation".into(),
            InstanceField::Float {
                value: elev,
                size: 8,
            },
        ));
    }
    let confidence = if elevation.is_some() { 0.75 } else { 0.55 };
    DecodedElement {
        id: None,
        class: "Level".into(),
        fields,
        byte_range: index..index,
        provenance: ElementProvenance::partition(
            "partition",
            index,
            "partition_schema_mvp",
            "partition_schema_mvp::level",
            confidence,
            if elevation.is_none() {
                Some("elevation_unknown")
            } else {
                None
            },
        ),
    }
}

fn materials_from_names(name_set: &BTreeSet<(NameBucket, String)>) -> Vec<DecodedElement> {
    name_set
        .iter()
        .filter(|(b, n)| *b == NameBucket::MaterialLike && is_strict_material_name(n))
        .enumerate()
        .map(|(i, (_, name))| {
            DecodedElement {
                id: None,
                class: "Material".into(),
                fields: vec![
                    ("m_name".into(), InstanceField::String(name.clone())),
                    (
                        "m_source".into(),
                        InstanceField::String("partition_schema_mvp".into()),
                    ),
                ],
                byte_range: i..i,
                provenance: Default::default(),
            }
            .with_provenance(ElementProvenance::partition(
                "partition",
                i,
                "partition_schema_mvp",
                "partition_schema_mvp::material_name",
                0.6,
                None::<String>,
            ))
        })
        .collect()
}

/// Reject schedule / schema / compound material path noise.
pub fn is_strict_material_name(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 || t.len() > 48 {
        return false;
    }
    if t.contains(':') || t.contains('/') || t.contains('\\') || t.contains('.') {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.contains("schema") || lower.ends_with(" material") || lower.contains("default") {
        return false;
    }
    // Keep classify_name's material bucket; just tighten.
    classify_name(t) == Some(NameBucket::MaterialLike)
}

fn partition_streams_largest_first(rf: &mut RevitFile) -> Vec<String> {
    let mut streams: Vec<(usize, String)> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .filter_map(|s| {
            let raw = rf.read_stream(&s).ok()?;
            Some((raw.len(), s))
        })
        .collect();
    streams.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    streams.into_iter().map(|(_, s)| s).collect()
}

fn rect_openings_from_partitions(
    rf: &mut RevitFile,
    revit_version: u32,
    limits: WalkerLimits,
) -> Result<Vec<DecodedElement>> {
    let mut out = Vec::new();
    let opening_cap = limits.max_candidates.min(5_000);

    // Confirm related ids against ElemTable when available — never invent
    // Door/Window classes from the index alone.
    let elem_ids: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => BTreeSet::new(),
    };

    // Largest partition first — 2024 Core Interior openings live in
    // the ~98 MiB Partitions/46 stream.
    for stream in partition_streams_largest_first(rf) {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let concat = inflated.bytes();
        let offsets = ArcWallRectOpeningIndex::find_all_for_revit_version(revit_version, concat);
        for off in offsets {
            if out.len() >= opening_cap {
                break;
            }
            let Ok(rec) = ArcWallRectOpeningIndex::decode(concat, off) else {
                continue;
            };
            let a_in = elem_ids.contains(&rec.related_id_a);
            let b_in = elem_ids.contains(&rec.related_id_b);
            // related_id_a is the historical host *candidate* (RE-15);
            // ElemTable confirmation only proves the id is declared —
            // not that it is a Wall host or a Door/Window instance.
            let host_confirmed = a_in;
            out.push(DecodedElement {
                id: None,
                class: "ArcWallRectOpening".into(),
                fields: vec![
                    (
                        "m_index".into(),
                        InstanceField::Integer {
                            value: i64::from(rec.index),
                            signed: false,
                            size: 4,
                        },
                    ),
                    (
                        "m_related_id_a".into(),
                        InstanceField::ElementId {
                            tag: 0,
                            id: rec.related_id_a,
                        },
                    ),
                    (
                        "m_related_id_b".into(),
                        InstanceField::ElementId {
                            tag: 0,
                            id: rec.related_id_b,
                        },
                    ),
                    (
                        "m_related_id_a_in_elem_table".into(),
                        InstanceField::Bool(a_in),
                    ),
                    (
                        "m_related_id_b_in_elem_table".into(),
                        InstanceField::Bool(b_in),
                    ),
                    (
                        "m_host_id".into(),
                        InstanceField::ElementId {
                            tag: 0,
                            id: rec.related_id_a,
                        },
                    ),
                    (
                        "m_host_elem_table_confirmed".into(),
                        InstanceField::Bool(host_confirmed),
                    ),
                    (
                        "m_host_provenance".into(),
                        InstanceField::String(if host_confirmed {
                            "related_id_a_in_elem_table".into()
                        } else {
                            "related_id_a_unvalidated".into()
                        }),
                    ),
                    (
                        "m_source_stream".into(),
                        InstanceField::String(stream.clone()),
                    ),
                    (
                        "m_source_offset".into(),
                        InstanceField::Integer {
                            value: off as i64,
                            signed: false,
                            size: 8,
                        },
                    ),
                    (
                        "m_source".into(),
                        InstanceField::String("partition_rect_opening_index".into()),
                    ),
                ],
                byte_range: off..off
                    .saturating_add(crate::rect_opening_index::OPENING_INDEX_STRIDE),
                provenance: ElementProvenance::partition(
                    &stream,
                    off,
                    "partition_rect_opening_index",
                    "partition_schema_mvp::arcwall_rect_opening",
                    if host_confirmed { 0.7 } else { 0.55 },
                    if host_confirmed {
                        None
                    } else {
                        Some("related_id_a_unvalidated")
                    },
                ),
            });
        }
        // Openings concentrate in one large partition; stop once we
        // found any validated index rows.
        if !out.is_empty() {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {

    /// Test shims: every corpus record in these fixtures is synthetic
    /// and names no `Level`, so the #219 join has nothing to resolve.
    fn columns_from_records_t(
        records: Vec<crate::partition_element_records::PartitionElementRecord>,
    ) -> Vec<DecodedElement> {
        columns_from_records(records, &BTreeSet::new())
    }

    fn instances_from_records_t(
        records: Vec<crate::partition_element_records::PartitionElementRecord>,
        class: &str,
    ) -> Vec<DecodedElement> {
        instances_from_records(records, class, &BTreeSet::new())
    }
    use super::*;

    #[test]
    fn strict_material_keeps_concrete_rejects_schema() {
        assert!(is_strict_material_name("Concrete"));
        assert!(is_strict_material_name("Masonry - Brick"));
        assert!(!is_strict_material_name("HardwoodSchema"));
        assert!(!is_strict_material_name("Glass/Glazing:Default:Glass"));
    }

    #[test]
    fn level_decoded_projects_elevation() {
        let el = level_decoded("Level 1", Some(10.0), 0);
        assert_eq!(el.class, "Level");
        let level = crate::elements::level::Level::from_decoded(&el);
        assert_eq!(level.name.as_deref(), Some("Level 1"));
        assert_eq!(level.elevation_feet, Some(10.0));
        assert_eq!(level.is_building_story, Some(true));
    }

    fn column_record(
        element_id: u32,
        bbox_feet: [f64; 6],
    ) -> crate::partition_element_records::PartitionElementRecord {
        element_record(
            element_id,
            crate::partition_element_records::OST_COLUMNS,
            bbox_feet,
        )
    }

    fn element_record(
        element_id: u32,
        builtin_category: i64,
        bbox_feet: [f64; 6],
    ) -> crate::partition_element_records::PartitionElementRecord {
        crate::partition_element_records::PartitionElementRecord {
            stream: "Partitions/46".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0141,
            builtin_category,
            container: crate::partition_element_records::CONTAINER_NONE,
            placement_kind: crate::partition_element_records::PLACEMENT_KIND_INSTANCE,
            bbox_feet,
            preceding_reference: None,
            owner_reference: None,
            references: Vec::new(),
            id_from_reference_order: false,
        }
    }

    #[test]
    fn selection_keeps_the_greatest_vertical_extent_for_one_id() {
        // A `Floor:Floor 1` plate is framed twice on Core Interior:
        // Partitions/46 sees the 2 in topping, Partitions/55 the full
        // 1 ft slab. The export's extrusion depth is 1 ft (#212).
        let mut thin = element_record(
            70433,
            crate::partition_element_records::OST_FLOORS,
            [20.0, 25.0, 30.667, 167.0, 114.0, 30.833],
        );
        thin.stream = "Partitions/46".into();
        let mut full = element_record(
            70433,
            crate::partition_element_records::OST_FLOORS,
            [20.0, 25.0, 29.833, 167.0, 114.0, 30.833],
        );
        full.stream = "Partitions/55".into();
        for records in [
            vec![thin.clone(), full.clone()],
            vec![full.clone(), thin.clone()],
        ] {
            let out = instances_from_records_t(records, "Floor");
            assert_eq!(out.len(), 1);
            let height =
                out[0]
                    .fields
                    .iter()
                    .find_map(|(name, value)| match (name.as_str(), value) {
                        ("m_bboxHeight", InstanceField::Float { value, .. }) => Some(*value),
                        _ => None,
                    });
            assert!(
                height.is_some_and(|h| (h - 1.0).abs() < 1e-3),
                "expected the 1 ft frame, got {height:?}"
            );
        }
    }

    #[test]
    fn selection_tie_breaks_on_the_first_stream_and_offset() {
        let mut a = element_record(
            20311,
            crate::partition_element_records::OST_FLOORS,
            [9.0, 16.0, 75.833, 177.0, 123.0, 76.0],
        );
        a.stream = "Partitions/51".into();
        let mut b = a.clone();
        b.stream = "Partitions/46".into();
        let out = instances_from_records_t(vec![a, b], "Floor");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, Some(20311));
    }

    #[test]
    fn selection_drops_type_symbol_envelopes() {
        let mut symbol = column_record(5755, [-1.0, -1.0, 0.0, 1.0, 1.0, 9.0]);
        symbol.placement_kind = crate::partition_element_records::PLACEMENT_KIND_SYMBOL;
        let records = vec![
            symbol,
            column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]),
        ];
        let out = columns_from_records_t(records);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, Some(20375));
        assert_eq!(out[0].class, "Column");
    }

    #[test]
    fn selection_drops_container_members_not_the_higher_id() {
        // The #204 rule kept the highest ElementId per footprint origin.
        // The #211 rule keeps whichever record is standalone — here the
        // *lower* id, which is what the container reference dictates.
        let mut member = column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]);
        member.container = 16_229;
        let records = vec![
            column_record(16347, [23.0, 109.0, 76.0, 25.0, 111.0, 91.0]),
            member,
            column_record(20376, [48.0, 109.0, 76.0, 50.0, 111.0, 90.33]),
        ];
        let out = columns_from_records_t(records);
        let ids: Vec<Option<u32>> = out.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![Some(16347), Some(20376)]);
    }

    #[test]
    fn selection_labels_each_category_with_its_own_class() {
        for (category, class) in [
            (crate::partition_element_records::OST_WALLS, "Wall"),
            (crate::partition_element_records::OST_DOORS, "Door"),
            (crate::partition_element_records::OST_WINDOWS, "Window"),
        ] {
            let out = instances_from_records_t(
                vec![element_record(
                    4242,
                    category,
                    [0.0, 0.0, 0.0, 4.0, 1.0, 8.0],
                )],
                class,
            );
            assert_eq!(out.len(), 1);
            assert_eq!(out[0].class, class);
            assert_eq!(out[0].id, Some(4242));
            assert_eq!(
                out[0].provenance.decoder.as_deref(),
                Some("partition_schema_mvp::element_category_record")
            );
        }
    }

    #[test]
    fn selection_keeps_one_record_per_element_id() {
        let mut second = column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]);
        second.offset = 999_999;
        let out = columns_from_records_t(vec![
            column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]),
            second,
        ]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn column_decoded_carries_plan_centre_and_extents() {
        let out = columns_from_records_t(vec![column_record(
            20375,
            [23.0, 109.0, 76.0, 25.0, 111.0, 90.33],
        )]);
        let column = &out[0];
        let field = |name: &str| {
            column
                .fields
                .iter()
                .find(|(n, _)| n == name)
                .and_then(|(_, v)| match v {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
        };
        assert_eq!(field("m_locationX"), Some(24.0));
        assert_eq!(field("m_locationY"), Some(110.0));
        assert_eq!(field("m_locationZ"), Some(76.0));
        assert_eq!(field("m_bboxWidth"), Some(2.0));
        assert_eq!(field("m_bboxDepth"), Some(2.0));
        assert!((field("m_bboxHeight").unwrap() - 14.33).abs() < 1e-9);
        assert!(column.provenance.confidence >= 0.55);
    }
}
