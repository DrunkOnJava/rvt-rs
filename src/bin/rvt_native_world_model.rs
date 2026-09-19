//! Native equipment inventory with source-qualified type and instance claims.
use anyhow::{Context, Result};
use clap::Parser;
use rvt::{
    native_document, native_embedded, native_equipment, native_family_geometry, native_lifecycle,
    native_materials, native_network, native_representations, native_revision,
    native_room_connections, native_spatial_boundaries, native_spatial_context, native_surfaces,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "rvt-native-world-model",
    about = "Extract native world-model observations with explicit provenance and coverage",
    after_help = "Output must not exist. Exit 0 means bounded extraction completed; 2 means incomplete graph coverage or unsupported projections; 1 means extraction failed. Neither 0 nor 2 establishes full equipment attribute parity."
)]
struct Args {
    file: PathBuf,
    #[arg(long)]
    json: PathBuf,
    #[arg(long, default_value_t = 100_000)]
    max_graph_values: usize,
    #[arg(long, default_value_t = 100_000)]
    max_graph_objects: usize,
    #[arg(long, default_value_t = 512 * 1024 * 1024)]
    max_stream_bytes: u64,
    #[arg(long, default_value_t = 256 * 1024 * 1024)]
    max_group_bytes: usize,
}

#[derive(Serialize)]
struct ProjectionStatus {
    status: &'static str,
    reason: Option<String>,
}

#[derive(Serialize)]
struct Report {
    format: &'static str,
    source: PathBuf,
    source_sha256: Option<String>,
    source_sha256_after: Option<String>,
    status: &'static str,
    complete_world_model: bool,
    coverage: Option<native_document::Summary>,
    inventory: Option<native_equipment::Inventory>,
    spatial_context: Option<native_spatial_context::Inventory>,
    network: Option<native_network::Inventory>,
    materials: Option<native_materials::Inventory>,
    lifecycle: Option<native_lifecycle::Inventory>,
    representations: Option<native_representations::Inventory>,
    family_geometry: Option<native_family_geometry::Inventory>,
    spatial_boundaries: Option<native_spatial_boundaries::Inventory>,
    embedded_coverage: Option<native_embedded::Summary>,
    revision_snapshot: Option<native_revision::Snapshot>,
    surfaces: Option<native_surfaces::Inventory>,
    room_connections: Option<native_room_connections::Inventory>,
    extensible_storage: Option<rvt::native_extensible_storage::Inventory>,
    wall_joins: Option<std::collections::BTreeMap<u64, rvt::native_wall_joins::JoinResult>>,
    error: Option<String>,
    projection_coverage: BTreeMap<String, ProjectionStatus>,
    projection_diagnostics: BTreeMap<String, String>,
}

fn run(args: Args) -> Result<u8> {
    anyhow::ensure!(
        args.max_graph_values > 0,
        "graph value budget must be positive"
    );
    anyhow::ensure!(
        args.max_graph_objects > 0,
        "graph object budget must be positive"
    );
    anyhow::ensure!(args.max_stream_bytes > 0, "stream budget must be positive");
    anyhow::ensure!(args.max_group_bytes > 0, "group budget must be positive");
    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.json)
            .context("create new inventory output")?,
    );
    let mut report = Report {
        format: "rvt-native-world-model/v2",
        source: args.file.clone(),
        source_sha256: None,
        source_sha256_after: None,
        status: "failed",
        complete_world_model: false,
        coverage: None,
        inventory: None,
        spatial_context: None,
        network: None,
        materials: None,
        lifecycle: None,
        representations: None,
        family_geometry: None,
        spatial_boundaries: None,
        embedded_coverage: None,
        revision_snapshot: None,
        surfaces: None,
        room_connections: None,
        extensible_storage: None,
        wall_joins: None,
        error: None,
        projection_coverage: BTreeMap::new(),
        projection_diagnostics: BTreeMap::new(),
    };
    let mut projection_diagnostics = BTreeMap::new();
    let result = (|| -> Result<()> {
        let mut source = File::open(&args.file)?;
        let mut digest = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            let count = source.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            digest.update(&chunk[..count]);
        }
        report.source_sha256 = Some(format!("{:x}", digest.finalize()));
        let mut file = rvt::RevitFile::open(&args.file)?;
        let mut revision = native_revision::InventoryBuilder::default();
        revision.set_source_sha256(report.source_sha256.clone().expect("source hash"))?;
        let mut surfaces = native_surfaces::InventoryBuilder::default();
        let mut builder = native_equipment::InventoryBuilder::default();
        let mut spatial = native_spatial_context::InventoryBuilder::default();
        let mut network = native_network::InventoryBuilder::default();
        let mut materials = native_materials::InventoryBuilder::default();
        let mut lifecycle = native_lifecycle::InventoryBuilder::default();
        let mut representations = native_representations::InventoryBuilder::default();
        let mut extensible_storage = rvt::native_extensible_storage::InventoryBuilder::default();
        let mut room_connections = native_room_connections::InventoryBuilder::default();
        let mut boundaries = native_spatial_boundaries::InventoryBuilder::default();
        let mut family_geometry = native_family_geometry::InventoryBuilder::default();
        let mut wall_ids = std::collections::BTreeSet::new();
        let options = native_document::Options {
            max_stream_bytes: args.max_stream_bytes,
            max_group_bytes: args.max_group_bytes,
            max_graph_values: args.max_graph_values,
            max_graph_objects: args.max_graph_objects,
            ..Default::default()
        };
        report.coverage = Some(native_document::extract(&mut file, &options, |record| {
            if record.class_name.as_deref() == Some("SWall") {
                wall_ids.insert(record.identity.element_id);
            }
            macro_rules! ingest {
                ($name:literal, $expr:expr) => {
                    if let Err(error) = $expr {
                        projection_diagnostics
                            .entry($name.into())
                            .or_insert_with(|| format!("{error:#}"));
                    }
                };
            }
            ingest!("extensible_storage", extensible_storage.ingest(&record));
            ingest!("revision_snapshot", revision.ingest(&record));
            ingest!("surfaces", surfaces.ingest(&record));
            ingest!("equipment", builder.ingest(&record));
            ingest!("spatial_context", spatial.ingest(&record));
            ingest!("network", network.ingest(&record));
            ingest!("materials", materials.ingest(&record));
            ingest!("lifecycle", lifecycle.ingest(&record));
            ingest!("representations", representations.ingest(&record));
            ingest!("spatial_boundaries", boundaries.ingest(&record));
            ingest!("room_connections", room_connections.ingest(&record));
            ingest!("family_geometry", family_geometry.ingest(&record));
            Ok(())
        })?);
        let embedded_options = native_embedded::ScanOptions {
            max_stream_bytes: args.max_stream_bytes,
            max_group_bytes: args.max_group_bytes,
            max_graph_values: args.max_graph_values,
            max_graph_objects: args.max_graph_objects,
        };
        report.embedded_coverage = Some(
            match native_embedded::scan_current_with_options(
                &mut file,
                &embedded_options,
                |record| {
                    if let Err(error) = representations.ingest_embedded(&record) {
                        projection_diagnostics
                            .entry("representations".into())
                            .or_insert_with(|| format!("{error:#}"));
                    }
                    if let Err(error) = family_geometry.ingest_embedded(&record) {
                        projection_diagnostics
                            .entry("family_geometry".into())
                            .or_insert_with(|| format!("{error:#}"));
                    }
                    Ok(())
                },
            ) {
                Ok(summary) => summary,
                Err(error) => native_embedded::Summary {
                    status: "failed".into(),
                    reason: Some(format!("{error:#}")),
                    budgets: embedded_options,
                    ..Default::default()
                },
            },
        );
        revision.set_expected_indexed_elements(
            report.coverage.as_ref().expect("coverage").indexed_elements,
        );
        report.extensible_storage = match extensible_storage.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("extensible_storage".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.wall_joins = match rvt::native_wall_joins::read(&mut file, wall_ids) {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("wall_joins".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.surfaces = match surfaces.finish(&mut file) {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("surfaces".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.representations = match representations.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("representations".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.spatial_boundaries = match boundaries.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("spatial_boundaries".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.family_geometry = match family_geometry.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("family_geometry".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.inventory = match builder.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("equipment".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.spatial_context = match spatial.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("spatial_context".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.network = match network.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("network".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.materials = match materials.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("materials".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        if let Some(materials) = report.materials.as_ref() {
            if let Err(error) = revision.ingest_material_dependencies(materials) {
                projection_diagnostics
                    .entry("revision_snapshot".into())
                    .or_insert_with(|| format!("{error:#}"));
            }
        }
        report.revision_snapshot = match revision.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("revision_snapshot".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.lifecycle = match lifecycle.finish() {
            Ok(value) => Some(value),
            Err(error) => {
                projection_diagnostics
                    .entry("lifecycle".into())
                    .or_insert_with(|| format!("{error:#}"));
                None
            }
        };
        report.room_connections = match (
            &report.spatial_context,
            &report.lifecycle,
            &report.spatial_boundaries,
        ) {
            (Some(context), Some(lifecycle), Some(boundaries)) => {
                match room_connections.finish(context, lifecycle, boundaries) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        projection_diagnostics
                            .entry("room_connections".into())
                            .or_insert_with(|| format!("{error:#}"));
                        None
                    }
                }
            }
            _ => {
                projection_diagnostics
                    .entry("room_connections".into())
                    .or_insert_with(|| "dependency projection unavailable".into());
                None
            }
        };
        Ok(())
    })();
    if let Ok(mut after) = File::open(&args.file) {
        let mut after_digest = Sha256::new();
        let mut after_chunk = [0u8; 65536];
        let mut after_error = None;
        loop {
            match after.read(&mut after_chunk) {
                Ok(0) => break,
                Ok(count) => after_digest.update(&after_chunk[..count]),
                Err(error) => {
                    after_error = Some(error.to_string());
                    break;
                }
            }
        }
        if let Some(error) = after_error {
            report.error = Some(format!("source hash after extraction failed: {error}"));
        } else {
            report.source_sha256_after = Some(format!("{:x}", after_digest.finalize()));
            if report.source_sha256_after != report.source_sha256 {
                report.error = Some("source changed during extraction".into());
            }
        }
    }
    let code = match result {
        Err(error) => {
            report.error = Some(format!("{error:#}"));
            report.projection_diagnostics = projection_diagnostics;
            update_projection_coverage(&mut report);
            1
        }
        Ok(()) => {
            report.projection_diagnostics = projection_diagnostics;
            update_projection_coverage(&mut report);
            let coverage = report
                .coverage
                .as_ref()
                .expect("successful extraction has coverage");
            let incomplete = report.inventory.is_none()
                || report.network.is_none()
                || report.spatial_context.is_none()
                || report.materials.is_none()
                || report.lifecycle.is_none()
                || report.representations.is_none()
                || report.family_geometry.is_none()
                || report.spatial_boundaries.is_none()
                || report.surfaces.is_none()
                || report.embedded_coverage.is_none()
                || !report
                    .network
                    .as_ref()
                    .expect("network")
                    .complete_supported_connector_geometry
                || report
                    .wall_joins
                    .as_ref()
                    .expect("wall joins")
                    .values()
                    .any(|join| join.diagnostic.is_some())
                || !report
                    .revision_snapshot
                    .as_ref()
                    .expect("revision")
                    .complete_current_record_inventory
                || !report
                    .spatial_boundaries
                    .as_ref()
                    .expect("boundaries")
                    .diagnostics
                    .is_empty()
                || !report
                    .surfaces
                    .as_ref()
                    .expect("surfaces")
                    .diagnostics
                    .is_empty()
                || !report
                    .surfaces
                    .as_ref()
                    .expect("surfaces")
                    .saved_texture_mapping
                    .diagnostics
                    .is_empty()
                || !report
                    .family_geometry
                    .as_ref()
                    .expect("family geometry")
                    .complete_supported_geometry
                || report
                    .embedded_coverage
                    .as_ref()
                    .expect("embedded coverage")
                    .status
                    != "complete"
                || !report
                    .embedded_coverage
                    .as_ref()
                    .expect("embedded coverage")
                    .unresolved_current_rows
                    .is_empty()
                || report
                    .embedded_coverage
                    .as_ref()
                    .expect("embedded coverage")
                    .unsupported_graph_records
                    > 0
                || !report
                    .representations
                    .as_ref()
                    .expect("representation inventory")
                    .complete_supported_declarations
                || !report
                    .lifecycle
                    .as_ref()
                    .expect("successful extraction has lifecycle context")
                    .complete_supported_lifecycle
                || !report
                    .materials
                    .as_ref()
                    .expect("successful extraction has material context")
                    .complete_supported_materials
                || !report
                    .network
                    .as_ref()
                    .expect("successful extraction has network context")
                    .complete_supported_network
                || !report
                    .spatial_context
                    .as_ref()
                    .expect("successful extraction has spatial context")
                    .complete_supported_context
                || !report
                    .inventory
                    .as_ref()
                    .expect("successful extraction has inventory")
                    .complete_selected_equipment
                || coverage.unsupported_graph_records > 0
                || coverage.unsupported_metadata_records > 0
                || !coverage.definition_diagnostics.is_empty()
                || !coverage.selected_ids_without_records.is_empty()
                || coverage.emitted_records != coverage.selected_indexed_elements;
            report.status = if incomplete {
                "partial_native_inventory"
            } else {
                "bounded_inventory_extracted"
            };
            report.complete_world_model = !incomplete;
            if incomplete { 2 } else { 0 }
        }
    };
    serde_json::to_writer_pretty(&mut output, &report)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(code)
}
fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn update_projection_coverage(report: &mut Report) {
    let embedded_complete = report
        .embedded_coverage
        .as_ref()
        .is_some_and(|embedded| embedded.status == "complete");
    let status = |name: &str, complete: bool| {
        let dependency_reason =
            if matches!(name, "representations" | "family_geometry") && !embedded_complete {
                report
                    .embedded_coverage
                    .as_ref()
                    .and_then(|embedded| embedded.reason.clone())
            } else {
                None
            };
        projection_status(
            true,
            report.projection_diagnostics.get(name).map(String::as_str),
            complete,
            dependency_reason,
        )
    };
    if let Some(coverage) = &report.coverage {
        report.projection_coverage.insert(
            "native_document".into(),
            projection_status(
                true,
                report
                    .projection_diagnostics
                    .get("native_document")
                    .map(String::as_str),
                coverage.unsupported_graph_records == 0
                    && coverage.unsupported_metadata_records == 0
                    && coverage.definition_diagnostics.is_empty()
                    && coverage.selected_ids_without_records.is_empty(),
                (!coverage.refused_graph_ids.is_empty())
                    .then(|| format!("{} graph records refused", coverage.refused_graph_ids.len())),
            ),
        );
    }
    if let Some(embedded) = &report.embedded_coverage {
        report.projection_coverage.insert(
            "embedded_content".into(),
            embedded_projection_status(embedded),
        );
    }
    if let Some(inventory) = &report.inventory {
        report.projection_coverage.insert(
            "equipment".into(),
            status("equipment", inventory.complete_selected_equipment),
        );
    }
    if let Some(network) = &report.network {
        report.projection_coverage.insert(
            "network".into(),
            status(
                "network",
                network.complete_supported_network && network.complete_supported_connector_geometry,
            ),
        );
    }
    if let Some(spatial) = &report.spatial_context {
        report.projection_coverage.insert(
            "spatial_context".into(),
            status("spatial_context", spatial.complete_supported_context),
        );
    }
    if let Some(materials) = &report.materials {
        report.projection_coverage.insert(
            "materials".into(),
            status("materials", materials.complete_supported_materials),
        );
    }
    if let Some(lifecycle) = &report.lifecycle {
        report.projection_coverage.insert(
            "lifecycle".into(),
            status("lifecycle", lifecycle.complete_supported_lifecycle),
        );
    }
    if let Some(representations) = &report.representations {
        report.projection_coverage.insert(
            "representations".into(),
            status(
                "representations",
                representations.complete_supported_declarations && embedded_complete,
            ),
        );
    }
    if let Some(geometry) = &report.family_geometry {
        report.projection_coverage.insert(
            "family_geometry".into(),
            status(
                "family_geometry",
                geometry.complete_supported_geometry && embedded_complete,
            ),
        );
    }
    if let Some(boundaries) = &report.spatial_boundaries {
        report.projection_coverage.insert(
            "spatial_boundaries".into(),
            status(
                "spatial_boundaries",
                boundaries.complete_boundary_parity && boundaries.diagnostics.is_empty(),
            ),
        );
    }
    if let Some(surfaces) = &report.surfaces {
        report.projection_coverage.insert(
            "surfaces".into(),
            status(
                "surfaces",
                surfaces.complete_surface_parity && surfaces.diagnostics.is_empty(),
            ),
        );
    }
    let wall_complete = report
        .wall_joins
        .as_ref()
        .is_some_and(|joins| joins.values().all(|join| join.diagnostic.is_none()));
    report.projection_coverage.insert(
        "revision_snapshot".into(),
        status(
            "revision_snapshot",
            report
                .revision_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.complete_current_record_inventory),
        ),
    );
    report.projection_coverage.insert(
        "room_connections".into(),
        status(
            "room_connections",
            report.room_connections.as_ref().is_some_and(|connections| {
                connections.complete_room_connection_parity && connections.diagnostics.is_empty()
            }),
        ),
    );
    report.projection_coverage.insert(
        "extensible_storage".into(),
        status(
            "extensible_storage",
            report.extensible_storage.as_ref().is_some_and(|storage| {
                storage.complete_extensible_storage && storage.diagnostics.is_empty()
            }),
        ),
    );
    report.projection_coverage.insert(
        "wall_joins".into(),
        status("wall_joins", report.wall_joins.is_some() && wall_complete),
    );
    for name in [
        "native_document",
        "embedded_content",
        "equipment",
        "network",
        "spatial_context",
        "materials",
        "lifecycle",
        "representations",
        "family_geometry",
        "spatial_boundaries",
        "surfaces",
        "revision_snapshot",
        "room_connections",
        "extensible_storage",
        "wall_joins",
    ] {
        report
            .projection_coverage
            .entry(name.into())
            .or_insert(ProjectionStatus {
                status: "not_run",
                reason: Some("extraction aborted before projection finalization".into()),
            });
    }
}

fn embedded_projection_status(summary: &native_embedded::Summary) -> ProjectionStatus {
    let status = match summary.status.as_str() {
        "complete"
            if summary.unsupported_graph_records == 0
                && summary.unresolved_current_rows.is_empty() =>
        {
            "complete"
        }
        "complete" => "partial",
        "unsupported_profile" => "unsupported",
        _ => "failed",
    };
    ProjectionStatus {
        status,
        reason: summary.reason.clone(),
    }
}

fn projection_status(
    present: bool,
    recorded_error: Option<&str>,
    semantically_complete: bool,
    reason: Option<String>,
) -> ProjectionStatus {
    if let Some(error) = recorded_error {
        return ProjectionStatus {
            status: "failed",
            reason: Some(error.into()),
        };
    }
    if !present {
        return ProjectionStatus {
            status: "not_run",
            reason,
        };
    }
    ProjectionStatus {
        status: if semantically_complete {
            "complete"
        } else {
            "partial"
        },
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_embedded_profile_is_explicitly_unsupported() {
        let summary = native_embedded::Summary {
            status: "unsupported_profile".into(),
            reason: Some("Revit 2023 framing is not qualified".into()),
            ..Default::default()
        };
        let projection = embedded_projection_status(&summary);
        assert_eq!(projection.status, "unsupported");
        assert!(projection.reason.is_some());
    }

    #[test]
    fn embedded_refusals_prevent_complete_projection_claim() {
        let summary = native_embedded::Summary {
            status: "complete".into(),
            unsupported_graph_records: 1,
            ..Default::default()
        };
        assert_eq!(embedded_projection_status(&summary).status, "partial");
    }

    #[test]
    fn clean_embedded_projection_can_be_complete() {
        let summary = native_embedded::Summary {
            status: "complete".into(),
            ..Default::default()
        };
        assert_eq!(embedded_projection_status(&summary).status, "complete");
    }

    #[test]
    fn projection_error_taints_present_projection() {
        let projection = projection_status(true, Some("builder failed"), true, None);
        assert_eq!(projection.status, "failed");
    }

    #[test]
    fn absent_projection_is_not_run_even_with_complete_semantics() {
        let projection = projection_status(false, None, true, None);
        assert_eq!(projection.status, "not_run");
    }

    #[test]
    fn semantic_incompleteness_is_partial() {
        let projection = projection_status(true, None, false, Some("parity unavailable".into()));
        assert_eq!(projection.status, "partial");
        assert_eq!(projection.reason.as_deref(), Some("parity unavailable"));
    }
}
