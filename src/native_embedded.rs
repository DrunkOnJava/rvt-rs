//! Bounded physical embedded records. These are not a content-local current index.
use crate::{
    RevitFile, compression, native_document,
    native_parameters::{self, GraphLimits, ObjectGraph},
    native_segments::{self, GroupSource, Statistics},
    schema_registry,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ScanOptions {
    pub max_stream_bytes: u64,
    pub max_group_bytes: usize,
    pub max_graph_values: usize,
    pub max_graph_objects: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_stream_bytes: 512 * 1024 * 1024,
            max_group_bytes: 256 * 1024 * 1024,
            max_graph_values: 100_000,
            max_graph_objects: 100_000,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Source {
    pub stream: String,
    pub group: GroupSource,
    pub group_record_offset: usize,
    pub body_sha256: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct CurrentSelection {
    pub identity: crate::native_content_documents::LocalIdentity,
    pub table_source: crate::native_content_documents::TableSource,
    pub selection_rule: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Record {
    pub current_selection: Option<CurrentSelection>,
    pub selection_status: &'static str,
    pub content_key_raw_hex: String,
    pub local_element_id: u64,
    pub class_name: Option<String>,
    pub graph: Option<ObjectGraph>,
    pub diagnostic: Option<String>,
    pub source: Source,
}
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub status: String,
    pub reason: Option<String>,
    pub budgets: ScanOptions,
    pub content_catalog: Option<crate::native_content_documents::Catalog>,
    pub selected_current_records: usize,
    pub unresolved_current_rows: Vec<CurrentRowIssue>,
    pub terminal_records: usize,
    pub current_state_established: bool,
    pub main_project_unique_ids_assigned: bool,
    pub selected_groups: usize,
    pub emitted_records: usize,
    pub unsupported_graph_records: usize,
    pub refused_record_ids: Vec<u64>,
    pub refused_record_classes: BTreeMap<String, usize>,
    pub namespace_records: BTreeMap<String, usize>,
    pub duplicate_local_locators: Vec<Duplicate>,
    pub partitions: BTreeMap<String, Statistics>,
}
#[derive(Debug, Serialize)]
pub struct CurrentRowIssue {
    pub content_key_raw_hex: String,
    pub local_element_id: u64,
    pub physical_candidates: usize,
}
#[derive(Debug, Serialize)]
pub struct Duplicate {
    pub content_key_raw_hex: String,
    pub local_element_id: u64,
    pub physical_occurrences: usize,
}
fn rows(bytes: &[u8], source: &GroupSource) -> Result<Vec<(u64, usize, usize, usize)>> {
    let mut pos = 0usize;
    let mut rows = Vec::new();
    let mut sum = 0u64;
    while pos < bytes.len() {
        ensure!(bytes.len() - pos >= 20, "truncated embedded record header");
        let id = u64::from_le_bytes(bytes[pos..pos + 8].try_into()?);
        let length = u32::from_le_bytes(bytes[pos + 12..pos + 16].try_into()?) as usize;
        let start = pos + 16;
        let end = start
            .checked_add(length)
            .ok_or_else(|| anyhow::anyhow!("embedded record length overflow"))?;
        ensure!(
            end <= bytes.len().saturating_sub(4),
            "truncated embedded body"
        );
        ensure!(
            u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize == length,
            "embedded dual lengths disagree"
        );
        rows.push((id, pos, start, end));
        sum += length as u64;
        pos = end + 4;
    }
    ensure!(
        rows.len() as u64 == source.declared_objects && sum == source.declared_body_bytes,
        "embedded group count/body size mismatch"
    );
    Ok(rows)
}
/// Scan physical channel 102 records in all embedded namespaces, preserving duplicates.
/// No project UniqueId or content-local liveness is inferred.
pub fn scan(
    file: &mut RevitFile,
    max_graph_values: usize,
    emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    scan_with_options(
        file,
        &ScanOptions {
            max_graph_values,
            ..Default::default()
        },
        emit,
    )
}

pub fn scan_with_options(
    file: &mut RevitFile,
    options: &ScanOptions,
    mut emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    ensure!(
        options.max_graph_values > 0,
        "zero embedded graph value budget"
    );
    ensure!(
        options.max_graph_objects > 0,
        "zero embedded graph object budget"
    );
    let version = file.basic_file_info()?.version;
    if version != 2027 {
        return Ok(Summary {
            status: "unsupported_profile".into(),
            reason: Some(format!(
                "embedded framing profile is qualified only for Revit 2027 (file version {version})"
            )),
            budgets: *options,
            ..Default::default()
        });
    }
    let registry = schema_registry::parse(&native_document::read_single(file, "Formats/Latest")?)?;
    let mut names: Vec<_> = file
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let mut summary = Summary {
        status: "complete".into(),
        budgets: *options,
        ..Default::default()
    };
    let mut locators = BTreeMap::new();
    for name in names {
        let stored = file.read_stream_with_limit(&name, options.max_stream_bytes)?;
        let prepared = compression::prepare_stream_for_inflate(&name, &stored);
        let stats = native_segments::walk(
            &prepared,
            &registry,
            options.max_group_bytes,
            |source, bytes| {
                let Some(key) = source.content_key else {
                    return Ok(());
                };
                if source.channel != 102 {
                    return Ok(());
                }
                summary.selected_groups += 1;
                let key: String = key.iter().map(|b| format!("{b:02x}")).collect();
                for (id, offset, start, end) in rows(bytes, source)? {
                    let body = &bytes[start..end];
                    if id == u64::MAX {
                        ensure!(body.is_empty(), "embedded terminal ID has nonempty body");
                        summary.terminal_records += 1;
                        continue;
                    }

                    let class_name = body
                        .get(..2)
                        .and_then(|b| registry.class(u16::from_le_bytes([b[0], b[1]])))
                        .map(|c| c.name.clone());
                    let (graph, diagnostic) =
                        match native_parameters::decode_graph_with_catalog_usage(
                            body,
                            &registry,
                            &GraphLimits {
                                max_values: options.max_graph_values,
                                max_objects: options.max_graph_objects,
                                ..Default::default()
                            },
                            None,
                        ) {
                            Ok((graph, _usage)) => (Some(graph), None),
                            Err(error) => {
                                summary.unsupported_graph_records += 1;
                                summary.refused_record_ids.push(id);
                                if let Some(class) = &class_name {
                                    *summary
                                        .refused_record_classes
                                        .entry(class.clone())
                                        .or_default() += 1;
                                }
                                (None, Some(format!("{error:#}")))
                            }
                        };
                    *locators.entry((key.clone(), id)).or_insert(0usize) += 1;
                    *summary.namespace_records.entry(key.clone()).or_default() += 1;
                    summary.emitted_records += 1;
                    emit(Record {
                        current_selection: None,
                        selection_status: "physical_only",
                        content_key_raw_hex: key.clone(),
                        local_element_id: id,
                        class_name,
                        graph,
                        diagnostic,
                        source: Source {
                            stream: name.clone(),
                            group: source.clone(),
                            group_record_offset: offset,
                            body_sha256: format!("{:x}", Sha256::digest(body)),
                        },
                    })?;
                }
                Ok(())
            },
        )?;
        summary.partitions.insert(name, stats);
    }
    summary.duplicate_local_locators = locators
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(
            |((content_key_raw_hex, local_element_id), physical_occurrences)| Duplicate {
                content_key_raw_hex,
                local_element_id,
                physical_occurrences,
            },
        )
        .collect();
    Ok(summary)
}
/// Qualify content-local current records against the explicit current ElemTable.
/// Two physical passes deliberately reject duplicate candidates regardless of order.
/// Unreferenced namespaces remain physical observations; project family activation
/// is a separate FamilyDocument content-key join.
pub fn scan_current(
    file: &mut RevitFile,
    max_graph_values: usize,
    emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    scan_current_with_options(
        file,
        &ScanOptions {
            max_graph_values,
            ..Default::default()
        },
        emit,
    )
}

pub fn scan_current_with_options(
    file: &mut RevitFile,
    options: &ScanOptions,
    mut emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    if file.basic_file_info()?.version != 2027 {
        return Ok(Summary {
            status: "unsupported_profile".into(),
            reason: Some(format!(
                "embedded framing profile is qualified only for Revit 2027 (file version {})",
                file.basic_file_info()?.version
            )),
            budgets: *options,
            ..Default::default()
        });
    }
    let registry = schema_registry::parse(&native_document::read_single(file, "Formats/Latest")?)?;
    let catalog = crate::native_content_documents::parse(
        &native_document::read_single(file, "Global/ContentDocuments")?,
        &registry,
        options.max_graph_values,
    )?;
    let mut counts = BTreeMap::new();
    scan_with_options(file, options, |record| {
        *counts
            .entry((record.content_key_raw_hex, record.local_element_id))
            .or_insert(0usize) += 1;
        Ok(())
    })?;
    let mut selected = 0usize;
    let mut summary = scan_with_options(file, options, |mut record| {
        let candidates = counts
            .get(&(record.content_key_raw_hex.clone(), record.local_element_id))
            .copied()
            .unwrap_or(0);
        let (selection, status) = selection_for(
            &catalog,
            &record.content_key_raw_hex,
            record.local_element_id,
            candidates,
        );
        if selection.is_some() {
            selected += 1;
        }
        record.current_selection = selection;
        record.selection_status = status;
        emit(record)
    })?;
    for (key, document) in &catalog.documents {
        for id in document.identities.keys() {
            let candidates = counts.get(&(key.clone(), *id)).copied().unwrap_or(0);
            if candidates != 1 {
                summary.unresolved_current_rows.push(CurrentRowIssue {
                    content_key_raw_hex: key.clone(),
                    local_element_id: *id,
                    physical_candidates: candidates,
                });
            }
        }
    }
    summary.current_state_established = summary.unresolved_current_rows.is_empty();
    summary.selected_current_records = selected;
    summary.content_catalog = Some(catalog);
    Ok(summary)
}

fn selection_for(
    catalog: &crate::native_content_documents::Catalog,
    key: &str,
    id: u64,
    candidates: usize,
) -> (Option<CurrentSelection>, &'static str) {
    let Some(document) = catalog.documents.get(key) else {
        return (None, "namespace_absent_from_current_content_catalog");
    };
    let Some(identity) = document.identities.get(&id) else {
        return (None, "not_in_current_content_element_table");
    };
    if candidates != 1 {
        return (None, "ambiguous_physical_candidates");
    }
    (
        Some(CurrentSelection {
            identity: identity.clone(),
            table_source: document.source.clone(),
            selection_rule: "explicit_content_ElemTable_membership_and_unique_physical_locator",
        }),
        "content_local_current",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_record_lengths_and_group_totals() {
        let source = GroupSource {
            content_key: Some([1; 16]),
            channel: 102,
            first_marker_offset: 0,
            segment_count: 1,
            declared_objects: 1,
            declared_body_bytes: 2,
        };
        let mut body = vec![0; 16];
        body[0] = 7;
        body[12] = 2;
        body.extend_from_slice(&[1, 2, 2, 0, 0, 0]);
        assert_eq!(rows(&body, &source).unwrap(), vec![(7, 0, 16, 18)]);
        body[18] = 3;
        assert!(rows(&body, &source).is_err());
        body[18] = 2;
        let mut wrong = source.clone();
        wrong.declared_objects = 2;
        assert!(rows(&body, &wrong).is_err());
        assert!(rows(&body[..19], &source).is_err());
    }
    #[test]
    fn explicit_table_selection_refuses_duplicates_and_orphan_namespaces_regardless_of_order() {
        use crate::native_content_documents::{
            Catalog, ContentDocument, LocalIdentity, TableSource,
        };
        let identity = LocalIdentity {
            local_element_id: 7,
            original_id_suffix: 70,
            creation_episode: 0,
            stored_revision: 4,
            other_revision: -1,
            owning_element_id: -1,
            partition_id: 0,
            row_index: 0,
            raw_fields: serde_json::json!({"test":true}),
        };
        let document = ContentDocument {
            content_key_raw_hex: "current".into(),
            owner_family_local_id: 3,
            identities: BTreeMap::from([(7, identity)]),
            graveyard_rows: vec![],
            local_history_pointer_present: false,
            source: TableSource {
                stream: "Global/ContentDocuments",
                content_marker_offset: 10,
                document_body_offset: 42,
                document_body_bytes: 100,
                document_body_sha256: "catalog-body".into(),
                element_table_object: 1,
                element_table_offset: 20,
                root_pointer_field: "ADocument.m_elemTable",
            },
        };
        let catalog = Catalog {
            source_sha256: "catalog".into(),
            schema_sha256: "schema".into(),
            documents: BTreeMap::from([("current".into(), document)]),
        };
        assert_eq!(
            selection_for(&catalog, "current", 7, 1)
                .0
                .unwrap()
                .identity
                .stored_revision,
            4
        );
        for order in [["old-body", "new-body"], ["new-body", "old-body"]] {
            for _body in order {
                assert!(
                    selection_for(&catalog, "current", 7, order.len())
                        .0
                        .is_none()
                );
            }
        }
        assert!(selection_for(&catalog, "old-namespace", 7, 1).0.is_none());
        assert!(selection_for(&catalog, "current", 8, 1).0.is_none());
        assert!(selection_for(&catalog, "current", 7, 0).0.is_none());
    }
}
