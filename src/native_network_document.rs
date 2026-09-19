//! Bounded selected-owner network extraction.
use crate::{
    RevitFile,
    native_document::{self, Options, Summary},
    native_network::Inventory,
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Serialize)]
pub struct DepthFrontier {
    pub source_owner_id: u64,
    pub target_owner_id: u64,
    pub source_connector_id: i64,
    pub target_connector_id: i64,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct GeometryFrontier {
    pub dependency_id: u64,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Coverage {
    pub requested_ids: Vec<u64>,
    pub observed_ids: Vec<u64>,
    pub decoded_ids: Vec<u64>,
    pub missing_requested_ids: Vec<u64>,
    pub closure_ids: Vec<u64>,
    pub closure_missing_ids: Vec<u64>,
    pub unsupported_graph_ids: Vec<u64>,
    pub projection_diagnostic_owner_ids: Vec<u64>,
    pub budget_reason: Option<&'static str>,
    pub depth_frontier: Vec<DepthFrontier>,
    pub closure_depth: usize,
    pub max_owners: usize,
    pub budget_refused: bool,
    pub native_summary: Summary,
    pub round_summaries: Vec<RoundSummary>,
    pub geometry_closure_ids: Vec<u64>,
    pub geometry_closure_missing_ids: Vec<u64>,
    pub geometry_frontier: Vec<GeometryFrontier>,
    pub geometry_closure_depth: usize,
    pub geometry_max_owners: usize,
    pub geometry_budget_refused: bool,
    pub geometry_budget_reason: Option<&'static str>,
    pub geometry_round_summaries: Vec<RoundSummary>,
}

#[derive(Debug, Serialize)]
pub struct RoundSummary {
    pub requested_ids: Vec<u64>,
    pub selected_indexed_elements: usize,
    pub emitted_records: usize,
    pub unsupported_graph_records: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub format: &'static str,
    pub source_sha256: String,
    pub status: &'static str,
    pub complete_network_parity: bool,
    pub coverage: Coverage,
    pub network: Inventory,
}

fn source_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path).context("open source for hashing")?;
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hash.update(&chunk[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

struct RoundState<'a> {
    builder: &'a mut crate::native_network::InventoryBuilder,
    observed: &'a mut BTreeSet<u64>,
    decoded: &'a mut BTreeSet<u64>,
    unsupported: &'a mut BTreeSet<u64>,
}

struct GraphLimits {
    max_values: usize,
    max_objects: usize,
}

/// Resource and closure bounds for one selected network extraction.
#[derive(Debug, Clone, Copy)]
pub struct ExtractionLimits {
    pub max_depth: usize,
    pub max_owners: usize,
    pub max_graph_values: usize,
    pub max_graph_objects: usize,
    pub max_geometry_depth: usize,
    pub max_geometry_owners: usize,
}

fn extract_round(
    file: &mut RevitFile,
    ids: &BTreeSet<u64>,
    state: &mut RoundState<'_>,
    limits: &GraphLimits,
) -> Result<Summary> {
    let options = Options {
        selected_ids: ids.clone(),
        max_graph_values: limits.max_values,
        max_graph_objects: limits.max_objects,
        ..Default::default()
    };
    let summary = native_document::extract(file, &options, |record| {
        state.observed.insert(record.identity.element_id);
        if record.status != "complete_bounded_graph" {
            state.unsupported.insert(record.identity.element_id);
        } else {
            state.decoded.insert(record.identity.element_id);
        }
        state.builder.ingest(&record)
    })?;
    Ok(summary)
}

fn plan_next_ids(
    selected: &BTreeSet<u64>,
    references: &BTreeSet<u64>,
    max_owners: usize,
) -> std::result::Result<BTreeSet<u64>, &'static str> {
    let next = references
        .difference(selected)
        .copied()
        .collect::<BTreeSet<_>>();
    if selected.len() + next.len() > max_owners {
        return Err("max_owners_reached");
    }
    Ok(next)
}

fn bounded_complete(
    missing_requested: &[u64],
    closure_missing: &[u64],
    unsupported_graph: &BTreeSet<u64>,
    frontier: &[DepthFrontier],
    diagnostics: &[crate::native_network::Diagnostic],
) -> bool {
    missing_requested.is_empty()
        && closure_missing.is_empty()
        && unsupported_graph.is_empty()
        && frontier.is_empty()
        && diagnostics.is_empty()
}

pub fn extract_selected(
    path: &Path,
    requested: BTreeSet<u64>,
    limits: ExtractionLimits,
) -> Result<Report> {
    extract_selected_with_geometry_limits(path, requested, limits)
}

pub fn extract_selected_with_geometry_limits(
    path: &Path,
    requested: BTreeSet<u64>,
    limits: ExtractionLimits,
) -> Result<Report> {
    let ExtractionLimits {
        max_depth,
        max_owners,
        max_graph_values,
        max_graph_objects,
        max_geometry_depth,
        max_geometry_owners,
    } = limits;

    ensure!(max_owners > 0, "max-owners must be positive");
    ensure!(
        max_geometry_owners > 0,
        "max-geometry-owners must be positive"
    );
    ensure!(
        !requested.is_empty(),
        "at least one selected ElementId is required"
    );
    let source_sha256 = source_hash(path)?;
    ensure!(
        requested.len() <= max_owners,
        "requested owners exceed max-owners budget"
    );
    let mut file = RevitFile::open(path)?;
    let mut builder = crate::native_network::InventoryBuilder::default();
    let mut observed_ids = BTreeSet::new();
    let mut decoded_ids = BTreeSet::new();
    let mut unsupported_ids = BTreeSet::new();
    let mut round_summaries = Vec::new();
    let limits = GraphLimits {
        max_values: max_graph_values,
        max_objects: max_graph_objects,
    };
    let mut summary;
    {
        let mut state = RoundState {
            builder: &mut builder,
            observed: &mut observed_ids,
            decoded: &mut decoded_ids,
            unsupported: &mut unsupported_ids,
        };
        summary = extract_round(&mut file, &requested, &mut state, &limits)?;
    }
    round_summaries.push(RoundSummary {
        requested_ids: requested.iter().copied().collect(),
        selected_indexed_elements: summary.selected_indexed_elements,
        emitted_records: summary.emitted_records,
        unsupported_graph_records: summary.unsupported_graph_records,
    });
    let mut selected = requested.clone();
    let mut closure = BTreeSet::new();
    let mut budget_reason = None;
    let mut geometry_selected = requested.clone();
    let mut geometry_closure = BTreeSet::new();
    let mut geometry_round_summaries = Vec::new();
    let mut geometry_budget_reason = None;
    for _ in 0..max_depth {
        let refs = match plan_next_ids(&selected, &builder.referenced_owner_ids(), max_owners) {
            Ok(refs) => refs,
            Err(reason) => {
                budget_reason = Some(reason);
                break;
            }
        };
        if refs.is_empty() {
            break;
        }
        selected.extend(refs.iter().copied());
        closure.extend(refs.clone());
        let mut state = RoundState {
            builder: &mut builder,
            observed: &mut observed_ids,
            decoded: &mut decoded_ids,
            unsupported: &mut unsupported_ids,
        };
        summary = extract_round(&mut file, &refs, &mut state, &limits)?;
        round_summaries.push(RoundSummary {
            requested_ids: refs.iter().copied().collect(),
            selected_indexed_elements: summary.selected_indexed_elements,
            emitted_records: summary.emitted_records,
            unsupported_graph_records: summary.unsupported_graph_records,
        });
    }
    for _ in 0..max_geometry_depth {
        let refs = match plan_next_ids(
            &geometry_selected,
            &builder.geometry_dependency_ids(),
            requested.len() + max_geometry_owners,
        ) {
            Ok(refs) => refs,
            Err(reason) => {
                geometry_budget_reason = Some(reason);
                break;
            }
        };
        if refs.is_empty() {
            break;
        }
        geometry_selected.extend(refs.iter().copied());
        geometry_closure.extend(refs.clone());
        let extract_ids = refs
            .iter()
            .filter(|id| !observed_ids.contains(id))
            .copied()
            .collect::<BTreeSet<_>>();
        if extract_ids.is_empty() {
            continue;
        }
        let mut state = RoundState {
            builder: &mut builder,
            observed: &mut observed_ids,
            decoded: &mut decoded_ids,
            unsupported: &mut unsupported_ids,
        };
        summary = extract_round(&mut file, &extract_ids, &mut state, &limits)?;
        geometry_round_summaries.push(RoundSummary {
            requested_ids: extract_ids.iter().copied().collect(),
            selected_indexed_elements: summary.selected_indexed_elements,
            emitted_records: summary.emitted_records,
            unsupported_graph_records: summary.unsupported_graph_records,
        });
    }
    let geometry_pending_ids = builder
        .geometry_dependency_ids()
        .difference(&geometry_selected)
        .copied()
        .collect::<BTreeSet<_>>();
    let network = builder.finish()?;
    ensure!(
        source_hash(path)? == source_sha256,
        "source RVT changed during extraction"
    );
    let geometry_budget_refused = geometry_budget_reason.is_some();
    let combined_budget_reason = budget_reason
        .as_ref()
        .copied()
        .or(geometry_budget_reason.as_ref().copied());
    if let Some(reason) = combined_budget_reason {
        let missing_requested_ids = requested.difference(&observed_ids).copied().collect();
        let closure_missing_ids = closure.difference(&observed_ids).copied().collect();
        let geometry_closure_missing_ids = geometry_closure
            .difference(&observed_ids)
            .copied()
            .chain(geometry_pending_ids.iter().copied())
            .collect::<Vec<_>>();
        let projection_diagnostic_owner_ids = network
            .diagnostics
            .iter()
            .map(|d| d.element_id)
            .collect::<BTreeSet<_>>();
        let depth_frontier = network
            .edges
            .iter()
            .filter_map(|e| {
                let source = u64::try_from(e.source_port.owner_element_id).ok()?;
                let target = u64::try_from(e.target_port.owner_element_id).ok()?;
                (selected.contains(&source) && !selected.contains(&target)).then_some(
                    DepthFrontier {
                        source_owner_id: source,
                        target_owner_id: target,
                        source_connector_id: e.source_port.connector_id,
                        target_connector_id: e.target_port.connector_id,
                        reason,
                    },
                )
            })
            .collect();
        let coverage = Coverage {
            requested_ids: requested.into_iter().collect(),
            observed_ids: observed_ids.iter().copied().collect(),
            decoded_ids: decoded_ids.into_iter().collect(),
            missing_requested_ids,
            closure_ids: closure.into_iter().collect(),
            closure_missing_ids,
            unsupported_graph_ids: unsupported_ids.into_iter().collect(),
            projection_diagnostic_owner_ids: projection_diagnostic_owner_ids.into_iter().collect(),
            depth_frontier,
            closure_depth: max_depth,
            max_owners,
            budget_refused: true,
            budget_reason: Some(reason),
            native_summary: summary,
            round_summaries,
            geometry_closure_ids: geometry_closure.into_iter().collect(),
            geometry_closure_missing_ids: geometry_closure_missing_ids.clone(),
            geometry_frontier: geometry_closure_missing_ids
                .iter()
                .map(|dependency_id| GeometryFrontier {
                    dependency_id: *dependency_id,
                    reason,
                })
                .collect(),
            geometry_closure_depth: max_geometry_depth,
            geometry_max_owners: max_geometry_owners,
            geometry_budget_refused,
            geometry_budget_reason: if geometry_budget_refused {
                Some(reason)
            } else {
                None
            },
            geometry_round_summaries,
        };
        return Ok(Report {
            format: "rvt-native-network-pilot/v1",
            source_sha256,
            status: "budget_refused",
            complete_network_parity: false,
            coverage,
            network,
        });
    }
    let missing_requested_ids = requested
        .difference(&observed_ids)
        .copied()
        .collect::<Vec<_>>();
    let closure_missing_ids = closure
        .difference(&observed_ids)
        .copied()
        .collect::<Vec<_>>();
    let geometry_closure_missing_ids = geometry_closure
        .difference(&observed_ids)
        .copied()
        .chain(geometry_pending_ids.iter().copied())
        .collect::<Vec<_>>();
    let geometry_frontier = geometry_closure_missing_ids
        .iter()
        .map(|dependency_id| GeometryFrontier {
            dependency_id: *dependency_id,
            reason: "geometry_dependency_unobserved",
        })
        .collect::<Vec<_>>();
    let projection_diagnostic_owner_ids = network
        .diagnostics
        .iter()
        .map(|d| d.element_id)
        .collect::<BTreeSet<_>>();
    let selected = requested.union(&closure).copied().collect::<BTreeSet<_>>();
    let depth_frontier: Vec<DepthFrontier> = network
        .edges
        .iter()
        .filter_map(|e| {
            let source = u64::try_from(e.source_port.owner_element_id).ok()?;
            let target = u64::try_from(e.target_port.owner_element_id).ok()?;
            (selected.contains(&source) && !selected.contains(&target)).then_some(DepthFrontier {
                source_owner_id: source,
                target_owner_id: target,
                source_connector_id: e.source_port.connector_id,
                target_connector_id: e.target_port.connector_id,
                reason: "max_depth_reached",
            })
        })
        .collect();
    let complete = budget_reason.is_none()
        && geometry_budget_reason.is_none()
        && geometry_closure_missing_ids.is_empty()
        && bounded_complete(
            &missing_requested_ids,
            &closure_missing_ids,
            &unsupported_ids,
            &depth_frontier,
            &network.diagnostics,
        );
    let coverage = Coverage {
        requested_ids: requested.into_iter().collect(),
        observed_ids: observed_ids.into_iter().collect(),
        decoded_ids: decoded_ids.into_iter().collect(),
        missing_requested_ids,
        closure_ids: closure.into_iter().collect(),
        closure_missing_ids,
        unsupported_graph_ids: unsupported_ids.into_iter().collect(),
        projection_diagnostic_owner_ids: projection_diagnostic_owner_ids.into_iter().collect(),
        depth_frontier,
        closure_depth: max_depth,
        max_owners,
        budget_refused: false,
        budget_reason,
        native_summary: summary,
        round_summaries,
        geometry_closure_ids: geometry_closure.into_iter().collect(),
        geometry_closure_missing_ids,
        geometry_frontier,
        geometry_closure_depth: max_geometry_depth,
        geometry_max_owners: max_geometry_owners,
        geometry_budget_refused: false,
        geometry_budget_reason,
        geometry_round_summaries,
    };
    ensure!(
        source_hash(path)? == source_sha256,
        "source RVT changed during extraction"
    );
    Ok(Report {
        format: "rvt-native-network-pilot/v1",
        source_sha256,
        status: if complete {
            "bounded_network_extracted"
        } else {
            "partial_bounded_network"
        },
        complete_network_parity: false,
        coverage,
        network,
    })
}

pub fn write_json(report: &Report, path: &Path) -> Result<()> {
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(&mut out, report)?;
    out.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::plan_next_ids;
    use std::collections::BTreeSet;

    fn set(ids: &[u64]) -> BTreeSet<u64> {
        ids.iter().copied().collect()
    }

    #[test]
    fn closure_cycles_are_empty_after_seen_filter() {
        assert_eq!(
            plan_next_ids(&set(&[1, 2]), &set(&[1, 2]), 4).unwrap(),
            set(&[])
        );
    }

    #[test]
    fn unknown_owner_is_planned_as_explicit_closure_candidate() {
        assert_eq!(
            plan_next_ids(&set(&[1]), &set(&[999]), 2).unwrap(),
            set(&[999])
        );
    }

    #[test]
    fn deterministic_budget_refusal_prevents_partial_planning() {
        assert_eq!(
            plan_next_ids(&set(&[1]), &set(&[2, 3]), 2),
            Err("max_owners_reached")
        );
    }

    #[test]
    fn depth_round_can_stop_without_replanning_seen_ids() {
        let selected = set(&[1, 2]);
        assert_eq!(
            plan_next_ids(&selected, &set(&[1, 2, 3]), 3).unwrap(),
            set(&[3])
        );
    }

    #[test]
    fn geometry_shared_symbol_context_is_planned_once() {
        assert_eq!(
            plan_next_ids(&set(&[100, 200]), &set(&[100, 200, 300]), 10).unwrap(),
            set(&[300])
        );
    }

    #[test]
    fn geometry_dependency_budget_refusal_is_independent() {
        assert_eq!(
            plan_next_ids(&set(&[100]), &set(&[100, 200, 300]), 2),
            Err("max_owners_reached")
        );
    }

    #[test]
    fn unresolved_frontier_is_not_complete() {
        assert!(!super::bounded_complete(
            &[],
            &[],
            &BTreeSet::new(),
            &[super::DepthFrontier {
                source_owner_id: 1,
                target_owner_id: 2,
                source_connector_id: 1,
                target_connector_id: 2,
                reason: "max_depth_reached",
            }],
            &[],
        ));
    }
}
