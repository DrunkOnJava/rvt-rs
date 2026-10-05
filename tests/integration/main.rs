//! Every tests/*.rs file as a module of one test binary (C6): 77 binaries
//! took 118 s of build time where this one takes 13 s. A file's tests are
//! named `<file>::<test>`; tools/ci/verify-real-files.sh fails when a file
//! has no line here.

// Each file declares its own `mod common;`, so tests/common is compiled once
// per file that uses it, as when every file was its own binary.
#![allow(clippy::duplicate_mod)]

#[path = "../arc_wall_corpus.rs"]
mod arc_wall_corpus;
#[path = "../binary_inventory.rs"]
mod binary_inventory;
#[path = "../cfb_roundtrip_delta.rs"]
mod cfb_roundtrip_delta;
#[path = "../cli_ergonomics.rs"]
mod cli_ergonomics;
#[path = "../cli_real_files.rs"]
mod cli_real_files;
#[path = "../common_reference.rs"]
mod common_reference;
#[path = "../control_cancellation.rs"]
mod control_cancellation;
#[path = "../corpus_tier2_health.rs"]
mod corpus_tier2_health;
#[path = "../covering_finish.rs"]
mod covering_finish;
#[path = "../curtain_part_materials.rs"]
mod curtain_part_materials;
#[path = "../curtain_wall_doors.rs"]
mod curtain_wall_doors;
#[path = "../curtain_walls.rs"]
mod curtain_walls;
#[path = "../design_options.rs"]
mod design_options;
#[path = "../door_type_global_ids.rs"]
mod door_type_global_ids;
#[path = "../duct_and_pipe_ports.rs"]
mod duct_and_pipe_ports;
#[path = "../duct_fields.rs"]
mod duct_fields;
#[path = "../duct_names.rs"]
mod duct_names;
#[path = "../duct_segment_shape.rs"]
mod duct_segment_shape;
#[path = "../duct_segment_type_common.rs"]
mod duct_segment_type_common;
#[path = "../elem_table_corpus.rs"]
mod elem_table_corpus;
#[path = "../elem_table_frame.rs"]
mod elem_table_frame;
#[path = "../element_materials.rs"]
mod element_materials;
#[path = "../element_names.rs"]
mod element_names;
#[path = "../element_records_2025.rs"]
mod element_records_2025;
#[path = "../family_instance_ports.rs"]
mod family_instance_ports;
#[path = "../field_type_coverage.rs"]
mod field_type_coverage;
#[path = "../fitting_nominal_diameter.rs"]
mod fitting_nominal_diameter;
#[path = "../fuzz_regressions.rs"]
mod fuzz_regressions;
#[path = "../graceful_degradation.rs"]
mod graceful_degradation;
#[path = "../ifc_export_overrides.rs"]
mod ifc_export_overrides;
#[path = "../ifc_roundtrip.rs"]
mod ifc_roundtrip;
#[path = "../ifc_synthetic_project.rs"]
mod ifc_synthetic_project;
#[path = "../ifc_synthetic_structural.rs"]
mod ifc_synthetic_structural;
#[path = "../invert_elevation.rs"]
mod invert_elevation;
#[path = "../is_external.rs"]
mod is_external;
#[path = "../iter_elements_typed.rs"]
mod iter_elements_typed;
#[path = "../json_schema_contracts.rs"]
mod json_schema_contracts;
#[path = "../level_names.rs"]
mod level_names;
#[path = "../mep_connections.rs"]
mod mep_connections;
#[path = "../mep_systems.rs"]
mod mep_systems;
#[path = "../native_current_records.rs"]
mod native_current_records;
#[path = "../native_partitions_walk.rs"]
mod native_partitions_walk;
#[path = "../native_phases.rs"]
mod native_phases;
#[path = "../parameter_diagnostics.rs"]
mod parameter_diagnostics;
#[path = "../partition_record_chain.rs"]
mod partition_record_chain;
#[path = "../partition_scanner.rs"]
mod partition_scanner;
#[path = "../port_nesting.rs"]
mod port_nesting;
#[path = "../project_corpus_smoke.rs"]
mod project_corpus_smoke;
#[path = "../project_count_fixtures.rs"]
mod project_count_fixtures;
#[path = "../proptest_parsers.rs"]
mod proptest_parsers;
#[path = "../proxy_ports.rs"]
mod proxy_ports;
#[path = "../railing_type.rs"]
mod railing_type;
#[path = "../re15_geometry_invariants.rs"]
mod re15_geometry_invariants;
#[path = "../re19_door_window_wall_negative.rs"]
mod re19_door_window_wall_negative;
#[path = "../reference_values.rs"]
mod reference_values;
#[path = "../revit_global_ids.rs"]
mod revit_global_ids;
#[path = "../rooms_floors_from_records_only.rs"]
mod rooms_floors_from_records_only;
#[path = "../rvt_dump_cli.rs"]
mod rvt_dump_cli;
#[path = "../rvt_ifc_diagnostics_cli.rs"]
mod rvt_ifc_diagnostics_cli;
#[path = "../rvt_info_cli.rs"]
mod rvt_info_cli;
#[path = "../rvt_inspect_cli.rs"]
mod rvt_inspect_cli;
#[path = "../rvt_schedule_cli.rs"]
mod rvt_schedule_cli;
#[path = "../samples.rs"]
mod samples;
#[path = "../schema_registry_catalogs.rs"]
mod schema_registry_catalogs;
#[path = "../segment_length.rs"]
mod segment_length;
#[path = "../serial_number.rs"]
mod serial_number;
#[path = "../space_common.rs"]
mod space_common;
#[path = "../space_containment.rs"]
mod space_containment;
#[path = "../space_types.rs"]
mod space_types;
#[path = "../spatial_property_sets.rs"]
mod spatial_property_sets;
#[path = "../support_matrix.rs"]
mod support_matrix;
#[path = "../type_global_ids.rs"]
mod type_global_ids;
#[path = "../typed_no_geometry_types.rs"]
mod typed_no_geometry_types;
#[path = "../unfilled_openings.rs"]
mod unfilled_openings;
#[path = "../walker_to_ifc_integration.rs"]
mod walker_to_ifc_integration;
#[path = "../wall_profile_openings.rs"]
mod wall_profile_openings;
#[path = "../witness_registry.rs"]
mod witness_registry;
#[path = "../witness_verdict.rs"]
mod witness_verdict;
