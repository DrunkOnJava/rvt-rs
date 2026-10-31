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

use crate::partition_name_candidates::{building_storey_name_candidates, collect_name_candidates};
use crate::rect_opening_index::ArcWallRectOpeningIndex;
use crate::walker::{DecodedElement, ElementProvenance, InstanceField, WalkerLimits};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

mod curtain;
mod fields;
mod framing;
mod layers;
mod levels;
mod materials;
mod mep;
mod mullion_ends;
mod openings;
mod parameters;
mod records;
mod records_2023;
mod roofs;
mod rooms;
mod slabs;
mod stair_bodies;
mod stairs;
mod types;
mod wall_joins;
mod walls;

pub use curtain::*;
pub use fields::*;
pub use framing::*;
pub use layers::*;
pub use levels::*;
pub use materials::*;
pub use mep::*;
pub use mullion_ends::*;
pub use openings::*;
use parameters::*;
pub use records::*;
use records_2023::*;
pub use roofs::*;
pub use rooms::*;
pub use slabs::*;
pub use stair_bodies::*;
use stairs::*;
pub use types::*;
pub use wall_joins::*;
pub use walls::*;

/// Bundle of partition-derived MVP `DecodedElement`s.
#[derive(Debug, Clone, Default)]
pub struct PartitionSchemaMvp {
    pub levels: Vec<DecodedElement>,
    pub materials: Vec<DecodedElement>,
    pub rooms: Vec<DecodedElement>,
    /// Placed Areas from their partition chain records, written as
    /// `IfcSpace` (#763, [`crate::partition_area_records`]).
    pub areas: Vec<DecodedElement>,
    /// 2024 ArcWallRectOpening index rows — not typed Door/Window.
    pub rect_openings: Vec<DecodedElement>,
    /// 2024 `OST_Columns` partition element records (#204).
    // Tracker: M4-09.
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
                + self.areas.len()
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
        out.extend(self.areas);
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

    let arc_walls = rf.memoised_arc_wall_scan(revit_version, limits);

    out.levels = levels_from_storeys_and_names(&arc_walls.walls, &level_names);
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

    // --- Revit 2015 to 2023 element records (RE-81, RE-180, RE-181, #421):
    // identity, category and box only; nothing else 2024 decodes on top of
    // records is read. ---
    if crate::partition_element_records_2023::is_32bit_release(revit_version) {
        recover_2023_records(rf, revit_version, &mut out);
        attach_room_outlines(rf, revit_version, &mut out.rooms);
        // --- Storeys from the Levels the records name (RE-107), by the
        // same rules as 2024's (RE-59, RE-68) ---
        let level_elevations: BTreeMap<u32, f64> =
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
        let mut type_materials =
            crate::partition_type_materials::type_material_names_32(rf, revit_version);
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
        // --- A wall's type Function from its native type record (RE-185),
        // the parameters named for IsExternal or a standard property from the
        // saved parameters (RE-193, RE-194), and the IsExternal Revit's
        // exporter calculates from them (RE-283) ---
        attach_wall_type_functions(rf, revit_version, out.walls.iter_mut());
        attach_native_named_parameters(
            rf,
            revit_version,
            out.walls
                .iter_mut()
                .chain(out.columns.iter_mut())
                .chain(out.doors.iter_mut())
                .chain(out.windows.iter_mut())
                .chain(out.slabs.iter_mut())
                .chain(out.rooms.iter_mut())
                .chain(out.products.iter_mut()),
        );
        attach_record_is_external(
            out.walls
                .iter_mut()
                .chain(out.columns.iter_mut())
                .chain(out.doors.iter_mut())
                .chain(out.windows.iter_mut())
                .chain(out.slabs.iter_mut())
                .chain(out.rooms.iter_mut())
                .chain(out.products.iter_mut()),
        );
        // --- Whether a wall's top is joined to a roof or floor, from its
        // native record (RE-228, RE-284) ---
        attach_wall_top_joins(rf, revit_version, &mut out.walls);
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
    // Their heights from their native records (RE-255).
    attach_room_heights(rf, revit_version, &mut out.rooms);
    // --- Areas, which carry no record box (#763) ---
    out.areas = crate::partition_area_records::areas_from_partition_chain_records(
        rf,
        revit_version,
        &level_ids,
    );

    // --- Other product categories from element records (RE-33) ---
    out.products = product_instances_from_partition_records(rf, revit_version, &level_ids)?;
    // --- Which floors, ceilings and roofs nothing cuts (RE-295) ---
    attach_slab_uncut(rf, &mut [&mut out.slabs, &mut out.products]);
    // --- Base constraints of elements naming two Levels (RE-59), then
    // railings by their host and elements by the Level objects they name
    // (RE-60) ---
    let level_elevations: BTreeMap<u32, f64> =
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
    attach_curtain_walls(
        rf,
        revit_version,
        &mut out.walls,
        &mut out.products,
        [&mut out.doors, &mut out.windows],
    );
    // --- Doors and windows in the nearest listed wall that is not a curtain wall (#439) ---
    bind_opening_hosts(&out.walls, &mut out.doors);
    bind_opening_hosts(&out.walls, &mut out.windows);
    // --- Stair and flight riser and tread dimensions (RE-47) ---
    attach_stair_dimensions(rf, revit_version, &mut out.products);
    // --- Stair runs' treads and risers, and their run type (RE-52) ---
    attach_stair_run_bodies(rf, revit_version, &mut out.products);
    attach_stair_nosing_lengths(&mut out.products);
    // --- Spiral runs as their treads (#357) ---
    attach_spiral_run_bodies(rf, revit_version, &mut out.products);
    // --- Monolithic landings as their slabs (#824) ---
    attach_landing_slabs(rf, revit_version, &mut out.products);
    // --- Beams along their location lines (RE-49) ---
    attach_beam_axes(rf, revit_version, &mut out.products);
    // --- Beams' Cross-Section Rotation, from their native records (RE-276) ---
    attach_beam_rotations(rf, &mut out.products);
    // --- Pipes as cylinders along their ends (RE-131) ---
    attach_pipe_axes(rf, revit_version, &mut out.products);
    // --- Which element each end of a duct or pipe is joined to (RE-138) ---
    attach_connector_pairs(rf, revit_version, &mut out.products);
    // --- Roof outlines from their sketch lines (RE-50) ---
    attach_roof_profiles(rf, revit_version, &mut out.products);
    // --- Shaft openings cut the outlines within their height (RE-99) ---
    attach_shaft_voids(rf, revit_version, [&mut out.slabs, &mut out.products]);

    // --- The Revit class each record names at +0x4a (RE-76, #154) ---
    if let Ok(classes) = rf.memoised_schema_classes() {
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
    // --- Each door's and window's facing, for its type's flip (B72) ---
    attach_opening_facings(rf, revit_version, [&mut out.doors, &mut out.windows]);
    // --- Each family instance's original symbol, whose GlobalId Revit's
    // export gives its type (RE-167, B60) ---
    attach_original_symbols(
        rf,
        revit_version,
        [
            &mut out.doors,
            &mut out.windows,
            &mut out.columns,
            &mut out.products,
        ],
    );

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
    // --- The phase each element was created in (spike 257) ---
    attach_phases_created(
        rf,
        [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.products,
        ],
    );
    // --- Pipes' types, which have no name entry (RE-130) ---
    attach_pipe_type_names(rf, &mut out.products);
    // --- Ducts' and pipes' sizes, and ducts' types (RE-134) ---
    attach_curve_fields(rf, revit_version, &mut out.products);
    // --- Which connector each end of a duct is (#847) ---
    attach_duct_end_keys(rf, revit_version, &mut out.products);
    // --- Pipe fittings' nominal sizes, from their connectors (RE-165) ---
    attach_fitting_nominal_diameters(rf, revit_version, &mut out.products);
    // --- The shared parameter Serial Number (RE-156) ---
    attach_serial_numbers(
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
    // --- The MEP systems each element is a member of (RE-162) ---
    attach_mep_systems(
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
    // --- Railings' heights (RE-231) ---
    attach_railing_heights(rf, revit_version, &mut out.products);
    // --- Stair railings' Length (RE-254) ---
    attach_railing_lengths(rf, revit_version, &mut out.products);
    // --- Each element's type Function (RE-158) ---
    attach_type_functions(
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
    // --- Pipes, wall sweeps and ramps typed in their own record (RE-240) ---
    attach_native_type_names(rf, &mut out.products);
    // --- Curtain mullions drawn as an extrusion of their section (RE-243) ---
    attach_mullion_plain_hosts(rf, &mut out.products);
    // --- Walls' layers and exterior side (RE-53) ---
    attach_wall_layers(rf, revit_version, &mut out.walls);
    // --- The openings a wall's edited elevation profile cuts (B55) ---
    attach_wall_profile_openings(rf, revit_version, &mut out.walls);
    // --- A shed roof's slope (RE-56) ---
    attach_roof_slopes(rf, revit_version, &mut out.products);
    // --- A roof by extrusion's section (RE-211) ---
    attach_extrusion_roofs(rf, revit_version, &mut out.products);
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
    attach_native_stair_names(rf, revit_version, &mut out.products);
    // --- Stair runs' shape, from their treads, and each stair's type from
    // its runs and landings, once every part names its stair (#323) ---
    attach_stair_run_shapes(rf, revit_version, &mut out.products);
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

    // --- Column bottoms from the geometry node (RE-173) ---
    if revit_version == 2024 {
        apply_column_node_bottoms(rf, [&mut out.columns, &mut out.products]);
        apply_column_cut_bodies(rf, [&mut out.columns, &mut out.products]);
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

    // --- Level parameters from the per-element association record (RE-172),
    // stairs' Base and Top Level from the same record and the Level named in
    // the element record's id lists where the field holds none (RE-174) ---
    let (level_parameters, (level_names, level_elevations)) =
        crate::partition_level_parameters::level_parameters(rf, revit_version);
    if !level_names.is_empty() {
        let mut lists = [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.products,
        ];
        for elements in lists.iter_mut() {
            for element in elements.iter_mut() {
                attach_level_parameters(
                    rf,
                    element,
                    &level_parameters,
                    &level_names,
                    &level_elevations,
                );
            }
        }
    }

    // --- The materials each family type's geometry uses (RE-82, #355) ---
    let mut type_materials =
        crate::partition_type_materials::type_material_names(rf, revit_version);
    // The map read by name, where the scan also takes a nested component's
    // materials (RE-203).
    let partly_unset = refine_type_materials(rf, revit_version, &mut type_materials, &out);
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
    if !partly_unset.is_empty() {
        let instance_materials =
            instance_parameter_material_names(rf, revit_version, &partly_unset, &out);
        for elements in [&mut out.columns, &mut out.products] {
            attach_partly_unset_materials(&partly_unset, &instance_materials, elements);
        }
    }

    // --- Every column's joins, in one native pass (RE-202, RE-247) ---
    let column_ids: BTreeSet<u64> = out
        .columns
        .iter()
        .chain(out.products.iter())
        .filter(|element| is_column_class(&element.class))
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    let column_joins = crate::native_column_joins::joins(rf, &column_ids).ok();

    // --- Family instances whose type draws with no material (RE-149, #355) ---
    // An instance whose own material parameters name its materials takes
    // them instead (RE-200).
    let unset_types = crate::partition_type_materials::unset_material_types(rf, revit_version);
    if !unset_types.is_empty() {
        let instance_materials =
            instance_parameter_material_names(rf, revit_version, &unset_types, &out);
        for elements in [&mut out.columns, &mut out.products] {
            attach_unnamed_materials(&unset_types, &instance_materials, elements);
        }
        // The first joined wall's layer on the column's side (RE-202), then
        // the joined walls' one shared set (#580).
        attach_first_join_materials(
            rf,
            column_joins.as_ref().map(|joins| &joins.order),
            &mut out.columns,
            &out.walls,
        );
        attach_joined_wall_materials(&mut out.columns, &out.walls);
    }
    // --- Columns no `CleanPairGStep` joins, which Revit's export draws as
    // an extrusion (RE-247) ---
    if let Some(joins) = &column_joins {
        for element in out.columns.iter_mut().chain(out.products.iter_mut()) {
            if is_column_class(&element.class)
                && element
                    .id
                    .is_some_and(|id| joins.without_clean_steps.contains(&u64::from(id)))
            {
                element
                    .fields
                    .push((COLUMN_UNJOINED_FIELD.into(), InstanceField::Bool(true)));
            }
        }
        // --- The elements each column's pair steps name (RE-291) ---
        for element in out.columns.iter_mut().chain(out.products.iter_mut()) {
            if !is_column_class(&element.class) {
                continue;
            }
            let Some(id) = element.id.map(u64::from) else {
                continue;
            };
            let named = |partners: &BTreeMap<u64, BTreeSet<u64>>| {
                partners
                    .get(&id)?
                    .iter()
                    .map(|&other| {
                        Some(InstanceField::ElementId {
                            tag: 0,
                            id: u32::try_from(other).ok()?,
                        })
                    })
                    .collect::<Option<Vec<InstanceField>>>()
            };
            if let (Some(cut), Some(clean)) =
                (named(&joins.cut_partners), named(&joins.clean_partners))
            {
                element
                    .fields
                    .push((COLUMN_CUT_PARTNERS_FIELD.into(), InstanceField::Vector(cut)));
                element.fields.push((
                    COLUMN_CLEAN_PARTNERS_FIELD.into(),
                    InstanceField::Vector(clean),
                ));
            }
        }
    }

    // --- Curtain mullions' and panels' materials, from their types (RE-166, B66) ---
    attach_curtain_materials(rf, revit_version, &mut out.products);

    // --- Rendered content (RPC, planting) is `<Unnamed>` (RE-202, #355) ---
    attach_rendered_content_materials(rf, &mut out.products);

    // --- Each window's opening from its type and transform (RE-93, #227) ---
    attach_window_openings(rf, revit_version, &mut out.windows);
    // --- Each door's rough opening from its type (RE-94, #227), after RE-84
    // marks the doors that are openings alone ---
    attach_door_openings(rf, revit_version, &mut out.doors);
    // --- Each door's Width and Height from its type (RE-187, #759) ---
    attach_door_sizes(rf, revit_version, &mut out.doors);
    // --- Each door's and window's size from the outline its symbol cuts
    // (RE-215, #844) ---
    let walls: BTreeSet<u32> = out.walls.iter().filter_map(|wall| wall.id).collect();
    let plain_walls: BTreeSet<u32> = out
        .walls
        .iter()
        .filter(|wall| wall.class != CURTAIN_WALL_CLASS)
        .filter_map(|wall| wall.id)
        .collect();
    attach_cut_loop_sizes(
        rf,
        revit_version,
        [&mut out.doors, &mut out.windows],
        &walls,
        &plain_walls,
    );
    // --- Steel beams' and columns' I sections from their type (RE-103, RE-104) ---
    attach_beam_sections(rf, revit_version, &mut out.products);
    // --- Hollow structural sections from their type's family parameters (W6) ---
    attach_column_sections(rf, revit_version, [&mut out.columns, &mut out.products]);
    // --- A wall's and a curtain panel wall's type Function, from its native
    // type record (RE-185), after RE-72 has relabelled the panels ---
    attach_wall_type_functions(
        rf,
        revit_version,
        out.walls.iter_mut().chain(
            out.products
                .iter_mut()
                .filter(|product| product.class == CURTAIN_PANEL_WALL_CLASS),
        ),
    );
    // --- IsExternal: the parameters named for it, and Revit's calculation (RE-193) ---
    attach_is_external(
        rf,
        revit_version,
        [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.rooms,
            &mut out.areas,
            &mut out.products,
        ],
    );
    // --- A curtain mullion's or panel's IsExternal: its curtain wall's (RE-193, RE-269) ---
    attach_curtain_member_is_external(&out.walls, &mut out.products);
    // --- The parameters named for a standard set's other properties (RE-194) ---
    attach_named_parameters(
        rf,
        revit_version,
        [
            &mut out.walls,
            &mut out.columns,
            &mut out.doors,
            &mut out.windows,
            &mut out.slabs,
            &mut out.rooms,
            &mut out.areas,
            &mut out.products,
        ],
    );
    Ok(out)
}

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

fn element_source_stream(element: &DecodedElement) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(stream) if name == "m_source_stream" => Some(stream.clone()),
        _ => None,
    })
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
