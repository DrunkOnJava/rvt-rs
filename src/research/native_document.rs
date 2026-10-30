//! Streaming current-record extraction with structural schemas and native IDs.
use crate::le_bytes;
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
    /// Records whose class is one of these are selected as well as
    /// `selected_ids`; with `selected_ids` empty, only they are.
    // Tracker: B79.
    pub selected_classes: BTreeSet<String>,
    pub channels: BTreeSet<u64>,
    /// A cap on a stored stream tighter than the file's own
    /// [`crate::reader::OpenLimits::max_stream_bytes`]. The default,
    /// `u64::MAX`, leaves the file's limit, which `--large-model` raises; a
    /// partition over the limit is named in [`RevitFile::unread_streams`] and
    /// skipped, so the rest of the file is still read (#1075).
    pub max_stream_bytes: u64,
    pub max_group_bytes: usize,
    pub max_graph_values: usize,
    pub max_graph_objects: usize,
    /// When set, only records of these classes are decoded as graphs; every
    /// other selected record is emitted with its Element base prefix alone
    /// ([`Record::base`], [`crate::native_element::decode_base`]). That reads
    /// a whole file's phase fields without its graph cost (#328).
    pub graph_classes: Option<BTreeSet<String>>,
    /// The releases the caller's reader is measured on. A release outside
    /// [`NATIVE_RELEASES`] is always refused. The default is 2023 to 2027,
    /// where every reader built on these records was calibrated; a reader
    /// measured on 2015 to 2022 as well (saved parameters, the phase order:
    /// RE-280, RE-281) names [`NATIVE_RELEASES`].
    pub releases: std::ops::RangeInclusive<u32>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            selected_ids: BTreeSet::new(),
            selected_classes: BTreeSet::new(),
            channels: BTreeSet::from([102]),
            max_stream_bytes: u64::MAX,
            max_group_bytes: 256 * 1024 * 1024,
            max_graph_values: 100_000,
            max_graph_objects: 100_000,
            graph_classes: None,
            releases: 2023..=2027,
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
    /// The record's Element base prefix, where [`Options::graph_classes`]
    /// left its graph undecoded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<crate::native_element::ElementBase>,
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
    /// Records emitted from the latest partition holding the element's
    /// records because the partition its stored revision routes to holds
    /// none of them (RE-145, #548).
    pub current_records_outside_route: usize,
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
#[derive(Debug, Clone, Default, Serialize)]
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
            opaque_suffix_sha256: Sha256::digest(suffix)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            suffix_interpreted: suffix.is_empty(),
        },
    ))
}

/// Stored partition `name` under the file's own stream limit, or `cap` where
/// that is tighter. `None` when it is over the limit: [`RevitFile::read_stream`]
/// names it in [`RevitFile::unread_streams`] under the file's limit.
fn read_partition(file: &mut RevitFile, name: &str, cap: u64) -> Result<Option<Vec<u8>>> {
    let read = if cap < file.limits().max_stream_bytes {
        file.read_stream_with_limit(name, cap)
    } else {
        file.read_stream(name)
    };
    match read {
        Ok(stored) => Ok(Some(stored)),
        Err(crate::Error::TooLarge { .. }) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
/// Read one CRC/ISIZE-validated member. Callers needing storage coverage should
/// use `extract`, whose summary records the uninterpreted suffix explicitly.
pub fn read_single(file: &mut RevitFile, name: &str) -> Result<Vec<u8>> {
    Ok(read_single_envelope(file, name)?.0)
}
fn read_single_envelope(file: &mut RevitFile, name: &str) -> Result<(Vec<u8>, StreamEnvelope)> {
    let stored = file.read_stream(name)?;
    let prepared = compression::prepare_stream_for_inflate(name, &stored);
    decode_single(&prepared, if name == "Formats/Latest" { 0 } else { 8 })
}
/// What every native extraction reads before the partitions: the global
/// streams' envelopes, the schema, the element index, the storage increments
/// and the extensible storage catalog, read once per file
/// ([`RevitFile::native_preamble`]).
pub(crate) struct Preamble {
    global_streams: BTreeMap<String, StreamEnvelope>,
    registry: schema_registry::Registry,
    index: native_index::ProjectIndex,
    increments: Vec<native_index::Increment>,
    es_catalog:
        std::result::Result<(crate::native_extensible_storage::Catalog, serde_json::Value), String>,
}
pub(crate) fn read_preamble(file: &mut RevitFile) -> Result<Preamble> {
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
    let es_catalog = read("Global/Latest")
        .and_then(|bytes| crate::native_es_catalog::decode(&bytes, &registry))
        .map_err(|error| format!("{error:#}"));
    Ok(Preamble {
        global_streams,
        registry,
        index,
        increments,
        es_catalog,
    })
}
/// The partition each of `ids` routes to by its stored revision
/// (`native_index::route_episode`), the partition [`extract`] reads its
/// current record from wherever that partition holds one (RE-145, #548). An
/// id the element index does not hold, or whose revision does not route, is
/// left out.
pub(crate) fn routed_partitions(
    file: &mut RevitFile,
    ids: &BTreeSet<u64>,
) -> Result<BTreeMap<u64, u32>> {
    let preamble = file.native_preamble()?;
    let present: BTreeSet<u32> = file
        .stream_names()
        .iter()
        .filter_map(|name| name.strip_prefix("Partitions/")?.parse().ok())
        .collect();
    Ok(ids
        .iter()
        .filter_map(|id| {
            let identity = preamble.index.identities.get(id)?;
            native_index::route_episode(identity.stored_revision, &preamble.increments, &present)
                .ok()
                .map(|partition| (*id, partition))
        })
        .collect())
}

/// For each id of `holders`, which maps an element to the partitions that
/// hold a copy of its data, the partition whose copy is current: the one its
/// stored revision routes to where that holds a copy, else the latest that
/// does, as [`extract`] reads records (RE-145, RE-216). An id the element
/// index does not route is left out.
pub fn current_partitions(
    file: &mut RevitFile,
    holders: &BTreeMap<u32, BTreeSet<u32>>,
) -> BTreeMap<u32, u32> {
    if holders.is_empty() {
        return BTreeMap::new();
    }
    let ids: BTreeSet<u64> = holders.keys().map(|&id| u64::from(id)).collect();
    let routed = routed_partitions(file, &ids).unwrap_or_default();
    holders
        .iter()
        .filter_map(|(&id, held)| {
            let routed = *routed.get(&u64::from(id))?;
            let current = if held.contains(&routed) {
                routed
            } else {
                *held.last()?
            };
            Some((id, current))
        })
        .collect()
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
    definition_options.selected_classes.clear();
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
/// Every current or historical record of every partition in [`Options`]'
/// default channels (#1032): its channel, ElementId, partition and the class
/// tag its body opens with. The first pass with the default channels and
/// budgets records it while it walks every partition for its own records,
/// and the handle keeps it; every later such pass finds the partitions
/// holding its selected records from it, rather than walking every
/// partition for them, and walks only those partitions.
#[derive(Debug)]
pub struct RecordIndex {
    records: Vec<IndexedRecord>,
}

/// One record of a [`RecordIndex`]: its channel, ElementId, partition and
/// the class tag its body opens with.
type IndexedRecord = (u64, u64, u32, Option<u16>);

/// Extract selected current indexed records' object graphs without the
/// parameter-definition pass [`extract`] makes over every definition owner
/// first, for callers that read objects' fields rather than saved parameter
/// values. Each record's saved metadata is read against no
/// definitions.
// Tracker: B80.
pub fn extract_graphs(
    file: &mut RevitFile,
    options: &Options,
    emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    extract_records(
        file,
        options,
        false,
        &crate::native_parameter_definitions::Registry::default(),
        emit,
    )
}
/// The releases whose native records frame: Revit 2015 to 2027. A record
/// group frames as Revit 2023 does from 2015 to 2023 (a 4-byte id) and with
/// an 8-byte id from 2024 (RE-280); earlier releases are not measured.
/// [`Options::releases`] says which of them a reader is measured on.
pub const NATIVE_RELEASES: std::ops::RangeInclusive<u32> = 2015..=2027;

/// A record group's records, `(id, offset, body start, body end)`: a
/// 4-byte (Revit 2015 to 2023) or 8-byte id, a header, the body and its
/// length repeated after it. The group's declared object count must equal
/// the records, and its declared body bytes their bodies', or, on 2015 to
/// 2017, their bodies plus the 4 bytes of trailing length each carries (all
/// 12,005 groups of those releases, none of the 15,757 of 2018 to 2022:
/// RE-280).
pub(crate) fn group_records(
    source: &GroupSource,
    bytes: &[u8],
    version: u32,
) -> Result<Vec<(u64, usize, usize, usize)>> {
    let id_bytes = if version <= 2023 { 4 } else { 8 };
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
        let length = u32::from_le_bytes(bytes[pos + header - 4..pos + header].try_into()?) as usize;
        let start = pos + header;
        let end = start
            .checked_add(length)
            .ok_or_else(|| anyhow::anyhow!("native record size overflow"))?;
        ensure!(
            end + 4 <= bytes.len()
                && u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize == length,
            "native record dual lengths disagree"
        );
        records.push((id, pos, start, end));
        body_sum += length as u64;
        pos = end + 4;
    }
    let count = records.len() as u64;
    ensure!(
        count == source.declared_objects
            && (body_sum == source.declared_body_bytes
                || (version <= 2017 && body_sum + 4 * count == source.declared_body_bytes)),
        "native group record counts/body sizes disagree"
    );
    Ok(records)
}
fn extract_records(
    file: &mut RevitFile,
    options: &Options,
    definitions_only: bool,
    definitions: &crate::native_parameter_definitions::Registry,
    mut emit: impl FnMut(Record) -> Result<()>,
) -> Result<Summary> {
    let version = file.layout_release()?;
    ensure!(
        NATIVE_RELEASES.contains(&version),
        "native record framing is unvalidated for Revit {version}"
    );
    ensure!(
        options.releases.contains(&version),
        "this native reader is not measured on Revit {version}"
    );
    let preamble = file.native_preamble()?;
    let Preamble {
        global_streams,
        registry,
        index,
        increments,
        es_catalog,
    } = &*preamble;
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
        .filter(|id| {
            (options.selected_ids.is_empty() && options.selected_classes.is_empty())
                || options.selected_ids.contains(id)
        })
        .copied()
        .collect();
    // A record is selected by its id or by the class its body's tag names.
    let is_selected = |id: u64, body: &[u8]| {
        selected.contains(&id)
            || (!options.selected_classes.is_empty()
                && index.identities.contains_key(&id)
                && le_bytes::u16_at(body, 0)
                    .and_then(|tag| registry.class(tag))
                    .is_some_and(|class| options.selected_classes.contains(&class.name)))
    };
    let mut summary = Summary {
        budgets: ResourceBudget {
            max_stream_bytes: options.max_stream_bytes.min(file.limits().max_stream_bytes),
            max_group_bytes: options.max_group_bytes,
            max_graph_values: options.max_graph_values,
            max_graph_objects: options.max_graph_objects,
        },
        global_streams: global_streams.clone(),
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
            summary.extensible_storage_catalog = Some(catalog.clone());
            summary.extensible_storage_catalog_source = Some(source.clone());
        }
        Err(error) => summary.extensible_storage_catalog_diagnostic = Some(error.clone()),
    }
    // An element's stored revision routes it to one partition
    // (`native_index::route_episode`), and its records elsewhere are
    // historical. On some files that partition holds none of the element's
    // records: 6 elements of Einhoven route to `Partitions/1` and 10 of
    // Core Interior to `Partitions/48` (RE-145, #548). Such an element's
    // current record is its copy in the latest partition that holds one,
    // the copy the route itself picks wherever it finds one (569 of 571
    // differing pairs on Core Interior, 887 of 887 on Autodesk's 2021
    // `rac_advanced`). A first pass finds the partitions that hold each
    // selected element's records.
    let mut holders: BTreeMap<(u64, u64), BTreeSet<u32>> = BTreeMap::new();
    // #1032: with the default budgets the handle's record index answers
    // this without a walk, and the walk below visits only the partitions
    // holding a selected record.
    let defaults = Options::default();
    let indexable = options.max_stream_bytes == defaults.max_stream_bytes
        && options.max_group_bytes == defaults.max_group_bytes
        && options.channels == defaults.channels;
    let record_index = indexable.then(|| file.native_record_index()).flatten();
    let mut indexed: Option<Vec<IndexedRecord>> =
        (indexable && record_index.is_none()).then(Vec::new);
    let walked: Option<BTreeSet<u32>> = record_index.as_ref().map(|record_index| {
        for &(channel, id, partition, tag) in &record_index.records {
            if !options.channels.contains(&channel) {
                continue;
            }
            let by_class = !options.selected_classes.is_empty()
                && index.identities.contains_key(&id)
                && tag
                    .and_then(|tag| registry.class(tag))
                    .is_some_and(|class| options.selected_classes.contains(&class.name));
            if selected.contains(&id) || by_class {
                holders.entry((channel, id)).or_default().insert(partition);
            }
        }
        holders.values().flatten().copied().collect()
    });
    for name in names.iter().filter(|_| walked.is_none()) {
        let partition: u32 = name[11..].parse()?;
        let Some(stored) = read_partition(file, name, options.max_stream_bytes)? else {
            continue;
        };
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        let inflated = file.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            registry,
            options.max_group_bytes,
            |source, bytes| {
                if source.content_key.is_some() || !options.channels.contains(&source.channel) {
                    return Ok(());
                }
                for (id, _, start, end) in group_records(source, bytes, version)? {
                    let body = &bytes[start..end];
                    if let Some(indexed) = indexed.as_mut() {
                        let tag = le_bytes::u16_at(body, 0);
                        indexed.push((source.channel, id, partition, tag));
                    }
                    if is_selected(id, body) {
                        holders
                            .entry((source.channel, id))
                            .or_default()
                            .insert(partition);
                    }
                }
                Ok(())
            },
        )?;
    }
    if let Some(records) = indexed {
        file.remember_native_record_index(RecordIndex { records });
    }
    let mut seen = BTreeSet::new();
    let mut seen_ids = BTreeSet::new();
    for name in names {
        let partition: u32 = name[11..].parse()?;
        if walked
            .as_ref()
            .is_some_and(|walked| !walked.contains(&partition))
        {
            continue;
        }
        let Some(stored) = read_partition(file, &name, options.max_stream_bytes)? else {
            continue;
        };
        let prepared = compression::prepare_stream_for_inflate(&name, &stored);
        let inflated = file.inflated_partition(&name).ok();
        let stats = native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            registry,
            options.max_group_bytes,
            |source, bytes| {
                if source.content_key.is_some() {
                    summary.skipped_embedded_content_groups += 1;
                    return Ok(());
                }
                let records = group_records(source, bytes, version)?;
                if !options.channels.contains(&source.channel) {
                    return Ok(());
                }
                for (id, offset, start, end) in records {
                    if !is_selected(id, &bytes[start..end]) {
                        continue;
                    }
                    let identity = &index.identities[&id];
                    let routed = native_index::route_episode(
                        identity.stored_revision,
                        increments,
                        &present,
                    )?;
                    let current = match holders.get(&(source.channel, id)) {
                        Some(held) if !held.contains(&routed) => {
                            *held.last().expect("an element in holders has a partition")
                        }
                        _ => routed,
                    };
                    if current != partition {
                        summary.skipped_historical_records += 1;
                        continue;
                    }
                    summary.current_records_outside_route += usize::from(current != routed);
                    ensure!(
                        seen.insert((source.channel, id)),
                        "ambiguous current record for {id} in channel{}",
                        source.channel
                    );
                    seen_ids.insert(id);
                    let body = &bytes[start..end];
                    let class_name = le_bytes::u16_at(body, 0)
                        .and_then(|tag| registry.class(tag))
                        .map(|c| c.name.clone());
                    if definitions_only {
                        let mut tag = le_bytes::u16_at(body, 0)
                            .ok_or_else(|| anyhow::anyhow!("truncated definition candidate tag"))?;
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
                    if let Some(graph_classes) = &options.graph_classes {
                        if !class_name
                            .as_ref()
                            .is_some_and(|c| graph_classes.contains(c))
                        {
                            let base = crate::native_element::decode_base(body, registry);
                            summary.emitted_records += 1;
                            emit(Record {
                                identity: identity.clone(),
                                derived_default_ifc_guid: None,
                                derived_identifier_diagnostic: None,
                                effective_ifc_parameter: None,
                                effective_ifc_parameter_diagnostic: None,
                                channel: source.channel,
                                class_name,
                                status: if base.is_ok() {
                                    "element_base"
                                } else {
                                    "unsupported_element_base"
                                }
                                .into(),
                                diagnostic: base.as_ref().err().map(ToString::to_string),
                                source: RecordSource {
                                    stream: name.clone(),
                                    group: source.clone(),
                                    group_record_offset: offset,
                                    body_bytes: body.len(),
                                    body_sha256: String::new(),
                                },
                                graph: None,
                                saved_metadata: None,
                                metadata_diagnostic: None,
                                base: base.ok(),
                            })?;
                            continue;
                        }
                    }
                    let decoded = native_parameters::decode_graph_with_catalog_usage(
                        body,
                        registry,
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
                            body_sha256: Sha256::digest(body)
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>(),
                        },
                        graph,
                        saved_metadata,
                        metadata_diagnostic,
                        base: None,
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
