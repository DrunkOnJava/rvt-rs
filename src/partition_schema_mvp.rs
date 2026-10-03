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
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::FRAC_PI_4;

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
    out.materials = match materials_from_records(rf, revit_version) {
        Some(materials) => materials,
        None => materials_from_names(&name_set),
    };

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

    // --- Revit 2023 element records (RE-81, #421): identity, category and
    // box only; nothing else 2024 decodes on top of records is read. ---
    if revit_version == crate::partition_element_records_2023::REVIT_2023 {
        recover_2023_records(rf, &mut out);
        attach_room_outlines(rf, revit_version, &mut out.rooms);
        // --- Storeys from the Levels the records name (RE-107), by the
        // same rules as 2024's (RE-59, RE-68) ---
        let level_elevations: std::collections::BTreeMap<u32, f64> =
            crate::partition_level_records::recover_partition_levels(rf, revit_version)
                .unwrap_or_default()
                .iter()
                .map(|level| (level.element_id, level.elevation_feet))
                .collect();
        let mut record_backed = [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.rooms,
            &mut out.products,
        ];
        resolve_framing_top_levels(&level_elevations, &mut record_backed);
        resolve_base_constraint_levels(&level_elevations, &mut record_backed);
        resolve_base_at_level(&level_elevations, &mut record_backed);
        resolve_remaining_levels(&level_elevations, &mut record_backed);
        // --- System families' names (RE-63); a wall's, roof's or
        // ceiling's needs its type's layers, not read on 2023 ---
        attach_system_family_names(
            rf,
            revit_version,
            &mut [&mut out.walls, &mut out.slabs, &mut out.products],
        );
        // --- Walls', floors' and roofs' layers, for their material layer
        // sets (RE-114) ---
        attach_wall_layers(rf, revit_version, &mut out.walls);
        attach_slab_layers(
            rf,
            revit_version,
            out.slabs
                .iter_mut()
                .chain(out.products.iter_mut())
                .collect(),
        );
        // --- The materials each family type's geometry uses (RE-113). A
        // system type's data holds its layers where a family type's holds
        // its map, and a layer reads as a one-entry map, so the types of
        // system-family elements are left out. ---
        let mut type_materials = crate::partition_type_materials::type_material_names_2023(rf);
        for element in out.walls.iter().chain(&out.slabs).chain(&out.products) {
            let system = element.fields.iter().any(|(name, value)| {
                name == FAMILY_NAME_SOURCE_FIELD
                    && matches!(value, InstanceField::String(source) if source == SYSTEM_FAMILY_SOURCE)
            });
            let type_id = element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
                _ => None,
            });
            if let (true, Some(type_id)) = (system, type_id) {
                type_materials.remove(&type_id);
            }
        }
        for elements in [
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.products,
        ] {
            attach_type_materials(&type_materials, elements);
        }
        return Ok(out);
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
    // Their outlines from the faces of their stored solids (RE-101).
    attach_room_outlines(rf, revit_version, &mut out.rooms);

    // --- Other product categories from element records (RE-33) ---
    out.products = product_instances_from_partition_records(rf, revit_version, &level_ids)?;
    // --- Base constraints of elements naming two Levels (RE-59), then
    // railings by their host and elements by the Level objects they name
    // (RE-60) ---
    let level_elevations: std::collections::BTreeMap<u32, f64> =
        crate::partition_level_records::recover_partition_levels(rf, revit_version)
            .unwrap_or_default()
            .iter()
            .map(|level| (level.element_id, level.elevation_feet))
            .collect();
    let mut record_backed = [
        &mut out.walls,
        &mut out.columns,
        &mut out.doors,
        &mut out.windows,
        &mut out.slabs,
        &mut out.rooms,
        &mut out.products,
    ];
    resolve_base_constraint_levels(&level_elevations, &mut record_backed);
    resolve_base_at_level(&level_elevations, &mut record_backed);
    resolve_hosted_levels(rf, &level_elevations, &mut record_backed);
    resolve_remaining_levels(&level_elevations, &mut record_backed);
    // --- Stair parts under their stairs (#323) ---
    attach_aggregate_wholes(rf, &mut out.products);
    // --- Curtain walls and their panels and mullions (RE-46) ---
    attach_curtain_walls(rf, revit_version, &mut out.walls, &mut out.products);
    // --- Doors and windows in the nearest listed wall that is not a curtain wall (#439) ---
    bind_opening_hosts(&out.walls, &mut out.doors);
    bind_opening_hosts(&out.walls, &mut out.windows);
    // --- Stair and flight riser and tread dimensions (RE-47) ---
    attach_stair_dimensions(rf, revit_version, &mut out.products);
    // --- Stair runs' treads and risers, and their run type (RE-52) ---
    attach_stair_run_bodies(rf, revit_version, &mut out.products);
    // --- Beams along their location lines (RE-49) ---
    attach_beam_axes(rf, revit_version, &mut out.products);
    // --- Pipes as cylinders along their ends (RE-131) ---
    attach_pipe_axes(rf, revit_version, &mut out.products);
    // --- Which element each end of a duct or pipe is joined to (RE-138) ---
    attach_connector_pairs(rf, revit_version, &mut out.products);
    // --- Roof outlines from their sketch lines (RE-50) ---
    attach_roof_profiles(rf, revit_version, &mut out.products);
    // --- Shaft openings cut the outlines within their height (RE-99) ---
    attach_shaft_voids(rf, revit_version, [&mut out.slabs, &mut out.products]);

    // --- The Revit class each record names at +0x4a (RE-76, #154) ---
    if let Ok(classes) = rf.schema_classes() {
        for elements in [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.rooms,
            &mut out.products,
        ] {
            attach_revit_classes(&classes, elements);
        }
    }

    // --- A turned family instance's plan axis (RE-87) ---
    attach_instance_axes(
        rf,
        revit_version,
        [&mut out.doors, &mut out.windows, &mut out.products],
    );
    // --- Curtain mullions and panels turned or tilted off the model's axes,
    // with their three axes (RE-106) ---
    attach_curtain_axes(rf, revit_version, &mut out.products);

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
    // --- Pipes' types, which have no name entry (RE-130) ---
    attach_pipe_type_names(rf, &mut out.products);
    // --- Ducts' and pipes' sizes, and ducts' types (RE-134) ---
    attach_curve_fields(rf, revit_version, &mut out.products);
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
    // --- Curtain panels that are walls (RE-64) ---
    let mut unnamed_panels: Vec<&mut DecodedElement> = out
        .products
        .iter_mut()
        .filter(|element| {
            element.class == "CurtainWallPanel"
                && !element
                    .fields
                    .iter()
                    .any(|(name, _)| name == TYPE_NAME_FIELD)
        })
        .collect();
    attach_panel_wall_types(rf, revit_version, &mut unnamed_panels);
    // --- Model text (RE-67) ---
    attach_model_text_types(rf, revit_version, &mut out.products);
    // --- Wall sweeps (RE-69) ---
    attach_wall_sweep_types(rf, revit_version, &mut out.products);
    // --- Walls' layers and exterior side (RE-53) ---
    attach_wall_layers(rf, revit_version, &mut out.walls);
    // --- A shed roof's slope (RE-56) ---
    attach_roof_slopes(rf, revit_version, &mut out.products);
    // --- Floors', roofs' and ceilings' layers (RE-57) ---
    attach_slab_layers(
        rf,
        revit_version,
        out.slabs
            .iter_mut()
            .chain(out.products.iter_mut())
            .collect(),
    );
    // --- System families' names (RE-63) ---
    attach_system_family_names(
        rf,
        revit_version,
        &mut [&mut out.walls, &mut out.slabs, &mut out.products],
    );
    // --- Stairs and their parts named as Revit names them (RE-65) ---
    attach_stair_names(rf, revit_version, &mut out.products);
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
    // --- Empty curtain panels are left out, as Revit leaves them out (#309) ---
    out.products
        .retain(|product| !is_empty_curtain_panel(product));
    // --- Curtain panels that are walls export as curtain walls (RE-72) ---
    // Last, so every step above still sees them as panels.
    for product in &mut out.products {
        if product.class == "CurtainWallPanel"
            && product.fields.iter().any(|(name, value)| {
                name == CURTAIN_PANEL_WALL_FIELD && matches!(value, InstanceField::Bool(true))
            })
        {
            product.class = CURTAIN_PANEL_WALL_CLASS.into();
        }
    }

    // --- Type text parameters (RE-77, #35) ---
    let type_parameters = crate::partition_type_parameters::type_text_parameters(rf, revit_version);
    if !type_parameters.is_empty() {
        for elements in [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.products,
        ] {
            attach_type_text_parameters(&type_parameters, elements);
        }
    }

    // --- The materials each family type's geometry uses (RE-82, #355) ---
    let type_materials = crate::partition_type_materials::type_material_names(rf, revit_version);
    if !type_materials.is_empty() {
        for elements in [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.products,
        ] {
            attach_type_materials(&type_materials, elements);
        }
    }

    // --- Family instances whose type draws with no material (RE-149, #355) ---
    let unset_types = crate::partition_type_materials::unset_material_types(rf, revit_version);
    if !unset_types.is_empty() {
        for elements in [&mut out.columns, &mut out.products] {
            attach_unnamed_materials(&unset_types, elements);
        }
        attach_joined_wall_materials(&mut out.columns, &out.walls);
    }

    // --- Each window's opening from its type and transform (RE-93, #227) ---
    attach_window_openings(rf, revit_version, &mut out.windows);
    // --- Each door's rough opening from its type (RE-94, #227), after RE-84
    // marks the doors that are openings alone ---
    attach_door_openings(rf, revit_version, &mut out.doors);
    // --- Steel beams' and columns' I sections from their type (RE-103, RE-104) ---
    attach_beam_sections(rf, revit_version, &mut out.products);
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
    let mut names = type_data_names(rf, &header, &wanted);
    // A railing type with no element data keeps its name in its type object
    // (RE-66, measured on Revit 2024 only).
    let railing_types: BTreeSet<u32> = picks
        .iter()
        .filter(|(index, id)| {
            elements[*index].class == "Railing" && !matches!(names.get(id), Some(Some(_)))
        })
        .map(|(_, id)| *id)
        .collect();
    if revit_version == 2024 && !railing_types.is_empty() {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            for (id, name) in
                crate::partition_names::find_railing_type_names(inflated.bytes(), &railing_types)
            {
                match names.get(&id) {
                    Some(Some(held)) if *held != name => {
                        names.insert(id, None);
                    }
                    Some(None) => {}
                    _ => {
                        names.insert(id, Some(name));
                    }
                }
            }
        }
    }
    attach_type_picks(elements, picks, &names);
}

/// Each wanted type's name, read from its serialised data
/// ([`crate::partition_names::find_element_data_names`]); `None` when two
/// copies of the type disagree.
fn type_data_names(
    rf: &mut RevitFile,
    header: &[u8; 10],
    wanted: &BTreeSet<u32>,
) -> std::collections::BTreeMap<u32, Option<String>> {
    let mut names: std::collections::BTreeMap<u32, Option<String>> =
        std::collections::BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in
            crate::partition_names::find_element_data_names(inflated.bytes(), header, wanted)
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
    names
}

/// Set [`TYPE_ID_FIELD`] and [`TYPE_NAME_FIELD`] on each picked element
/// whose type has one agreed name.
fn attach_type_picks(
    elements: &mut [&mut DecodedElement],
    picks: Vec<(usize, u32)>,
    names: &std::collections::BTreeMap<u32, Option<String>>,
) {
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

/// Give wall sweeps their type and family, or Revit's name for one without a
/// type (RE-69).
///
/// A wall sweep's type is a system-family type with no type-definition
/// record of the sweep category, so RE-44's join finds none. Its record
/// names it among other things: its profile (a family type with a name
/// entry, RE-38), its material, the walls it runs along. The type is the
/// one id the record names that has a type object
/// ([`crate::partition_names::find_type_object_ids`]), no name entry, and
/// an element-data name; the sweep is then named `Wall Sweep:<type>:<id>`.
/// A sweep whose record names no such object at all is a sweep of a wall
/// type's own structure, which Revit names by its ElementId alone.
///
/// Measured on Snowdon Towers only (Revit 2024): 249 sweeps take a type and
/// 9 are named by ElementId, all as Revit's export names them.
fn attach_wall_sweep_types(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if revit_version != 2024 {
        return;
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return;
    };
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let mut lists: Vec<(usize, BTreeSet<u32>)> = Vec::new();
    for (index, element) in products.iter().enumerate() {
        if element.class != "WallSweep"
            || has(element, TYPE_NAME_FIELD)
            || has(element, FAMILY_NAME_FIELD)
            || has(element, ELEMENT_NAME_FIELD)
        {
            continue;
        }
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        let named: BTreeSet<u32> = references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| Some(*id) != element.id)
            .collect();
        lists.push((index, named));
    }
    if lists.is_empty() {
        return;
    }
    let entries = rf.element_names();
    let wanted: BTreeSet<u32> = lists
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .filter(|id| !entries.entries.contains_key(id))
        .collect();
    let mut objects: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        objects.extend(crate::partition_names::find_type_object_ids(
            inflated.bytes(),
            &wanted,
        ));
    }
    let names = type_data_names(rf, &header, &objects);
    for (index, ids) in lists {
        let types: Vec<u32> = ids
            .iter()
            .copied()
            .filter(|id| objects.contains(id))
            .collect();
        let element = &mut products[index];
        match types.as_slice() {
            [] => {
                if let Some(id) = element.id {
                    element.fields.push((
                        ELEMENT_NAME_FIELD.into(),
                        InstanceField::String(id.to_string()),
                    ));
                }
            }
            [type_id] => {
                let Some(Some(name)) = names.get(type_id) else {
                    continue;
                };
                element.fields.push((
                    TYPE_ID_FIELD.into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: *type_id,
                    },
                ));
                element
                    .fields
                    .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
                element.fields.push((
                    FAMILY_NAME_FIELD.into(),
                    InstanceField::String("Wall Sweep".into()),
                ));
                element.fields.push((
                    FAMILY_NAME_SOURCE_FIELD.into(),
                    InstanceField::String(SYSTEM_FAMILY_SOURCE.into()),
                ));
            }
            _ => {}
        }
    }
}

/// Give model text its type and family (RE-67).
///
/// A model text element is in the Generic Models category, but it is not a
/// family instance: its reference list names a text type, which has no name
/// entry (RE-38) and no type-definition record of the category. The text
/// type is the one id the list names whose type object carries a font
/// ([`crate::partition_names::find_text_type_names`]), and Revit names the
/// element `Model Text:<type>:<ElementId>`. Measured on Snowdon Towers only
/// (Revit 2024), where it names all 7 model texts as Revit's export does.
fn attach_model_text_types(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if revit_version != 2024 {
        return;
    }
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let mut lists: Vec<(usize, BTreeSet<u32>)> = Vec::new();
    for (index, element) in products.iter().enumerate() {
        if element.class != "GenericModel"
            || has(element, TYPE_NAME_FIELD)
            || has(element, FAMILY_NAME_FIELD)
        {
            continue;
        }
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        let named: BTreeSet<u32> = references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| Some(*id) != element.id)
            .collect();
        lists.push((index, named));
    }
    if lists.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = lists
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let mut names: std::collections::BTreeMap<u32, Option<String>> =
        std::collections::BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in crate::partition_names::find_text_type_names(inflated.bytes(), &wanted) {
            match names.get(&id) {
                Some(Some(held)) if *held != name => {
                    names.insert(id, None);
                }
                Some(None) => {}
                _ => {
                    names.insert(id, Some(name));
                }
            }
        }
    }
    for (index, ids) in lists {
        let text_types: Vec<(u32, &String)> = ids
            .iter()
            .filter_map(|id| match names.get(id) {
                Some(Some(name)) => Some((*id, name)),
                _ => None,
            })
            .collect();
        let [(type_id, name)] = text_types.as_slice() else {
            continue;
        };
        let element = &mut products[index];
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: *type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String((*name).clone()),
        ));
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String("Model Text".into()),
        ));
        element.fields.push((
            FAMILY_NAME_SOURCE_FIELD.into(),
            InstanceField::String(TYPE_KIND_FAMILY_SOURCE.into()),
        ));
    }
}

/// Give a curtain panel that is a wall its wall type (RE-64).
///
/// Revit lets a curtain grid cell hold a basic wall instead of a panel. The
/// element keeps the panel category, but its reference list names a wall
/// type rather than a panel type, so [`attach_system_type_names`] finds no
/// type for it. Here the type is the one wall-type record the list names,
/// taken only when that type has compound layers
/// ([`crate::partition_compound_structure::scan_type_layers`]): a panel's
/// list also names its curtain wall, whose type is a wall type with none.
/// [`attach_system_family_names`] then names its family "Basic Wall".
fn attach_panel_wall_types(
    rf: &mut RevitFile,
    revit_version: u32,
    panels: &mut [&mut DecodedElement],
) {
    use crate::partition_type_records as ptr;
    if panels.is_empty()
        || !ptr::supports_revit_version(revit_version)
        || !crate::partition_compound_structure::COMPOUND_STRUCTURE_SUPPORTED_REVIT_VERSIONS
            .contains(&revit_version)
    {
        return;
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return;
    };
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let wall_types = ptr::type_definition_ids(
        &ptr::scan_type_records(
            rf,
            revit_version,
            crate::partition_element_records::OST_WALLS,
            &declared,
        )
        .unwrap_or_default(),
    );
    let mut picks: Vec<(usize, u32)> = Vec::new();
    for (index, panel) in panels.iter().enumerate() {
        let Some((references, _)) = record_references(rf, panel) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &wall_types) {
            picks.push((index, type_id));
        }
    }
    if picks.is_empty() {
        return;
    }
    let candidates: BTreeSet<u32> = picks.iter().map(|(_, id)| *id).collect();
    let materials: BTreeSet<u32> =
        ptr::scan_type_records(rf, revit_version, ptr::OST_MATERIALS, &declared)
            .unwrap_or_default()
            .iter()
            .map(|record| record.element_id)
            .collect();
    let layered: BTreeSet<u32> = crate::partition_compound_structure::scan_type_layers(
        rf,
        revit_version,
        &candidates,
        &materials,
        &declared,
    )
    .unwrap_or_default()
    .into_iter()
    .filter(|(_, layers)| !layers.is_empty())
    .map(|(id, _)| id)
    .collect();
    picks.retain(|(_, id)| layered.contains(id));
    let names = type_data_names(rf, &header, &layered);
    attach_type_picks(panels, picks, &names);
    // RE-72: Revit exports such a panel as a curtain wall of its own.
    for panel in panels.iter_mut() {
        let wall_type = panel.fields.iter().any(|(name, value)| {
            matches!(value, InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD && layered.contains(id))
        });
        if wall_type {
            panel
                .fields
                .push((CURTAIN_PANEL_WALL_FIELD.into(), InstanceField::Bool(true)));
        }
    }
}

/// The family Revit names its empty curtain panel type after (#309). Revit's
/// IFC4 export leaves every panel of it out: all 24 on Snowdon Towers. The
/// name is Revit's English one, so a file saved in another language keeps
/// its empty panels. The panel type's unset material
/// ([`crate::partition_names::find_panel_type_materials`]) is only a
/// cross-check, since a panel set to "By Category" may store no material
/// either.
pub const EMPTY_PANEL_FAMILY_NAME: &str = "Empty System Panel";

/// Whether `element` is a curtain panel of the [`EMPTY_PANEL_FAMILY_NAME`]
/// family.
fn is_empty_curtain_panel(element: &DecodedElement) -> bool {
    element.class == "CurtainWallPanel"
        && element.fields.iter().any(|(name, value)| {
            matches!(value, InstanceField::String(family) if name == FAMILY_NAME_FIELD && family == EMPTY_PANEL_FAMILY_NAME)
        })
}

/// The curtain panels left out as empty (#309): how many, and the types of
/// theirs whose material is set, which would contradict the name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmptyCurtainPanels {
    /// Placed panels of the [`EMPTY_PANEL_FAMILY_NAME`] family.
    pub count: usize,
    /// Their types whose material is set.
    pub types_with_material: BTreeSet<u32>,
}

/// Count the placed curtain panels of the [`EMPTY_PANEL_FAMILY_NAME`]
/// family, the ones recovery leaves out, through the same name-entry type
/// and family join ([`crate::partition_names::resolve_type`],
/// [`crate::partition_names::resolve_family`]).
pub fn scan_empty_curtain_panels(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<EmptyCurtainPanels> {
    use crate::partition_element_records as per;
    let mut out = EmptyCurtainPanels::default();
    if !per::supports_revit_version(revit_version) {
        return Ok(out);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(out),
    };
    let names = rf.element_names();
    let empty_families: BTreeSet<u32> = names
        .entries
        .iter()
        .filter(|(_, entry)| entry.name == EMPTY_PANEL_FAMILY_NAME)
        .map(|(id, _)| *id)
        .collect();
    if empty_families.is_empty() {
        return Ok(out);
    }
    let records =
        per::scan_category_records(rf, revit_version, per::OST_CURTAIN_WALL_PANELS, &declared)?;
    let options = rf.design_options();
    let mut panels: BTreeSet<u32> = BTreeSet::new();
    let mut types: BTreeSet<u32> = BTreeSet::new();
    for record in records
        .iter()
        .filter(|r| r.is_exported_instance() && !options.excludes(r) && r.has_volume())
    {
        let Some(type_id) = crate::partition_names::resolve_type(
            &names,
            &record.references,
            record.builtin_category,
            record.element_id,
        ) else {
            continue;
        };
        if crate::partition_names::resolve_family(&names, type_id)
            .is_some_and(|family| empty_families.contains(&family))
        {
            panels.insert(record.element_id);
            types.insert(type_id);
        }
    }
    out.count = panels.len();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, material) in
            crate::partition_names::find_panel_type_materials(inflated.bytes(), &types)
        {
            if material.is_some() {
                out.types_with_material.insert(id);
            }
        }
    }
    Ok(out)
}

/// Field marking a curtain panel that holds a basic wall (RE-64, RE-72).
pub const CURTAIN_PANEL_WALL_FIELD: &str = "m_curtain_panel_is_wall";

/// Class of a curtain panel that holds a basic wall. Revit's IFC4 export
/// writes each as an `IfcCurtainWall` with its own body, nested under the
/// curtain wall it is a panel of: all 18 on Snowdon Towers (RE-72).
pub const CURTAIN_PANEL_WALL_CLASS: &str = "CurtainPanelWall";

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
/// Field carrying the class tag an element record holds at `+0x4a` (RE-76).
pub const CLASS_TAG_FIELD: &str = "m_class_tag";
/// Field carrying the name of that class in the file's own schema (RE-76).
pub const REVIT_CLASS_FIELD: &str = "m_revit_class";

/// Name each record-backed element's class (RE-76): the tag its record
/// holds at `+0x4a` is the definition ordinal of a class in the file's own
/// schema, `SWall`, `ArcWall`, `Floor`, `FamilyInstance` and so on. An
/// element whose tag resolves to no class gets no name.
fn attach_revit_classes(classes: &crate::formats::SchemaClasses, elements: &mut [DecodedElement]) {
    for element in elements.iter_mut() {
        let tag = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if name == CLASS_TAG_FIELD => {
                u16::try_from(*value).ok()
            }
            _ => None,
        });
        if let Some(class) = tag.and_then(|tag| classes.by_tag(tag)) {
            element.fields.push((
                REVIT_CLASS_FIELD.into(),
                InstanceField::String(class.name.clone()),
            ));
        }
    }
}

/// Fields holding the three axes of a curtain mullion or panel turned or
/// tilted off the model's axes (RE-106), from its transform: X, then Y,
/// then Z, each as model x, y and z.
pub const CURTAIN_AXES_FIELDS: [&str; 9] = [
    "m_curtain_axis_xx",
    "m_curtain_axis_xy",
    "m_curtain_axis_xz",
    "m_curtain_axis_yx",
    "m_curtain_axis_yy",
    "m_curtain_axis_yz",
    "m_curtain_axis_zx",
    "m_curtain_axis_zy",
    "m_curtain_axis_zz",
];

/// Classes whose turned or tilted instances carry [`CURTAIN_AXES_FIELDS`] (RE-106).
pub const CURTAIN_AXES_CLASSES: [&str; 2] = ["CurtainWallMullion", "CurtainWallPanel"];

/// The three axes [`CURTAIN_AXES_FIELDS`] record, X, Y and Z.
pub fn curtain_axes_from_fields(fields: &[(String, InstanceField)]) -> Option<[[f64; 3]; 3]> {
    let values = CURTAIN_AXES_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    let mut axes = [[0.0; 3]; 3];
    for (index, value) in values.into_iter().enumerate() {
        axes[index / 3][index % 3] = value?;
    }
    Some(axes)
}

/// Give each curtain mullion and panel whose transform (RE-87) turns or
/// tilts it off the model's axes its three axes (RE-106). One on the
/// model's axes gets nothing: its record box is already its body's box.
fn attach_curtain_axes(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    let ids: BTreeSet<u32> = products
        .iter()
        .filter(|element| CURTAIN_AXES_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| element.id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for element in products.iter_mut() {
        let Some(transform) = element.id.and_then(|id| transforms.get(&id)) else {
            continue;
        };
        // On the model's axes: each axis has one component, the other two
        // within INSTANCE_TURN_MIN_RADIANS of zero.
        let on_axes = transform.axes.iter().all(|axis| {
            axis.iter()
                .filter(|v| v.abs() > INSTANCE_TURN_MIN_RADIANS)
                .count()
                == 1
        });
        if on_axes {
            continue;
        }
        let values = transform.axes.iter().flatten().copied();
        for (name, value) in CURTAIN_AXES_FIELDS.iter().zip(values) {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}

/// Fields holding the plan direction of a family instance turned off the
/// model's axes (RE-87), from its transform: its X axis where its Z axis is
/// the model's, or else its first flat axis where one axis is vertical, as
/// for a fixture hosted on a ceiling or a wall (RE-90).
pub const INSTANCE_X_AXIS_FIELDS: [&str; 2] = ["m_instance_x_axis_x", "m_instance_x_axis_y"];
/// Fields holding the plan origin of a turned family instance, model feet,
/// from its transform (RE-87): the point a window's opening in a tapered
/// wall is centred on (RE-89).
pub const INSTANCE_ORIGIN_FIELDS: [&str; 2] = ["m_instance_origin_x", "m_instance_origin_y"];

/// How far off the model's axes a family instance must be turned to be
/// drawn turned (RE-87): 1e-4 radians, 0.001 ft at 10 ft. RE1 Architecture
/// holds 34 instances off them by less than 0.001 degrees, round-off rather
/// than a turn; Snowdon Towers' 628 turned ones are all 0.01 degrees or more
/// off.
pub const INSTANCE_TURN_MIN_RADIANS: f64 = 1e-4;

/// Give each family instance ([`REVIT_CLASS_FIELD`] `FamilyInstance`) with
/// a vertical axis that is turned off the model's axes by at least
/// [`INSTANCE_TURN_MIN_RADIANS`] its plan direction
/// ([`crate::partition_instance_transforms::InstanceTransform::plan_axis`],
/// RE-87, RE-90). Any other keeps its record box as it is.
fn attach_instance_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 3],
) {
    let is_instance = |element: &DecodedElement| {
        element.fields.iter().any(|(name, value)| {
            name == REVIT_CLASS_FIELD
                && matches!(value, InstanceField::String(class) if class == "FamilyInstance")
        })
    };
    let ids: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter())
        .filter(|element| is_instance(element))
        .filter_map(|element| element.id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some((transform, [x, y])) = element
            .id
            .and_then(|id| transforms.get(&id))
            .and_then(|transform| Some((transform, transform.plan_axis()?)))
        else {
            continue;
        };
        let turn = y.abs().atan2(x.abs());
        if turn.min(std::f64::consts::FRAC_PI_2 - turn) < INSTANCE_TURN_MIN_RADIANS {
            continue;
        }
        let [ox, oy, _] = transform.origin;
        for (name, value) in INSTANCE_X_AXIS_FIELDS
            .iter()
            .zip([x, y])
            .chain(INSTANCE_ORIGIN_FIELDS.iter().zip([ox, oy]))
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}

/// Fields holding a window's or door's opening as Revit's export cuts it
/// (RE-93, RE-94): the plan point it is centred on (the element's origin),
/// its plan direction (the element's X axis), its base elevation (the
/// origin's, plus a window type's Default Sill Height), and its width and
/// height, feet: a window type's Width and Height, a door type's Rough
/// Width and Rough Height.
pub const FILLER_OPENING_FIELDS: [&str; 7] = [
    "m_filler_opening_x",
    "m_filler_opening_y",
    "m_filler_opening_axis_x",
    "m_filler_opening_axis_y",
    "m_filler_opening_base",
    "m_filler_opening_width",
    "m_filler_opening_height",
];

/// A door's or window's opening as [`FILLER_OPENING_FIELDS`] records it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillerOpening {
    /// Plan point the opening is centred on, model feet.
    pub centre: [f64; 2],
    /// Unit plan direction of its width.
    pub axis: [f64; 2],
    /// Elevation of its bottom, model feet.
    pub base_feet: f64,
    /// Feet.
    pub width_feet: f64,
    /// Feet.
    pub height_feet: f64,
}

/// Give each upright window whose type's Width, Height and Default Sill
/// Height are read its opening (RE-93,
/// [`crate::partition_type_parameters::type_window_openings`]).
fn attach_window_openings(rf: &mut RevitFile, revit_version: u32, windows: &mut [DecodedElement]) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = windows.iter().filter_map(type_of).collect();
    let openings =
        crate::partition_type_parameters::type_window_openings(rf, revit_version, &types);
    if openings.is_empty() {
        return;
    }
    let ids: BTreeSet<u32> = windows
        .iter()
        .filter(|element| type_of(element).is_some_and(|id| openings.contains_key(&id)))
        .filter_map(|element| element.id)
        .collect();
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for element in windows.iter_mut() {
        let Some((opening, transform)) = type_of(element)
            .and_then(|id| openings.get(&id))
            .zip(element.id.and_then(|id| transforms.get(&id)))
        else {
            continue;
        };
        push_filler_opening(
            element,
            transform,
            opening.default_sill_feet,
            opening.width_feet,
            opening.height_feet,
        );
    }
}

/// How close a door's record box height must be to its type's Rough Height
/// for the door to take its rough opening (RE-94), feet.
pub const DOOR_ROUGH_HEIGHT_TOLERANCE_FEET: f64 = 1e-3;

/// Give each upright door whose record box is its type's Rough Height tall
/// its type's rough opening, from its origin up (RE-94,
/// [`crate::partition_type_parameters::type_door_openings`]). On Snowdon
/// Towers every door Revit cuts at its rough opening has a body that tall,
/// and every other door a body its frame's height instead.
/// Fields carrying a beam's I section from its type (RE-103): flange
/// width, overall depth, web thickness and flange thickness, feet.
pub const BEAM_I_SECTION_FIELDS: [&str; 4] = [
    "m_i_section_width",
    "m_i_section_depth",
    "m_i_section_web",
    "m_i_section_flange",
];

/// The I section [`BEAM_I_SECTION_FIELDS`] record.
pub fn beam_i_section_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_type_parameters::ISection> {
    let [width, depth, web, flange] = BEAM_I_SECTION_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some(crate::partition_type_parameters::ISection {
        width_feet: width?,
        depth_feet: depth?,
        web_feet: web?,
        flange_feet: flange?,
    })
}

/// Fields carrying the plan direction of an I-section structural column's
/// X axis, from its transform (RE-104): its flanges run along it.
pub const COLUMN_I_AXIS_FIELDS: [&str; 2] = ["m_i_section_axis_x", "m_i_section_axis_y"];

/// Classes whose type's I section is read (RE-103, RE-104).
const I_SECTION_CLASSES: [&str; 2] = ["StructuralFraming", "StructuralColumn"];

/// Give each structural-framing element and structural column whose type is
/// an I section ([`crate::partition_type_parameters::type_i_sections`]) that
/// section (RE-103, RE-104), and each such upright column its X axis in plan
/// from its transform. The exporter draws a beam's section only where its
/// body along its line holds it, and a column's only where the section
/// turned to that axis fills its record box.
fn attach_beam_sections(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = products
        .iter()
        .filter(|element| I_SECTION_CLASSES.contains(&element.class.as_str()))
        .filter_map(type_of)
        .collect();
    let sections = crate::partition_type_parameters::type_i_sections(rf, revit_version, &types);
    let columns: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "StructuralColumn")
        .filter(|element| type_of(element).is_some_and(|id| sections.contains_key(&id)))
        .filter_map(|element| element.id)
        .collect();
    let transforms = if columns.is_empty() {
        BTreeMap::new()
    } else {
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &columns)
            .unwrap_or_default()
    };
    for element in products
        .iter_mut()
        .filter(|element| I_SECTION_CLASSES.contains(&element.class.as_str()))
    {
        let Some(section) = type_of(element).and_then(|id| sections.get(&id)).copied() else {
            continue;
        };
        let values = [
            section.width_feet,
            section.depth_feet,
            section.web_feet,
            section.flange_feet,
        ];
        for (name, value) in BEAM_I_SECTION_FIELDS.iter().zip(values) {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        let axis = element
            .id
            .and_then(|id| transforms.get(&id))
            .filter(|transform| transform.is_upright())
            .and_then(|transform| transform.plan_axis());
        if let Some(axis) = axis {
            for (name, value) in COLUMN_I_AXIS_FIELDS.iter().zip(axis) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
    }
}

fn attach_door_openings(rf: &mut RevitFile, revit_version: u32, doors: &mut [DecodedElement]) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let box_height = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == "m_bboxHeight" => Some(*value),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = doors.iter().filter_map(type_of).collect();
    let openings = crate::partition_type_parameters::type_door_openings(rf, revit_version, &types);
    // A door whose type draws no geometry (RE-84) is an opening alone, which
    // Revit does not cut at its rough size: RE1 Architecture's cased
    // opening is 1.0 m wide against a Rough Width of 1.1 m.
    let rough = |element: &DecodedElement| {
        if element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_WITHOUT_GEOMETRY_FIELD)
        {
            return None;
        }
        let opening = type_of(element).and_then(|id| openings.get(&id))?;
        let height = box_height(element)?;
        ((height - opening.rough_height_feet).abs() <= DOOR_ROUGH_HEIGHT_TOLERANCE_FEET)
            .then_some(*opening)
    };
    let ids: BTreeSet<u32> = doors
        .iter()
        .filter(|element| rough(element).is_some())
        .filter_map(|element| element.id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for element in doors.iter_mut() {
        let Some((opening, transform)) =
            rough(element).zip(element.id.and_then(|id| transforms.get(&id)))
        else {
            continue;
        };
        push_filler_opening(
            element,
            transform,
            0.0,
            opening.rough_width_feet,
            opening.rough_height_feet,
        );
    }
}

/// Record on an upright door or window the opening centred on its origin
/// along its X axis, `base` above the origin ([`FILLER_OPENING_FIELDS`]).
fn push_filler_opening(
    element: &mut DecodedElement,
    transform: &crate::partition_instance_transforms::InstanceTransform,
    base: f64,
    width: f64,
    height: f64,
) {
    if !transform.is_upright() {
        return;
    }
    let Some([ax, ay]) = transform.plan_axis() else {
        return;
    };
    let [ox, oy, oz] = transform.origin;
    let values = [ox, oy, ax, ay, oz + base, width, height];
    for (name, value) in FILLER_OPENING_FIELDS.iter().zip(values) {
        element
            .fields
            .push(((*name).into(), InstanceField::Float { value, size: 8 }));
    }
}

/// The opening [`FILLER_OPENING_FIELDS`] record.
pub fn filler_opening_from_fields(fields: &[(String, InstanceField)]) -> Option<FillerOpening> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y, ax, ay, base, width, height] = FILLER_OPENING_FIELDS.map(float);
    Some(FillerOpening {
        centre: [x?, y?],
        axis: [ax?, ay?],
        base_feet: base?,
        width_feet: width?,
        height_feet: height?,
    })
}

/// The plan origin [`INSTANCE_ORIGIN_FIELDS`] record.
pub fn instance_origin_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = INSTANCE_ORIGIN_FIELDS.map(float);
    Some([x?, y?])
}

/// The plan X axis [`INSTANCE_X_AXIS_FIELDS`] record.
pub fn instance_x_axis_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = INSTANCE_X_AXIS_FIELDS.map(float);
    Some([x?, y?])
}

/// Prefix of the fields carrying an element's type text parameters (RE-77):
/// `m_type_parameter:Type Mark` and so on.
pub const TYPE_PARAMETER_FIELD_PREFIX: &str = "m_type_parameter:";

/// Give each element the text parameters of its type ([`TYPE_ID_FIELD`],
/// RE-38 and #322), as Revit shows a type's parameters on its instances.
/// Field carrying, once per material, the name of a material an element's
/// type draws its geometry in (RE-82), in the order the type names them.
pub const TYPE_MATERIAL_FIELD: &str = "m_type_material";
/// Field set on an element whose type has a parameter value block with no
/// geometry-material map: a type that draws no geometry (RE-84). Revit's
/// export writes such a door or window as its opening alone.
pub const TYPE_WITHOUT_GEOMETRY_FIELD: &str = "m_type_without_geometry";

/// Give each element the materials its type's geometry uses (RE-82).
fn attach_type_materials(materials: &BTreeMap<u32, Vec<String>>, elements: &mut [DecodedElement]) {
    for element in elements.iter_mut() {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let Some(names) = type_id.and_then(|id| materials.get(&id)) else {
            continue;
        };
        if names.is_empty() {
            element.fields.push((
                TYPE_WITHOUT_GEOMETRY_FIELD.into(),
                InstanceField::Bool(true),
            ));
        }
        for name in names {
            element.fields.push((
                TYPE_MATERIAL_FIELD.into(),
                InstanceField::String(name.clone()),
            ));
        }
    }
}

/// Give each element whose type draws its geometry with no material of its
/// own, and that has no type material yet, the material Revit's export
/// writes for such geometry, [`crate::partition_type_materials::UNNAMED_MATERIAL`]
/// (RE-149). Doors, windows, walls and slabs are left alone: a door or
/// window type without a material map is written as its opening (RE-84).
fn attach_unnamed_materials(unset_types: &BTreeSet<u32>, elements: &mut [DecodedElement]) {
    for element in elements.iter_mut() {
        if element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_MATERIAL_FIELD)
        {
            continue;
        }
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        if type_id.is_some_and(|id| unset_types.contains(&id)) {
            // The type draws geometry, so it is not a type without
            // geometry (RE-84), which `attach_type_materials` took its empty
            // map to mean; a hosted instance would otherwise be written as
            // its opening alone (reported by Cursor Bugbot on #580).
            element
                .fields
                .retain(|(name, _)| name != TYPE_WITHOUT_GEOMETRY_FIELD);
            element.fields.push((
                TYPE_MATERIAL_FIELD.into(),
                InstanceField::String(crate::partition_type_materials::UNNAMED_MATERIAL.into()),
            ));
        }
    }
}

/// A column whose type draws with no material and that walls are joined to
/// takes the joined walls' material (#355): Revit's export gives the 149
/// columns of `2024_Core_Interior.rvt` that walls are joined to "Default
/// Wall", the material of those walls, and `<Unnamed>` to the 107 no wall
/// joins. Only when every joined wall's layers name the same one set of
/// materials; otherwise the column keeps `<Unnamed>`.
fn attach_joined_wall_materials(columns: &mut [DecodedElement], walls: &[DecodedElement]) {
    use crate::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD;
    use crate::partition_type_materials::UNNAMED_MATERIAL;

    let wall_materials: BTreeMap<u32, BTreeSet<String>> = walls
        .iter()
        .filter_map(|wall| {
            let layers = element_layers_from_fields(&wall.fields)?;
            let names: Option<BTreeSet<String>> =
                layers.layers.iter().map(|band| band.name.clone()).collect();
            Some((wall.id?, names.filter(|n| !n.is_empty())?))
        })
        .collect();
    for column in columns.iter_mut() {
        let unnamed_only = column
            .fields
            .iter()
            .filter(|(name, _)| name == TYPE_MATERIAL_FIELD)
            .all(|(_, value)| matches!(value, InstanceField::String(s) if s == UNNAMED_MATERIAL));
        let has_material = column
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_MATERIAL_FIELD);
        if !(unnamed_only && has_material) {
            continue;
        }
        let joined: Vec<u32> = column
            .fields
            .iter()
            .filter_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == COLUMN_JOINED_WALL_FIELD => {
                    Some(*id)
                }
                _ => None,
            })
            .collect();
        let sets: Vec<&BTreeSet<String>> = joined
            .iter()
            .filter_map(|id| wall_materials.get(id))
            .collect();
        let Some(first) = sets.first() else {
            continue;
        };
        if sets.len() != joined.len() || sets.iter().any(|set| set != first) {
            continue;
        }
        let names = (*first).clone();
        column
            .fields
            .retain(|(name, _)| name != TYPE_MATERIAL_FIELD);
        for name in names {
            column
                .fields
                .push((TYPE_MATERIAL_FIELD.into(), InstanceField::String(name)));
        }
    }
}

fn attach_type_text_parameters(
    parameters: &BTreeMap<u32, BTreeMap<&'static str, String>>,
    elements: &mut [DecodedElement],
) {
    for element in elements.iter_mut() {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let Some(values) = type_id.and_then(|id| parameters.get(&id)) else {
            continue;
        };
        for (name, value) in values {
            element.fields.push((
                format!("{TYPE_PARAMETER_FIELD_PREFIX}{name}"),
                InstanceField::String(value.clone()),
            ));
        }
    }
}

/// Give each pipe without a type the one id in its record's reference
/// list that carries a pipe or duct type name
/// ([`crate::partition_names::find_mep_curve_type_names`], RE-130). Pipe
/// and duct types have no name entry, so
/// [`crate::partition_names::resolve_type`] finds none for them. Ducts are
/// left alone: their system family follows their type's shape, which is not
/// read.
fn attach_pipe_type_names(rf: &mut RevitFile, elements: &mut [DecodedElement]) {
    let has_type = |element: &DecodedElement| {
        element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_NAME_FIELD)
    };
    if !elements
        .iter()
        .any(|element| element.class == "Pipe" && !has_type(element))
    {
        return;
    }
    let mut pending: Vec<(usize, BTreeSet<u32>)> = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if element.class != "Pipe" || has_type(element) {
            continue;
        }
        let (Some(own), Some((references, _))) = (element.id, record_references(rf, element))
        else {
            continue;
        };
        // The list's first slot is a constant 3, not a reference (#228); the
        // name pattern can match after it by chance.
        let references: BTreeSet<u32> = references
            .iter()
            .skip(1)
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|&id| id != own)
            .collect();
        pending.push((index, references));
    }
    if pending.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = pending
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in
            crate::partition_names::find_mep_curve_type_names(inflated.bytes(), &wanted)
        {
            match names.get_mut(&id) {
                None => {
                    names.insert(id, Some(name));
                }
                Some(held) => {
                    if held.as_deref() != Some(name.as_str()) {
                        *held = None;
                    }
                }
            }
        }
    }
    for (index, references) in pending {
        let mut typed = references
            .iter()
            .filter_map(|id| Some((*id, names.get(id)?.clone()?)));
        let (Some((type_id, type_name)), None) = (typed.next(), typed.next()) else {
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
            .push((TYPE_NAME_FIELD.into(), InstanceField::String(type_name)));
    }
}

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
/// stays a standalone element here; [`attach_stair_names`] joins one that
/// names several when exactly one of them has a type listing the part's
/// type (RE-65).
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

/// Field holding an element's IFC `Name` where Revit's export does not name
/// it `Family:Type:ElementId`: a stair and its parts (RE-65).
pub const ELEMENT_NAME_FIELD: &str = "m_element_name";

/// Value of [`FAMILY_NAME_SOURCE_FIELD`] for a stair component's family,
/// read from its type's construction flag (RE-65).
pub const TYPE_KIND_FAMILY_SOURCE: &str = "system_family_by_type_kind";

/// The word Revit's export puts before a stair part's number (RE-65).
fn stair_part_word(class: &str) -> Option<&'static str> {
    match class {
        "StairsRun" => Some("Run"),
        "StairsLanding" => Some("Landing"),
        "StairsStringer" => Some("Stringer"),
        _ => None,
    }
}

/// Name stairs and their parts as Revit's export names them (RE-65).
///
/// - A stair's system family is its type's construction (Assembled or
///   Cast-In-Place Stair), and Revit names the stair `Family:Stair:ElementId`.
/// - A run's family is Monolithic or Non-Monolithic Run, by its run type's
///   monolithic flag (RE-52); a landing's is Non-Monolithic Landing; a
///   stringer's or carriage's is Stringer or Carriage
///   ([`crate::partition_stairs::component_type_at`]). With the type's name
///   these give the part's `ObjectType`.
/// - A part is named after its stair, `<stair> Run 2`, numbered in ElementId
///   order among every record of its category that names the stair,
///   exported or not.
///
/// A stair type lists its run, landing and support types
/// ([`crate::partition_stairs::STAIR_TYPE_COMPONENTS_OFFSET`]). When a
/// part's reference list names several types of its category, or several
/// stairs, the one pair its stair's type lists decides both, and the part
/// joins that stair (#323).
///
/// Measured on Snowdon Towers only (Revit 2024): the other oracles have no
/// stairs.
fn attach_stair_names(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_element_records as per;
    use crate::partition_stairs as ps;
    use crate::partition_type_records as ptr;
    use std::collections::BTreeMap;
    if !ps::supports_revit_version(revit_version) || !ptr::supports_revit_version(revit_version) {
        return;
    }
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let id_field = |element: &DecodedElement, field: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == field => Some(*id),
            _ => None,
        })
    };
    if !products.iter().any(|element| element.class == "Stair") {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_ids = |rf: &mut RevitFile, category: i64| {
        ptr::type_definition_ids(
            &ptr::scan_type_records(rf, revit_version, category, &declared).unwrap_or_default(),
        )
    };
    let named = |references: &[u64], set: &BTreeSet<u32>| -> BTreeSet<u32> {
        references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| set.contains(id))
            .collect()
    };

    // Stairs: their type, family and name.
    let stair_type_ids = type_ids(rf, per::OST_STAIRS);
    let mut stair_type_of: BTreeMap<u32, u32> = BTreeMap::new();
    for element in products.iter() {
        let (Some(id), "Stair") = (element.id, element.class.as_str()) else {
            continue;
        };
        if let Some((references, _)) = record_references(rf, element) {
            if let Some(type_id) = ptr::unique_type_reference(&references, &stair_type_ids) {
                stair_type_of.insert(id, type_id);
            }
        }
    }
    let stair_types = ps::scan_component_types(
        rf,
        revit_version,
        ps::ComponentKind::Stair,
        &stair_type_of.values().copied().collect(),
    )
    .unwrap_or_default();
    let stair_ids: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Stair")
        .filter_map(|element| element.id)
        .collect();
    let components_of = |stair: u32| {
        stair_type_of
            .get(&stair)
            .and_then(|type_id| stair_types.get(type_id))
            .map(|found| &found.components)
    };

    // Parts: their type, and their stair where the list names several.
    let part_kinds = [
        ("StairsRun", per::OST_STAIRS_RUNS, None),
        (
            "StairsLanding",
            per::OST_STAIRS_LANDINGS,
            Some(ps::ComponentKind::Landing),
        ),
        (
            "StairsStringer",
            per::OST_STAIRS_STRINGER_CARRIAGE,
            Some(ps::ComponentKind::Support),
        ),
    ];
    let mut part_type: BTreeMap<usize, u32> = BTreeMap::new();
    let mut joins: Vec<(usize, u32)> = Vec::new();
    for (class, category, _) in part_kinds {
        let own_types = type_ids(rf, category);
        for (index, element) in products.iter().enumerate() {
            if element.class != class {
                continue;
            }
            let Some((references, _)) = record_references(rf, element) else {
                continue;
            };
            let candidates = match id_field(element, TYPE_ID_FIELD) {
                Some(id) => BTreeSet::from([id]),
                None => named(&references, &own_types),
            };
            let stairs: Vec<u32> = match id_field(element, AGGREGATE_WHOLE_FIELD) {
                Some(stair) => vec![stair],
                None => named(&references, &stair_ids)
                    .into_iter()
                    .filter(|id| Some(*id) != element.id)
                    .collect(),
            };
            let pairs: Vec<(u32, u32)> = stairs
                .iter()
                .flat_map(|&stair| candidates.iter().map(move |&type_id| (stair, type_id)))
                .filter(|(stair, type_id)| {
                    components_of(*stair).is_some_and(|listed| listed.contains(type_id))
                })
                .collect();
            let chosen_type = match (candidates.len(), pairs.as_slice()) {
                (1, _) => candidates.iter().next().copied(),
                (_, [(_, type_id)]) => Some(*type_id),
                _ => None,
            };
            if let Some(type_id) = chosen_type {
                part_type.insert(index, type_id);
            }
            if let ([_, _, ..], [(stair, _)]) = (stairs.as_slice(), pairs.as_slice()) {
                joins.push((index, *stair));
            }
        }
    }
    for (index, stair) in joins {
        products[index].fields.push((
            AGGREGATE_WHOLE_FIELD.into(),
            InstanceField::ElementId { tag: 0, id: stair },
        ));
    }

    // Families and type names.
    let mut found: BTreeMap<usize, (Option<&'static str>, Option<String>)> = BTreeMap::new();
    for (index, element) in products.iter().enumerate() {
        if let (Some(id), "Stair") = (element.id, element.class.as_str()) {
            if let Some(stair_type) = stair_type_of.get(&id).and_then(|t| stair_types.get(t)) {
                found.insert(index, (stair_type.family, Some(stair_type.name.clone())));
            }
        }
    }
    for (class, _, kind) in part_kinds {
        let ids: BTreeSet<u32> = part_type
            .iter()
            .filter(|(index, _)| products[**index].class == class)
            .map(|(_, id)| *id)
            .collect();
        let of_class = part_type
            .iter()
            .filter(|(index, _)| products[**index].class == class);
        match kind {
            Some(kind) => {
                let types =
                    ps::scan_component_types(rf, revit_version, kind, &ids).unwrap_or_default();
                for (&index, type_id) in of_class {
                    if let Some(read) = types.get(type_id) {
                        found.insert(index, (read.family, Some(read.name.clone())));
                    }
                }
            }
            None => {
                let types = ps::scan_run_types(rf, revit_version, &ids).unwrap_or_default();
                for (&index, type_id) in of_class {
                    if let Some(read) = types.get(type_id) {
                        let family = if read.monolithic {
                            "Monolithic Run"
                        } else {
                            "Non-Monolithic Run"
                        };
                        found.insert(index, (Some(family), read.name.clone()));
                    }
                }
            }
        }
    }
    for (index, (family, name)) in found {
        let element = &mut products[index];
        if has(element, FAMILY_NAME_FIELD) {
            continue;
        }
        if let (Some(name), false) = (&name, has(element, TYPE_NAME_FIELD)) {
            let type_id = part_type
                .get(&index)
                .copied()
                .or_else(|| element.id.and_then(|id| stair_type_of.get(&id).copied()));
            if let Some(type_id) = type_id {
                element.fields.push((
                    TYPE_ID_FIELD.into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: type_id,
                    },
                ));
            }
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        if let (Some(family), true) = (family, has(element, TYPE_NAME_FIELD)) {
            element.fields.push((
                FAMILY_NAME_FIELD.into(),
                InstanceField::String(family.into()),
            ));
            element.fields.push((
                FAMILY_NAME_SOURCE_FIELD.into(),
                InstanceField::String(TYPE_KIND_FAMILY_SOURCE.into()),
            ));
        }
    }

    // The stairs' names, then each part's number among the records of its
    // category that name its stair.
    let mut stair_names: BTreeMap<u32, String> = BTreeMap::new();
    for element in products.iter_mut() {
        let (Some(id), "Stair") = (element.id, element.class.as_str()) else {
            continue;
        };
        let Some(family) = stair_type_of
            .get(&id)
            .and_then(|type_id| stair_types.get(type_id))
            .and_then(|stair_type| stair_type.family)
        else {
            continue;
        };
        let name = format!("{family}:Stair:{id}");
        element.fields.push((
            ELEMENT_NAME_FIELD.into(),
            InstanceField::String(name.clone()),
        ));
        stair_names.insert(id, name);
    }
    if stair_names.is_empty() {
        return;
    }
    let Some(marker) = per::bbox_marker(revit_version) else {
        return;
    };
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut members: BTreeMap<(u32, &'static str), BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        let categories: Vec<i64> = part_kinds
            .iter()
            .map(|(_, category, _)| *category)
            .collect();
        let found = per::find_categories_records_assigned(
            &stream,
            inflated.bytes(),
            &categories,
            &declared,
            &marker,
            ids,
        );
        for ((class, _, _), records) in part_kinds.iter().zip(found) {
            let Some(word) = stair_part_word(class) else {
                continue;
            };
            for record in records {
                for stair in named(&record.references, &stair_ids) {
                    if stair_names.contains_key(&stair) {
                        members
                            .entry((stair, word))
                            .or_default()
                            .insert(record.element_id);
                    }
                }
            }
        }
    }
    for element in products.iter_mut() {
        let (Some(id), Some(word)) = (element.id, stair_part_word(&element.class)) else {
            continue;
        };
        let Some(stair) = id_field(element, AGGREGATE_WHOLE_FIELD) else {
            continue;
        };
        let (Some(stair_name), Some(ids)) = (stair_names.get(&stair), members.get(&(stair, word)))
        else {
            continue;
        };
        let Some(rank) = ids.iter().position(|member| *member == id) else {
            continue;
        };
        element.fields.push((
            ELEMENT_NAME_FIELD.into(),
            InstanceField::String(format!("{stair_name} {word} {}", rank + 1)),
        ));
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
/// Fields holding a curved wall's location arc (RE-75), in plan: its centre
/// `x` and `y`, radius, start and end angle (radians), and the `x` and `y`
/// of its unit X and Y axes. The arc runs from
/// `centre + radius · (cos a · X + sin a · Y)` at the start angle to the end
/// angle.
pub const WALL_ARC_FIELDS: [&str; 9] = [
    "m_wall_arc_centre_x",
    "m_wall_arc_centre_y",
    "m_wall_arc_radius",
    "m_wall_arc_start_angle",
    "m_wall_arc_end_angle",
    "m_wall_arc_x_axis_x",
    "m_wall_arc_x_axis_y",
    "m_wall_arc_y_axis_x",
    "m_wall_arc_y_axis_y",
];
/// Field holding a wall's thickness, its type's layers summed (RE-54).
pub const WALL_TYPE_THICKNESS_FIELD: &str = "m_wall_type_thickness";
/// Field holding how far a tapered wall's exterior face leans from
/// vertical, radians, wider at the base (RE-86). Its line and
/// [`WALL_TYPE_THICKNESS_FIELD`] give the body at the wall's top.
pub const WALL_EXTERIOR_FACE_ANGLE_FIELD: &str = "m_wall_exterior_face_angle";
/// Fields holding how far past the start and the end of its centreline a
/// wall's body reaches at a butt join, feet, negative where it stops short
/// (RE-70), or at a T joint (RE-73). Absent at an end neither decides.
pub const WALL_JOIN_REACH_FIELDS: [&str; 2] = ["m_wall_join_start_reach", "m_wall_join_end_reach"];
/// Fields holding the one wall a wall butt-joins at its start and at its
/// end, where the join lists decide which of the two runs through (RE-70),
/// or the wall it stops against at a T joint (RE-73).
pub const WALL_JOIN_PARTNER_FIELDS: [&str; 2] = ["m_wall_join_start_wall", "m_wall_join_end_wall"];
/// Fields holding how far past the start and the end of its centreline
/// each of a wall's layers reaches at a layered butt join (RE-71) or T
/// joint (RE-73), exterior first, feet.
pub const WALL_JOIN_LAYER_REACH_FIELDS: [&str; 2] = [
    "m_wall_join_start_layer_reaches",
    "m_wall_join_end_layer_reaches",
];
/// Fields holding, at an angled join, how far past the start and the end of
/// its centreline each of a wall's layers reaches along its exterior-side
/// and its interior-side edge, exterior first, two values per layer, feet
/// (RE-74).
pub const WALL_JOIN_LAYER_EDGE_REACH_FIELDS: [&str; 2] = [
    "m_wall_join_start_layer_edge_reaches",
    "m_wall_join_end_layer_edge_reaches",
];
/// Fields holding whether a wall runs through that join (true) or stops at
/// it (false) (RE-70).
pub const WALL_JOIN_THROUGH_FIELDS: [&str; 2] =
    ["m_wall_join_start_through", "m_wall_join_end_through"];

/// A wall's centreline and its type's thickness (RE-54).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallAxis {
    /// Plan start, model feet.
    pub start: [f64; 2],
    /// Plan end, model feet.
    pub end: [f64; 2],
    /// The type's layers summed, feet.
    pub thickness_feet: f64,
    /// How far past its start and its end the body reaches at a butt join,
    /// where the join lists decide it (RE-70).
    pub join_reach_feet: [Option<f64>; 2],
}

/// The centreline and thickness [`WALL_AXIS_START_FIELDS`],
/// [`WALL_AXIS_END_FIELDS`] and [`WALL_TYPE_THICKNESS_FIELD`] record, with
/// the [`WALL_JOIN_REACH_FIELDS`] present.
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
        join_reach_feet: WALL_JOIN_REACH_FIELDS.map(float),
    })
}

/// A curved wall's location arc and its type's thickness (RE-75).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallArc {
    /// Plan centre, model feet.
    pub centre: [f64; 2],
    /// Radius, feet.
    pub radius: f64,
    /// Start and end angle, radians.
    pub angles: [f64; 2],
    /// Plan unit X axis.
    pub x_axis: [f64; 2],
    /// Plan unit Y axis.
    pub y_axis: [f64; 2],
    /// The type's layers summed, feet.
    pub thickness_feet: f64,
}

impl WallArc {
    /// The plan point `offset` feet out from the arc (away from its centre)
    /// at `angle`.
    pub fn point(&self, angle: f64, offset: f64) -> [f64; 2] {
        let (sin, cos) = angle.sin_cos();
        let radius = self.radius + offset;
        [0, 1].map(|axis| {
            self.centre[axis] + radius * (cos * self.x_axis[axis] + sin * self.y_axis[axis])
        })
    }
}

/// A tapered wall's exterior face (RE-86).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallTaper {
    /// How far the exterior face leans from vertical, radians, wider at
    /// the base.
    pub angle_radians: f64,
    /// Plan unit vector towards the exterior face.
    pub exterior: [f64; 2],
}

/// Whether the wall carries [`WALL_EXTERIOR_FACE_ANGLE_FIELD`], so its line
/// alone does not give its body.
pub fn wall_is_tapered(fields: &[(String, InstanceField)]) -> bool {
    fields
        .iter()
        .any(|(name, _)| name == WALL_EXTERIOR_FACE_ANGLE_FIELD)
}

/// The [`WALL_EXTERIOR_FACE_ANGLE_FIELD`] and [`WALL_EXTERIOR_FIELDS`]
/// record; `None` without either.
pub fn wall_taper_from_fields(fields: &[(String, InstanceField)]) -> Option<WallTaper> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = WALL_EXTERIOR_FIELDS.map(float);
    Some(WallTaper {
        angle_radians: float(WALL_EXTERIOR_FACE_ANGLE_FIELD)?,
        exterior: [x?, y?],
    })
}

/// The arc [`WALL_ARC_FIELDS`] and [`WALL_TYPE_THICKNESS_FIELD`] record.
pub fn wall_arc_from_fields(fields: &[(String, InstanceField)]) -> Option<WallArc> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [cx, cy, radius, start, end, xx, xy, yx, yy] = WALL_ARC_FIELDS.map(float);
    Some(WallArc {
        centre: [cx?, cy?],
        radius: radius?,
        angles: [start?, end?],
        x_axis: [xx?, xy?],
        y_axis: [yx?, yy?],
        thickness_feet: float(WALL_TYPE_THICKNESS_FIELD)?,
    })
}

/// Where each layer of a wall ends at its layered butt joins (RE-71).
#[derive(Debug, Clone, PartialEq)]
pub struct WallLayerEnds {
    /// Plan unit vector towards the wall's exterior face.
    pub exterior: [f64; 2],
    /// The layers' widths, exterior first, feet.
    pub widths: Vec<f64>,
    /// How far past its start and its end each layer reaches along its
    /// exterior-side and its interior-side edge, exterior first, where a
    /// layered or angled join decides it. The two are equal where the end
    /// is square.
    pub reach_feet: [Option<Vec<[f64; 2]>>; 2],
}

/// The [`WALL_JOIN_LAYER_EDGE_REACH_FIELDS`] or
/// [`WALL_JOIN_LAYER_REACH_FIELDS`] a wall carries, with its layers' widths
/// and exterior side. `None` without a layered or angled join at either
/// end, or where a reach list does not match the layers.
pub fn wall_layer_ends_from_fields(fields: &[(String, InstanceField)]) -> Option<WallLayerEnds> {
    let floats = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == wanted => items
                .iter()
                .map(|item| match item {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
                .collect::<Option<Vec<f64>>>(),
            _ => None,
        })
    };
    let square = WALL_JOIN_LAYER_REACH_FIELDS.map(floats);
    let slanted = WALL_JOIN_LAYER_EDGE_REACH_FIELDS.map(floats);
    if square.iter().chain(&slanted).all(Option::is_none) {
        return None;
    }
    let widths: Vec<f64> = fields
        .iter()
        .find_map(|(name, value)| {
            (name == WALL_LAYERS_FIELD).then(|| layer_bands_from_field(value))
        })
        .flatten()?
        .iter()
        .map(|band| band.width_feet)
        .collect();
    if square
        .iter()
        .flatten()
        .any(|reach| reach.len() != widths.len())
        || slanted
            .iter()
            .flatten()
            .any(|reach| reach.len() != 2 * widths.len())
    {
        return None;
    }
    let [start, end] = [0, 1].map(|slot| match (&slanted[slot], &square[slot]) {
        (Some(edges), _) => Some(edges.chunks(2).map(|pair| [pair[0], pair[1]]).collect()),
        (None, Some(reaches)) => Some(reaches.iter().map(|reach| [*reach, *reach]).collect()),
        (None, None) => None,
    });
    let reach_feet = [start, end];
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [Some(ex), Some(ey)] = WALL_EXTERIOR_FIELDS.map(float) else {
        return None;
    };
    Some(WallLayerEnds {
        exterior: [ex, ey],
        widths,
        reach_feet,
    })
}

/// Field holding a floor's, roof's or ceiling's layers, top first, in the
/// form of [`WALL_LAYERS_FIELD`] (RE-57).
pub const SLAB_LAYERS_FIELD: &str = "m_slab_layers";

/// The layers of a [`WALL_LAYERS_FIELD`] or [`SLAB_LAYERS_FIELD`] value.
/// The layers [`WALL_LAYERS_FIELD`] records on a wall with no
/// [`WALL_EXTERIOR_FIELDS`]: its type's, where its data does not place it
/// (RE-88).
pub fn unplaced_wall_layers_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<Vec<crate::ifc::LayerBand>> {
    if fields
        .iter()
        .any(|(name, _)| WALL_EXTERIOR_FIELDS.contains(&name.as_str()))
    {
        return None;
    }
    fields
        .iter()
        .find_map(|(name, value)| (name == WALL_LAYERS_FIELD).then_some(value))
        .and_then(layer_bands_from_field)
}

fn layer_bands_from_field(value: &InstanceField) -> Option<Vec<crate::ifc::LayerBand>> {
    let InstanceField::Vector(items) = value else {
        return None;
    };
    let bands = items
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
                    rest @ ..,
                ] => Some(crate::ifc::LayerBand {
                    width_feet: *width,
                    color_packed: u32::try_from(*colour).ok(),
                    transparency: *transparency,
                    name: match rest {
                        [InstanceField::String(name)] => Some(name.clone()),
                        _ => None,
                    },
                }),
                _ => None,
            },
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!bands.is_empty()).then_some(bands)
}

/// The field form of a type's layers, each with its material's shading and,
/// where it is read, its name (RE-58); membranes, which have no width, are
/// left out.
fn layer_bands_field(
    layers: &[crate::partition_compound_structure::CompoundLayer],
    appearances: &std::collections::BTreeMap<u32, crate::partition_materials::MaterialAppearance>,
    names: &std::collections::BTreeMap<u32, String>,
) -> InstanceField {
    InstanceField::Vector(
        layers
            .iter()
            .filter(|layer| layer.width_feet > 0.0)
            .map(|layer| {
                let appearance = layer.material.and_then(|m| appearances.get(&m));
                let mut band = vec![
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
                ];
                if let Some(name) = layer.material.and_then(|m| names.get(&m)) {
                    band.push(InstanceField::String(name.clone()));
                }
                InstanceField::Vector(band)
            })
            .collect(),
    )
}

/// The layers [`WALL_LAYERS_FIELD`] and [`WALL_EXTERIOR_FIELDS`], or
/// [`SLAB_LAYERS_FIELD`], record.
pub fn element_layers_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::ifc::ElementLayers> {
    let field = |wanted: &str| {
        fields
            .iter()
            .find_map(|(name, value)| (name == wanted).then_some(value))
    };
    if let Some(layers) = field(SLAB_LAYERS_FIELD).and_then(layer_bands_from_field) {
        return Some(crate::ifc::ElementLayers {
            exterior_normal: [0.0, 0.0],
            layers,
            stacked: true,
            system_family: None,
        });
    }
    let float = |wanted: &str| match field(wanted) {
        Some(InstanceField::Float { value, .. }) => Some(*value),
        _ => None,
    };
    let [x, y] = WALL_EXTERIOR_FIELDS.map(float);
    let layers = field(WALL_LAYERS_FIELD).and_then(layer_bands_from_field)?;
    Some(crate::ifc::ElementLayers {
        exterior_normal: [x?, y?],
        layers,
        stacked: false,
        system_family: None,
    })
}

/// The Revit system family of a record-backed element's type (RE-61,
/// RE-63). Revit derives the name rather than storing it, from the type's
/// kind, and the category gives it where only one system family fits:
/// - a curtain wall (RE-46) is a Curtain Wall, and a railing a Railing;
/// - a floor is a Floor, and a building pad a Pad;
/// - a slab edge is a Slab Edge, and a ramp a Ramp (RE-64);
/// - a wall, ceiling or roof whose type has compound layers (`has_layers`)
///   is a Basic Wall, a Compound Ceiling or a Basic Roof, since a Curtain
///   or Stacked Wall, a Basic Ceiling and Sloped Glazing have none;
/// - a curtain panel whose type is a wall type with layers is a Basic Wall
///   (RE-64);
/// - a pipe is a Pipe Types, Revit's one pipe system family (RE-130).
///
/// Measured against the `Family:Type:ElementId` names Revit's own IFC
/// export gives the same elements, on every record-backed wall, floor,
/// ceiling, roof and building pad of Snowdon Towers, Core Interior and RE1
/// Architecture: 1,444 walls, 283 floors, 74 ceilings, 20 roofs and 1 pad.
pub fn system_family(class: &str, has_layers: bool) -> Option<&'static str> {
    match (class, has_layers) {
        (CURTAIN_WALL_CLASS, _) => Some("Curtain Wall"),
        ("Railing", _) => Some("Railing"),
        ("Floor", _) => Some("Floor"),
        ("BuildingPad", _) => Some("Pad"),
        ("SlabEdge", _) => Some("Slab Edge"),
        ("Ramp", _) => Some("Ramp"),
        ("Pipe", _) => Some("Pipe Types"),
        ("Wall" | "CurtainWallPanel", true) => Some("Basic Wall"),
        ("Ceiling", true) => Some("Compound Ceiling"),
        ("Roof", true) => Some("Basic Roof"),
        _ => None,
    }
}

/// The name Revit's export gives system family `family` in a file saved
/// in `locale` (`BasicFileInfo`'s "Locale when saved", e.g. `ENU`, `PTB`).
///
/// A system family's name is not in the file: Revit supplies it in the
/// language the file is saved in. Four files saved in `PTB` (two Revit
/// 2023, two Revit 2025) name their walls, floors and roofs this way
/// in Revit's own export, 28 walls, 3 floors and 1 roof, and no other
/// system family occurs in them (RE-123). Every other family, and every
/// other locale, keeps the `ENU` name [`system_family`] gives, which is
/// what the `ENU` files measure.
pub fn localized_system_family(family: &'static str, locale: Option<&str>) -> &'static str {
    match (locale, family) {
        (Some("PTB"), "Basic Wall") => "Parede básica",
        (Some("PTB"), "Floor") => "Piso",
        (Some("PTB"), "Basic Roof") => "Telhado básico",
        _ => family,
    }
}

/// Field recording that [`FAMILY_NAME_FIELD`] is a system family's name
/// derived by [`system_family`], not read from the file (RE-63).
pub const FAMILY_NAME_SOURCE_FIELD: &str = "m_family_name_source";

/// Value of [`FAMILY_NAME_SOURCE_FIELD`].
pub const SYSTEM_FAMILY_SOURCE: &str = "system_family_by_category";

/// Give each system-family element with a type name (#322) its system
/// family's name ([`system_family`]), so it is named `Family:Type:ElementId`
/// as Revit names it (RE-63). Whether a wall's, ceiling's or roof's type has
/// compound layers is the type's: an element of the type drawn in layers
/// shows it, and otherwise the type's own data is read
/// ([`crate::partition_compound_structure::scan_type_layers`]).
fn attach_system_family_names(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::partition_compound_structure as pcs;
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let shows_layers = |element: &DecodedElement| {
        element.fields.iter().any(|(name, _)| {
            name == WALL_LAYERS_FIELD
                || name == SLAB_LAYERS_FIELD
                || name == ROOF_TYPE_THICKNESS_FIELD
        })
    };
    let mut layered: BTreeSet<u32> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| shows_layers(element))
        .filter_map(type_of)
        .collect();
    let pending: BTreeSet<u32> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| {
            matches!(
                element.class.as_str(),
                "Wall" | "Ceiling" | "Roof" | "CurtainWallPanel"
            )
        })
        .filter_map(type_of)
        .filter(|id| !layered.contains(id))
        .collect();
    if !pending.is_empty() && pcs::layer_layout(revit_version).is_some() {
        if let Ok(records) = crate::elem_table::parse_records(rf) {
            let declared = crate::elem_table::declared_ids(&records);
            // Revit 2023 has no type records, and its materials no element
            // records either: a layer's material is only held to be
            // declared (RE-112).
            let materials: BTreeSet<u32> =
                if revit_version == crate::partition_element_records_2023::REVIT_2023 {
                    declared.clone()
                } else {
                    crate::partition_type_records::scan_type_records(
                        rf,
                        revit_version,
                        crate::partition_type_records::OST_MATERIALS,
                        &declared,
                    )
                    .unwrap_or_default()
                    .iter()
                    .map(|record| record.element_id)
                    .collect()
                };
            if let Ok(types) =
                pcs::scan_type_layers(rf, revit_version, &pending, &materials, &declared)
            {
                layered.extend(
                    types
                        .into_iter()
                        .filter(|(_, layers)| !layers.is_empty())
                        .map(|(id, _)| id),
                );
            }
        }
    }
    let locale = rf.basic_file_info().ok().and_then(|info| info.locale);
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        let has = |field: &str| element.fields.iter().any(|(name, _)| name == field);
        if !has(TYPE_NAME_FIELD) || has(FAMILY_NAME_FIELD) {
            continue;
        }
        let has_layers = type_of(element).is_some_and(|id| layered.contains(&id));
        let Some(family) = system_family(&element.class, has_layers) else {
            continue;
        };
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(localized_system_family(family, locale.as_deref()).into()),
        ));
        element.fields.push((
            FAMILY_NAME_SOURCE_FIELD.into(),
            InstanceField::String(SYSTEM_FAMILY_SOURCE.into()),
        ));
    }
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
        || pcs::layer_layout(revit_version).is_none()
    {
        return;
    }
    // A 2023 wall keeps its record body, cut back at its joins (RE-120),
    // and takes only its layers and exterior side: its location line is not
    // read (RE-114).
    let centreline_bodies = revit_version != crate::partition_element_records_2023::REVIT_2023;
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
    let Some((materials, names)) = materials_and_names(rf, revit_version, &declared) else {
        return;
    };
    let (Ok(layers), Ok(orientations), Ok(lines), Ok(arcs), Ok(appearances), Ok(face_angles)) = (
        pcs::scan_type_layers(rf, revit_version, &types, &materials, &declared),
        pcs::scan_wall_orientations(rf, revit_version, &wall_ids),
        pcs::scan_wall_lines(rf, revit_version, &wall_ids),
        pcs::scan_wall_arcs(rf, revit_version, &wall_ids),
        crate::partition_materials::scan_material_appearances(rf, revit_version, &declared),
        pcs::scan_wall_type_face_angles(rf, revit_version, &types),
    ) else {
        return;
    };
    // RE-91: a layer that takes its category's material takes the one the
    // document's object styles give walls, where it is set and named.
    let category_material = crate::partition_materials::scan_category_material(
        rf,
        revit_version,
        crate::partition_element_records::OST_WALLS,
    )
    .ok()
    .flatten()
    .filter(|id| names.contains_key(id));
    let layers: std::collections::BTreeMap<u32, Vec<pcs::CompoundLayer>> = layers
        .into_iter()
        .map(|(type_id, type_layers)| {
            let type_layers = type_layers
                .into_iter()
                .map(|layer| pcs::CompoundLayer {
                    material: layer.material.or(category_material),
                    ..layer
                })
                .collect();
            (type_id, type_layers)
        })
        .collect();
    for wall in walls.iter_mut() {
        let (Some(id), Some(type_id)) = (wall.id, type_of(wall)) else {
            continue;
        };
        let (Some(type_layers), Some(orientation)) = (layers.get(&type_id), orientations.get(&id))
        else {
            continue;
        };
        let thickness: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        let measured = thickness.is_finite() && thickness > 0.0;
        // RE-86: a wall with word 2 whose type leans only its exterior face
        // is tapered; its line is still its centreline at the top. Only a
        // straight tapered wall is measured.
        let taper = face_angles
            .get(&type_id)
            .filter(|_| orientation.word == 2 && lines.contains_key(&id))
            .and_then(|&[first, exterior, third]| {
                (first == 0.0 && third == 0.0 && exterior > 0.0 && exterior < FRAC_PI_4)
                    .then_some(exterior)
            });
        let centred = centreline_bodies && measured && (orientation.word == 1 || taper.is_some());
        let float = |value: f64| InstanceField::Float { value, size: 8 };
        // The direction the exterior side is taken from: the line's, or the
        // arc's tangent at its middle (RE-75).
        let (dx, dy) = if let Some(line) = lines.get(&id) {
            let (start, end) = (line.start(), line.end());
            let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
            let length = dx.hypot(dy);
            if !length.is_finite() || length <= 1e-9 {
                continue;
            }
            if centred {
                for (names, point) in [(WALL_AXIS_START_FIELDS, start), (WALL_AXIS_END_FIELDS, end)]
                {
                    for (name, value) in names.iter().zip(point) {
                        wall.fields.push(((*name).into(), float(value)));
                    }
                }
            }
            (dx / length, dy / length)
        } else if let Some(arc) = arcs.get(&id) {
            if arc.x_axis[2].abs() > 1e-9 || arc.y_axis[2].abs() > 1e-9 {
                continue;
            }
            if centred {
                let values = [
                    arc.centre[0],
                    arc.centre[1],
                    arc.radius,
                    arc.start_angle,
                    arc.end_angle,
                    arc.x_axis[0],
                    arc.x_axis[1],
                    arc.y_axis[0],
                    arc.y_axis[1],
                ];
                for (name, value) in WALL_ARC_FIELDS.iter().zip(values) {
                    wall.fields.push(((*name).into(), float(value)));
                }
            }
            let (sin, cos) = ((arc.start_angle + arc.end_angle) * 0.5).sin_cos();
            let tangent = [
                -sin * arc.x_axis[0] + cos * arc.y_axis[0],
                -sin * arc.x_axis[1] + cos * arc.y_axis[1],
            ];
            let length = tangent[0].hypot(tangent[1]);
            if !length.is_finite() || length <= 1e-9 {
                continue;
            }
            (tangent[0] / length, tangent[1] / length)
        } else {
            continue;
        };
        if centred {
            wall.fields
                .push((WALL_TYPE_THICKNESS_FIELD.into(), float(thickness)));
            if let Some(angle) = taper {
                wall.fields
                    .push((WALL_EXTERIOR_FACE_ANGLE_FIELD.into(), float(angle)));
            }
        }
        let exterior = if orientation.flip {
            [dy, -dx]
        } else {
            [-dy, dx]
        };
        let bands = layer_bands_field(type_layers, &appearances, &names);
        if matches!(&bands, InstanceField::Vector(items) if items.is_empty()) {
            continue;
        }
        for (name, value) in WALL_EXTERIOR_FIELDS.iter().zip(exterior) {
            wall.fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        wall.fields.push((WALL_LAYERS_FIELD.into(), bands));
    }
    // RE-88: a wall its data does not place (no orientation, line or arc)
    // still carries its type's layers, for its materials. With no exterior
    // side they are not drawn.
    for wall in walls.iter_mut() {
        if wall
            .fields
            .iter()
            .any(|(name, _)| name == WALL_LAYERS_FIELD)
        {
            continue;
        }
        let Some(type_layers) = type_of(wall).and_then(|type_id| layers.get(&type_id)) else {
            continue;
        };
        let bands = layer_bands_field(type_layers, &appearances, &names);
        if matches!(&bands, InstanceField::Vector(items) if items.is_empty()) {
            continue;
        }
        wall.fields.push((WALL_LAYERS_FIELD.into(), bands));
    }
    if centreline_bodies {
        attach_wall_butt_joins(rf, revit_version, walls, &layers);
    }
}

/// Give each wall with a centreline the wall it butt-joins at each end and
/// whether it runs through there, where its and its partner's join lists
/// decide it, and, between single-layer walls, how far past the end its
/// body reaches ([`crate::element_record_wall_joins::butt_joins`], RE-70).
fn attach_wall_butt_joins(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
    type_layers: &std::collections::BTreeMap<
        u32,
        Vec<crate::partition_compound_structure::CompoundLayer>,
    >,
) {
    use crate::element_record_wall_joins as joins;
    let float = |wall: &DecodedElement, wanted: &str| {
        wall.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let lines: Vec<joins::WallLine> = walls
        .iter()
        .filter_map(|wall| {
            let axis = wall_axis_from_fields(&wall.fields)?;
            let base = float(wall, "m_locationZ")?;
            let type_id = wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
                _ => None,
            })?;
            let layers: Vec<(f64, u32)> = type_layers
                .get(&type_id)?
                .iter()
                .filter(|layer| layer.width_feet > 0.0)
                .map(|layer| (layer.width_feet, layer.function))
                .collect();
            let [Some(ex), Some(ey)] = WALL_EXTERIOR_FIELDS.map(|name| float(wall, name)) else {
                return None;
            };
            Some(joins::WallLine {
                element_id: wall.id?,
                start: axis.start,
                end: axis.end,
                thickness_feet: axis.thickness_feet,
                layers,
                exterior: [ex, ey],
                base_feet: base,
                top_feet: base + float(wall, "m_bboxHeight")?,
            })
        })
        .collect();
    if lines.len() < 2 {
        return;
    }
    let every: BTreeSet<u32> = walls.iter().filter_map(|wall| wall.id).collect();
    let read: BTreeSet<u32> = lines.iter().map(|line| line.element_id).collect();
    let Ok(partners) = crate::partition_compound_structure::scan_wall_join_partners(
        rf,
        revit_version,
        &every,
        &read,
    ) else {
        return;
    };
    // RE-127: the joined-wall lists decide the L joints the join lists do not.
    let joined = crate::partition_compound_structure::scan_wall_joined_entries(
        rf,
        revit_version,
        &every,
        &read,
    )
    .unwrap_or_default();
    let found = joins::butt_joins(&lines, &partners, &joined);
    for wall in walls.iter_mut() {
        let Some(ends) = wall.id.and_then(|id| found.get(&id)) else {
            continue;
        };
        for (slot, join) in ends.iter().enumerate() {
            let Some(join) = join else {
                continue;
            };
            wall.fields.push((
                WALL_JOIN_PARTNER_FIELDS[slot].into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: join.partner,
                },
            ));
            wall.fields.push((
                WALL_JOIN_THROUGH_FIELDS[slot].into(),
                InstanceField::Bool(join.runs_through),
            ));
            if let Some(reach) = join.reach_feet {
                wall.fields.push((
                    WALL_JOIN_REACH_FIELDS[slot].into(),
                    InstanceField::Float {
                        value: reach,
                        size: 8,
                    },
                ));
            }
            if let Some(edges) = &join.layer_edge_reach_feet {
                wall.fields.push((
                    WALL_JOIN_LAYER_EDGE_REACH_FIELDS[slot].into(),
                    InstanceField::Vector(
                        edges
                            .iter()
                            .flatten()
                            .map(|value| InstanceField::Float {
                                value: *value,
                                size: 8,
                            })
                            .collect(),
                    ),
                ));
            }
            if let Some(reaches) = &join.layer_reach_feet {
                wall.fields.push((
                    WALL_JOIN_LAYER_REACH_FIELDS[slot].into(),
                    InstanceField::Vector(
                        reaches
                            .iter()
                            .map(|value| InstanceField::Float {
                                value: *value,
                                size: 8,
                            })
                            .collect(),
                    ),
                ));
            }
        }
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
/// whose run type reads that type's id and name. A run ending with a tread
/// has one riser line more than risers (RE-92); a run whose riser lines do
/// not number its risers so is left to its record box.
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
    let sketches: std::collections::BTreeMap<u32, ps::RunSketch> = lines
        .iter()
        .filter_map(|(&id, lines)| Some((id, ps::run_sketch(lines)?)))
        .collect();
    let ends = ps::scan_run_ends(rf, revit_version, &sketches).unwrap_or_default();
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
        let end_with_riser = element
            .id
            .and_then(|id| ends.get(&id))
            .map(|ends| ends.end_with_riser);
        let riser_lines = risers + i64::from(end_with_riser == Some(false));
        let Some(sketch) = element
            .id
            .and_then(|id| sketches.get(&id))
            .filter(|sketch| i64::try_from(sketch.risers.len()) == Ok(riser_lines))
        else {
            continue;
        };
        let Some(profile) = ps::run_side_profile(sketch, run_type, riser_height, end_with_riser)
        else {
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

/// Fields holding the first end of a pipe's centreline, model feet (RE-131).
pub const PIPE_AXIS_START_FIELDS: [&str; 3] = [
    "m_pipe_axis_start_x",
    "m_pipe_axis_start_y",
    "m_pipe_axis_start_z",
];
/// Fields holding the second end of a pipe's centreline, model feet (RE-131).
pub const PIPE_AXIS_END_FIELDS: [&str; 3] = [
    "m_pipe_axis_end_x",
    "m_pipe_axis_end_y",
    "m_pipe_axis_end_z",
];
/// Field holding a pipe's outside radius, feet (RE-131).
pub const PIPE_RADIUS_FIELD: &str = "m_pipe_radius";
/// Fields holding a duct's or pipe's `m_dWidthOrDiameter` and `m_dHeight`,
/// feet (RE-134).
pub const CURVE_SIZE_FIELDS: [&str; 2] = ["m_curve_width", "m_curve_height"];
/// Field holding a duct's length along its axis, feet: the dimension of its
/// record box its width and height leave (RE-134).
pub const DUCT_LENGTH_FIELD: &str = "m_duct_length";
/// How closely a duct's width and height must match two of its box's
/// dimensions, feet.
pub const CURVE_BOX_TOLERANCE_FEET: f64 = 2e-3;

/// A duct's or pipe's width and height, feet (RE-134), when read.
pub fn curve_size_from_fields(fields: &[(String, InstanceField)]) -> Option<(f64, f64)> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    Some((field(CURVE_SIZE_FIELDS[0])?, field(CURVE_SIZE_FIELDS[1])?))
}

/// Give each duct and pipe the width and height its curve object holds, and
/// each duct without a type its curve's type and that type's name
/// ([`crate::partition_curve_fields`], RE-134;
/// [`crate::partition_names::find_mep_curve_type_names`], RE-130). An element
/// whose anchor is not found gets nothing.
fn attach_curve_fields(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_curve_fields as pcf;
    if !pcf::supports_revit_version(revit_version) {
        return;
    }
    let is_curve = |element: &DecodedElement| matches!(element.class.as_str(), "Duct" | "Pipe");
    let mut by_stream: BTreeMap<String, Vec<(u32, [f64; 6])>> = BTreeMap::new();
    for element in products.iter().filter(|element| is_curve(element)) {
        if let (Some(id), Some(bbox), Some(stream)) = (
            element.id,
            element_record_bbox(element),
            element_source_stream(element),
        ) {
            by_stream.entry(stream).or_default().push((id, bbox));
        }
    }
    let Ok(fields) = pcf::scan_curve_fields(rf, revit_version, &by_stream) else {
        return;
    };
    if fields.is_empty() {
        return;
    }
    let has = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().any(|(name, _)| name == wanted)
    };
    let wanted: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Duct" && !has(element, TYPE_NAME_FIELD))
        .filter_map(|element| Some(fields.get(&element.id?)?.type_id))
        .collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    if !wanted.is_empty() {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            for (id, name) in
                crate::partition_names::find_mep_curve_type_names(inflated.bytes(), &wanted)
            {
                match names.get_mut(&id) {
                    None => {
                        names.insert(id, Some(name));
                    }
                    Some(held) => {
                        if held.as_deref() != Some(name.as_str()) {
                            *held = None;
                        }
                    }
                }
            }
        }
    }
    for element in products.iter_mut().filter(|element| is_curve(element)) {
        let Some(curve) = element.id.and_then(|id| fields.get(&id)).copied() else {
            continue;
        };
        for (name, value) in CURVE_SIZE_FIELDS
            .iter()
            .zip([curve.width_feet, curve.height_feet])
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        // A duct along one of the model's axes: its box is its section by its
        // length (RE-134: 25 of 25 RE1 ducts).
        if element.class == "Duct" {
            if let Some(length) = element_record_bbox(element)
                .and_then(|bbox| curve.length_in_box(bbox, CURVE_BOX_TOLERANCE_FEET))
            {
                element.fields.push((
                    DUCT_LENGTH_FIELD.into(),
                    InstanceField::Float {
                        value: length,
                        size: 8,
                    },
                ));
            }
        }
        if element.class != "Duct" || has(element, TYPE_NAME_FIELD) {
            continue;
        }
        let Some(Some(type_name)) = names.get(&curve.type_id) else {
            continue;
        };
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: curve.type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String(type_name.clone()),
        ));
    }
}

/// The cylinder the partition MVP gave a pipe, when its connector entries
/// and record box made one (RE-131).
pub fn pipe_body_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_pipe_axes::PipeBody> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    Some(crate::partition_pipe_axes::PipeBody {
        start: point(PIPE_AXIS_START_FIELDS)?,
        end: point(PIPE_AXIS_END_FIELDS)?,
        radius_feet: field(PIPE_RADIUS_FIELD)?,
    })
}

/// Give each pipe the cylinder its connector entries and its record box make
/// ([`crate::partition_pipe_axes`], RE-131). A pipe whose ends are not found,
/// or whose two ends and box no cylinder reproduces, is not given one and
/// keeps its box.
fn attach_pipe_axes(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_pipe_axes as ppa;
    if !ppa::supports_revit_version(revit_version) {
        return;
    }
    let mut by_stream: BTreeMap<String, Vec<(u32, [f64; 6])>> = BTreeMap::new();
    for element in products.iter().filter(|element| element.class == "Pipe") {
        if let (Some(id), Some(bbox), Some(stream)) = (
            element.id,
            element_record_bbox(element),
            element_source_stream(element),
        ) {
            by_stream.entry(stream).or_default().push((id, bbox));
        }
    }
    if by_stream.is_empty() {
        return;
    }
    let Ok(bodies) = ppa::scan_pipe_bodies(rf, revit_version, &by_stream) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Pipe")
    {
        let Some(body) = element.id.and_then(|id| bodies.get(&id)) else {
            continue;
        };
        for (names, point) in [
            (PIPE_AXIS_START_FIELDS, body.start),
            (PIPE_AXIS_END_FIELDS, body.end),
        ] {
            for (name, value) in names.iter().zip(point) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
        element.fields.push((
            PIPE_RADIUS_FIELD.into(),
            InstanceField::Float {
                value: body.radius_feet,
                size: 8,
            },
        ));
    }
}

/// The field naming the element joined at connector `connector` of a duct,
/// pipe or fitting (RE-138, RE-141).
pub fn connector_element_field(connector: u32) -> String {
    format!("m_connector_{connector}_element")
}

/// The field naming that element's connector index (RE-138, RE-141).
pub fn connector_index_field(connector: u32) -> String {
    format!("m_connector_{connector}_index")
}

/// The joins of an element: its connector, then the element and connector
/// index it is joined to (RE-138, RE-141).
pub type ConnectorJoins = Vec<(u32, u32, u32)>;

/// The joins of a duct, pipe or fitting, from its fields (RE-138, RE-141).
pub fn connector_joins_from_fields(fields: &[(String, InstanceField)]) -> ConnectorJoins {
    let mut joins = ConnectorJoins::new();
    for (name, value) in fields {
        let InstanceField::ElementId { id: other, .. } = value else {
            continue;
        };
        let Some(connector) = name
            .strip_prefix("m_connector_")
            .and_then(|rest| rest.strip_suffix("_element"))
            .and_then(|number| number.parse::<u32>().ok())
        else {
            continue;
        };
        let index_name = connector_index_field(connector);
        let other_index = fields.iter().find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if *name == index_name => {
                u32::try_from(*value).ok()
            }
            _ => None,
        });
        if let Some(other_index) = other_index {
            joins.push((connector, *other, other_index));
        }
    }
    joins
}

/// Give each duct, pipe and fitting the element and connector joined at each
/// of its connectors ([`crate::partition_connector_pairs`], RE-138, RE-141). A
/// connector with no join read is left without the fields.
fn attach_connector_pairs(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_connector_pairs as pcp;
    if !pcp::supports_revit_version(revit_version) {
        return;
    }
    let is_curve = |element: &DecodedElement| matches!(element.class.as_str(), "Duct" | "Pipe");
    let is_fitting =
        |element: &DecodedElement| matches!(element.class.as_str(), "DuctFitting" | "PipeFitting");
    let ids_of = |keep: &dyn Fn(&DecodedElement) -> bool| -> BTreeSet<u64> {
        products
            .iter()
            .filter(|element| keep(element))
            .filter_map(|element| element.id)
            .map(u64::from)
            .collect()
    };
    let curves = ids_of(&is_curve);
    let fittings = ids_of(&is_fitting);
    if curves.is_empty() && fittings.is_empty() {
        return;
    }
    let everything = ids_of(&|_| true);
    let mut pairs = pcp::scan_connector_pairs(rf, &curves).unwrap_or_default();
    pairs.extend(pcp::scan_fitting_pairs(rf, &fittings, &everything).unwrap_or_default());
    for element in products
        .iter_mut()
        .filter(|element| is_curve(element) || is_fitting(element))
    {
        let Some(id) = element.id else {
            continue;
        };
        let mut joined: BTreeSet<u32> = BTreeSet::new();
        for pair in pairs.iter().filter(|pair| pair.element == u64::from(id)) {
            let Ok(other) = u32::try_from(pair.other) else {
                continue;
            };
            if !joined.insert(pair.index) {
                continue;
            }
            element.fields.push((
                connector_element_field(pair.index),
                InstanceField::ElementId { tag: 0, id: other },
            ));
            element.fields.push((
                connector_index_field(pair.index),
                InstanceField::Integer {
                    value: i64::from(pair.other_index),
                    signed: false,
                    size: 4,
                },
            ));
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
///
/// Some parts name no wall at all, only their sibling mullions: on Snowdon,
/// the mullions of the "Solar Panels" curtain walls. Such a part takes the
/// one curtain wall whose record box contains its own (RE-62). That is
/// Revit's parent for all 89 such mullions and the 3 such panels that
/// Revit's export aggregates.
fn attach_curtain_walls(
    rf: &mut RevitFile,
    revit_version: u32,
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
    // RE-72: a part names the curtain grid it lies on, and the grid's own
    // data names its curtain wall.
    let unrecorded: BTreeSet<u32> = references
        .iter()
        .flatten()
        .flatten()
        .filter_map(|&id| u32::try_from(id).ok())
        .filter(|id| !wall_ids.contains(id))
        .collect();
    let grids = scan_curtain_grid_walls(rf, revit_version, &unrecorded, &wall_ids);
    curtain_walls.extend(grids.values().copied());
    if curtain_walls.is_empty() {
        return;
    }
    for wall in walls.iter_mut() {
        if wall.id.is_some_and(|id| curtain_walls.contains(&id)) {
            wall.class = CURTAIN_WALL_CLASS.into();
        }
    }
    // RE-62: the record boxes of the curtain walls, for parts whose record
    // names none.
    let wall_boxes: Vec<(u32, [f64; 6])> = walls
        .iter()
        .filter_map(|wall| Some((wall.id?, element_record_bbox(wall)?)))
        .filter(|(id, _)| curtain_walls.contains(id))
        .collect();
    for (product, list) in products.iter_mut().zip(references) {
        let Some(list) = list else {
            continue;
        };
        let named: BTreeSet<u32> = list
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| curtain_walls.contains(id) && Some(*id) != product.id)
            .collect();
        let on_grid: BTreeSet<u32> = list
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter_map(|id| grids.get(&id).copied())
            .collect();
        let whole = match (on_grid.len(), named.len()) {
            (1, _) => on_grid.iter().next().copied(),
            (0, 1) => named.iter().next().copied(),
            (0, 0) => element_record_bbox(product).and_then(|part| {
                let mut inside = wall_boxes
                    .iter()
                    .filter(|(_, wall)| box_contains(wall, &part, CURTAIN_PART_BOX_TOLERANCE_FEET))
                    .map(|(id, _)| *id);
                let first = inside.next()?;
                inside.next().is_none().then_some(first)
            }),
            _ => None,
        };
        if let Some(whole) = whole {
            product.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        }
    }
}

/// Where a curtain grid's element data names its curtain wall, as a `u64`
/// at both offsets past the data's ElementId (RE-72).
pub const CURTAIN_GRID_WALL_OFFSETS: [usize; 2] = [85, 478];

/// The curtain wall each id in `candidates` names as a curtain grid, by
/// the grid's ElementId (RE-72): its element data carries a wall in
/// `walls` at both [`CURTAIN_GRID_WALL_OFFSETS`]. A grid whose copies
/// disagree is left out.
fn scan_curtain_grid_walls(
    rf: &mut RevitFile,
    revit_version: u32,
    candidates: &BTreeSet<u32>,
    walls: &BTreeSet<u32>,
) -> BTreeMap<u32, u32> {
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return BTreeMap::new();
    };
    let u64_at = |buf: &[u8], at: usize| {
        buf.get(at..at.checked_add(8)?)
            .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
    };
    let mut found: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &header) {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| candidates.contains(id))
            else {
                continue;
            };
            let [first, second] =
                CURTAIN_GRID_WALL_OFFSETS.map(|offset| u64_at(buf, id_at + 8 + offset));
            let wall = match (first, second) {
                (Some(first), Some(second)) if first == second => u32::try_from(first)
                    .ok()
                    .filter(|wall| walls.contains(wall)),
                _ => None,
            };
            let Some(wall) = wall else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(wall));
                }
                Some(held) => {
                    if *held != Some(wall) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, wall)| wall.map(|wall| (id, wall)))
        .collect()
}

/// How far past a curtain wall's record box a part's box may reach and
/// still lie inside it (RE-62): 0.05 ft places every measured part, and
/// 0.01 ft misses mullions whose profile reaches past the wall's box.
const CURTAIN_PART_BOX_TOLERANCE_FEET: f64 = 0.05;

/// `inner` lies inside `outer`, both `[min x, min y, min z, max x, max y,
/// max z]`, within `tolerance` on every side.
fn box_contains(outer: &[f64; 6], inner: &[f64; 6], tolerance: f64) -> bool {
    (0..3).all(|axis| {
        inner[axis] >= outer[axis] - tolerance && inner[axis + 3] <= outer[axis + 3] + tolerance
    })
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
    let mut rooms = room_instances_from_records(records, &parameters, level_ids);
    // RE-117: a room's number and name are the ones its parameter entries
    // give it; RE-29's block stands in where they are not read, and still
    // gives the Level. On Snowdon Towers the block names 1 of 54 rooms, and
    // that one wrongly.
    attach_room_parameter_entries(rf, revit_version, &room_ids, &mut rooms);
    // RE-153: a room's Floor Finish.
    let finishes = crate::partition_room_parameters::scan_room_text_parameter(
        rf,
        revit_version,
        &room_ids,
        crate::partition_room_parameters::ROOM_FINISH_FLOOR_PARAMETER,
    );
    for room in rooms.iter_mut() {
        if let Some(finish) = room.id.and_then(|id| finishes.get(&id)) {
            room.fields.push((
                ROOM_FLOOR_FINISH_FIELD.into(),
                InstanceField::String(finish.clone()),
            ));
        }
    }
    Ok(rooms)
}

/// Field carrying a room's Floor Finish (RE-153).
pub const ROOM_FLOOR_FINISH_FIELD: &str = "m_room_floor_finish";

/// Give each room in `rooms` the number and name its parameter entries
/// hold (RE-117), in place of any read before that differs; a room whose
/// block agrees keeps the block as its source.
fn attach_room_parameter_entries(
    rf: &mut RevitFile,
    revit_version: u32,
    room_ids: &BTreeSet<u32>,
    rooms: &mut [DecodedElement],
) {
    let entries =
        crate::partition_room_parameters::scan_room_parameter_entries(rf, revit_version, room_ids);
    for room in rooms.iter_mut() {
        let Some((number, name)) = room.id.and_then(|id| entries.get(&id)) else {
            continue;
        };
        let text = |wanted: &str| {
            room.fields.iter().find_map(|(field, value)| match value {
                InstanceField::String(text) if field == wanted => Some(text.clone()),
                _ => None,
            })
        };
        // Where RE-29's block already agrees, it keeps its provenance.
        if text("m_name").as_ref() == Some(name) && text(ROOM_NUMBER_FIELD).as_ref() == Some(number)
        {
            continue;
        }
        room.fields.retain(|(field, _)| {
            field != "m_name" && field != ROOM_NUMBER_FIELD && field != ROOM_PARAMETER_SOURCE_FIELD
        });
        room.fields
            .push(("m_name".into(), InstanceField::String(name.clone())));
        room.fields.push((
            ROOM_NUMBER_FIELD.into(),
            InstanceField::String(number.clone()),
        ));
        room.fields.push((
            ROOM_PARAMETER_SOURCE_FIELD.into(),
            InstanceField::String(
                crate::partition_room_parameters::ROOM_PARAMETER_ENTRY_SOURCE.into(),
            ),
        ));
    }
}

/// Give each room the outline of its stored solid (RE-101,
/// [`crate::partition_room_boundaries`]), read from the partition of the
/// room's own record and kept only where it spans the record's box. A room
/// whose solid does not close keeps its box.
fn attach_room_outlines(rf: &mut RevitFile, revit_version: u32, rooms: &mut [DecodedElement]) {
    use crate::partition_room_boundaries as prb;
    if !prb::ROOM_SOLID_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    for room in rooms.iter_mut() {
        let (Some(id), Some(stream)) = (room.id, element_source_stream(room)) else {
            continue;
        };
        let float = |wanted: &str| {
            room.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let (Some(cx), Some(cy), Some(z), Some(dx), Some(dy), Some(dz)) = (
            float("m_locationX"),
            float("m_locationY"),
            float("m_locationZ"),
            float("m_bboxWidth"),
            float("m_bboxDepth"),
            float("m_bboxHeight"),
        ) else {
            continue;
        };
        let bbox = [
            cx - dx / 2.0,
            cy - dy / 2.0,
            z,
            cx + dx / 2.0,
            cy + dy / 2.0,
            z + dz,
        ];
        if let Ok(Some(profile)) = prb::room_outline(rf, &stream, id, bbox) {
            let mut fields = profile.fields();
            for (name, value) in fields.iter_mut() {
                if name == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD {
                    *value = InstanceField::String(prb::ROOM_OUTLINE_SOURCE.into());
                }
            }
            room.fields.extend(fields);
        }
    }
}

fn element_source_stream(element: &DecodedElement) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(stream) if name == "m_source_stream" => Some(stream.clone()),
        _ => None,
    })
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

/// Field carrying the exported walls a door's or window's reference list
/// names, nearest to its own ElementId first (#439). The
/// host is chosen from them once curtain walls are known
/// ([`bind_opening_hosts`]); the field is then removed.
const OPENING_HOST_CANDIDATES_FIELD: &str = "m_hostCandidateIds";

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
fn attach_host_candidates(
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
fn bind_opening_hosts(walls: &[DecodedElement], openings: &mut [DecodedElement]) {
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
        let host = candidates
            .into_iter()
            .find(|id| !curtain_walls.contains(id));
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
fn without_non_primary_options(
    rf: &mut RevitFile,
    mut records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> Vec<crate::partition_element_records::PartitionElementRecord> {
    let options = rf.design_options();
    records.retain(|record| !options.excludes(record));
    records
}

/// Revit 2023 elements from their partition element records (RE-81): each
/// placed instance of a [`crate::partition_element_records::RECOVERED_CATEGORIES`]
/// category, with its box as its body, into the bucket its category
/// feeds. Components nested in doors and windows are left out
/// ([`nested_in_openings`]). Doors and windows bind to their host wall by
/// the 2024 rule (RE-85) on 2023's reference lists, and every element to
/// the one Level its list names (RE-107). No names, types, joins or design
/// options: their 2023 layouts are not decoded.
fn recover_2023_records(rf: &mut RevitFile, out: &mut PartitionSchemaMvp) {
    use crate::partition_element_records as per;
    let records = crate::partition_element_records_2023::scan_records(
        rf,
        crate::partition_element_records_2023::REVIT_2023,
    );
    let names = type_and_family_names_2023(rf, &records);
    let system_types = system_type_names_2023(rf, &records);
    let selected = select_newest_instance_records(records);
    // Only a family instance can be nested; a host wall and the doors it
    // hosts name each other too (RE-76's class tag, at the same place on
    // 2023).
    let family_instance = rf.schema_classes().ok().and_then(|classes| {
        classes
            .classes
            .iter()
            .find(|class| class.name == "FamilyInstance")
            .map(|class| class.tag)
    });
    let nested = match family_instance {
        Some(tag) => nested_in_openings(&selected, tag),
        None => BTreeSet::new(),
    };
    // Each record names its Level in its reference list, as on 2024
    // (RE-107).
    let level_ids = level_element_ids(rf, crate::partition_element_records_2023::REVIT_2023)
        .unwrap_or_default();
    for record in selected.values() {
        if nested.contains(&record.element_id) {
            continue;
        }
        let Some((_, class)) = per::RECOVERED_CATEGORIES
            .iter()
            .find(|(category, _)| *category == record.builtin_category)
        else {
            continue;
        };
        let mut decoded = element_record_decoded(record, class, &level_ids);
        if let Some((type_id, type_name, family_name)) = names.get(&record.element_id) {
            decoded.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: *type_id,
                },
            ));
            decoded.fields.push((
                TYPE_NAME_FIELD.into(),
                InstanceField::String(type_name.clone()),
            ));
            decoded.fields.push((
                FAMILY_NAME_FIELD.into(),
                InstanceField::String(family_name.clone()),
            ));
        } else if let Some((type_id, type_name)) = system_types.get(&record.element_id) {
            decoded.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: *type_id,
                },
            ));
            decoded.fields.push((
                TYPE_NAME_FIELD.into(),
                InstanceField::String(type_name.clone()),
            ));
        }
        match record.builtin_category {
            per::OST_WALLS => out.walls.push(decoded),
            per::OST_DOORS => out.doors.push(decoded),
            per::OST_WINDOWS => out.windows.push(decoded),
            per::OST_FLOORS | per::OST_BUILDING_PAD => out.slabs.push(decoded),
            per::OST_ROOMS => out.rooms.push(decoded),
            per::OST_COLUMNS => out.columns.push(decoded),
            _ => out.products.push(decoded),
        }
    }
    // --- Walls cut back by the walls they join, by 2024's solver (RE-26,
    // RE-29): a 2023 record box is the untrimmed wall too, and its
    // reference list names the walls it joins (RE-120) ---
    let wall_records: Vec<per::PartitionElementRecord> = out
        .walls
        .iter()
        .filter_map(|wall| wall.id.and_then(|id| selected.get(&id)).cloned())
        .collect();
    let trims = crate::element_record_wall_joins::join_trims(&wall_records);
    for wall in out.walls.iter_mut() {
        if let Some((record, trim)) = wall
            .id
            .and_then(|id| Some((selected.get(&id)?, trims.get(&id)?)))
        {
            apply_wall_join_trim(wall, record, trim);
        }
    }
    // --- Beams stopped at the faces of the columns their records name
    // (RE-122) ---
    let framing: Vec<per::PartitionElementRecord> = out
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming")
        .filter_map(|element| element.id.and_then(|id| selected.get(&id)).cloned())
        .collect();
    let columns: Vec<per::PartitionElementRecord> = selected
        .values()
        .filter(|record| {
            matches!(
                record.builtin_category,
                per::OST_COLUMNS | per::OST_STRUCTURAL_COLUMNS
            ) && !nested.contains(&record.element_id)
        })
        .cloned()
        .collect();
    let beam_trims = crate::element_record_beam_cuts::beam_column_trims(&framing, &columns);
    for beam in out.products.iter_mut() {
        if let Some(trim) = beam.id.and_then(|id| beam_trims.get(&id)) {
            apply_beam_column_trim(beam, trim);
        }
    }
    // --- Rooms' numbers and names (RE-117) ---
    let room_ids: BTreeSet<u32> = out.rooms.iter().filter_map(|room| room.id).collect();
    attach_room_parameter_entries(
        rf,
        crate::partition_element_records_2023::REVIT_2023,
        &room_ids,
        &mut out.rooms,
    );
    let wall_ids: BTreeSet<u32> = out.walls.iter().filter_map(|wall| wall.id).collect();
    for element in out.doors.iter_mut().chain(out.windows.iter_mut()) {
        if let Some(record) = element.id.and_then(|id| selected.get(&id)) {
            attach_host_candidates(record, &wall_ids, element);
        }
    }
    bind_opening_hosts(&out.walls, &mut out.doors);
    bind_opening_hosts(&out.walls, &mut out.windows);
}

/// ElementIds of the instances in `selected` nested in a door or window
/// (RE-81): a family instance (class tag `family_instance`) whose
/// reference list names a placed door or window that names it back. Revit's export writes some of these and leaves out
/// others, by a rule not decoded (on a 2023 project it keeps a window's
/// frame and sill cut and drops its mullion pattern), so all are left out
/// rather than guessed.
fn nested_in_openings(
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

/// Each Revit 2023 family instance's type ElementId, type name and family
/// name (RE-109), by ElementId. Its type is the one id in its reference
/// list, wherever it sits, that is a symbol record of its own category with
/// a 2023 name entry ([`crate::partition_names::name_entries_2023`]). Its
/// family is the one id the type's own record references that has a name
/// entry and no element record. An instance whose type or family is not
/// unique gets nothing.
/// Categories whose types are system-family types (RE-111): walls, floors,
/// roofs and ceilings.
const SYSTEM_TYPE_CATEGORIES_2023: [i64; 4] = [
    crate::partition_element_records::OST_WALLS,
    crate::partition_element_records::OST_FLOORS,
    -2_000_035,
    -2_000_038,
];

/// Each exported 2023 wall's, floor's, roof's and ceiling's type and its
/// name, by ElementId (RE-111).
///
/// A 2023 system-family type has no record and no name entry (RE-109). Its
/// name is in its element data, read as on 2024
/// ([`crate::partition_names::find_element_data_names_2023`]). Of the ids
/// an element's record names that have such a name and neither a record
/// nor a name entry, the type is the one whose naming records are of a
/// strict subset of the categories that name each of the others: a wall
/// type is named by walls and the doors and windows they host, a floor
/// type by floors, where the document's other named elements are named by
/// records of more categories. No such one, no type.
fn system_type_names_2023(
    rf: &mut RevitFile,
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> BTreeMap<u32, (u32, String)> {
    let Ok(table) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&table);
    let entries = crate::partition_names::name_entries_2023(rf, &declared);
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let mut named_by: BTreeMap<u32, BTreeSet<i64>> = BTreeMap::new();
    for record in records {
        for &reference in &record.references {
            if let Ok(id) = u32::try_from(reference) {
                if !recorded.contains(&id) && !entries.contains_key(&id) {
                    named_by
                        .entry(id)
                        .or_default()
                        .insert(record.builtin_category);
                }
            }
        }
    }
    let wanted: BTreeSet<u32> = named_by.keys().copied().collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in
            crate::partition_names::find_element_data_names_2023(inflated.bytes(), &wanted)
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
    let mut out = BTreeMap::new();
    for record in records.iter().filter(|record| {
        record.is_exported_instance()
            && SYSTEM_TYPE_CATEGORIES_2023.contains(&record.builtin_category)
    }) {
        let candidates: BTreeSet<u32> = record
            .references
            .iter()
            .filter_map(|&reference| u32::try_from(reference).ok())
            .filter(|id| matches!(names.get(id), Some(Some(_))))
            .collect();
        let most_specific = candidates.iter().copied().filter(|id| {
            candidates.iter().all(|other| {
                other == id
                    || (named_by[id].is_subset(&named_by[other]) && named_by[id] != named_by[other])
            })
        });
        let mut picked = most_specific;
        let (Some(type_id), None) = (picked.next(), picked.next()) else {
            continue;
        };
        if let Some(Some(name)) = names.get(&type_id) {
            out.insert(record.element_id, (type_id, name.clone()));
        }
    }
    out
}

fn type_and_family_names_2023(
    rf: &mut RevitFile,
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> BTreeMap<u32, (u32, String, String)> {
    use crate::partition_element_records::PLACEMENT_KIND_SYMBOL;
    let Ok(table) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let names =
        crate::partition_names::name_entries_2023(rf, &crate::elem_table::declared_ids(&table));
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let mut symbols: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut symbol_records: BTreeMap<
        u32,
        &crate::partition_element_records::PartitionElementRecord,
    > = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.placement_kind == PLACEMENT_KIND_SYMBOL)
    {
        symbols
            .entry(record.builtin_category)
            .or_default()
            .insert(record.element_id);
        symbol_records.insert(record.element_id, record);
    }
    let family_of = |type_id: u32| {
        let record = symbol_records.get(&type_id)?;
        let mut families = record.references.iter().filter_map(|&reference| {
            let id = u32::try_from(reference).ok()?;
            (id != type_id && !recorded.contains(&id))
                .then(|| names.get(&id))
                .flatten()
        });
        let family = families.next()?;
        families.next().is_none().then(|| family.clone())
    };
    let mut out = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.is_exported_instance())
    {
        // The type is anywhere in the list: a type made after the instance
        // has the larger id, and another type can sit between them.
        let Some(set) = symbols.get(&record.builtin_category) else {
            continue;
        };
        let mut types = record.references.iter().filter_map(|&reference| {
            let id = u32::try_from(reference).ok()?;
            (set.contains(&id) && names.contains_key(&id)).then_some(id)
        });
        let (Some(type_id), None) = (types.next(), types.next()) else {
            continue;
        };
        let (Some(type_name), Some(family_name)) = (names.get(&type_id), family_of(type_id)) else {
            continue;
        };
        out.insert(record.element_id, (type_id, type_name.clone(), family_name));
    }
    out
}

/// [`select_instance_records`] for Revit 2023 (RE-107): of an element's
/// frames, the newest, in the highest-numbered partition and then the
/// latest in it. A 2023 project's frames can disagree about their height,
/// where a Level moved after an earlier save: on modelo_bim four columns
/// run from -13.12 ft in `Partitions/1` and from -5.58 ft in
/// `Partitions/2`, and Revit's export draws them from -1.7 m (-5.58 ft),
/// on the Level the newer frame names. The greatest extent would keep the
/// stale one.
fn select_newest_instance_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> std::collections::BTreeMap<u32, crate::partition_element_records::PartitionElementRecord> {
    use crate::partition_element_records::PartitionElementRecord;
    let partition = |record: &PartitionElementRecord| {
        record
            .stream
            .rsplit('/')
            .next()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let mut by_id: BTreeMap<u32, PartitionElementRecord> = BTreeMap::new();
    for record in records {
        if !record.is_exported_instance() || !record.has_volume() {
            continue;
        }
        let newer = by_id.get(&record.element_id).is_none_or(|held| {
            (partition(&record), record.offset) > (partition(held), held.offset)
        });
        if newer {
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
fn without_nested_components(
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
    let mut plates: std::collections::BTreeMap<u32, [f64; 4]> = Default::default();
    for record in scanned.iter().filter(|record| {
        record.builtin_category == per::OST_FLOORS
            || record.builtin_category == per::OST_BUILDING_PAD
    }) {
        let b = record.bbox_feet;
        plates
            .entry(record.element_id)
            .or_insert([b[0], b[1], b[3], b[4]]);
    }
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

/// How closely an outline read from its sketch lines' own curves must span
/// its element's record box (RE-96), feet.
pub const SKETCH_BOX_TOLERANCE_FEET: f64 = 1e-3;

/// Sketched plan profiles of the elements in `owners` (their plan record
/// boxes, `[min x, min y, max x, max y]`), from their Sketch's curve list
/// where it has one (RE-97) or else the lines naming them, and their
/// `OST_SketchLines` records: the RE-25 solve over the records' boxes and,
/// where that does not close, the exact ends each sketch line's own data
/// carries (RE-50, [`crate::partition_beam_axes::scan_bounded_lines`]). A
/// line counts only when it is level and both its ends lie in its own
/// record's box; an element any of whose lines does not count keeps no
/// profile, except that a zero-length line on one of the others' ends is
/// left out (RE-95). A sketch line's curve may be an arc instead (RE-96,
/// [`crate::partition_beam_axes::scan_sketch_curves`]): it counts where it
/// lies in a horizontal plane and every point of it, drawn as chords, lies
/// in its record's box, and where its record does not also hold as a line.
/// An outline with an arc is kept only where it spans its element's record
/// box.
fn sketch_plan_profiles(
    rf: &mut RevitFile,
    revit_version: u32,
    sketch_lines: &[crate::partition_element_records::PartitionElementRecord],
    owners: &std::collections::BTreeMap<u32, [f64; 4]>,
) -> std::collections::BTreeMap<u32, crate::element_record_plan_profiles::PlanProfile> {
    use crate::element_record_plan_profiles as erpp;
    use std::collections::BTreeMap;
    // RE-97: where an element's Sketch lists its curves, that list is the
    // sketch's membership; a line's own owner reference names another
    // element where the sketch was edited.
    let line_ids: BTreeSet<u32> = sketch_lines.iter().map(|r| r.element_id).collect();
    let lists: BTreeMap<u32, Vec<u32>> =
        erpp::scan_sketch_curve_lists(rf, revit_version, &line_ids)
            .into_iter()
            .filter(|(owner, _)| owners.contains_key(owner))
            .collect();
    let listed: BTreeMap<u32, u32> = lists
        .iter()
        .flat_map(|(owner, curves)| curves.iter().map(move |id| (*id, *owner)))
        .collect();
    let remapped: Vec<crate::partition_element_records::PartitionElementRecord> = sketch_lines
        .iter()
        .filter_map(|record| {
            let owner = match listed.get(&record.element_id) {
                Some(owner) => Some(*owner),
                None => record
                    .owner_reference
                    .filter(|owner| !lists.contains_key(owner)),
            }?;
            let mut record = record.clone();
            record.owner_reference = Some(owner);
            Some(record)
        })
        .collect();
    // An element whose list differs from the lines naming it keeps an outline
    // only where it spans its record box, as an arc outline does (RE-96).
    let mut named: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for record in sketch_lines {
        if let Some(owner) = record.owner_reference {
            named.entry(owner).or_default().insert(record.element_id);
        }
    }
    let relisted: BTreeSet<u32> = lists
        .iter()
        .filter(|(owner, curves)| {
            named.get(owner) != Some(&curves.iter().copied().collect::<BTreeSet<u32>>())
        })
        .map(|(owner, _)| *owner)
        .collect();
    let spans_box = |owner: u32, profile: &erpp::PlanProfile| {
        owners.get(&owner).is_some_and(|plan_box| {
            profile.plan_extent_feet().is_some_and(|extent| {
                extent
                    .iter()
                    .zip(plan_box)
                    .all(|(a, b)| (a - b).abs() <= SKETCH_BOX_TOLERANCE_FEET)
            })
        })
    };
    let sketch_lines = remapped.as_slice();
    let mut profiles = erpp::plan_profiles_from_sketch_line_records(sketch_lines);
    profiles.retain(|owner, profile| !relisted.contains(owner) || spans_box(*owner, profile));
    profiles.retain(|owner, _| owners.contains_key(owner));
    let mut unsolved: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in sketch_lines {
        let Some(owner) = record.owner_reference else {
            continue;
        };
        if owners.contains_key(&owner) && !profiles.contains_key(&owner) {
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
    let (Ok(lines), Ok(curves)) = (
        crate::partition_beam_axes::scan_bounded_lines(rf, revit_version, &ids),
        crate::partition_beam_axes::scan_sketch_curves(rf, revit_version, &ids),
    ) else {
        return profiles;
    };
    let arcs: BTreeMap<u32, crate::partition_beam_axes::BoundedArc> = curves
        .iter()
        .filter_map(|(&id, curve)| Some((id, curve.arc?)))
        .collect();
    let eps = erpp::VERTEX_EPS_FEET;
    // RE-95: a sketch line whose box is a single point is a zero-length
    // segment with no line of its own. It adds no edge, and is dropped where
    // its point ends another of the sketch's lines.
    let is_point = |bbox: &[f64; 6]| (0..3).all(|axis| (bbox[axis + 3] - bbox[axis]).abs() <= eps);
    for (owner, segments) in unsolved {
        let ends: Vec<[f64; 3]> = segments
            .keys()
            .filter_map(|id| lines.get(id))
            .flat_map(|line| [line.start(), line.end()])
            .chain(
                segments
                    .keys()
                    .filter_map(|id| arcs.get(id))
                    .flat_map(|arc| [arc.point(arc.start_angle), arc.point(arc.end_angle)]),
            )
            .collect();
        let exact: Option<ExactSketch> = segments
            .iter()
            .filter(|(id, bbox)| {
                let point = [bbox[0], bbox[1], bbox[2]];
                !(is_point(bbox)
                    && !lines.contains_key(id)
                    && !arcs.contains_key(id)
                    && ends
                        .iter()
                        .any(|end| (0..3).all(|axis| (end[axis] - point[axis]).abs() <= eps)))
            })
            .map(|(id, bbox)| {
                let inside = |p: [f64; 3]| {
                    (0..3)
                        .all(|axis| p[axis] >= bbox[axis] - eps && p[axis] <= bbox[axis + 3] + eps)
                };
                let line = lines.get(id).and_then(|line| {
                    let (a, b) = (line.start(), line.end());
                    (inside(a) && inside(b) && (a[2] - b[2]).abs() <= eps)
                        .then(|| vec![[a[0], a[1], b[0], b[1]]])
                });
                let arc = arcs.get(id).and_then(|arc| {
                    let points = erpp::arc_points(arc)?;
                    let flat = arc.x_axis[2].abs() <= eps && arc.y_axis[2].abs() <= eps;
                    (flat && points.iter().all(|&p| inside(p))).then(|| {
                        points
                            .windows(2)
                            .map(|pair| [pair[0][0], pair[0][1], pair[1][0], pair[1][1]])
                            .collect::<Vec<_>>()
                    })
                });
                match (line, arc) {
                    (Some(chords), None) => Some((*id, chords, false)),
                    (None, Some(chords)) => Some((*id, chords, true)),
                    _ => None,
                }
            })
            .collect::<Option<Vec<(u32, Vec<[f64; 4]>, bool)>>>()
            .map(|curves| {
                let has_arc = curves.iter().any(|(_, _, arc)| *arc);
                // RE-151: each line's first chord is an edge of its loop.
                let points = curves
                    .iter()
                    .filter_map(|(id, chords, _)| {
                        let c = chords.first()?;
                        Some((*id, ((c[0] + c[2]) / 2.0, (c[1] + c[3]) / 2.0)))
                    })
                    .collect();
                let chords: Vec<[f64; 4]> = curves
                    .into_iter()
                    .flat_map(|(_, chords, _)| chords)
                    .collect();
                (chords, has_arc, points)
            });
        let Some((mut profile, has_arc, points)) = exact.and_then(|(chords, has_arc, points)| {
            Some((erpp::plan_profile_from_lines(&chords)?, has_arc, points))
        }) else {
            continue;
        };
        // RE-96: an outline with an arc spans the element's own record box,
        // or the sketch holds curves that are not the element's edge.
        if !(has_arc || relisted.contains(&owner)) || spans_box(owner, &profile) {
            erpp::tag_voids(&mut profile, &points);
            profile.segment_ids = segments.keys().copied().collect();
            profiles.insert(owner, profile);
        }
    }
    profiles
}

/// A sketch read from its lines' exact ends: the chords, whether any is an
/// arc's, and each line's ElementId with a point on its edge (RE-151).
type ExactSketch = (Vec<[f64; 4]>, bool, Vec<(u32, (f64, f64))>);

/// Classes whose plan outline is their sketch's: roofs (RE-50) and, since
/// RE-98, ceilings.
pub const SKETCHED_PRODUCT_CLASSES: &[&str] = &["Roof", "Ceiling"];

/// Give each roof and ceiling the plan outline its sketch lines close
/// (RE-50, RE-98), as a floor has (RE-25). The body is the record box's
/// height; a shed roof's slope is attached later ([`attach_roof_slopes`],
/// RE-56).
fn attach_roof_profiles(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_element_records as per;
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let roofs: std::collections::BTreeMap<u32, [f64; 4]> = products
        .iter()
        .filter(|element| SKETCHED_PRODUCT_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| {
            let (x, y) = (
                float(element, "m_locationX")?,
                float(element, "m_locationY")?,
            );
            let (w, d) = (
                float(element, "m_bboxWidth")?,
                float(element, "m_bboxDepth")?,
            );
            Some((
                element.id?,
                [x - w / 2.0, y - d / 2.0, x + w / 2.0, y + d / 2.0],
            ))
        })
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
        .filter(|element| SKETCHED_PRODUCT_CLASSES.contains(&element.class.as_str()))
    {
        if let Some(profile) = roof.id.and_then(|id| profiles.get(&id)) {
            roof.fields.extend(profile.fields());
        }
    }
}

/// One curve of a shaft's sketch as chords, and whether it is a straight
/// line (RE-100).
type ShaftCurve = (Vec<[(f64, f64); 2]>, bool);

/// A shaft's plan outline and the bottom and top of its height, feet
/// (RE-99).
type ShaftOutline = (Vec<(f64, f64)>, (f64, f64));

/// Cross product (square feet) below which a line's end counts as on
/// another line when testing whether two sketch lines cross (RE-99).
pub const SHAFT_CROSSING_EPS: f64 = 1e-6;

/// How far an element may reach past a shaft's height and still be cut by it
/// (RE-99), feet.
pub const SHAFT_HEIGHT_TOLERANCE_FEET: f64 = 1e-3;

/// Cut each shaft opening's outline from the floors, roofs and ceilings its
/// height passes through (RE-99). A shaft (`OST_ShaftOpening`) is sketched
/// as a floor is, and its record box spans its height. An element whose
/// outline is one piece, lies within the shaft's height, and holds the
/// shaft's outline strictly inside its outer loop and clear of its voids
/// takes the outline as a void. An element the shaft only partly overlaps
/// is left as it is.
fn attach_shaft_voids(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 2],
) {
    use crate::element_record_plan_profiles as erpp;
    use crate::partition_element_records as per;
    if !per::supports_revit_version(revit_version) {
        return;
    }
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(scanned) = per::scan_category_records_multi(
        rf,
        revit_version,
        &[per::OST_SHAFT_OPENING, per::OST_SKETCH_LINES],
        &declared,
    ) else {
        return;
    };
    let mut boxes: std::collections::BTreeMap<u32, [f64; 4]> = Default::default();
    let mut heights: std::collections::BTreeMap<u32, (f64, f64)> = Default::default();
    for record in scanned
        .iter()
        .filter(|record| record.builtin_category == per::OST_SHAFT_OPENING)
    {
        let b = record.bbox_feet;
        boxes
            .entry(record.element_id)
            .or_insert([b[0], b[1], b[3], b[4]]);
        heights.entry(record.element_id).or_insert((b[2], b[5]));
    }
    if boxes.is_empty() {
        return;
    }
    let sketch_lines: Vec<per::PartitionElementRecord> = scanned
        .into_iter()
        .filter(|record| record.builtin_category == per::OST_SKETCH_LINES)
        .collect();
    let shafts: Vec<ShaftOutline> = shaft_outlines(rf, revit_version, &sketch_lines, &boxes)
        .into_iter()
        .filter_map(|(id, outline)| Some((outline, *heights.get(&id)?)))
        .collect();
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some(profile) = erpp::plan_profile_from_fields(&element.fields) else {
            continue;
        };
        let (Some(base), Some(height)) = (
            float(element, "m_locationZ"),
            float(element, "m_bboxHeight"),
        ) else {
            continue;
        };
        if !profile.pieces.is_empty() {
            continue;
        }
        let eps = SHAFT_HEIGHT_TOLERANCE_FEET;
        let voids: Vec<Vec<(f64, f64)>> = shafts
            .iter()
            .filter(|(outline, (bottom, top))| {
                base >= bottom - eps
                    && base + height <= top + eps
                    && erpp::loop_within(&profile.outer_xy, outline)
                    && profile
                        .inner_xy
                        .iter()
                        .all(|void| erpp::loops_disjoint(void, outline))
            })
            .map(|(outline, _)| outline.clone())
            .collect();
        if !voids.is_empty() {
            erpp::add_voids_to_fields(&mut element.fields, &voids);
        }
    }
}

/// Each shaft's outline (RE-99), `shafts` its plan record boxes. A shaft's
/// sketch holds its boundary and the symbolic lines drawn across it, the
/// diagonals of the X Revit shows in plan; lines that cross another of the
/// sketch's lines are those, and are left out. The rest must be straight
/// lines read exactly (RE-50), or a full circle (RE-100), that close into one
/// loop spanning the shaft's record box. Its lines are those its Sketch
/// lists (RE-97), or else those naming it.
fn shaft_outlines(
    rf: &mut RevitFile,
    revit_version: u32,
    sketch_lines: &[crate::partition_element_records::PartitionElementRecord],
    shafts: &std::collections::BTreeMap<u32, [f64; 4]>,
) -> std::collections::BTreeMap<u32, Vec<(f64, f64)>> {
    use crate::element_record_plan_profiles as erpp;
    use std::collections::BTreeMap;
    let line_ids: BTreeSet<u32> = sketch_lines.iter().map(|r| r.element_id).collect();
    let lists = erpp::scan_sketch_curve_lists(rf, revit_version, &line_ids);
    let mut members: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for &shaft in shafts.keys() {
        let listed = lists
            .get(&shaft)
            .map(|curves| curves.iter().copied().collect());
        let named = || {
            sketch_lines
                .iter()
                .filter(|r| r.owner_reference == Some(shaft))
                .map(|r| r.element_id)
                .collect()
        };
        members.insert(shaft, listed.unwrap_or_else(named));
    }
    let ids: BTreeSet<u32> = members.values().flatten().copied().collect();
    let (Ok(lines), Ok(circles)) = (
        crate::partition_beam_axes::scan_bounded_lines(rf, revit_version, &ids),
        crate::partition_beam_axes::scan_sketch_circles(rf, revit_version, &ids),
    ) else {
        return BTreeMap::new();
    };
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    // Strictly across: each end of one line clearly off the other, so lines
    // meeting at a corner, whose ends agree only to round-off, do not count.
    let apart = |o: (f64, f64), p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
        let (c1, c2) = (cross(o, p, q), cross(o, p, r));
        c1 * c2 < 0.0 && c1.abs() > SHAFT_CROSSING_EPS && c2.abs() > SHAFT_CROSSING_EPS
    };
    let crossing =
        |[a, b]: [(f64, f64); 2], [c, d]: [(f64, f64); 2]| apart(a, b, c, d) && apart(c, d, a, b);
    let mut out = BTreeMap::new();
    for (shaft, ids) in members {
        // Each line, or a full circle drawn as chords (RE-100); only lines
        // can be the crossing diagonals left out.
        let curves: Option<Vec<ShaftCurve>> = ids
            .iter()
            .map(|id| {
                if let Some(line) = lines.get(id) {
                    let (a, b) = (line.start(), line.end());
                    return ((a[2] - b[2]).abs() <= erpp::VERTEX_EPS_FEET)
                        .then(|| (vec![[(a[0], a[1]), (b[0], b[1])]], true));
                }
                let circle = circles.get(id)?;
                let flat = circle.x_axis[2].abs() <= erpp::VERTEX_EPS_FEET
                    && circle.y_axis[2].abs() <= erpp::VERTEX_EPS_FEET;
                let points = erpp::arc_points(circle).filter(|_| flat)?;
                let chords = points
                    .windows(2)
                    .map(|pair| [(pair[0][0], pair[0][1]), (pair[1][0], pair[1][1])])
                    .collect();
                Some((chords, false))
            })
            .collect();
        let Some(curves) = curves else {
            continue;
        };
        let straight: Vec<[(f64, f64); 2]> = curves
            .iter()
            .filter(|(_, line)| *line)
            .flat_map(|(chords, _)| chords.iter().copied())
            .collect();
        let boundary: Vec<[f64; 4]> = curves
            .iter()
            .filter(|(chords, line)| !*line || !straight.iter().any(|t| crossing(chords[0], *t)))
            .flat_map(|(chords, _)| chords.iter().map(|[a, b]| [a.0, a.1, b.0, b.1]))
            .collect();
        let Some(profile) = erpp::plan_profile_from_lines(&boundary) else {
            continue;
        };
        let spans = profile.plan_extent_feet().is_some_and(|extent| {
            extent
                .iter()
                .zip(&shafts[&shaft])
                .all(|(a, b)| (a - b).abs() <= SKETCH_BOX_TOLERANCE_FEET)
        });
        if spans && profile.inner_xy.is_empty() && profile.pieces.is_empty() {
            out.insert(shaft, profile.outer_xy);
        }
    }
    out
}

/// Fields holding the plan start and end of the one roof edge that defines
/// the roof's slope, model feet (RE-56).
pub const ROOF_SLOPE_EDGE_FIELDS: [&str; 4] = [
    "m_roof_slope_edge_start_x",
    "m_roof_slope_edge_start_y",
    "m_roof_slope_edge_end_x",
    "m_roof_slope_edge_end_y",
];
/// Field holding that edge's slope angle from horizontal, radians (RE-56).
pub const ROOF_SLOPE_ANGLE_FIELD: &str = "m_roof_slope_angle";
/// Field holding the roof's type thickness, its layers summed (RE-56).
pub const ROOF_TYPE_THICKNESS_FIELD: &str = "m_roof_type_thickness";

/// A shed roof's slope: the one edge that defines it, its angle, and the
/// roof's thickness measured square to the slope (RE-56).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoofSlope {
    /// Plan start of the defining edge, model feet.
    pub edge_start: [f64; 2],
    /// Plan end of the defining edge, model feet.
    pub edge_end: [f64; 2],
    /// Slope from horizontal, radians.
    pub angle_radians: f64,
    /// The roof type's layers summed, feet.
    pub thickness_feet: f64,
}

/// The slope [`ROOF_SLOPE_EDGE_FIELDS`], [`ROOF_SLOPE_ANGLE_FIELD`] and
/// [`ROOF_TYPE_THICKNESS_FIELD`] record.
pub fn roof_slope_from_fields(fields: &[(String, InstanceField)]) -> Option<RoofSlope> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [sx, sy, ex, ey] = ROOF_SLOPE_EDGE_FIELDS.map(float);
    Some(RoofSlope {
        edge_start: [sx?, sy?],
        edge_end: [ex?, ey?],
        angle_radians: float(ROOF_SLOPE_ANGLE_FIELD)?,
        thickness_feet: float(ROOF_TYPE_THICKNESS_FIELD)?,
    })
}

/// Give each roof whose sketch lines all carry a slope block, exactly one
/// of them defining the roof's slope, that edge, its angle and its type's
/// thickness (RE-56, [`crate::partition_roof_slopes`]). A roof with no
/// defining edge is flat and gets nothing; one with two or more (a hip or
/// a gable) gets nothing either, as its surfaces are not built.
fn attach_roof_slopes(rf: &mut RevitFile, revit_version: u32, products: &mut [DecodedElement]) {
    use crate::partition_element_records as per;
    use crate::partition_roof_slopes as prs;
    if !prs::ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let roofs: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Roof" && type_of(element).is_some())
        .filter(|element| {
            crate::element_record_plan_profiles::plan_profile_from_fields(&element.fields).is_some()
        })
        .filter_map(|element| element.id)
        .collect();
    if roofs.is_empty() {
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
    let mut edges: std::collections::BTreeMap<u32, BTreeSet<u32>> = Default::default();
    for line in &sketch_lines {
        if let Some(owner) = line.owner_reference.filter(|owner| roofs.contains(owner)) {
            edges.entry(owner).or_default().insert(line.element_id);
        }
    }
    let line_ids: BTreeSet<u32> = edges.values().flatten().copied().collect();
    let Ok(slopes) = prs::scan_edge_slopes(rf, revit_version, &line_ids) else {
        return;
    };
    let mut defining: std::collections::BTreeMap<u32, (u32, f64)> = Default::default();
    for (&roof, lines) in &edges {
        if !lines.iter().all(|line| slopes.contains_key(line)) {
            continue;
        }
        let mut sloped = lines
            .iter()
            .filter_map(|line| slopes.get(line).map(|slope| (*line, slope)))
            .filter(|(_, slope)| slope.defines_slope);
        if let (Some((line, slope)), None) = (sloped.next(), sloped.next()) {
            defining.insert(roof, (line, slope.angle_radians));
        }
    }
    if defining.is_empty() {
        return;
    }
    let defining_lines: BTreeSet<u32> = defining.values().map(|(line, _)| *line).collect();
    let types: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.id.is_some_and(|id| defining.contains_key(&id)))
        .filter_map(type_of)
        .collect();
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
    let (Ok(lines), Ok(layers)) = (
        crate::partition_beam_axes::scan_bounded_lines(rf, revit_version, &defining_lines),
        crate::partition_compound_structure::scan_type_layers(
            rf,
            revit_version,
            &types,
            &materials,
            &declared,
        ),
    ) else {
        return;
    };
    for roof in products
        .iter_mut()
        .filter(|element| element.class == "Roof")
    {
        let (Some(id), Some(type_id)) = (roof.id, type_of(roof)) else {
            continue;
        };
        let Some(&(line_id, angle)) = defining.get(&id) else {
            continue;
        };
        let (Some(line), Some(type_layers)) = (lines.get(&line_id), layers.get(&type_id)) else {
            continue;
        };
        let thickness: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        if !(thickness.is_finite() && thickness > 0.0) {
            continue;
        }
        let (start, end) = (line.start(), line.end());
        for (name, value) in ROOF_SLOPE_EDGE_FIELDS
            .iter()
            .zip([start[0], start[1], end[0], end[1]])
        {
            roof.fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        roof.fields.push((
            ROOF_SLOPE_ANGLE_FIELD.into(),
            InstanceField::Float {
                value: angle,
                size: 8,
            },
        ));
        roof.fields.push((
            ROOF_TYPE_THICKNESS_FIELD.into(),
            InstanceField::Float {
                value: thickness,
                size: 8,
            },
        ));
    }
}

/// Classes whose type's layers stack from the top down (RE-57).
pub const STACKED_LAYER_CLASSES: &[&str] = &["Floor", "BuildingPad", "Roof", "Ceiling"];

/// How closely a slab's type layers must add up to its record box's height
/// for them to be its layers, feet.
pub const SLAB_LAYER_HEIGHT_TOLERANCE_FEET: f64 = 1e-4;

/// The materials layers may name and their names, on `revit_version`:
/// the `OST_Materials` type records and RE-58's names on 2024 and 2025, and
/// RE-113's material objects and names on 2023.
fn materials_and_names(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Option<(BTreeSet<u32>, BTreeMap<u32, String>)> {
    if revit_version == crate::partition_element_records_2023::REVIT_2023 {
        return Some(crate::partition_materials::scan_materials_2023(
            rf, declared,
        ));
    }
    let materials = crate::partition_type_records::scan_type_records(
        rf,
        revit_version,
        crate::partition_type_records::OST_MATERIALS,
        declared,
    )
    .unwrap_or_default()
    .iter()
    .map(|record| record.element_id)
    .collect();
    let names =
        crate::partition_materials::scan_material_names(rf, revit_version, declared).ok()?;
    Some((materials, names))
}

/// Give each floor, building pad, roof and ceiling its type's layers, top
/// first, each with its material's shading (RE-57), where they add up to its
/// record box's height: the plate is then exactly its layers. A roof drawn
/// along its slope (RE-56) is left out.
fn attach_slab_layers(
    rf: &mut RevitFile,
    revit_version: u32,
    mut elements: Vec<&mut DecodedElement>,
) {
    use crate::partition_compound_structure as pcs;
    if pcs::layer_layout(revit_version).is_none() {
        return;
    }
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    elements.retain(|element| {
        STACKED_LAYER_CLASSES.contains(&element.class.as_str())
            && type_of(element).is_some()
            && roof_slope_from_fields(&element.fields).is_none()
    });
    if elements.is_empty() {
        return;
    }
    let types: BTreeSet<u32> = elements
        .iter()
        .filter_map(|element| type_of(element))
        .collect();
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Some((materials, names)) = materials_and_names(rf, revit_version, &declared) else {
        return;
    };
    let (Ok(layers), Ok(appearances)) = (
        pcs::scan_type_layers(rf, revit_version, &types, &materials, &declared),
        crate::partition_materials::scan_material_appearances(rf, revit_version, &declared),
    ) else {
        return;
    };
    // RE-115: a layer that takes its category's material takes the one the
    // document's object styles give the element's category, as a wall's
    // does (RE-91), where it is set and named.
    let mut category_materials: BTreeMap<&str, Option<u32>> = BTreeMap::new();
    for (class, category) in [
        ("Floor", crate::partition_element_records::OST_FLOORS),
        ("Roof", crate::partition_element_records::OST_ROOFS),
        ("Ceiling", crate::partition_element_records::OST_CEILINGS),
    ] {
        let material =
            crate::partition_materials::scan_category_material(rf, revit_version, category)
                .ok()
                .flatten()
                .filter(|id| names.contains_key(id));
        category_materials.insert(class, material);
    }
    for element in elements {
        let Some(type_layers) = type_of(element).and_then(|id| layers.get(&id)) else {
            continue;
        };
        let category_material = category_materials
            .get(element.class.as_str())
            .copied()
            .flatten();
        let type_layers: Vec<pcs::CompoundLayer> = type_layers
            .iter()
            .map(|layer| pcs::CompoundLayer {
                material: layer.material.or(category_material),
                ..*layer
            })
            .collect();
        let type_layers = &type_layers;
        let height = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == "m_bboxHeight" => Some(*value),
            _ => None,
        });
        let total: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        if !height.is_some_and(|h| (h - total).abs() <= SLAB_LAYER_HEIGHT_TOLERANCE_FEET) {
            continue;
        }
        let bands = layer_bands_field(type_layers, &appearances, &names);
        if matches!(&bands, InstanceField::Vector(items) if !items.is_empty()) {
            element.fields.push((SLAB_LAYERS_FIELD.into(), bands));
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
    let joined = crate::element_record_column_cuts::column_joined_walls(&instances, wall_records);
    selected
        .values()
        .map(|record: &PartitionElementRecord| {
            let mut decoded = element_record_decoded(record, "Column", level_ids);
            for &wall in joined.get(&record.element_id).into_iter().flatten() {
                decoded.fields.push((
                    crate::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: wall },
                ));
            }
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

/// Rewrite a beam's plan centre and extent along its run to the
/// column-cut body (RE-122).
fn apply_beam_column_trim(
    decoded: &mut DecodedElement,
    trim: &crate::element_record_beam_cuts::BeamColumnTrim,
) {
    use crate::element_record_beam_cuts as cuts;
    let (centre_field, extent_field) = if trim.axis == 0 {
        ("m_locationX", "m_bboxWidth")
    } else {
        ("m_locationY", "m_bboxDepth")
    };
    for (name, value) in decoded.fields.iter_mut() {
        if name == centre_field {
            *value = InstanceField::Float {
                value: (trim.low_feet + trim.high_feet) * 0.5,
                size: 8,
            };
        } else if name == extent_field {
            *value = InstanceField::Float {
                value: trim.high_feet - trim.low_feet,
                size: 8,
            };
        }
    }
    decoded.fields.push((
        cuts::BEAM_BODY_SOURCE_FIELD.into(),
        InstanceField::String(cuts::BEAM_BODY_COLUMN_CUT.into()),
    ));
}

/// RE-59: give each record-backed element whose record names two Levels
/// its base constraint ([`crate::element_record_level_refs::base_constraint_level`])
/// as its host Level, from the Levels' own elevations (RE-51). Nothing
/// changes where the Levels' elevations are not recovered.
fn resolve_base_constraint_levels(
    elevations: &std::collections::BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    if elevations.is_empty() {
        return;
    }
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        let mut named = Vec::new();
        let mut base = None;
        for (name, value) in &element.fields {
            match (name.as_str(), value) {
                (refs::CONSTRAINT_LEVELS_FIELD, InstanceField::Vector(ids)) => {
                    named = ids
                        .iter()
                        .filter_map(|id| match id {
                            InstanceField::ElementId { id, .. } => Some(*id),
                            _ => None,
                        })
                        .collect();
                }
                ("m_locationZ", InstanceField::Float { value, .. }) => base = Some(*value),
                _ => {}
            }
        }
        let Some(level) = base.and_then(|z| refs::base_constraint_level(&named, elevations, z))
        else {
            continue;
        };
        bind_record_level(element, level, refs::BASE_CONSTRAINT_SOURCE);
    }
}

/// Give a record-backed element `level` as its host Level, recording how.
/// Value of the level-bind source field for a beam bound by its top
/// (`resolve_framing_top_levels`).
pub const FRAMING_TOP_LEVEL_SOURCE: &str = "structural_framing_top_level";

/// Bind each Revit 2023 structural framing element to the one Level at its
/// record box's top, in place of the Level its record names (RE-119). A
/// beam sits under its reference Level: on a 2023 project Revit's export
/// puts four beams spanning 3.2 to 4.0 m on the 4.0 m Level, and four
/// foundation beams spanning -0.8 to 0.0 m on the 0.0 m Level, where their
/// records name the 0.0 m Level and none. Nothing changes where no Level, or
/// more than one, is at the top.
fn resolve_framing_top_levels(
    elevations: &std::collections::BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    const TOLERANCE_FEET: f64 = 1e-3;
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if element.class != "StructuralFraming" {
            continue;
        }
        let float = |wanted: &str| {
            element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let (Some(base), Some(height)) = (float("m_locationZ"), float("m_bboxHeight")) else {
            continue;
        };
        let top = base + height;
        let mut at_top = elevations
            .iter()
            .filter(|(_, elevation)| (**elevation - top).abs() <= TOLERANCE_FEET)
            .map(|(id, _)| *id);
        let (Some(level), None) = (at_top.next(), at_top.next()) else {
            continue;
        };
        element.fields.retain(|(name, _)| {
            name != refs::LEVEL_REFERENCE_FIELD && name != refs::LEVEL_BIND_SOURCE_FIELD
        });
        bind_record_level(element, level, FRAMING_TOP_LEVEL_SOURCE);
    }
}

fn bind_record_level(element: &mut DecodedElement, level: u32, source: &str) {
    use crate::element_record_level_refs as refs;
    for (name, value) in element.fields.iter_mut() {
        if name == "m_level_bound" {
            *value = InstanceField::Bool(true);
        }
    }
    element.fields.push((
        refs::LEVEL_REFERENCE_FIELD.into(),
        InstanceField::ElementId { tag: 0, id: level },
    ));
    element.fields.push((
        refs::LEVEL_BIND_SOURCE_FIELD.into(),
        InstanceField::String(source.into()),
    ));
}

fn element_ids_field(element: &DecodedElement, field: &str) -> Vec<u32> {
    element
        .fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(ids) if name == field => Some(
                ids.iter()
                    .filter_map(|id| match id {
                        InstanceField::ElementId { id, .. } => Some(*id),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn record_level(element: &DecodedElement) -> Option<u32> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. }
            if name == crate::element_record_level_refs::LEVEL_REFERENCE_FIELD =>
        {
            Some(*id)
        }
        _ => None,
    })
}

/// RE-60: a Level for record-backed elements whose record names none.
///
/// - A railing takes the Level of the one stair or ramp its record names,
///   where that has one.
/// - A plumbing fixture takes the Level the objects its record names carry
///   ([`crate::partition_level_records::scan_level_objects`]), where they
///   carry one (RE-68 §6). On Snowdon Towers that is Revit's storey for all
///   4 such fixtures, where their base elevation gives another.
/// - Any other element takes that Level only where it is also the highest
///   Level at or below the element's base
///   ([`crate::element_record_level_refs::level_at_or_below`]). For light
///   fixtures, slab edges and wall sweeps the objects alone name a Level
///   other than Revit's on 73 of the 214 that name one.
fn resolve_hosted_levels(
    rf: &mut RevitFile,
    elevations: &std::collections::BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    if elevations.is_empty() {
        return;
    }
    let pending = elements.iter().flat_map(|list| list.iter()).any(|element| {
        record_level(element).is_none()
            && !element_ids_field(element, refs::REFERENCES_FIELD).is_empty()
    });
    if !pending {
        return;
    }
    let hosts: std::collections::BTreeMap<u32, Option<u32>> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| matches!(element.class.as_str(), "Stair" | "Ramp"))
        .filter_map(|element| Some((element.id?, record_level(element))))
        .collect();
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let level_ids: BTreeSet<u32> = elevations.keys().copied().collect();
    let level_objects =
        crate::partition_level_records::scan_level_objects(rf, &declared, &level_ids);
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if record_level(element).is_some() {
            continue;
        }
        let references = element_ids_field(element, refs::REFERENCES_FIELD);
        if references.is_empty() {
            continue;
        }
        if element.class == "Railing" {
            let named: Vec<&Option<u32>> =
                references.iter().filter_map(|id| hosts.get(id)).collect();
            if let [Some(level)] = named.as_slice() {
                bind_record_level(element, *level, refs::HOST_SOURCE);
            }
            continue;
        }
        let carried: BTreeSet<u32> = references
            .iter()
            .filter_map(|id| level_objects.get(id).copied())
            .collect();
        let base = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == "m_locationZ" => Some(*value),
            _ => None,
        });
        let (Some(&level), 1) = (carried.first(), carried.len()) else {
            continue;
        };
        if element.class == "PlumbingFixture" {
            bind_record_level(element, level, refs::PLUMBING_LEVEL_OBJECT_SOURCE);
            continue;
        }
        let Some(base) = base else {
            continue;
        };
        if refs::level_at_or_below(elevations, base) == Some(level) {
            bind_record_level(element, level, refs::LEVEL_OBJECT_SOURCE);
        }
    }
}

/// A record-backed element's base elevation, feet.
fn record_base_feet(element: &DecodedElement) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == "m_locationZ" => Some(*value),
        _ => None,
    })
}

/// RE-68: a stair or ramp that names no Level, or several (a multistory
/// stair names three), takes the one Level its base sits exactly at, so
/// its railings can follow it (RE-60). On Snowdon Towers these are the
/// three multistory stairs, whose base is L3's elevation; Revit writes
/// each one on L3 and again on L4.
fn resolve_base_at_level(
    elevations: &std::collections::BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if record_level(element).is_some() || !matches!(element.class.as_str(), "Stair" | "Ramp") {
            continue;
        }
        let Some(base) = record_base_feet(element) else {
            continue;
        };
        let mut at = elevations
            .iter()
            .filter(|(_, at)| (**at - base).abs() <= 1e-3);
        if let (Some((&level, _)), None) = (at.next(), at.next()) {
            bind_record_level(element, level, refs::BASE_AT_LEVEL_SOURCE);
        }
    }
}

/// Classes Revit's export places by elevation when their record names no
/// Level of their own (RE-68): on Snowdon Towers every unplaced slab edge,
/// light fixture, wall sweep, generic model, hardscape element and stair is
/// on the Level [`crate::element_record_level_refs::elevation_band_level`]
/// gives, while plumbing fixtures follow the Level object they name instead.
pub const ELEVATION_PLACED_CLASSES: &[&str] = &[
    "SlabEdge",
    "LightingFixture",
    "WallSweep",
    "GenericModel",
    "Hardscape",
    "Stair",
];

/// RE-68: a Level for record-backed elements still without one after RE-59
/// and RE-60.
///
/// - An element takes the Level of the one element of its own class its
///   record names, where that has one: the parts of a nested light-fixture
///   family follow the fixture that holds them.
/// - Otherwise an element of [`ELEVATION_PLACED_CLASSES`] takes the Level
///   its base elevation gives, within a fail-closed band
///   ([`crate::element_record_level_refs::elevation_band_level`]).
///
/// Measured on Snowdon Towers against every copy Revit writes of each
/// element (a multistory stair and its railings appear once per storey).
fn resolve_remaining_levels(
    elevations: &std::collections::BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    if elevations.is_empty() {
        return;
    }
    let leveled: std::collections::BTreeMap<u32, (String, u32)> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter_map(|element| Some((element.id?, (element.class.clone(), record_level(element)?))))
        .collect();
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if record_level(element).is_some() {
            continue;
        }
        let references = element_ids_field(element, refs::REFERENCES_FIELD);
        let same_class: BTreeSet<u32> = references
            .iter()
            .filter_map(|id| leveled.get(id))
            .filter(|(class, _)| *class == element.class)
            .map(|(_, level)| *level)
            .collect();
        let hosts = references
            .iter()
            .filter(|id| {
                leveled
                    .get(id)
                    .is_some_and(|(class, _)| *class == element.class)
            })
            .count();
        if let (1, Some(&level)) = (hosts, same_class.first()) {
            bind_record_level(element, level, refs::SAME_CLASS_HOST_SOURCE);
            continue;
        }
        if !ELEVATION_PLACED_CLASSES.contains(&element.class.as_str()) {
            continue;
        }
        if let Some(level) =
            record_base_feet(element).and_then(|base| refs::elevation_band_level(elevations, base))
        {
            bind_record_level(element, level, refs::BASE_ELEVATION_SOURCE);
        }
    }
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
            CLASS_TAG_FIELD.into(),
            InstanceField::Integer {
                value: i64::from(record.class_tag),
                signed: false,
                size: 2,
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
    if level_id.is_none() {
        // RE-59: base and top constraint, resolved against the Levels'
        // elevations by `resolve_base_constraint_levels`.
        let named = crate::element_record_level_refs::named_levels(&record.references, level_ids);
        if named.is_empty() {
            // RE-60: what the record names instead, resolved by
            // `resolve_hosted_levels`.
            let mut references: Vec<u32> = Vec::new();
            for slot in &record.references {
                if let Ok(id) = u32::try_from(*slot) {
                    if id != record.element_id && !references.contains(&id) {
                        references.push(id);
                    }
                }
            }
            if !references.is_empty() {
                fields.push((
                    crate::element_record_level_refs::REFERENCES_FIELD.into(),
                    InstanceField::Vector(
                        references
                            .iter()
                            .map(|&id| InstanceField::ElementId { tag: 0, id })
                            .collect(),
                    ),
                ));
            }
        }
        if named.len() == 2 {
            fields.push((
                crate::element_record_level_refs::CONSTRAINT_LEVELS_FIELD.into(),
                InstanceField::Vector(
                    named
                        .iter()
                        .map(|&id| InstanceField::ElementId { tag: 0, id })
                        .collect(),
                ),
            ));
        }
    }
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
    // A name that no Level's elevation confirms is not a storey: the names
    // are candidates from partition strings, and on a file whose Levels are
    // not read they are often not the model's (a Revit 2026 house gave
    // `Ground floor`, `Level 1` and `Roof`, none of its four Levels, and a
    // family with no Levels gave `Level 1` and `Roof`), each written at
    // elevation 0 (RE-124). Such a file has no storeys (fail closed).
    if recovery.storeys.is_empty() {
        return Vec::new();
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

/// Every material the file declares, from its `OST_Materials` record
/// (RE-28), with the name (RE-58) and shading colour (RE-53) read from its
/// own object, in ElementId order; on Revit 2023 from its material objects
/// (RE-113, RE-116). A record whose name is not found is left out. `None`
/// on a release those layouts are not measured on, where the partition
/// display-name strings stand in ([`materials_from_names`]).
fn materials_from_records(rf: &mut RevitFile, revit_version: u32) -> Option<Vec<DecodedElement>> {
    use crate::partition_materials as pm;
    let revit_2023 = revit_version == crate::partition_element_records_2023::REVIT_2023;
    if !pm::MATERIALS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) && !revit_2023 {
        return None;
    }
    let declared = crate::elem_table::declared_ids(&crate::elem_table::parse_records(rf).ok()?);
    // Revit 2023 has no material records; its materials are the objects
    // carrying the 2023 material tag (RE-113).
    let (ids, names) = if revit_2023 {
        pm::scan_materials_2023(rf, &declared)
    } else {
        let ids: BTreeSet<u32> = crate::partition_type_records::scan_type_records(
            rf,
            revit_version,
            crate::partition_type_records::OST_MATERIALS,
            &declared,
        )
        .ok()?
        .iter()
        .map(|record| record.element_id)
        .collect();
        (
            ids,
            pm::scan_material_names(rf, revit_version, &declared).ok()?,
        )
    };
    let appearances = pm::scan_material_appearances(rf, revit_version, &declared).ok()?;
    Some(
        ids.into_iter()
            .filter_map(|id| {
                let name = names.get(&id)?;
                let mut fields = vec![("m_name".into(), InstanceField::String(name.clone()))];
                if let Some(appearance) = appearances.get(&id) {
                    fields.push((
                        "m_color".into(),
                        InstanceField::Integer {
                            value: i64::from(appearance.color_packed()),
                            signed: false,
                            size: 4,
                        },
                    ));
                    fields.push((
                        "m_transparency".into(),
                        InstanceField::Float {
                            value: f64::from(appearance.transparency),
                            size: 4,
                        },
                    ));
                }
                fields.push((
                    "m_source".into(),
                    InstanceField::String("partition_materials".into()),
                ));
                Some(DecodedElement {
                    id: Some(id),
                    class: "Material".into(),
                    fields,
                    byte_range: 0..0,
                    provenance: ElementProvenance::partition(
                        "partition",
                        0,
                        "partition_materials",
                        "partition_materials::material_record",
                        0.9,
                        None::<String>,
                    ),
                })
            })
            .collect(),
    )
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
