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
    // --- Stair parts under their stairs (#323) ---
    attach_aggregate_wholes(rf, &mut out.products);
    // --- Curtain walls and their panels and mullions (RE-46) ---
    attach_curtain_walls(rf, &mut out.walls, &mut out.products);
    // --- Stair and flight riser and tread dimensions (RE-47) ---
    attach_stair_dimensions(rf, revit_version, &mut out.products);
    // --- Stair runs' treads and risers, and their run type (RE-52) ---
    attach_stair_run_bodies(rf, revit_version, &mut out.products);
    // --- Beams along their location lines (RE-49) ---
    attach_beam_axes(rf, revit_version, &mut out.products);
    // --- Roof outlines from their sketch lines (RE-50) ---
    attach_roof_profiles(rf, revit_version, &mut out.products);

    // --- Family and type names (RE-38) ---
    for elements in [
        &mut out.walls,
        &mut out.columns,
        &mut out.doors,
        &mut out.windows,
        &mut out.slabs,
        &mut out.products,
    ] {
        attach_family_and_type_names(rf, elements);
    }
    // --- System-family type names (#322) ---
    let mut unnamed: Vec<&mut DecodedElement> = [&mut out.walls, &mut out.slabs, &mut out.products]
        .into_iter()
        .flat_map(|elements| elements.iter_mut())
        .filter(|element| {
            !element
                .fields
                .iter()
                .any(|(name, _)| name == TYPE_NAME_FIELD)
        })
        .collect();
    attach_system_type_names(rf, revit_version, &mut unnamed);
    // --- Walls' layers and exterior side (RE-53) ---
    attach_wall_layers(rf, revit_version, &mut out.walls);
    // --- IFC export overrides, the element's own or its type's (RE-45) ---
    attach_ifc_export_overrides(
        rf,
        revit_version,
        [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.products,
        ],
    );

    Ok(out)
}

/// Field naming the IFC entity an element's "Export to IFC As" override,
/// or its type's "Export Type to IFC As", names (#212, RE-45). The decoder
/// does not act on it; [`crate::ifc::category_map::lookup_export_override`]
/// decides which values the IFC writer honours.
pub const IFC_EXPORT_AS_FIELD: &str = "m_ifc_export_as";

/// Field naming the IFC predefined type an element's or its type's export
/// parameters set (RE-45). The IFC writer uses it only when it is an
/// enumerator of the entity the element exports as.
pub const IFC_PREDEFINED_TYPE_FIELD: &str = "m_ifc_predefined_type";

/// Attach each element's IFC export overrides (RE-45): the element's own
/// "Export to IFC As" and predefined type, else those its type sets
/// ([`TYPE_ID_FIELD`], from RE-38 or #322). A type's `IfcCoveringType`
/// names `IfcCovering`.
fn attach_ifc_export_overrides(
    rf: &mut RevitFile,
    revit_version: u32,
    lists: [&mut Vec<DecodedElement>; 6],
) {
    use crate::partition_ifc_export_overrides as ieo;
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let parameters = ieo::scan_export_parameters(rf, revit_version, &declared).unwrap_or_default();
    if parameters.is_empty() {
        return;
    }
    let none = ieo::ExportParameters::default();
    for elements in lists {
        for element in elements.iter_mut() {
            let own = element
                .id
                .and_then(|id| parameters.get(&id))
                .unwrap_or(&none);
            let of_type = element
                .fields
                .iter()
                .find_map(|(name, value)| match value {
                    InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
                    _ => None,
                })
                .and_then(|id| parameters.get(&id));
            if let Some(entity) = own.effective_export_as(of_type) {
                element
                    .fields
                    .push((IFC_EXPORT_AS_FIELD.into(), InstanceField::String(entity)));
            }
            if let Some(predefined) = own.effective_predefined_type(of_type) {
                element.fields.push((
                    IFC_PREDEFINED_TYPE_FIELD.into(),
                    InstanceField::String(predefined),
                ));
            }
        }
    }
}

/// Give elements of system families (walls, floors, roofs, ceilings,
/// railings) their type's name (#322).
///
/// The type is the one type-definition record of the element's category
/// that its reference list names (RE-28,
/// [`crate::partition_type_records::unique_type_reference`]). Its name is
/// read from its serialised data
/// ([`crate::partition_names::find_element_data_names`]). Such a type has no
/// family in the file, since Revit names the system family ("Basic Wall")
/// in its own UI language, so only [`TYPE_ID_FIELD`] and [`TYPE_NAME_FIELD`]
/// are set.
fn attach_system_type_names(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &mut [&mut DecodedElement],
) {
    use crate::partition_type_records as ptr;
    if elements.is_empty() || !ptr::supports_revit_version(revit_version) {
        return;
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return;
    };
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let mut type_ids_by_category: std::collections::BTreeMap<i64, BTreeSet<u32>> =
        std::collections::BTreeMap::new();
    let mut picks: Vec<(usize, u32)> = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        let Some((references, category)) = record_references(rf, element) else {
            continue;
        };
        let type_ids = type_ids_by_category.entry(category).or_insert_with(|| {
            let records =
                ptr::scan_type_records(rf, revit_version, category, &declared).unwrap_or_default();
            ptr::type_definition_ids(&records)
        });
        if let Some(type_id) = ptr::unique_type_reference(&references, type_ids) {
            picks.push((index, type_id));
        }
    }
    if picks.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = picks.iter().map(|(_, id)| *id).collect();
    let mut names: std::collections::BTreeMap<u32, Option<String>> =
        std::collections::BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in
            crate::partition_names::find_element_data_names(inflated.bytes(), &header, &wanted)
        {
            match names.get(&id) {
                None => {
                    names.insert(id, Some(name));
                }
                Some(held) if held.as_deref() != Some(name.as_str()) => {
                    names.insert(id, None);
                }
                _ => {}
            }
        }
    }
    for (index, type_id) in picks {
        let Some(Some(name)) = names.get(&type_id) else {
            continue;
        };
        let element = &mut elements[index];
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        element
            .fields
            .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
    }
}

/// Field naming an element-record element's type, from the partition name
/// entries (RE-38).
pub const TYPE_NAME_FIELD: &str = "m_type_name";
/// Field naming the family of that type (RE-38).
pub const FAMILY_NAME_FIELD: &str = "m_family_name";
/// Field carrying the type's ElementId (RE-38).
pub const TYPE_ID_FIELD: &str = "m_type_id";

/// Give each element-record element its family and type names (RE-38).
///
/// The record's reference list is read again at its source offset. Its type
/// is the one reference with a name entry of the record's category, and
/// the family is the one such id in the type's own partition record
/// ([`crate::partition_names::resolve_type`] /
/// [`crate::partition_names::resolve_family`]). An element gets the fields
/// only when both joins are unique.
fn attach_family_and_type_names(rf: &mut RevitFile, elements: &mut [DecodedElement]) {
    let names = rf.element_names();
    if names.entries.is_empty() {
        return;
    }
    for element in elements.iter_mut() {
        let Some(own) = element.id else {
            continue;
        };
        let Some((references, category)) = record_references(rf, element) else {
            continue;
        };
        let Some(type_id) =
            crate::partition_names::resolve_type(&names, &references, category, own)
        else {
            continue;
        };
        let Some(family_id) = crate::partition_names::resolve_family(&names, type_id) else {
            continue;
        };
        let (Some(type_entry), Some(family_entry)) =
            (names.entries.get(&type_id), names.entries.get(&family_id))
        else {
            continue;
        };
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String(type_entry.name.clone()),
        ));
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(family_entry.name.clone()),
        ));
    }
}

/// Field naming the ElementId of the stair a run, landing or stringer
/// belongs to (#323).
pub const AGGREGATE_WHOLE_FIELD: &str = "m_aggregate_whole";

/// Classes that aggregate parts instead of carrying a body of their own.
pub const AGGREGATE_WHOLE_CLASSES: &[&str] = &["Stair", CURTAIN_WALL_CLASS];

/// Class of a wall that is a curtain wall (RE-46): one a curtain-wall
/// mullion names in its reference list.
pub const CURTAIN_WALL_CLASS: &str = "CurtainWall";

/// Classes that are parts of a curtain wall when their reference list names
/// exactly one (RE-46).
///
/// Doors are not among them: of the doors whose lists name a Snowdon
/// curtain wall, Revit's export aggregates only the panel doors, and nothing
/// read so far tells those from doors inserted in the wall.
pub const CURTAIN_WALL_PART_CLASSES: &[&str] = &["CurtainWallPanel", "CurtainWallMullion"];

/// Classes that are parts of a stair when their reference list names one.
///
/// Railings are not among them: Revit's export aggregates 65 of the 70
/// Snowdon railings whose lists name a stair, and nothing read so far tells
/// the other five apart, so railings stay standalone elements.
pub const STAIR_PART_CLASSES: &[&str] = &["StairsRun", "StairsLanding", "StairsStringer"];

/// The reference list and category of an element-record element, read
/// again at its source offset.
fn record_references(rf: &mut RevitFile, element: &DecodedElement) -> Option<(Vec<u64>, i64)> {
    let mut stream = None;
    let mut offset = None;
    let mut category = None;
    for (name, value) in &element.fields {
        match (name.as_str(), value) {
            ("m_source_stream", InstanceField::String(v)) => stream = Some(v.clone()),
            ("m_source_offset", InstanceField::Integer { value, .. }) => {
                offset = usize::try_from(*value).ok();
            }
            ("m_builtinCategory", InstanceField::Integer { value, .. }) => {
                category = Some(*value);
            }
            _ => {}
        }
    }
    let inflated = rf.inflated_partition(&stream?).ok()?;
    let references = offset?
        .checked_add(crate::partition_element_records::REFERENCE_LIST_OFFSET)
        .and_then(|at| {
            crate::partition_element_records::decode_reference_list(inflated.bytes(), at)
        })
        .unwrap_or_default();
    Some((references, category?))
}

/// Give each stair part the ElementId of the one exported stair its
/// reference list names (#323). On Snowdon Towers every run, landing and
/// stringer Revit aggregates under a stair, but one stringer, names that
/// stair in its reference list. A part that names none, or more than one,
/// stays a standalone element.
fn attach_aggregate_wholes(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let stairs: BTreeSet<u32> = products
        .iter()
        .filter(|element| AGGREGATE_WHOLE_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| element.id)
        .collect();
    if stairs.is_empty() {
        return;
    }
    for element in products.iter_mut() {
        if !STAIR_PART_CLASSES.contains(&element.class.as_str()) {
            continue;
        }
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        let named: BTreeSet<u32> = references
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| stairs.contains(id) && Some(*id) != element.id)
            .collect();
        if named.len() == 1 {
            let whole = *named.iter().next().expect("one stair");
            element.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        }
    }
}

/// Field holding a stair's or flight's number of risers (RE-47).
pub const STAIR_RISER_COUNT_FIELD: &str = "m_stair_riser_count";
/// Field holding a stair's or flight's riser height, feet (RE-47).
pub const STAIR_RISER_HEIGHT_FIELD: &str = "m_stair_riser_height";
/// Field holding a stair's or flight's tread depth, feet (RE-47).
pub const STAIR_TREAD_DEPTH_FIELD: &str = "m_stair_tread_depth";

/// Give each stair its riser height, tread depth and number of risers, and
/// each of its runs the same riser height and tread depth and, where
/// [`crate::partition_stairs::flight_riser_counts`] can tell, its own number
/// of risers (RE-47). Runs are the stair's parts (#323).
fn attach_stair_dimensions(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_stairs as ps;
    if !ps::supports_revit_version(revit_version) {
        return;
    }
    let stairs: Vec<u32> = products
        .iter()
        .filter(|element| element.class == "Stair")
        .filter_map(|element| element.id)
        .collect();
    let whole_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == AGGREGATE_WHOLE_FIELD => Some(*id),
            _ => None,
        })
    };
    let runs: Vec<(u32, u32)> = products
        .iter()
        .filter(|element| element.class == "StairsRun")
        .filter_map(|element| Some((element.id?, whole_of(element)?)))
        .filter(|(_, stair)| stairs.contains(stair))
        .collect();
    if stairs.is_empty() {
        return;
    }
    let run_ids: Vec<u32> = runs.iter().map(|(run, _)| *run).collect();
    let Ok((dimensions, run_counts)) =
        ps::scan_stair_dimensions(rf, revit_version, &stairs, &run_ids)
    else {
        return;
    };
    let mut flight_counts: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    for (stair, found) in &dimensions {
        let of_stair: Vec<(u32, Option<u32>)> = runs
            .iter()
            .filter(|(_, whole)| whole == stair)
            .map(|(run, _)| (*run, run_counts.get(run).copied()))
            .collect();
        flight_counts.extend(ps::flight_riser_counts(found.riser_count, &of_stair));
    }
    let push = |element: &mut DecodedElement, found: &ps::StairDimensions, risers: Option<u32>| {
        if let Some(risers) = risers {
            element.fields.push((
                STAIR_RISER_COUNT_FIELD.into(),
                InstanceField::Integer {
                    value: i64::from(risers),
                    signed: false,
                    size: 4,
                },
            ));
        }
        element.fields.push((
            STAIR_RISER_HEIGHT_FIELD.into(),
            InstanceField::Float {
                value: found.riser_height_feet,
                size: 8,
            },
        ));
        element.fields.push((
            STAIR_TREAD_DEPTH_FIELD.into(),
            InstanceField::Float {
                value: found.tread_depth_feet,
                size: 8,
            },
        ));
    };
    for element in products.iter_mut() {
        let Some(id) = element.id else {
            continue;
        };
        if element.class == "Stair" {
            if let Some(found) = dimensions.get(&id) {
                push(element, found, Some(found.riser_count));
            }
        } else if element.class == "StairsRun" {
            let Some(found) = whole_of(element).and_then(|stair| dimensions.get(&stair)) else {
                continue;
            };
            let found = *found;
            push(element, &found, flight_counts.get(&id).copied());
        }
    }
}

/// Fields holding the unit plan direction from a wall's inside to its
/// exterior face, model axes (RE-53).
pub const WALL_EXTERIOR_FIELDS: [&str; 2] = ["m_wall_exterior_x", "m_wall_exterior_y"];
/// Field holding a wall's layers, exterior first: each a vector of width
/// (feet), shading colour (`0x00BBGGRR`, or -1 for the category's) and
/// transparency (RE-53).
pub const WALL_LAYERS_FIELD: &str = "m_wall_layers";

/// Fields holding a wall's centreline start and end in plan, model feet
/// (RE-54).
pub const WALL_AXIS_START_FIELDS: [&str; 2] = ["m_wall_axis_start_x", "m_wall_axis_start_y"];
/// See [`WALL_AXIS_START_FIELDS`].
pub const WALL_AXIS_END_FIELDS: [&str; 2] = ["m_wall_axis_end_x", "m_wall_axis_end_y"];
/// Field holding a wall's thickness, its type's layers summed (RE-54).
pub const WALL_TYPE_THICKNESS_FIELD: &str = "m_wall_type_thickness";

/// A wall's centreline and its type's thickness (RE-54).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallAxis {
    /// Plan start, model feet.
    pub start: [f64; 2],
    /// Plan end, model feet.
    pub end: [f64; 2],
    /// The type's layers summed, feet.
    pub thickness_feet: f64,
}

/// The centreline and thickness [`WALL_AXIS_START_FIELDS`],
/// [`WALL_AXIS_END_FIELDS`] and [`WALL_TYPE_THICKNESS_FIELD`] record.
pub fn wall_axis_from_fields(fields: &[(String, InstanceField)]) -> Option<WallAxis> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [sx, sy] = WALL_AXIS_START_FIELDS.map(float);
    let [ex, ey] = WALL_AXIS_END_FIELDS.map(float);
    Some(WallAxis {
        start: [sx?, sy?],
        end: [ex?, ey?],
        thickness_feet: float(WALL_TYPE_THICKNESS_FIELD)?,
    })
}

/// The layers [`WALL_LAYERS_FIELD`] and [`WALL_EXTERIOR_FIELDS`] record.
pub fn element_layers_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::ifc::ElementLayers> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = WALL_EXTERIOR_FIELDS.map(float);
    let layers = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(items) if name == WALL_LAYERS_FIELD => items
            .iter()
            .map(|item| match item {
                InstanceField::Vector(parts) => match parts.as_slice() {
                    [
                        InstanceField::Float { value: width, .. },
                        InstanceField::Integer { value: colour, .. },
                        InstanceField::Float {
                            value: transparency,
                            ..
                        },
                    ] => Some(crate::ifc::LayerBand {
                        width_feet: *width,
                        color_packed: u32::try_from(*colour).ok(),
                        transparency: *transparency,
                    }),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Option<Vec<_>>>(),
        _ => None,
    })?;
    (!layers.is_empty()).then_some(crate::ifc::ElementLayers {
        exterior_normal: [x?, y?],
        layers,
    })
}

/// Give each wall its type's layers, exterior first, each with its
/// material's shading, and the plan direction of its exterior face: the
/// right of its location line's direction when its flip flag is set, the
/// left otherwise (RE-53, [`crate::partition_compound_structure`],
/// [`crate::partition_materials`]). A wall whose word is 1 also gets its
/// centreline and its type's thickness, from which the exporter builds its
/// body (RE-54). A wall without a type, a location line or an orientation
/// gets none of these. Membranes, which have no width, are left out.
fn attach_wall_layers(rf: &mut RevitFile, revit_version: u32, walls: &mut [DecodedElement]) {
    use crate::partition_compound_structure as pcs;
    if !pcs::WALL_LINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
        || !pcs::COMPOUND_STRUCTURE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
    {
        return;
    }
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let wall_ids: BTreeSet<u32> = walls
        .iter()
        .filter(|wall| type_of(wall).is_some())
        .filter_map(|wall| wall.id)
        .collect();
    if wall_ids.is_empty() {
        return;
    }
    let types: BTreeSet<u32> = walls.iter().filter_map(type_of).collect();
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let materials: BTreeSet<u32> = crate::partition_type_records::scan_type_records(
        rf,
        revit_version,
        crate::partition_type_records::OST_MATERIALS,
        &declared,
    )
    .unwrap_or_default()
    .iter()
    .map(|record| record.element_id)
    .collect();
    let (Ok(layers), Ok(orientations), Ok(lines), Ok(appearances)) = (
        pcs::scan_type_layers(rf, revit_version, &types, &materials, &declared),
        pcs::scan_wall_orientations(rf, revit_version, &wall_ids),
        pcs::scan_wall_lines(rf, revit_version, &wall_ids),
        crate::partition_materials::scan_material_appearances(rf, revit_version, &declared),
    ) else {
        return;
    };
    for wall in walls.iter_mut() {
        let (Some(id), Some(type_id)) = (wall.id, type_of(wall)) else {
            continue;
        };
        let (Some(type_layers), Some(orientation), Some(line)) =
            (layers.get(&type_id), orientations.get(&id), lines.get(&id))
        else {
            continue;
        };
        let (start, end) = (line.start(), line.end());
        let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
        let length = dx.hypot(dy);
        if !length.is_finite() || length <= 1e-9 {
            continue;
        }
        let thickness: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        if orientation.word == 1 && thickness.is_finite() && thickness > 0.0 {
            for (names, point) in [(WALL_AXIS_START_FIELDS, start), (WALL_AXIS_END_FIELDS, end)] {
                for (name, value) in names.iter().zip(point) {
                    wall.fields
                        .push(((*name).into(), InstanceField::Float { value, size: 8 }));
                }
            }
            wall.fields.push((
                WALL_TYPE_THICKNESS_FIELD.into(),
                InstanceField::Float {
                    value: thickness,
                    size: 8,
                },
            ));
        }
        let (dx, dy) = (dx / length, dy / length);
        let exterior = if orientation.flip {
            [dy, -dx]
        } else {
            [-dy, dx]
        };
        let bands: Vec<InstanceField> = type_layers
            .iter()
            .filter(|layer| layer.width_feet > 0.0)
            .map(|layer| {
                let appearance = layer.material.and_then(|m| appearances.get(&m));
                InstanceField::Vector(vec![
                    InstanceField::Float {
                        value: layer.width_feet,
                        size: 8,
                    },
                    InstanceField::Integer {
                        value: appearance.map_or(-1, |a| i64::from(a.color_packed())),
                        signed: true,
                        size: 8,
                    },
                    InstanceField::Float {
                        value: appearance.map_or(0.0, |a| f64::from(a.transparency)),
                        size: 8,
                    },
                ])
            })
            .collect();
        if bands.is_empty() {
            continue;
        }
        for (name, value) in WALL_EXTERIOR_FIELDS.iter().zip(exterior) {
            wall.fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        wall.fields
            .push((WALL_LAYERS_FIELD.into(), InstanceField::Vector(bands)));
    }
}

/// Fields holding where a stair run's sketch starts, model feet (RE-52).
pub const STAIR_RUN_ORIGIN_FIELDS: [&str; 3] = [
    "m_stair_run_origin_x",
    "m_stair_run_origin_y",
    "m_stair_run_origin_z",
];
/// Fields holding the unit plan direction a stair run climbs in (RE-52).
pub const STAIR_RUN_CLIMB_FIELDS: [&str; 2] = ["m_stair_run_climb_x", "m_stair_run_climb_y"];
/// Fields holding the unit plan direction across a stair run (RE-52).
pub const STAIR_RUN_ACROSS_FIELDS: [&str; 2] = ["m_stair_run_across_x", "m_stair_run_across_y"];
/// Field holding a stair run's width, feet (RE-52).
pub const STAIR_RUN_WIDTH_FIELD: &str = "m_stair_run_width";
/// Field holding a stair run's side view, `[along, up]` feet from its
/// sketch origin (RE-52).
pub const STAIR_RUN_PROFILE_FIELD: &str = "m_stair_run_profile";

/// A stair run's treads and risers as [`STAIR_RUN_PROFILE_FIELD`] and its
/// companions record them (RE-52).
#[derive(Debug, Clone, PartialEq)]
pub struct StairRunBody {
    /// Where the run's sketch starts, model feet.
    pub origin: [f64; 3],
    /// Unit plan direction the run climbs in.
    pub climb: [f64; 2],
    /// Unit plan direction across the run.
    pub across: [f64; 2],
    /// Feet.
    pub width_feet: f64,
    /// Side view, `[along, up]` feet from `origin`, counter-clockwise.
    pub profile: Vec<[f64; 2]>,
}

/// The run body recorded in `fields`, when every part of it is.
pub fn stair_run_body_from_fields(fields: &[(String, InstanceField)]) -> Option<StairRunBody> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let profile = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(points) if name == STAIR_RUN_PROFILE_FIELD => points
            .iter()
            .map(|point| match point {
                InstanceField::Vector(pair) => match pair.as_slice() {
                    [
                        InstanceField::Float { value: u, .. },
                        InstanceField::Float { value: z, .. },
                    ] => Some([*u, *z]),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Option<Vec<[f64; 2]>>>(),
        _ => None,
    })?;
    let [ox, oy, oz] = STAIR_RUN_ORIGIN_FIELDS.map(float);
    let [cx, cy] = STAIR_RUN_CLIMB_FIELDS.map(float);
    let [ax, ay] = STAIR_RUN_ACROSS_FIELDS.map(float);
    Some(StairRunBody {
        origin: [ox?, oy?, oz?],
        climb: [cx?, cy?],
        across: [ax?, ay?],
        width_feet: float(STAIR_RUN_WIDTH_FIELD)?,
        profile: (profile.len() >= 3).then_some(profile)?,
    })
}

/// Give each straight stair run with separate treads and risers its side
/// view, from the plan sketch in its own data, its run type and its
/// stair's riser height (RE-52, [`crate::partition_stairs`]), and each run
/// whose run type reads that type's id and name. A run whose riser lines
/// do not number its risers is left to its record box.
fn attach_stair_run_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_stairs as ps;
    use crate::partition_type_records as ptr;
    if !ps::supports_revit_version(revit_version) {
        return;
    }
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let integer = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let run_indices: Vec<usize> = products
        .iter()
        .enumerate()
        .filter(|(_, element)| element.class == "StairsRun" && element.id.is_some())
        .map(|(index, _)| index)
        .collect();
    if run_indices.is_empty() {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_records = ptr::scan_type_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_STAIRS_RUNS,
        &declared,
    )
    .unwrap_or_default();
    let run_types = ptr::type_definition_ids(&type_records);
    let mut type_of: std::collections::BTreeMap<usize, u32> = std::collections::BTreeMap::new();
    for &index in &run_indices {
        if let Some((references, _)) = record_references(rf, &products[index]) {
            if let Some(type_id) = ptr::unique_type_reference(&references, &run_types) {
                type_of.insert(index, type_id);
            }
        }
    }
    let run_ids: BTreeSet<u32> = run_indices
        .iter()
        .filter_map(|&index| products[index].id)
        .collect();
    let type_ids: BTreeSet<u32> = type_of.values().copied().collect();
    let (Ok(lines), Ok(types)) = (
        ps::scan_run_lines(rf, revit_version, &run_ids),
        ps::scan_run_types(rf, revit_version, &type_ids),
    ) else {
        return;
    };
    for index in run_indices {
        let element = &mut products[index];
        let Some(run_type) = type_of.get(&index).and_then(|id| types.get(id)) else {
            continue;
        };
        let has_type_name = element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_NAME_FIELD);
        if let (Some(name), false) = (&run_type.name, has_type_name) {
            element.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: type_of[&index],
                },
            ));
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        let (Some(riser_height), Some(risers)) = (
            float(element, STAIR_RISER_HEIGHT_FIELD),
            integer(element, STAIR_RISER_COUNT_FIELD),
        ) else {
            continue;
        };
        let Some(sketch) = element
            .id
            .and_then(|id| lines.get(&id))
            .and_then(|lines| ps::run_sketch(lines))
            .filter(|sketch| i64::try_from(sketch.risers.len()) == Ok(risers))
        else {
            continue;
        };
        let Some(profile) = ps::run_side_profile(&sketch, run_type, riser_height) else {
            continue;
        };
        let scalar = |value: f64| InstanceField::Float { value, size: 8 };
        for (name, value) in STAIR_RUN_ORIGIN_FIELDS.iter().zip(sketch.origin) {
            element.fields.push(((*name).into(), scalar(value)));
        }
        for (name, value) in STAIR_RUN_CLIMB_FIELDS.iter().zip(sketch.climb) {
            element.fields.push(((*name).into(), scalar(value)));
        }
        for (name, value) in STAIR_RUN_ACROSS_FIELDS.iter().zip(sketch.across) {
            element.fields.push(((*name).into(), scalar(value)));
        }
        element
            .fields
            .push((STAIR_RUN_WIDTH_FIELD.into(), scalar(sketch.width_feet)));
        element.fields.push((
            STAIR_RUN_PROFILE_FIELD.into(),
            InstanceField::Vector(
                profile
                    .iter()
                    .map(|&[u, z]| InstanceField::Vector(vec![scalar(u), scalar(z)]))
                    .collect(),
            ),
        ));
    }
}

/// Fields holding the first end of a beam's location line, model feet
/// (RE-49).
pub const BEAM_AXIS_START_FIELDS: [&str; 3] = [
    "m_beam_axis_start_x",
    "m_beam_axis_start_y",
    "m_beam_axis_start_z",
];
/// Fields holding the second end of a beam's location line, model feet
/// (RE-49).
pub const BEAM_AXIS_END_FIELDS: [&str; 3] = [
    "m_beam_axis_end_x",
    "m_beam_axis_end_y",
    "m_beam_axis_end_z",
];

/// The record box `[min x, min y, min z, max x, max y, max z]` of an element
/// built from a partition element record: its location fields hold the plan
/// centre and the base.
pub fn element_record_bbox(element: &DecodedElement) -> Option<[f64; 6]> {
    let field = |wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let (x, y, z) = (
        field("m_locationX")?,
        field("m_locationY")?,
        field("m_locationZ")?,
    );
    let (width, depth, height) = (
        field("m_bboxWidth")?,
        field("m_bboxDepth")?,
        field("m_bboxHeight")?,
    );
    Some([
        x - width / 2.0,
        y - depth / 2.0,
        z,
        x + width / 2.0,
        y + depth / 2.0,
        z + height,
    ])
}

/// The two ends of a beam's location line, when the partition MVP gave it one
/// (RE-49).
pub fn beam_axis_from_fields(fields: &[(String, InstanceField)]) -> Option<([f64; 3], [f64; 3])> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    Some((point(BEAM_AXIS_START_FIELDS)?, point(BEAM_AXIS_END_FIELDS)?))
}

/// Give each structural-framing element the location line its data carries,
/// when the line and the element's record box make a beam solid
/// ([`crate::partition_beam_axes::beam_body`], RE-49). A line that leaves the
/// box, or a box no solid along the line reproduces, is not attached.
fn attach_beam_axes(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_beam_axes as pba;
    if !pba::supports_revit_version(revit_version) {
        return;
    }
    let beams: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "StructuralFraming")
        .filter_map(|element| element.id)
        .collect();
    if beams.is_empty() {
        return;
    }
    let Ok(lines) = pba::scan_bounded_lines(rf, revit_version, &beams) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StructuralFraming")
    {
        let Some(line) = element.id.and_then(|id| lines.get(&id)) else {
            continue;
        };
        let (start, end) = (line.start(), line.end());
        let resolves = element_record_bbox(element)
            .and_then(|bbox| pba::beam_body(bbox, start, end))
            .is_some();
        if !resolves {
            continue;
        }
        for (names, point) in [(BEAM_AXIS_START_FIELDS, start), (BEAM_AXIS_END_FIELDS, end)] {
            for (name, value) in names.iter().zip(point) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
    }
}

/// Make every wall a curtain-wall mullion names a curtain wall, and give
/// each panel and mullion that names exactly one of them that wall as its
/// aggregate whole (RE-46).
///
/// A curtain wall owns its grid's mullions, and each mullion's reference
/// list names its wall. On Snowdon Towers the walls named by a mullion are
/// exactly the 42 Revit exports as `IfcCurtainWall` aggregates, and on RE1
/// Architecture the one; no other wall is named by a mullion. Walls named
/// only by a panel are basic walls used as a panel's infill, which Revit
/// exports as `IfcWall`, so a panel alone does not make a curtain wall.
fn attach_curtain_walls(
    rf: &mut RevitFile,
    walls: &mut [DecodedElement],
    products: &mut [DecodedElement],
) {
    let wall_ids: BTreeSet<u32> = walls.iter().filter_map(|wall| wall.id).collect();
    if wall_ids.is_empty() {
        return;
    }
    let mut references: Vec<Option<Vec<u64>>> = Vec::with_capacity(products.len());
    let mut curtain_walls: BTreeSet<u32> = BTreeSet::new();
    for product in products.iter() {
        if !CURTAIN_WALL_PART_CLASSES.contains(&product.class.as_str()) {
            references.push(None);
            continue;
        }
        let list = record_references(rf, product).map(|(list, _)| list);
        if product.class == "CurtainWallMullion" {
            for id in list.iter().flatten() {
                if let Ok(id) = u32::try_from(*id) {
                    if wall_ids.contains(&id) {
                        curtain_walls.insert(id);
                    }
                }
            }
        }
        references.push(list);
    }
    if curtain_walls.is_empty() {
        return;
    }
    for wall in walls.iter_mut() {
        if wall.id.is_some_and(|id| curtain_walls.contains(&id)) {
            wall.class = CURTAIN_WALL_CLASS.into();
        }
    }
    for (product, list) in products.iter_mut().zip(references) {
        let Some(list) = list else {
            continue;
        };
        let named: BTreeSet<u32> = list
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| curtain_walls.contains(id) && Some(*id) != product.id)
            .collect();
        if named.len() == 1 {
            let whole = *named.iter().next().expect("one curtain wall");
            product.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        }
    }
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
        Ok(records) => crate::elem_table::declared_ids(&records),
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
fn without_non_primary_options(
    rf: &mut RevitFile,
    mut records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> Vec<crate::partition_element_records::PartitionElementRecord> {
    let options = rf.design_options();
    records.retain(|record| !options.excludes(record));
    records
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
/// Per-element IFC export-type overrides are attached later, with every
/// other element's, by `attach_ifc_export_overrides` (RE-45).
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
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let categories = [
        per::OST_FLOORS,
        per::OST_BUILDING_PAD,
        per::OST_SKETCH_LINES,
    ];
    let scanned = per::scan_category_records_multi(rf, revit_version, &categories, &declared)?;
    let scanned = without_non_primary_options(rf, scanned);
    let sketch_lines: Vec<per::PartitionElementRecord> = scanned
        .iter()
        .filter(|record| record.builtin_category == per::OST_SKETCH_LINES)
        .cloned()
        .collect();
    let plates: BTreeSet<u32> = scanned
        .iter()
        .filter(|record| {
            record.builtin_category == per::OST_FLOORS
                || record.builtin_category == per::OST_BUILDING_PAD
        })
        .map(|record| record.element_id)
        .collect();
    let profiles = sketch_plan_profiles(rf, revit_version, &sketch_lines, &plates);

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
                if let Some(profile) = profiles.get(&id) {
                    decoded.fields.extend(profile.fields());
                }
            }
            out.push(decoded);
        }
    }
    Ok(out)
}

/// Sketched plan profiles of the elements in `owners`, from their
/// `OST_SketchLines` records: the RE-25 solve over the records' boxes and,
/// where that does not close, the exact ends each sketch line's own data
/// carries (RE-50, [`crate::partition_beam_axes::scan_bounded_lines`]). A
/// line counts only when it is level and both its ends lie in its own
/// record's box; an element any of whose lines does not count keeps no
/// profile.
fn sketch_plan_profiles(
    rf: &mut RevitFile,
    revit_version: u32,
    sketch_lines: &[crate::partition_element_records::PartitionElementRecord],
    owners: &BTreeSet<u32>,
) -> std::collections::BTreeMap<u32, crate::element_record_plan_profiles::PlanProfile> {
    use crate::element_record_plan_profiles as erpp;
    use std::collections::BTreeMap;
    let mut profiles = erpp::plan_profiles_from_sketch_line_records(sketch_lines);
    profiles.retain(|owner, _| owners.contains(owner));
    let mut unsolved: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in sketch_lines {
        let Some(owner) = record.owner_reference else {
            continue;
        };
        if owners.contains(&owner) && !profiles.contains_key(&owner) {
            unsolved
                .entry(owner)
                .or_default()
                .entry(record.element_id)
                .or_insert(record.bbox_feet);
        }
    }
    if unsolved.is_empty() {
        return profiles;
    }
    let ids: BTreeSet<u32> = unsolved
        .values()
        .flat_map(|lines| lines.keys().copied())
        .collect();
    let Ok(lines) = crate::partition_beam_axes::scan_bounded_lines(rf, revit_version, &ids) else {
        return profiles;
    };
    let eps = erpp::VERTEX_EPS_FEET;
    for (owner, segments) in unsolved {
        let exact: Option<Vec<[f64; 4]>> = segments
            .iter()
            .map(|(id, bbox)| {
                let line = lines.get(id)?;
                let (a, b) = (line.start(), line.end());
                let inside = |p: [f64; 3]| {
                    (0..3)
                        .all(|axis| p[axis] >= bbox[axis] - eps && p[axis] <= bbox[axis + 3] + eps)
                };
                (inside(a) && inside(b) && (a[2] - b[2]).abs() <= eps)
                    .then_some([a[0], a[1], b[0], b[1]])
            })
            .collect();
        if let Some(mut profile) = exact.and_then(|exact| erpp::plan_profile_from_lines(&exact)) {
            profile.segment_ids = segments.keys().copied().collect();
            profiles.insert(owner, profile);
        }
    }
    profiles
}

/// Give each roof the plan outline its sketch lines close (RE-50), as a
/// floor has (RE-25). The body stays the record box's height: a sloped
/// roof's slope is not read.
fn attach_roof_profiles(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_element_records as per;
    let roofs: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Roof")
        .filter_map(|element| element.id)
        .collect();
    if roofs.is_empty() || !per::supports_revit_version(revit_version) {
        return;
    }
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(sketch_lines) =
        per::scan_category_records_multi(rf, revit_version, &[per::OST_SKETCH_LINES], &declared)
    else {
        return;
    };
    let profiles = sketch_plan_profiles(rf, revit_version, &sketch_lines, &roofs);
    for roof in products
        .iter_mut()
        .filter(|element| element.class == "Roof")
    {
        if let Some(profile) = roof.id.and_then(|id| profiles.get(&id)) {
            roof.fields.extend(profile.fields());
        }
    }
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
    let provenance = ElementProvenance::partition(
        &record.stream,
        record.offset,
        "partition_element_record",
        "partition_schema_mvp::element_category_record",
        0.8,
        Some("level_binding_unresolved"),
    );
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
        Ok(records) => crate::elem_table::declared_ids(&records),
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
            id_from_enclosing_record: false,
            design_option: None,
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
