//! Streaming current-record extraction with structural schemas and native IDs.
use crate::{
    RevitFile, compression,
    native_index::{self, Identity},
    native_parameters::{self, ObjectGraph},
    native_segments::{self, GroupSource},
    schema_registry,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ResourceBudget {
    pub max_stream_bytes: u64,
    pub max_group_bytes: usize,
    pub max_graph_values: usize,
    pub max_graph_objects: usize,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub selected_ids: BTreeSet<u64>,
    pub channels: BTreeSet<u64>,
    pub max_stream_bytes: u64,
    pub max_group_bytes: usize,
    pub max_graph_values: usize,
    pub max_graph_objects: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            selected_ids: BTreeSet::new(),
            channels: BTreeSet::from([102]),
            max_stream_bytes: 512 * 1024 * 1024,
            max_group_bytes: 256 * 1024 * 1024,
            max_graph_values: 100_000,
            max_graph_objects: 100_000,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Record {
    pub identity: Identity,
    pub derived_default_ifc_guid: Option<String>,
    pub derived_identifier_diagnostic: Option<String>,
    pub effective_ifc_parameter: Option<crate::native_metadata::EffectiveIfcParameter>,
    pub effective_ifc_parameter_diagnostic: Option<String>,
    pub channel: u64,
    pub class_name: Option<String>,
    pub status: String,
    pub diagnostic: Option<String>,
    pub source: RecordSource,
    pub graph: Option<ObjectGraph>,
    pub saved_metadata: Option<crate::native_metadata::SavedMetadata>,
    pub metadata_diagnostic: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct RecordSource {
    pub stream: String,
    pub group: GroupSource,
    pub group_record_offset: usize,
    pub body_bytes: usize,
    pub body_sha256: String,
}
#[derive(Debug, Default, Serialize)]
pub struct ClassCoverage {
    pub records: usize,
    pub complete_graphs: usize,
    pub unsupported_graphs: usize,
    pub body_bytes: u64,
    pub complete_graph_body_bytes: u64,
    pub diagnostics: BTreeMap<String, usize>,
}
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub budgets: ResourceBudget,
    pub global_streams: BTreeMap<String, StreamEnvelope>,
    pub extensible_storage_catalog: Option<crate::native_extensible_storage::Catalog>,
    pub extensible_storage_catalog_source: Option<serde_json::Value>,
    pub extensible_storage_catalog_diagnostic: Option<String>,
    pub classes: BTreeMap<String, ClassCoverage>,
    pub revit_version: u32,
    pub schema_sha256: String,
    pub indexed_elements: usize,
    pub graveyard_records: usize,
    pub selected_indexed_elements: usize,
    pub complete_document_geometry: bool,
    pub serialized_values_not_evaluated: bool,
    pub emitted_records: usize,
    pub complete_graph_records: usize,
    pub unsupported_graph_records: usize,
    pub refused_graph_ids: Vec<u64>,
    pub refused_graph_classes: BTreeMap<String, usize>,
    pub max_graph_values_observed: usize,
    pub max_graph_objects_observed: usize,
    pub projected_metadata_records: usize,
    pub unsupported_metadata_records: usize,
    pub skipped_embedded_content_groups: usize,
    pub skipped_historical_records: usize,
    pub requested_ids_absent_from_index: Vec<u64>,
    pub selected_ids_without_records: Vec<u64>,
    pub partitions: BTreeMap<String, native_segments::Statistics>,
    pub parameter_definitions: BTreeMap<i64, crate::native_parameter_definitions::Definition>,
    pub definition_diagnostics: BTreeMap<u64, String>,
    pub unit_formats: BTreeMap<String, serde_json::Value>,
    pub builtin_catalog_provenance: Option<serde_json::Value>,
    pub parameter_binding_context: serde_json::Value,
}
/// Integrity-checked member boundaries. The suffix is retained as opaque storage
/// evidence; its meaning and checksum algorithm are not yet decoded.
#[derive(Debug, Default, Serialize)]
pub struct StreamEnvelope {
    pub prepared_bytes: usize,
    pub member_offset: usize,
    pub member_bytes: usize,
    pub opaque_suffix_bytes: usize,
    pub opaque_suffix_sha256: String,
    pub suffix_interpreted: bool,
}
fn decode_single(prepared: &[u8], offset: usize) -> Result<(Vec<u8>, StreamEnvelope)> {
    let compressed = prepared
        .get(offset..)
        .ok_or_else(|| anyhow::anyhow!("truncated native stream prefix"))?;
    let mut decoder = flate2::bufread::GzDecoder::new(compressed);
    let mut bytes = Vec::new();
    (&mut decoder)
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 256 * 1024 * 1024,
        "native stream inflate budget exceeded"
    );
    let suffix = *decoder.get_ref();
    ensure!(
        suffix.len() <= compression::REVIT_STORED_PAGE_BYTES,
        "native stream opaque suffix exceeds one storage page"
    );
    Ok((
        bytes,
        StreamEnvelope {
            prepared_bytes: prepared.len(),
            member_offset: offset,
            member_bytes: compressed.len() - suffix.len(),
            opaque_suffix_bytes: suffix.len(),
            opaque_suffix_sha256: format!("{:x}", Sha256::digest(suffix)),
            suffix_interpreted: suffix.is_empty(),
        },
    ))
}

/// Read one CRC/ISIZE-validated member. Callers needing storage coverage should
/// use `extract`, whose summary records the uninterpreted suffix explicitly.
pub fn read_single(file: &mut RevitFile, name: &str) -> Result<Vec<u8>> {
    Ok(read_single_envelope(file, name)?.0)
}
fn read_single_envelope(file: &mut RevitFile, name: &str) -> Result<(Vec<u8>, StreamEnvelope)> {
    let stored = file.read_stream_with_limit(name, 512 * 1024 * 1024)?;
    let prepared = compression::prepare_stream_for_inflate(name, &stored);
    decode_single(&prepared, if name == "Formats/Latest" { 0 } else { 8 })
}
/// Extract selected current indexed records. Unsupported individual object
/// graphs produce explicit records and do not discard other decoded owners.
/// Framing, identity and routing errors abort instead of choosing a candidate.
pub fn extract(
    file: &mut RevitFile,
    options: &Options,
    emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    // Definition owners may follow their users physically. A bounded first pass
    // decodes only schema-declared definition owners; selected output still streams.
    let mut definitions = crate::native_parameter_definitions::Registry::default();
    let mut diagnostics = BTreeMap::new();
    let mut definition_options = options.clone();
    definition_options.selected_ids.clear();
    definition_options.channels = BTreeSet::from([102]);
    let definition_summary =
        extract_records(
            file,
            &definition_options,
            true,
            &crate::native_parameter_definitions::Registry::default(),
            |record| {
                let result =
                    match record.graph.as_ref() {
                        Some(graph)
                            if graph
                                .objects
                                .first()
                                .is_some_and(|root| root.class_name == "UnitsElem") =>
                        {
                            definitions.ingest_units(graph)
                        }
                        Some(graph)
                            if graph.objects.first().is_some_and(|root| {
                                matches!(
                                    root.class_name.as_str(),
                                    "ParamBinding" | "Family" | "FamilySymbol"
                                )
                            }) =>
                        {
                            definitions.ingest_binding_context(graph)
                        }
                        Some(graph) => crate::native_parameter_definitions::project(graph)
                            .and_then(|definition| match definition {
                                Some(definition) => definitions.insert(definition),
                                None => Err(anyhow::anyhow!(
                                    "definition owner has no nonnull definition"
                                )),
                            }),
                        None => Err(anyhow::anyhow!(
                            record
                                .diagnostic
                                .unwrap_or_else(|| "unsupported definition graph".into())
                        )),
                    };
                if let Err(error) = result {
                    diagnostics.insert(record.identity.element_id, error.to_string());
                }
                Ok(())
            },
        )?;
    definitions.enable_builtin_catalog(&definition_summary.schema_sha256)?;
    let mut summary = extract_records(file, options, false, &definitions, emit)?;
    summary.parameter_binding_context = serde_json::json!({
        "bindings": definitions.bindings,
        "family_categories": definitions.family_categories,
        "symbol_families": definitions.symbol_families,
    });
    summary.parameter_definitions = definitions.definitions;
    summary.unit_formats = definitions.unit_formats;
    summary.builtin_catalog_provenance = definitions.builtin_catalog_provenance;
    summary.definition_diagnostics = diagnostics;
    Ok(summary)
}
fn extract_records(
    file: &mut RevitFile,
    options: &Options,
    definitions_only: bool,
    definitions: &crate::native_parameter_definitions::Registry,
    mut emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    let version = file.basic_file_info()?.version;
    ensure!(
        matches!(version, 2023 | 2024 | 2027),
        "native record framing is unvalidated for Revit {version}"
    );
    let mut global_streams = BTreeMap::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let (bytes, envelope) = read_single_envelope(file, name)?;
        global_streams.insert(name.to_string(), envelope);
        Ok(bytes)
    };
    let registry = schema_registry::parse(&read("Formats/Latest")?)?;
    let episodes = native_index::creation_episodes(&read("Global/History")?, &registry)?;
    let index = native_index::parse(&read("Global/ElemTable")?, &registry, &episodes)?;
    let increments =
        native_index::storage_increments(&read("Global/DocumentIncrementTable")?, &registry)?;
    let es_catalog =
        read("Global/Latest").and_then(|bytes| crate::native_es_catalog::decode(&bytes, &registry));
    let mut names: Vec<_> = file
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let present: BTreeSet<u32> = names
        .iter()
        .map(|n| n[11..].parse())
        .collect::<Result<_, _>>()?;
    let selected: BTreeSet<_> = index
        .identities
        .keys()
        .filter(|id| options.selected_ids.is_empty() || options.selected_ids.contains(id))
        .copied()
        .collect();
    let mut summary = Summary {
        budgets: ResourceBudget {
            max_stream_bytes: options.max_stream_bytes,
            max_group_bytes: options.max_group_bytes,
            max_graph_values: options.max_graph_values,
            max_graph_objects: options.max_graph_objects,
        },
        global_streams,
        revit_version: version,
        schema_sha256: registry.source_sha256.clone(),
        indexed_elements: index.identities.len(),
        graveyard_records: index.graveyard_rows.len(),
        selected_indexed_elements: selected.len(),
        serialized_values_not_evaluated: true,
        requested_ids_absent_from_index: options
            .selected_ids
            .iter()
            .filter(|id| !index.identities.contains_key(id))
            .copied()
            .collect(),
        ..Default::default()
    };
    match es_catalog {
        Ok((catalog, source)) => {
            summary.extensible_storage_catalog = Some(catalog);
            summary.extensible_storage_catalog_source = Some(source);
        }
        Err(error) => summary.extensible_storage_catalog_diagnostic = Some(format!("{error:#}")),
    }
    let mut seen = BTreeSet::new();
    let mut seen_ids = BTreeSet::new();
    for name in names {
        let partition: u32 = name[11..].parse()?;
        let stored = file.read_stream_with_limit(&name, options.max_stream_bytes)?;
        let prepared = compression::prepare_stream_for_inflate(&name, &stored);
        let stats = native_segments::walk(
            &prepared,
            &registry,
            options.max_group_bytes,
            |source, bytes| {
                if source.content_key.is_some() {
                    summary.skipped_embedded_content_groups += 1;
                    return Ok(());
                }
                let id_bytes = if version == 2023 { 4 } else { 8 };
                let header = match source.channel {
                    101 => id_bytes + 4,
                    102 | 103 => id_bytes + 8,
                    _ => anyhow::bail!("unsupported native record channel {}", source.channel),
                };
                let mut pos = 0;
                let mut records = Vec::new();
                let mut body_sum = 0u64;
                while pos < bytes.len() {
                    ensure!(
                        bytes.len() - pos >= header + 4,
                        "truncated channel record header"
                    );
                    let id = if id_bytes == 4 {
                        u64::from(u32::from_le_bytes(bytes[pos..pos + 4].try_into()?))
                    } else {
                        u64::from_le_bytes(bytes[pos..pos + 8].try_into()?)
                    };
                    let length =
                        u32::from_le_bytes(bytes[pos + header - 4..pos + header].try_into()?)
                            as usize;
                    let start = pos + header;
                    let end = start
                        .checked_add(length)
                        .ok_or_else(|| anyhow::anyhow!("native record size overflow"))?;
                    ensure!(
                        end + 4 <= bytes.len()
                            && u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize
                                == length,
                        "native record dual lengths disagree"
                    );
                    records.push((id, pos, start, end));
                    body_sum += length as u64;
                    pos = end + 4;
                }
                ensure!(
                    records.len() as u64 == source.declared_objects
                        && body_sum == source.declared_body_bytes,
                    "native group record counts/body sizes disagree"
                );
                if !options.channels.contains(&source.channel) {
                    return Ok(());
                }
                for (id, offset, start, end) in records {
                    if !selected.contains(&id) {
                        continue;
                    }
                    let identity = &index.identities[&id];
                    if native_index::route_episode(identity.stored_revision, &increments, &present)?
                        != partition
                    {
                        summary.skipped_historical_records += 1;
                        continue;
                    }
                    ensure!(
                        seen.insert((source.channel, id)),
                        "ambiguous current record for {id} in channel{}",
                        source.channel
                    );
                    seen_ids.insert(id);
                    let body = &bytes[start..end];
                    let class_name = body
                        .get(..2)
                        .and_then(|b| registry.class(u16::from_le_bytes([b[0], b[1]])))
                        .map(|c| c.name.clone());
                    if definitions_only {
                        let mut tag = u16::from_le_bytes(
                            body.get(..2)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("truncated definition candidate tag")
                                })?
                                .try_into()?,
                        );
                        let mut owns_definition = matches!(
                            class_name.as_deref(),
                            Some("UnitsElem" | "ParamBinding" | "Family" | "FamilySymbol")
                        );
                        for _ in 0..128 {
                            let Some(class) = registry.class(tag) else {
                                break;
                            };
                            owns_definition |=
                                class.fields.iter().any(|field| field.name == "m_pParamDef");
                            tag = class.parent_reference.tag;
                        }
                        if !owns_definition {
                            continue;
                        }
                    }
                    let decoded = native_parameters::decode_graph_with_catalog_usage(
                        body,
                        &registry,
                        &native_parameters::GraphLimits {
                            max_values: options.max_graph_values,
                            max_objects: options.max_graph_objects,
                            ..Default::default()
                        },
                        summary.extensible_storage_catalog.as_ref(),
                    );
                    let decoded = decoded.and_then(|(graph, usage)| {
                        if source.channel == 102 {
                            let root = graph
                                .objects
                                .first()
                                .ok_or_else(|| anyhow::anyhow!("empty owner graph"))?;
                            let saved_id =
                                crate::native_metadata::identifier(&root.fields["m_id"])?;
                            ensure!(
                                u64::try_from(saved_id).ok() == Some(id),
                                "serialized owner identity disagrees with indexed record {id}"
                            );
                        }
                        Ok((graph, usage))
                    });
                    let (status, diagnostic, graph) = match decoded {
                        Ok((graph, usage)) => {
                            summary.max_graph_objects_observed =
                                summary.max_graph_objects_observed.max(usage.objects);
                            summary.max_graph_values_observed =
                                summary.max_graph_values_observed.max(usage.values);
                            summary.complete_graph_records += 1;
                            ("complete_bounded_graph", None, Some(graph))
                        }
                        Err(e) => {
                            summary.unsupported_graph_records += 1;
                            summary.refused_graph_ids.push(id);
                            if let Some(class_name) = &class_name {
                                *summary
                                    .refused_graph_classes
                                    .entry(class_name.clone())
                                    .or_default() += 1;
                            }
                            ("unsupported_graph", Some(e.to_string()), None)
                        }
                    };
                    let coverage = summary
                        .classes
                        .entry(
                            class_name
                                .clone()
                                .unwrap_or_else(|| "<unregistered>".into()),
                        )
                        .or_default();
                    coverage.records += 1;
                    coverage.body_bytes += body.len() as u64;
                    if graph.is_some() {
                        coverage.complete_graphs += 1;
                        coverage.complete_graph_body_bytes += body.len() as u64;
                    } else {
                        coverage.unsupported_graphs += 1;
                        *coverage
                            .diagnostics
                            .entry(diagnostic.clone().unwrap_or_default())
                            .or_default() += 1;
                    }
                    let (saved_metadata, metadata_diagnostic) = match graph.as_ref().map(|graph| {
                        crate::native_metadata::project_with_definitions(graph, definitions)
                    }) {
                        Some(Ok(value)) => (Some(value), None),
                        Some(Err(error)) => (None, Some(error.to_string())),
                        None => (None, None),
                    };
                    summary.projected_metadata_records += usize::from(saved_metadata.is_some());
                    summary.unsupported_metadata_records +=
                        usize::from(metadata_diagnostic.is_some());
                    summary.emitted_records += 1;
                    let (derived_default_ifc_guid, derived_identifier_diagnostic) =
                        match crate::native_metadata::derive_default_ifc_guid(&identity.unique_id) {
                            Ok(value) => (Some(value), None),
                            Err(error) => (None, Some(error.to_string())),
                        };
                    let (effective_ifc_parameter, effective_ifc_parameter_diagnostic) = match (
                        class_name.as_deref(),
                        saved_metadata.as_ref(),
                        derived_default_ifc_guid.as_deref(),
                    ) {
                        (Some(class), Some(metadata), Some(default_guid)) => {
                            match crate::native_metadata::effective_ifc_parameter(
                                class,
                                metadata,
                                default_guid,
                                definitions,
                            ) {
                                Ok(value) => (value, None),
                                Err(error) => (None, Some(error.to_string())),
                            }
                        }
                        _ => (None, None),
                    };
                    emit(Record {
                        identity: identity.clone(),
                        derived_default_ifc_guid,
                        derived_identifier_diagnostic,
                        effective_ifc_parameter,
                        effective_ifc_parameter_diagnostic,
                        channel: source.channel,
                        class_name,
                        status: status.into(),
                        diagnostic,
                        source: RecordSource {
                            stream: name.clone(),
                            group: source.clone(),
                            group_record_offset: offset,
                            body_bytes: body.len(),
                            body_sha256: format!("{:x}", Sha256::digest(body)),
                        },
                        graph,
                        saved_metadata,
                        metadata_diagnostic,
                    })?;
                }
                Ok(())
            },
        )?;
        summary.partitions.insert(name, stats);
    }
    summary.selected_ids_without_records = selected.difference(&seen_ids).copied().collect();
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn member() -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"saved state").unwrap();
        encoder.finish().unwrap()
    }
    #[test]
    fn opaque_suffix_is_accounted_separately_from_verified_payload() {
        let mut data = member();
        let size = data.len();
        data.extend_from_slice(&[0, 3, 9, 0]);
        let (payload, envelope) = decode_single(&data, 0).unwrap();
        assert_eq!(payload, b"saved state");
        assert_eq!(envelope.member_bytes, size);
        assert_eq!(envelope.opaque_suffix_bytes, 4);
        assert!(!envelope.suffix_interpreted);
    }
    #[test]
    fn damaged_or_truncated_member_is_not_an_opaque_suffix() {
        let mut data = member();
        data.pop();
        assert!(decode_single(&data, 0).is_err());
        let mut data = member();
        let crc = data.len() - 8;
        data[crc] ^= 1;
        assert!(decode_single(&data, 0).is_err());
    }
}
