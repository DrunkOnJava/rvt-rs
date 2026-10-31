//! Research readers: the partition, native and element-record decoders
//! behind the format findings in `reports/`.
//!
//! Available with the `unstable-research` feature. Everything here changes
//! whenever a finding does, in any release, and is outside the semver promise
//! the rest of the crate makes (`RELEASE.md`, "What 1.0 keeps stable"). The
//! stable API reads through these modules internally, so turning the feature
//! on adds paths to them and changes nothing else.
//!
//! Each module states the fact it reads and the report that measures it:
//! `RE-NN` is
//! [`reports/element-framing/RE-NN-*.md`](https://github.com/DrunkOnJava/rvt-rs/tree/main/reports/element-framing)
//! in the repository, and `#NNN` an issue or pull request there.
//!
//! ## Partition records
//!
//! - [`partition_arc_walls`]: Shared partition ArcWall iteration (RE-15)
//! - [`partition_area_records`]: Revit 2024 Areas from their partition chain records (#763)
//! - [`partition_beam_axes`]: Beam location lines from their serialised element data (RE-49)
//! - [`partition_column_nodes`]: A column's true z-range from its instance geometry node (RE-173)
//! - [`partition_compound_structure`]: Host types' layers and which side of a wall is its exterior (RE-53)
//! - [`partition_connector_pairs`]: Which element each end of a duct or pipe is joined to (RE-138, #528)
//! - [`partition_curtain_materials`]: A curtain wall mullion's and panel's material, from its type's data object (RE-166)
//! - [`partition_curve_fields`]: A duct's or pipe's size and type, from its curve object (RE-134, #96, #35)
//! - [`partition_design_options`]: Design option sets and their primary options (#319, RE-40)
//! - [`partition_element_records`]: Partition element-record headers
//! - [`partition_element_records_2023`]: Revit 2023 partition element records (RE-81, #421), and Revit 2008 to 2022's, which take the same layout (RE-180, RE-181, RE-259, RE-260)
//! - [`partition_fitting_sizes`]: A pipe fitting's nominal size, from its own data object (RE-165, #35)
//! - [`partition_id_objects`]: Serialised objects keyed by their ElementId, found in one pass
//! - [`partition_ifc_export_overrides`]: Per-element IFC export-type overrides carried in the Revit 2024 partition parameter blocks (#212, RE-22)
//! - [`partition_instance_transforms`]: A family instance's transform, from its own element data (RE-87)
//! - [`partition_level_parameters`]: Level parameters of an element, from its per-element association record (RE-172)
//! - [`partition_level_records`]: Revit `Level` elements recovered from the partition streams (#218, RE-24)
//! - [`partition_materials`]: Material shading colours and transparency (RE-53)
//! - [`partition_mep_systems`]: MEP systems and their members (RE-162, #528)
//! - [`partition_name_candidates`]: Partition UTF-16LE display-name candidates (RE-15, #86)
//! - [`partition_names`]: Family and type names from partition name entries (RE-38)
//! - [`partition_phases`]: Phase names, and the phase an element was created in (#755)
//! - [`partition_pipe_axes`]: Pipes as their cylinders, from their connector entries (RE-131, #96)
//! - [`partition_roof_slopes`]: Roof edges' slopes (RE-56)
//! - [`partition_room_boundaries`]: Room plan outlines from the faces of each room's stored solid (RE-101)
//! - [`partition_room_parameters`]: Room number / name / host Level from the Revit 2024 partition parameter block a room owns (#90, RE-29)
//! - [`partition_scanner`]: Generic partition-stream record scanner
//! - [`partition_schema_mvp`]: Schema / partition MVP recovers for production `iter_elements`
//! - [`partition_stairs`]: Stair and stair-run dimensions from their serialised element data (RE-47)
//! - [`partition_type_materials`]: The materials a family type's geometry uses (#355, RE-82)
//! - [`partition_type_parameters`]: Text parameters of element types, stored under their BuiltInParameter (#35, RE-77)
//! - [`partition_type_records`]: Revit *type* records recovered from the partition streams (#88, RE-28)
//!
//! ## Element records
//!
//! - [`element_record_beam_cuts`]: Beam bodies on Revit 2023
//! - [`element_record_column_cuts`]: Column bodies
//! - [`element_record_level_refs`]: Level ElementId binding from a partition element record's counted reference list (#219, RE-27)
//! - [`element_record_plan_profiles`]: Plan profiles for sketched elements, from `OST_SketchLines` partition element records (#31, RE-25)
//! - [`element_record_storeys`]: Storey elevations recovered from partition element-record bboxes
//! - [`element_record_wall_joins`]: Wall bodies
//!
//! ## Native records
//!
//! - [`native_column_joins`]: The wall a column is first joined to, and the side of it the column stands on (RE-202, #355)
//! - [`native_connectors`]: A family symbol's connectors, from its own native record and its family's (#528)
//! - [`native_document`]: Streaming current-record extraction with structural schemas and native IDs
//! - [`native_duct_shapes`]: A duct type's shape, from its native record
//! - [`native_element`]: Bounded native Element base fields, using the structurally decoded schema
//! - [`native_element_parameters`]: An element's saved parameter values, named by the file's definitions, type values after its own (#35, RE-207)
//! - [`native_element_types`]: The type a pipe, wall sweep or ramp names in its own native record (RE-240)
//! - [`native_empty_faces`]: Strict classifier for the observed empty-trim saved-graphics records
//! - [`native_es_catalog`]: File-local Extensible Storage catalog from the owned Global/Latest graph
//! - [`native_extensible_storage`]: File-local Extensible Storage definitions
//! - [`native_family_forms`]: Whether a loaded family draws visible geometry with no material, from its embedded document (RE-204, #355)
//! - [`native_graphics_traversal`]: Bounded traversal of saved graphics under the default 3D context
//! - [`native_index`]: Native project identities and creation-episode UniqueIds
//! - [`native_instance_materials`]: The materials a family instance's own parameters give it (RE-200, #355)
//! - [`native_metadata`]: Lossless projections of selected saved metadata from a complete native graph
//! - [`native_mullion_hosts`]: Which curtain mullions Revit's export draws as an extrusion of their section (RE-243)
//! - [`native_parameter_definitions`]: Parameter definitions recovered through native owner-to-definition edges
//! - [`native_parameters`]: Schema-directed native value prefixes and deferred parameter sets
//! - [`native_parametric_mesh`]: Bounded UV-grid meshing for decoded parametric surfaces
//! - [`native_parametric_surface`]: Typed evaluators for the non-planar surfaces observed in saved graphs
//! - [`native_phases`]: The project's phase order, and the elements a last-phase export leaves out (RE-184, #328)
//! - [`native_rendered_content`]: Family types whose family is rendered content (RE-202, #355)
//! - [`native_saved_mesh`]: Tessellation of bounded saved planar and cylindrical BRep faces
//! - [`native_saved_scene`]: Current native saved-graphics extraction with explicit shared-symbol closure
//! - [`native_segments`]: Forward-only physical partition markers and continuation groups
//! - [`native_symbol_cuts`]: The outline a door or window cuts in its host, from the native record of the symbol that draws it (RE-215)
//! - [`native_stairs`]: A Revit 2025 stair run's sketch and type, read by member name from its native records (#357)
//! - [`native_type_materials`]: The materials a family type's geometry is drawn in (RE-203, #355)
//! - [`native_wall_types`]: A wall's type Function, from the native records of the wall and its type (RE-185, RE-193)
//!
//! ## Other research
//!
//! - [`es_refs`]: ES reference occurrence locator contracts, with no decoder yet
//! - [`evidence`]: Named evidence tiers and lightweight ledgers
//! - [`identity`]: Document-scoped identity and source-span contracts
//! - [`rect_opening_index`]: Version-gated decoder for Revit 2024 ArcWallRectOpening 60 B index records (RE-15, #89)
//! - [`revit_global_ids`]: The IFC `GlobalId` Revit's own exporter gives an element, rebuilt from the file (RE-48)
//! - [`schema_registry`]: Forward-only Formats/Latest registry framing, without byte-name scanning
// Tracker: D20, B66, B44, B33, M3-03, M3-04, B54, B59, B43, L5B-10.

pub mod element_record_beam_cuts;
pub mod element_record_column_cuts;
pub mod element_record_level_refs;
pub mod element_record_plan_profiles;
pub mod element_record_storeys;
pub mod element_record_wall_joins;
pub mod es_refs;
pub mod evidence;
pub mod identity;
pub mod native_column_joins;
pub mod native_connectors;
pub mod native_document;
pub mod native_duct_shapes;
pub mod native_element;
pub mod native_element_parameters;
pub mod native_element_types;
pub mod native_empty_faces;
pub mod native_es_catalog;
pub mod native_extensible_storage;
pub mod native_family_forms;
pub mod native_graphics_traversal;
pub mod native_index;
pub mod native_instance_materials;
pub(crate) mod native_json;
pub mod native_metadata;
pub mod native_mullion_hosts;
pub mod native_parameter_definitions;
pub mod native_parameters;
pub mod native_parametric_mesh;
pub mod native_parametric_surface;
pub mod native_phases;
pub mod native_rendered_content;
pub mod native_saved_mesh;
pub mod native_saved_scene;
pub mod native_segments;
pub mod native_stairs;
pub mod native_symbol_cuts;
pub mod native_type_materials;
pub mod native_wall_types;
pub mod native_walls;
pub mod partition_arc_walls;
pub mod partition_area_records;
pub mod partition_beam_axes;
pub mod partition_column_nodes;
pub mod partition_compound_structure;
pub mod partition_connector_pairs;
pub mod partition_curtain_materials;
pub mod partition_curve_fields;
pub mod partition_design_options;
pub mod partition_element_records;
pub mod partition_element_records_2023;
pub mod partition_fitting_sizes;
pub mod partition_id_objects;
pub mod partition_ifc_export_overrides;
pub mod partition_instance_transforms;
pub mod partition_level_parameters;
pub mod partition_level_records;
pub mod partition_materials;
pub mod partition_mep_systems;
pub mod partition_name_candidates;
pub mod partition_names;
pub mod partition_phases;
pub mod partition_pipe_axes;
pub mod partition_roof_slopes;
pub mod partition_room_boundaries;
pub mod partition_room_parameters;
pub mod partition_scanner;
pub mod partition_schema_mvp;
pub mod partition_stairs;
pub mod partition_type_materials;
pub mod partition_type_parameters;
pub mod partition_type_records;
pub mod rect_opening_index;
pub mod revit_global_ids;
pub mod schema_registry;
