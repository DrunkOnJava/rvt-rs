//! Schema / partition MVP recovers for production `iter_elements`.
//!
//! Extends the ArcWall-only partition merge with fail-closed recovers
//! for Level and Material, plus `OST_Columns` / `OST_Walls` / `OST_Doors` /
//! `OST_Windows` element records
//! ([`instances_from_partition_category_records`]),
//! `OST_Floors` / `OST_BuildingPad` slab instances
//! ([`slabs_from_partition_category_records`], #212 / RE-22) and
//! `OST_Rooms` room instances
//! ([`rooms_from_partition_category_records`], #90 / RE-29) on the
//! releases whose element records decode.
//!
//! Typed `Wall` / `Door` / `Window` come from the element record's own
//! `BuiltInCategory` field, which names the category outright (#211).
//! The 2024 rows [`crate::rect_opening_index`] decodes are not merged:
//! they are not wall openings (RE-376). Every id they name is a system
//! element (`ConceptualConstructionType`, `GStyleElem`,
//! `FillPatternElem`, ...), none is an element that cuts a wall, and
//! a 2024 file's rectangular wall openings (`SWallRectOpening`) give
//! none of them. Wall openings are written from the walls' own records
//! (RE-321, RE-338).
//!
//! # Version guard
//!
//! Partition byte scans reuse [`crate::partition_scanner`] /
//! [`crate::partition_arc_walls`] version gates. Unsupported releases yield empty lists.
//!
//! # Honesty
//!
//! - Never invent ElementIds, host walls, elevations, or geometry.
//! - Tier1 synthetics have no partition building elements — callers
//!   must observe zero Level/Material/Floor hits there.
//! - Rooms and floors come from element records or not at all. The
//!   name-only rooms and plan-loop floors that once filled in were
//!   removed after they matched nothing in Revit's own exports.

use crate::partition_name_candidates::{building_storey_name_candidates, collect_name_candidates};
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
mod placement_origins;
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
pub use placement_origins::*;
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
    let mut wall_reads = crate::native_walls::WallReads::default();

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

    // --- Revit 2015 to 2023 element records (RE-81, RE-180, RE-181, #421):
    // identity, category, box and the readers called here; nothing else 2024
    // decodes on top of records is read. ---
    if crate::partition_element_records_2023::is_32bit_release(revit_version) {
        let references = recover_2023_records(rf, revit_version, &mut out, &mut wall_reads);
        attach_room_outlines(rf, revit_version, &mut out.rooms);
        // --- Pipes' types and centrelines, from their own native record
        // (RE-370) ---
        attach_native_pipes(rf, revit_version, &mut out.products);
        // --- Ducts' types, lines and sections, from their own native record
        // (RE-371) ---
        attach_native_ducts(rf, revit_version, &mut out.products);
        // --- Which element each connector of a family instance is joined to
        // (RE-171), on 2015 to 2023 too (RE-371) ---
        attach_connector_pairs(rf, revit_version, &mut out.products);
        // --- Storeys from the Levels the records name (RE-107), by the
        // same rules as 2024's (RE-59, RE-68) ---
        let levels = crate::partition_level_records::recover_partition_levels(rf, revit_version)
            .unwrap_or_default();
        let level_elevations: BTreeMap<u32, f64> = levels
            .iter()
            .map(|level| (level.element_id, level.elevation_feet))
            .collect();
        // --- Areas, which carry no record box (#763), and the bodies their
        // boundary lines give them (RE-317, RE-324) ---
        let level_ids = level_element_ids(rf, revit_version).unwrap_or_default();
        out.areas = crate::partition_area_records::areas_from_partition_chain_records(
            rf,
            revit_version,
            &level_ids,
            &levels,
        );
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
        // --- Curtain walls, their parts and those parts' axes (RE-46, RE-106,
        // RE-366), then doors and windows in the nearest wall that is not a
        // curtain wall (#439) ---
        if CURTAIN_32BIT_REVIT_VERSIONS.contains(&revit_version) {
            attach_curtain_walls(
                rf,
                revit_version,
                &mut out.walls,
                &mut out.products,
                [&mut out.doors, &mut out.windows],
                Some(&references),
            );
            attach_curtain_axes(rf, revit_version, &mut out.products);
            attach_curtain_part_types_32(rf, revit_version, &references, &mut out.products);
        }
        bind_opening_hosts(&out.walls, &mut out.doors);
        bind_opening_hosts(&out.walls, &mut out.windows);
        // --- Each door and window in the wall its own native record names
        // (RE-448), as RE-245 does on 2024 to 2027 ---
        rebind_opening_hosts_to_native(rf, &out.walls, [&mut out.doors, &mut out.windows]);
        // --- Each door's and window's opening from the outline its symbol
        // cuts (RE-450), as on 2024 to 2027 where no type opening is read ---
        attach_cut_loop_openings_32(rf, revit_version, [&mut out.doors, &mut out.windows]);
        // --- The Level parameters each element's own native record names
        // (RE-369) ---
        attach_levels_32(
            rf,
            revit_version,
            &levels,
            [
                &mut out.walls,
                &mut out.columns,
                &mut out.doors,
                &mut out.windows,
                &mut out.slabs,
                &mut out.products,
            ],
        );
        // --- Beams along their location lines, the rotation first (RE-276, RE-49,
        // RE-362). A beam a named column trims keeps its cut box (RE-122): both
        // readers skip it, since the trim rewrote the record box they would test
        // a line against. ---
        if crate::partition_beam_axes::beam_axis_read_on(revit_version) {
            attach_beam_rotations(rf, &mut out.products);
            attach_beam_axes(rf, revit_version, &mut out.products);
        }
        // --- A steel beam's I section from its type (RE-103, RE-365) ---
        attach_beam_sections(rf, revit_version, &mut out.products);
        // --- System families' names (RE-63); a wall's, roof's or
        // ceiling's needs its type's layers, not read on 2023 ---
        attach_system_family_names(
            rf,
            revit_version,
            &mut [&mut out.walls, &mut out.slabs, &mut out.products],
        );
        // --- Walls', floors' and roofs' layers, for their material layer
        // sets (RE-114) ---
        attach_wall_layers(rf, revit_version, &mut out.walls, &mut wall_reads);
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
            // A curtain part's type holds no such map: its material is its
            // `m_renderStyleId` (RE-166, RE-367).
            let part = CURTAIN_AXES_CLASSES.contains(&element.class.as_str());
            if let (true, Some(type_id)) = (system || part, type_id) {
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
        // --- Curtain mullions' and panels' materials, from their types (RE-166, RE-367) ---
        attach_curtain_materials(rf, revit_version, &mut out.products);
        // --- A wall's type Function from its native type record (RE-185),
        // the parameters named for IsExternal or a standard property from the
        // saved parameters (RE-193, RE-194), and the IsExternal Revit's
        // exporter calculates from them (RE-283) ---
        read_wall_records(rf, revit_version, &out.walls, &mut wall_reads);
        attach_wall_type_functions(rf, revit_version, out.walls.iter_mut(), &mut wall_reads);
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
        // --- A curtain mullion's or panel's IsExternal: its curtain wall's (RE-193, RE-269) ---
        attach_curtain_member_is_external(&out.walls, &mut out.products);
        // --- Whether a wall's top is joined to a roof or floor, from its
        // native record (RE-228, RE-284) ---
        attach_wall_top_joins(rf, revit_version, &mut out.walls, &mut wall_reads);

        // --- A structural beam's or column's LoadBearing is its category's
        // (RE-285), unless a parameter named for it says otherwise ---
        attach_load_bearing(out.columns.iter_mut().chain(out.products.iter_mut()));
        // --- The phase each element was created in, from its own body
        // (RE-368) ---
        attach_phases_created_32(
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
    let symbol_frames = rf
        .recording_value_sources()
        .then(|| type_symbol_frames(&column_records));
    out.columns = column_instances_from_records(column_records, &level_ids, &wall_instances);
    if let Some(frames) = symbol_frames {
        note_type_sections(rf, &out.columns, &frames);
    }
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
    // --- Areas, which carry no record box (#763), and the bodies their
    // boundary lines give them (RE-317) ---
    let levels = crate::partition_level_records::recover_partition_levels(rf, revit_version)
        .unwrap_or_default();
    out.areas = crate::partition_area_records::areas_from_partition_chain_records(
        rf,
        revit_version,
        &level_ids,
        &levels,
    );

    // --- Other product categories from element records (RE-33) ---
    out.products = product_instances_from_partition_records(rf, revit_version, &level_ids)?;
    // --- Which floors, ceilings and roofs nothing cuts (RE-295) ---
    attach_slab_uncut(rf, &mut [&mut out.slabs, &mut out.products]);
    // --- Base constraints of elements naming two Levels (RE-59), then
    // railings by their host and elements by the Level objects they name
    // (RE-60) ---
    let level_elevations: BTreeMap<u32, f64> = levels
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
        None,
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
    // --- Beams' Cross-Section Rotation, from their native records (RE-276),
    // before the axes, which RE-328's level beams longer than their lines
    // need it for ---
    attach_beam_rotations(rf, &mut out.products);
    // --- Beams along their location lines (RE-49, RE-328) ---
    attach_beam_axes(rf, revit_version, &mut out.products);
    // --- Pipes as cylinders along their ends (RE-131) ---
    attach_pipe_axes(rf, revit_version, &mut out.products);
    // --- Which element each end of a duct or pipe is joined to (RE-138) ---
    attach_connector_pairs(rf, revit_version, &mut out.products);
    // --- Roof outlines from their sketch lines (RE-50) ---
    attach_roof_profiles(rf, revit_version, &mut out.products);
    // --- Shaft openings cut the outlines within their height (RE-99; 2024 only, RE-345) ---
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
    // --- Ducts' types, lines and sections, from their own native record,
    // before RE-134's scan (RE-371) ---
    attach_native_ducts(rf, revit_version, &mut out.products);
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
    attach_native_type_names(rf, revit_version, &mut out.products);
    // --- Curtain mullions drawn as an extrusion of their section (RE-243) ---
    attach_mullion_plain_hosts(rf, &mut out.products);
    // --- Walls' layers and exterior side (RE-53) ---
    attach_wall_layers(rf, revit_version, &mut out.walls, &mut wall_reads);
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
    // --- Family instances written as spaces that have no 3D body are left
    // out, as Revit leaves them out (RE-312, #1262) ---
    without_bodiless_family_spaces(rf, &mut out.products);
    // --- Empty curtain panels are left out, as Revit leaves them out (#309) ---
    out.products
        .retain(|product| !is_empty_curtain_panel(product));
    // --- Plumbing fixtures whose type draws nothing visible are left out, as
    // Revit leaves them out (RE-363, #1496) ---
    without_invisible_plumbing_fixture_types(rf, revit_version, &mut out.products);
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
    // --- Roofs whose underside the walls under them are cut along (#1478) ---
    if SLOPED_ROOF_WALL_TOP_REVIT_VERSIONS.contains(&revit_version) {
        for roof in out
            .slabs
            .iter_mut()
            .chain(out.products.iter_mut())
            .filter(|element| element.class == "Roof")
        {
            roof.fields
                .push((ROOF_CUTS_WALL_TOPS_FIELD.into(), InstanceField::Bool(true)));
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
    let (partly_unset, unread_types) =
        refine_type_materials(rf, revit_version, &mut type_materials, &out);
    // #1247: these doors and windows are written as such, though nothing read
    // tells whether their type draws geometry; the diagnostics count them.
    if !unread_types.is_empty() {
        let ids: Vec<u32> = out
            .doors
            .iter()
            .chain(&out.windows)
            .filter(|element| {
                element.fields.iter().any(|(name, value)| {
                    matches!(value, InstanceField::ElementId { id, .. }
                        if name == TYPE_ID_FIELD && unread_types.contains(id))
                })
            })
            .filter_map(|element| element.id)
            .collect();
        rf.note_unread_type_doors_windows(ids);
    }
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
                // The hollow section's four quantities are written on the
                // releases they are measured on only (#1945).
                if HOLLOW_SECTION_QUANTITIES_REVIT_VERSIONS.contains(&revit_version) {
                    element.fields.push((
                        COLUMN_HOLLOW_QUANTITIES_FIELD.into(),
                        InstanceField::Bool(true),
                    ));
                }
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
                element.fields.push((
                    COLUMN_RELEASE_FIELD.into(),
                    InstanceField::Integer {
                        value: i64::from(revit_version),
                        signed: false,
                        size: 4,
                    },
                ));
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
    // --- The X axis of the placement Revit gives a door or window set in a
    // plain wall (RE-347), after the flips and the host are final ---
    attach_opening_placement_axes(
        rf,
        revit_version,
        &out.walls,
        [&mut out.doors, &mut out.windows],
    );
    // --- The origin of the placement Revit gives a door set in no wall
    // (RE-361), after the host is final ---
    attach_opening_placement_origins(rf, revit_version, &mut out.doors);
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
        &mut wall_reads,
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
    // --- A structural beam's or column's LoadBearing is its category's
    // (RE-285), unless a parameter named for it says otherwise ---
    attach_load_bearing(out.columns.iter_mut().chain(out.products.iter_mut()));
    // --- A wall's LoadBearing: its Structural box set and its usage
    // Bearing, from its native record, curtain panel walls included (RE-429) ---
    attach_wall_load_bearing(
        rf,
        revit_version,
        out.walls.iter_mut().chain(
            out.products
                .iter_mut()
                .filter(|product| product.class == CURTAIN_PANEL_WALL_CLASS),
        ),
        &mut wall_reads,
    );
    // --- A floor's or stair's LoadBearing from its Structural check box
    // (RE-429), where no category or parameter named for it gives one ---
    attach_structural_load_bearing(
        rf,
        revit_version,
        out.slabs.iter_mut().chain(out.products.iter_mut()),
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
    // RE-454: a beam's or brace's Level is the one its record's second counted
    // reference list names where that names any, else the first's (RE-174), the
    // list its Reference Level is read from: Revit puts it in that Level's
    // storey. The first list alone gave 63 beams of each BIM4LCA timber office a
    // storey other than Revit's.
    let level_list: &[u64] = if class == "StructuralFraming"
        && !crate::element_record_level_refs::named_levels(&record.second_references, level_ids)
            .is_empty()
    {
        &record.second_references
    } else {
        &record.references
    };
    let level_id = crate::element_record_level_refs::unique_level_reference(level_list, level_ids);
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
        let named = crate::element_record_level_refs::named_levels(level_list, level_ids);
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
        // A beam or brace that names more than two keeps them too: its
        // Reference Level is the highest (RE-175, RE-454).
        if named.len() == 2 || (named.len() > 2 && class == "StructuralFraming") {
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
