//! Compare independently recovered owner-bound Mark values with a local JSON export.
//!
//! Usage: `cargo run --release --example probe_export_coverage -- FILE REFERENCE.json [DETAILS.json]`.
//! Reference shape: `{ "instances": { "key": { "element_id": 123, "mark": "AHU-1" } } }`.
//! The reference is used only after byte decoding, never to choose an owner or
//! value. Standard output contains aggregate counts only. Optional DETAILS.json
//! contains private source values and locators: keep it local. Set
//! RVT_PROBE_CARRY_BYTES=262144 to include bounded cross-member windows; carry
//! resets after failed inflate candidates. RVT_PROBE_PROPERTY_PARAMETER_ID
//! requests a source-model-scoped property ID for comparison only. Repeat over
//! the eleven-release corpus and matching local exports; unsupported profiles
//! intentionally produce no owner records. No private corpus is redistributed.
use rvt::{RevitFile, compression, elem_table, partition_parameter_records as parameters};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufReader;

#[derive(Deserialize)]
#[serde(untagged)]
enum ReferenceId {
    Number(u32),
    Text(String),
}
impl ReferenceId {
    fn id(&self) -> Option<u32> {
        match self {
            Self::Number(id) => Some(*id),
            Self::Text(s) => s.parse().ok(),
        }
    }
}
#[derive(Deserialize)]
struct ReferenceElement {
    element_id: ReferenceId,
    #[serde(default)]
    mark: Option<String>,
    #[serde(default)]
    type_mark: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    params: ReferenceParameters,
}
#[derive(Deserialize, Default)]
struct ReferenceParameters {
    #[serde(default, rename = "Panel Name")]
    panel_name: Option<ReferenceStringValue>,
    #[serde(default, rename = "Panel Name DLB")]
    panel_name_dlb: Option<ReferenceStringValue>,
}
#[derive(Deserialize)]
struct ReferenceStringValue {
    value: Option<String>,
}
#[derive(Deserialize)]
struct Reference {
    instances: BTreeMap<String, ReferenceElement>,
    #[serde(default)]
    types: BTreeMap<String, ReferenceElement>,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        (3..=4).contains(&args.len()),
        "expected FILE REFERENCE.json [DETAILS.json]"
    );
    let limits = rvt::reader::OpenLimits {
        max_stream_bytes: std::fs::metadata(&args[1])?.len(),
        ..Default::default()
    };
    let mut rf = RevitFile::open_with_limits(&args[1], limits)?;
    let version = rf.basic_file_info()?.version;
    let header = elem_table::parse_header(&mut rf)?;
    let records = elem_table::parse_records(&mut rf)?;
    let declared: BTreeSet<_> = records.iter().map(|r| r.id_primary).collect();
    let indexed_records = records.len();
    drop(records);
    let formats =
        compression::diagnose_formats_latest_integrity(&rf.read_stream("Formats/Latest")?);
    let reference: Reference =
        serde_json::from_reader(BufReader::new(std::fs::File::open(&args[2])?))?;
    let expected: BTreeMap<_, _> = reference
        .instances
        .values()
        .filter_map(|e| {
            e.element_id
                .id()
                .map(|id| (id, e.mark.clone().unwrap_or_default()))
        })
        .collect();
    let mut owned_ids = BTreeSet::new();
    let mut values: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut type_marks: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut tail_labels: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut panel_names: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let property_parameter_id: Option<i32> = std::env::var("RVT_PROBE_PROPERTY_PARAMETER_ID")
        .ok()
        .map(|s| s.parse())
        .transpose()?;
    let mut property_values: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut member_failures = 0usize;
    let mut member_count = 0usize;
    let mut inflated_bytes = 0usize;
    let mut frame_count = 0usize;
    let carry_bytes: usize = std::env::var("RVT_PROBE_CARRY_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    anyhow::ensure!(
        carry_bytes <= parameters::MAX_RECORD_BODY_BYTES + 272,
        "carry exceeds bounded-record cap"
    );
    let mut locations: BTreeMap<u32, Vec<serde_json::Value>> = BTreeMap::new();
    for name in rf
        .stream_names()
        .into_iter()
        .filter(|n| n.starts_with("Partitions/"))
    {
        let stored = rf.read_stream(&name)?;
        let prepared = compression::prepare_stream_for_inflate(&name, &stored);
        let mut carry = Vec::new();
        let mut logical_offset = 0usize;
        for offset in compression::find_gzip_offsets(&prepared) {
            let member = match compression::inflate_at(&prepared, offset) {
                Ok(b) => b,
                Err(_) => {
                    member_failures += 1;
                    carry.clear();
                    continue;
                }
            };
            member_count += 1;
            inflated_bytes += member.len();
            let old_length = carry.len();
            let window_offset = logical_offset.saturating_sub(old_length);
            logical_offset += member.len();
            let mut bytes = std::mem::take(&mut carry);
            bytes.extend_from_slice(&member);
            for record in parameters::scan_owned_parameter_records(&bytes, &declared, version) {
                if record.body_range.end + 4 <= old_length {
                    continue;
                }
                frame_count += 1;
                owned_ids.insert(record.element_id);
                if let Some(parameter_id) = property_parameter_id {
                    for value in
                        parameters::find_property_string_occurrences(&bytes, &record, parameter_id)
                    {
                        property_values
                            .entry(value.record_element_id)
                            .or_default()
                            .insert(value.value);
                    }
                }
                if let Some(label) =
                    parameters::find_instance_tail_label(&bytes, &record, &declared)
                {
                    tail_labels
                        .entry(label.element_id)
                        .or_default()
                        .insert(label.value);
                }
                for value in parameters::find_string_parameter(&bytes, &record, -1_140_078) {
                    panel_names
                        .entry(value.element_id)
                        .or_default()
                        .insert(value.value);
                }
                if args.len() == 4 && expected.contains_key(&record.element_id) {
                    locations.entry(record.element_id).or_default().push(serde_json::json!({"stream":name,"gzip_offset":offset,"window_offset":window_offset,"record_offset":record.offset,"body_len":record.body_range.len(),"class_tag":record.class_tag}));
                }
                for value in parameters::find_string_parameter(
                    &bytes,
                    &record,
                    parameters::INSTANCE_MARK_PARAMETER_ID,
                ) {
                    values
                        .entry(value.element_id)
                        .or_default()
                        .insert(value.value);
                }
                for value in parameters::find_string_parameter(
                    &bytes,
                    &record,
                    parameters::TYPE_MARK_PARAMETER_ID,
                ) {
                    type_marks
                        .entry(value.element_id)
                        .or_default()
                        .insert(value.value);
                }
            }
            let start = bytes.len().saturating_sub(carry_bytes);
            carry.extend_from_slice(&bytes[start..]);
        }
    }
    let nonempty: BTreeMap<_, _> = expected.iter().filter(|(_, v)| !v.is_empty()).collect();
    let compared = expected
        .iter()
        .filter(|(id, _)| values.contains_key(id))
        .count();
    let mismatches = expected
        .iter()
        .filter(|(id, v)| values.get(id).is_some_and(|found| !found.contains(*v)))
        .count();
    let conflicts = expected
        .keys()
        .filter(|id| values.get(id).is_some_and(|found| found.len() > 1))
        .count();
    let expected_type_marks: BTreeMap<_, _> = reference
        .types
        .values()
        .filter_map(|e| {
            e.element_id
                .id()
                .map(|id| (id, e.type_mark.clone().unwrap_or_default()))
        })
        .collect();
    let expected_panels: BTreeMap<_, _> = reference
        .instances
        .values()
        .filter_map(|e| {
            Some((
                e.element_id.id()?,
                e.params.panel_name.as_ref()?.value.clone()?,
            ))
        })
        .filter(|(_, v)| !v.is_empty())
        .collect();
    let expected_project_panels: BTreeMap<_, _> = reference
        .instances
        .values()
        .filter_map(|e| {
            Some((
                e.element_id.id()?,
                e.params.panel_name_dlb.as_ref()?.value.clone()?,
            ))
        })
        .filter(|(_, v)| !v.is_empty())
        .collect();
    if let Some(path) = args.get(3) {
        let rows: Vec<_> = reference.instances.values().filter_map(|e|e.element_id.id().map(|id|serde_json::json!({"id":id,"category":e.category,"family":e.family,"expected_mark":e.mark,"native_marks":values.get(&id),"expected_panel_name":expected_panels.get(&id),"native_panel_names":panel_names.get(&id),"native_project_property_values":property_values.get(&id),"expected_project_panel_name":expected_project_panels.get(&id),"native_tail_labels":tail_labels.get(&id),"locations":locations.get(&id)}))).collect();
        serde_json::to_writer(std::io::BufWriter::new(std::fs::File::create(path)?), &rows)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "revit_version":version, "formats_integrity":formats,
            "declared_record_count":header.record_count, "indexed_records":indexed_records,
            "declared_unique_ids":declared.len(), "reference_instances":expected.len(),
            "reference_ids_in_index":expected.keys().filter(|id|declared.contains(id)).count(),
            "members_inflated":member_count, "gzip_candidate_failures":member_failures,
            "inflated_bytes":inflated_bytes, "bounded_frames":frame_count, "bounded_unique_ids":owned_ids.len(),
            "carry_bytes":carry_bytes,
            "reference_ids_with_bounded_frame":expected.keys().filter(|id|owned_ids.contains(id)).count(),
            "all_ids_with_mark_occurrences":values.len(), "reference_ids_with_mark_occurrences":compared,
            "reference_nonempty_marks":nonempty.len(),
            "reference_nonempty_marks_recovered":nonempty.iter().filter(|(id,v)|values.get(id).is_some_and(|set|set.contains(**v))).count(),
            "reference_nonempty_marks_covered_by_parameter_or_tail":nonempty.iter().filter(|(id,v)|values.get(id).is_some_and(|set|set.contains(**v)) || tail_labels.get(id).is_some_and(|set|set.contains(**v))).count(),
            "reference_nonempty_marks_with_different_tail_label":nonempty.iter().filter(|(id,v)|tail_labels.get(id).is_some_and(|set|!set.contains(**v))).count(),
            "reference_ids_with_tail_labels":expected.keys().filter(|id|tail_labels.contains_key(id)).count(),
            "all_ids_with_tail_labels":tail_labels.len(),
            "reference_mark_mismatches":mismatches, "reference_ids_with_conflicting_mark_values":conflicts,
            "reference_nonempty_type_marks": expected_type_marks.values().filter(|v|!v.is_empty()).count(),
            "reference_nonempty_type_marks_recovered": expected_type_marks.iter().filter(|(id,v)|!v.is_empty() && type_marks.get(id).is_some_and(|set|set.contains(*v))).count(),
            "reference_type_mark_mismatches": expected_type_marks.iter().filter(|(id,v)|type_marks.get(id).is_some_and(|set|!set.contains(*v))).count(),
            "reference_ids_with_conflicting_type_marks": expected_type_marks.keys().filter(|id|type_marks.get(id).is_some_and(|set|set.len()>1)).count(),
            "reference_nonempty_panel_names":expected_panels.len(),
            "reference_panel_names_recovered":expected_panels.iter().filter(|(id,v)|panel_names.get(id).is_some_and(|set|set.contains(*v))).count(),
            "reference_panel_names_covered_by_parameter_or_tail":expected_panels.iter().filter(|(id,v)|panel_names.get(id).is_some_and(|set|set.contains(*v)) || tail_labels.get(id).is_some_and(|set|set.contains(*v))).count(),
            "reference_panel_names_with_empty_native_value":expected_panels.keys().filter(|id|panel_names.get(id).is_some_and(|set|set.contains(""))).count(),
            "reference_panel_names_with_nonempty_native_disagreement":expected_panels.iter().filter(|(id,v)|panel_names.get(id).is_some_and(|set|set.iter().any(|found|!found.is_empty() && found != *v))).count(),
            "project_property_parameter_id":property_parameter_id,
            "reference_project_panel_names":expected_project_panels.len(),
            "reference_project_panel_names_recovered":expected_project_panels.iter().filter(|(id,v)|property_values.get(id).is_some_and(|set|set.contains(*v))).count(),
            "reference_project_panel_name_disagreements":expected_project_panels.iter().filter(|(id,v)|property_values.get(id).is_some_and(|set|set.iter().any(|found|found != *v))).count(),
            "reference_panel_names_covered_by_builtin_or_project_parameter":expected_panels.iter().filter(|(id,v)|panel_names.get(id).is_some_and(|set|set.contains(*v)) || property_values.get(id).is_some_and(|set|set.contains(*v))).count(),
            "limitation":"Bounded carry can omit larger cross-member records and resets on failed inflate candidates; alternate record encodings, source-version applicability, and typed objects remain unresolved."
        }))?
    );
    Ok(())
}
