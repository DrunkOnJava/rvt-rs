//! Saved-record change tracking. Same saved UID is not a cross-document entity match.
use crate::{native_document::Record, native_index::Identity, native_metadata::identifier};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSource {
    pub channel: u64,
    pub source_object: usize,
    pub source_field: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub storage_type: String,
    pub raw_value: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParameterState {
    pub candidates: Vec<Candidate>,
    pub definition_type_id: Option<String>,
    pub has_value: Option<bool>,
    pub is_read_only: Option<bool>,
    pub sources: Vec<ValueSource>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyState {
    pub sha256: String,
    pub byte_count: usize,
    pub stream: String,
    pub group_record_offset: usize,
    pub group_first_marker_offset: usize,
    pub graph_status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementState {
    pub identity: Identity,
    pub class_name: Option<String>,
    pub bodies: BTreeMap<u64, BodyState>,
    pub projected_parameters: BTreeMap<i64, ParameterState>,
    pub parameter_projection_available: bool,
    pub saved_references: BTreeMap<String, i64>,
    pub family_content_key_raw_hex: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub format: String,
    pub source_sha256: Option<String>,
    pub expected_indexed_elements: Option<usize>,
    pub complete_current_record_inventory: bool,
    pub identity_scope: String,
    pub elements: BTreeMap<u64, ElementState>,
    pub diagnostics: Vec<String>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    source_sha256: Option<String>,
    expected_indexed_elements: Option<usize>,
    elements: BTreeMap<u64, ElementState>,
    diagnostics: Vec<String>,
}
impl InventoryBuilder {
    pub fn set_source_sha256(&mut self, sha256: String) -> Result<()> {
        ensure!(
            sha256.len() == 64 && sha256.bytes().all(|c| c.is_ascii_hexdigit()),
            "invalid source SHA256"
        );
        self.source_sha256 = Some(sha256);
        Ok(())
    }
    pub fn set_expected_indexed_elements(&mut self, count: usize) {
        self.expected_indexed_elements = Some(count);
    }
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        let state = self
            .elements
            .entry(record.identity.element_id)
            .or_insert_with(|| ElementState {
                identity: record.identity.clone(),
                class_name: record.class_name.clone(),
                bodies: BTreeMap::new(),
                projected_parameters: BTreeMap::new(),
                parameter_projection_available: false,
                saved_references: BTreeMap::new(),
                family_content_key_raw_hex: None,
            });
        ensure!(
            state.identity.unique_id == record.identity.unique_id,
            "current ID maps to conflicting saved UIDs"
        );
        ensure!(
            state
                .bodies
                .insert(
                    record.channel,
                    BodyState {
                        sha256: record.source.body_sha256.clone(),
                        byte_count: record.source.body_bytes,
                        stream: record.source.stream.clone(),
                        group_record_offset: record.source.group_record_offset,
                        group_first_marker_offset: record.source.group.first_marker_offset,
                        graph_status: record.status.clone()
                    }
                )
                .is_none(),
            "duplicate current record channel"
        );
        if record.channel != 102 {
            return Ok(());
        }
        if let Some(metadata) = &record.saved_metadata {
            state.parameter_projection_available = true;
            state.saved_references = metadata.root_references.clone();
            for set in &metadata.parameter_sets {
                for parameter in &set.parameters {
                    insert_parameter(
                        state,
                        parameter.serialized_parameter_id,
                        &parameter.storage_type,
                        &parameter.raw_value,
                        ValueSource {
                            channel: record.channel,
                            source_object: set.source_object,
                            source_field: set.root_pointer_field.clone(),
                        },
                    );
                }
            }
            for parameter in &metadata.resolved_family_parameters {
                insert_parameter(
                    state,
                    parameter.parameter_id,
                    &parameter.storage_type,
                    &parameter.raw_value,
                    ValueSource {
                        channel: record.channel,
                        source_object: parameter.source_object,
                        source_field: "FamilyParams.m_params".into(),
                    },
                );
            }
            for parameter in &metadata.raw_field_parameters {
                insert_parameter(
                    state,
                    parameter.parameter_id,
                    &parameter.storage_type,
                    &parameter.raw_value,
                    ValueSource {
                        channel: record.channel,
                        source_object: parameter.source_object,
                        source_field: parameter.source_field.clone(),
                    },
                );
            }
            for definition in &metadata.parameter_definitions {
                state
                    .projected_parameters
                    .entry(definition.parameter_id)
                    .or_default()
                    .definition_type_id = definition.type_id.clone();
            }
            for declared in &metadata.declared_custom_parameters {
                let parameter = state
                    .projected_parameters
                    .entry(declared.parameter_id)
                    .or_default();
                parameter.has_value = Some(declared.has_value);
                parameter.is_read_only = declared.is_read_only;
            }
        }
        if let Some(graph) = &record.graph
            && let Some(root) = graph.objects.first()
        {
            for field in [
                "m_masterSymbolId",
                "m_familyId",
                "m_WallAttributesId",
                "m_floorTypeId",
                "m_idType",
                "m_hostId",
                "m_superInstanceId",
            ] {
                if let Some(raw) = root.fields.get(field) {
                    state
                        .saved_references
                        .insert(field.into(), identifier(raw)?);
                }
            }
            if root.class_name == "Family"
                && let Some(pointer) = root.fields.get("m_oFamDoc")
                && pointer["pointer_token"].as_u64() != Some(0)
            {
                let edges: Vec<_> = graph
                    .edges
                    .iter()
                    .filter(|e| {
                        e.source_object_index == 0
                            && Some(e.pointer_offset as u64) == pointer["offset"].as_u64()
                    })
                    .collect();
                if edges.len() == 1 {
                    let document = &graph.objects[edges[0].target_object_index];
                    if document.class_name == "FamilyDocument"
                        && let Some(bytes) =
                            document.fields["m_contentDocGUID"]["m_guid"]["guid_bytes"].as_array()
                    {
                        ensure!(bytes.len() == 16, "content namespace GUID length");
                        let bytes: Vec<_> = bytes
                            .iter()
                            .map(|v| {
                                v.as_u64()
                                    .filter(|b| *b <= 255)
                                    .ok_or_else(|| anyhow::anyhow!("content key byte invalid"))
                            })
                            .collect::<Result<_>>()?;
                        state.family_content_key_raw_hex =
                            Some(bytes.iter().map(|b| format!("{b:02x}")).collect());
                    }
                } else {
                    self.diagnostics.push(format!(
                        "family {} content pointer unresolved",
                        record.identity.element_id
                    ));
                }
            }
        }
        Ok(())
    }
    /// Add class-qualified, current project material relationships already
    /// decoded by the material inventory. These are recomputation dependencies.
    pub fn ingest_material_dependencies(
        &mut self,
        inventory: &crate::native_materials::Inventory,
    ) -> Result<()> {
        for reference in inventory
            .materials
            .iter()
            .flat_map(|m| m.references.iter())
            .chain(
                inventory
                    .constructions
                    .iter()
                    .flat_map(|c| c.layers.iter().map(|l| &l.material)),
            )
        {
            if reference.resolution != "resolved_current_element" {
                continue;
            }
            let target = reference
                .target_identity
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("resolved material dependency lacks identity"))?;
            let target_state = self.elements.get(&target.element_id).ok_or_else(|| {
                anyhow::anyhow!("material dependency target missing from snapshot")
            })?;
            ensure!(
                target_state.identity.unique_id == target.unique_id,
                "material dependency target UID differs"
            );
            let owner = self
                .elements
                .get_mut(&reference.source.owner.element_id)
                .ok_or_else(|| {
                    anyhow::anyhow!("material dependency owner missing from snapshot")
                })?;
            ensure!(
                owner.identity.unique_id == reference.source.owner.unique_id,
                "material dependency owner UID differs"
            );
            let key = format!(
                "material:{}:{}",
                reference.source.source_object, reference.source.source_field
            );
            ensure!(
                owner
                    .saved_references
                    .insert(key, reference.raw_target_id)
                    .is_none(),
                "duplicate material dependency source"
            );
        }
        Ok(())
    }
    pub fn finish(mut self) -> Result<Snapshot> {
        let mut uids = BTreeSet::new();
        for state in self.elements.values_mut() {
            ensure!(
                uids.insert(state.identity.unique_id.clone()),
                "duplicate saved UID across current IDs"
            );
            for (parameter_id, parameter) in &state.projected_parameters {
                if parameter.candidates.len() == 1 {
                    let candidate = &parameter.candidates[0];
                    if candidate.storage_type == "ElementId"
                        && let Some(target) = candidate.raw_value.as_i64().filter(|v| *v > 0)
                    {
                        state
                            .saved_references
                            .insert(format!("parameter:{parameter_id}"), target);
                    }
                }
            }
            for parameter in state.projected_parameters.values_mut() {
                parameter
                    .candidates
                    .sort_by_key(|c| serde_json::to_string(c).unwrap_or_default());
            }
        }
        let complete = self.expected_indexed_elements == Some(self.elements.len());
        Ok(Snapshot{format:"rvt-native-revision-snapshot/v1".into(),source_sha256:self.source_sha256,expected_indexed_elements:self.expected_indexed_elements,complete_current_record_inventory:complete,identity_scope:"Current saved element identities in one supplied document snapshot; copied documents may retain these UIDs. No world-entity or document-lineage equivalence is inferred.".into(),elements:self.elements,diagnostics:self.diagnostics})
    }
}
fn insert_parameter(
    state: &mut ElementState,
    id: i64,
    storage: &str,
    value: &Value,
    source: ValueSource,
) {
    let parameter = state.projected_parameters.entry(id).or_default();
    let candidate = Candidate {
        storage_type: storage.into(),
        raw_value: value.clone(),
    };
    if !parameter.candidates.contains(&candidate) {
        parameter.candidates.push(candidate);
    }
    parameter.sources.push(source);
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterChange {
    pub parameter_id: i64,
    pub before: Option<ParameterState>,
    pub after: Option<ParameterState>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementChange {
    pub before: Identity,
    pub after: Identity,
    pub body_changed_channels: Vec<u64>,
    pub storage_location_changed_channels: Vec<u64>,
    pub index_revision_changed: bool,
    pub projected_parameter_changes: Vec<ParameterChange>,
    pub projected_parameter_comparison_available: bool,
    pub saved_references_changed: bool,
    pub family_content_namespace_changed: bool,
    pub before_family_content_key_raw_hex: Option<String>,
    pub after_family_content_key_raw_hex: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInvalidation {
    pub element_id: u64,
    pub saved_unique_id: String,
    pub directly_changed_reference_targets: Vec<u64>,
    pub transitive_changed_reference_targets: Vec<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub format: String,
    pub before_source_sha256: Option<String>,
    pub after_source_sha256: Option<String>,
    pub file_bytes_changed: Option<bool>,
    pub comparison_scope_complete: bool,
    pub added: Vec<Identity>,
    pub removed: Vec<Identity>,
    pub replaced_local_ids: Vec<u64>,
    pub changed: Vec<ElementChange>,
    pub unchanged_saved_records: usize,
    pub dependency_invalidations: Vec<DependencyInvalidation>,
    pub limitations: Vec<String>,
}
fn parameters_equal(a: &ParameterState, b: &ParameterState) -> bool {
    a.candidates == b.candidates
        && a.definition_type_id == b.definition_type_id
        && a.has_value == b.has_value
        && a.is_read_only == b.is_read_only
}
fn by_uid(snapshot: &Snapshot) -> Result<BTreeMap<String, &ElementState>> {
    let mut result = BTreeMap::new();
    for (id, state) in &snapshot.elements {
        ensure!(
            *id == state.identity.element_id,
            "snapshot key/identity mismatch"
        );
        ensure!(
            result
                .insert(state.identity.unique_id.clone(), state)
                .is_none(),
            "duplicate snapshot UID"
        );
    }
    Ok(result)
}
/// Compare supplied snapshots by saved UID. The caller supplies comparison scope;
/// this function does not establish that two files belong to one document lineage.
pub fn compare(before: &Snapshot, after: &Snapshot) -> Result<Delta> {
    ensure!(
        before.format == "rvt-native-revision-snapshot/v1" && after.format == before.format,
        "revision snapshot format mismatch"
    );
    let old = by_uid(before)?;
    let new = by_uid(after)?;
    let mut delta=Delta{format:"rvt-native-revision-delta/v1".into(),before_source_sha256:before.source_sha256.clone(),after_source_sha256:after.source_sha256.clone(),file_bytes_changed:before.source_sha256.as_ref().zip(after.source_sha256.as_ref()).map(|(a,b)|a!=b),comparison_scope_complete:before.complete_current_record_inventory&&after.complete_current_record_inventory,added:Vec::new(),removed:Vec::new(),replaced_local_ids:Vec::new(),changed:Vec::new(),unchanged_saved_records:0,dependency_invalidations:Vec::new(),limitations:vec!["Saved UID correspondence does not establish cross-document world-entity or document-lineage identity.".into(),"Body hashes compare decoded current record bodies, independently of whole-file bytes and physical offsets.".into(),"Projected parameter differences cover only available native projections, not all API-evaluated semantics. Unchanged instance bodies can depend on changed type or family records.".into(),"Dependency invalidation follows explicit saved references; it is a recomputation hint, not proof that evaluated geometry or semantics changed.".into(),"Changed family content namespaces are reported without aliasing their local owners across reloads.".into()]};
    for (uid, state) in &old {
        if !new.contains_key(uid) {
            delta.removed.push(state.identity.clone());
        }
    }
    for (uid, state) in &new {
        if !old.contains_key(uid) {
            delta.added.push(state.identity.clone());
        }
    }
    for (id, a) in &before.elements {
        if let Some(b) = after.elements.get(id)
            && a.identity.unique_id != b.identity.unique_id
        {
            delta.replaced_local_ids.push(*id);
        }
    }
    let mut changed_ids: BTreeSet<u64> = delta
        .added
        .iter()
        .chain(delta.removed.iter())
        .map(|i| i.element_id)
        .collect();
    for (uid, a) in old {
        let Some(b) = new.get(&uid) else { continue };
        let channels: BTreeSet<_> = a.bodies.keys().chain(b.bodies.keys()).copied().collect();
        let body_changed_channels: Vec<_> = channels
            .into_iter()
            .filter(|c| a.bodies.get(c).map(|b| &b.sha256) != b.bodies.get(c).map(|b| &b.sha256))
            .collect();
        let storage_location_changed_channels: Vec<_> = a
            .bodies
            .iter()
            .filter_map(|(channel, old)| {
                b.bodies
                    .get(channel)
                    .filter(|new| {
                        old.stream != new.stream
                            || old.group_first_marker_offset != new.group_first_marker_offset
                            || old.group_record_offset != new.group_record_offset
                    })
                    .map(|_| *channel)
            })
            .collect();
        let mut projected_parameter_changes = Vec::new();
        let comparable = a.parameter_projection_available && b.parameter_projection_available;
        if comparable {
            let ids: BTreeSet<_> = a
                .projected_parameters
                .keys()
                .chain(b.projected_parameters.keys())
                .copied()
                .collect();
            for id in ids {
                let (x, y) = (
                    a.projected_parameters.get(&id),
                    b.projected_parameters.get(&id),
                );
                if !matches!((x,y),(Some(x),Some(y))if parameters_equal(x,y)) {
                    projected_parameter_changes.push(ParameterChange {
                        parameter_id: id,
                        before: x.cloned(),
                        after: y.cloned(),
                    });
                }
            }
        }
        let index_revision_changed = a.identity.stored_revision != b.identity.stored_revision
            || a.identity.other_revision != b.identity.other_revision;
        let references_changed = a.saved_references != b.saved_references;
        let namespace_changed = a.family_content_key_raw_hex != b.family_content_key_raw_hex;
        if !body_changed_channels.is_empty()
            || !storage_location_changed_channels.is_empty()
            || index_revision_changed
            || !projected_parameter_changes.is_empty()
            || references_changed
            || namespace_changed
            || a.identity.element_id != b.identity.element_id
        {
            if !body_changed_channels.is_empty()
                || !projected_parameter_changes.is_empty()
                || references_changed
                || namespace_changed
            {
                changed_ids.insert(b.identity.element_id);
            }
            delta.changed.push(ElementChange {
                before: a.identity.clone(),
                after: b.identity.clone(),
                body_changed_channels,
                storage_location_changed_channels,
                index_revision_changed,
                projected_parameter_changes,
                projected_parameter_comparison_available: comparable,
                saved_references_changed: references_changed,
                family_content_namespace_changed: namespace_changed,
                before_family_content_key_raw_hex: a.family_content_key_raw_hex.clone(),
                after_family_content_key_raw_hex: b.family_content_key_raw_hex.clone(),
            });
        } else {
            delta.unchanged_saved_records += 1;
        }
    }
    // Reverse-reference traversal reaches each owner once, including cycles.
    let mut reverse = BTreeMap::<u64, Vec<u64>>::new();
    for (id, state) in &after.elements {
        for target in state
            .saved_references
            .values()
            .filter_map(|r| u64::try_from(*r).ok())
        {
            reverse.entry(target).or_default().push(*id);
        }
    }
    let mut affected = changed_ids.clone();
    let mut queue: VecDeque<_> = changed_ids.iter().copied().collect();
    while let Some(target) = queue.pop_front() {
        if let Some(owners) = reverse.get(&target) {
            for owner in owners {
                if affected.insert(*owner) {
                    queue.push_back(*owner);
                }
            }
        }
    }
    for (id, state) in &after.elements {
        let targets: BTreeSet<_> = state
            .saved_references
            .values()
            .filter_map(|v| u64::try_from(*v).ok())
            .collect();
        let direct: Vec<_> = targets.intersection(&changed_ids).copied().collect();
        let transitive: Vec<_> = targets
            .intersection(&affected)
            .filter(|id| !changed_ids.contains(id))
            .copied()
            .collect();
        if !direct.is_empty() || !transitive.is_empty() {
            delta.dependency_invalidations.push(DependencyInvalidation {
                element_id: *id,
                saved_unique_id: state.identity.unique_id.clone(),
                directly_changed_reference_targets: direct,
                transitive_changed_reference_targets: transitive,
            });
        }
    }
    Ok(delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn state(id: u64, uid: &str) -> ElementState {
        ElementState {
            identity: Identity {
                element_id: id,
                original_id_suffix: id,
                creation_episode: 0,
                stored_revision: 0,
                other_revision: 0,
                row_offset: 0,
                raw_fields: Vec::new(),
                owning_element_id: -1,
                partition_id: 0,
                unique_id: uid.into(),
            },
            class_name: Some("Test".into()),
            bodies: BTreeMap::from([(
                102,
                BodyState {
                    sha256: "body-a".into(),
                    byte_count: 20,
                    stream: "Partitions/0".into(),
                    group_record_offset: 10,
                    group_first_marker_offset: 0,
                    graph_status: "complete".into(),
                },
            )]),
            projected_parameters: BTreeMap::new(),
            parameter_projection_available: true,
            saved_references: BTreeMap::new(),
            family_content_key_raw_hex: None,
        }
    }
    fn snapshot(states: Vec<ElementState>) -> Snapshot {
        Snapshot {
            format: "rvt-native-revision-snapshot/v1".into(),
            source_sha256: Some("file-a".into()),
            expected_indexed_elements: Some(states.len()),
            complete_current_record_inventory: true,
            identity_scope: "test".into(),
            elements: states
                .into_iter()
                .map(|s| (s.identity.element_id, s))
                .collect(),
            diagnostics: Vec::new(),
        }
    }
    #[test]
    fn element_id_parameter_adds_dependency_but_conflicts_do_not() {
        let mut builder = InventoryBuilder::default();
        let mut owner = state(1, "owner");
        insert_parameter(
            &mut owner,
            10,
            "ElementId",
            &json!(2),
            ValueSource {
                channel: 102,
                source_object: 0,
                source_field: "owned parameter".into(),
            },
        );
        insert_parameter(
            &mut owner,
            11,
            "ElementId",
            &json!(-1),
            ValueSource {
                channel: 102,
                source_object: 0,
                source_field: "owned parameter".into(),
            },
        );
        insert_parameter(
            &mut owner,
            12,
            "ElementId",
            &json!(3),
            ValueSource {
                channel: 102,
                source_object: 0,
                source_field: "owned parameter".into(),
            },
        );
        insert_parameter(
            &mut owner,
            12,
            "ElementId",
            &json!(4),
            ValueSource {
                channel: 102,
                source_object: 0,
                source_field: "other candidate".into(),
            },
        );
        builder.elements.insert(1, owner);
        let result = builder.finish().unwrap();
        assert_eq!(
            result.elements[&1].saved_references,
            BTreeMap::from([("parameter:10".into(), 2)])
        );
    }
    #[test]
    fn file_only_and_physical_relocation_are_not_body_or_parameter_changes() {
        let before = snapshot(vec![state(1, "saved-uid")]);
        let mut after = before.clone();
        after.source_sha256 = Some("file-b".into());
        let delta = compare(&before, &after).unwrap();
        assert_eq!(delta.file_bytes_changed, Some(true));
        assert!(delta.changed.is_empty());
        assert_eq!(delta.unchanged_saved_records, 1);
        after
            .elements
            .get_mut(&1)
            .unwrap()
            .bodies
            .get_mut(&102)
            .unwrap()
            .group_record_offset = 90;
        let delta = compare(&before, &after).unwrap();
        assert!(delta.changed[0].body_changed_channels.is_empty());
        assert_eq!(
            delta.changed[0].storage_location_changed_channels,
            vec![102]
        );
        assert!(delta.changed[0].projected_parameter_changes.is_empty());
        assert!(delta.dependency_invalidations.is_empty());
    }
    #[test]
    fn changed_type_invalidates_unchanged_instances_without_claiming_instance_value_change() {
        let mut ty = state(2, "type");
        ty.projected_parameters.insert(
            10,
            ParameterState {
                candidates: vec![Candidate {
                    storage_type: "Double".into(),
                    raw_value: json!(3.0),
                }],
                ..ParameterState::default()
            },
        );
        let mut instance = state(1, "instance");
        instance
            .saved_references
            .insert("m_masterSymbolId".into(), 2);
        let before = snapshot(vec![instance, ty]);
        let mut after = before.clone();
        let ty = after.elements.get_mut(&2).unwrap();
        ty.bodies.get_mut(&102).unwrap().sha256 = "body-b".into();
        ty.projected_parameters.get_mut(&10).unwrap().candidates[0].raw_value = json!(8.0);
        let delta = compare(&before, &after).unwrap();
        assert_eq!(delta.changed.len(), 1);
        assert_eq!(delta.changed[0].after.element_id, 2);
        assert_eq!(delta.changed[0].projected_parameter_changes.len(), 1);
        assert_eq!(delta.dependency_invalidations[0].element_id, 1);
        assert_eq!(
            delta.dependency_invalidations[0].directly_changed_reference_targets,
            vec![2]
        );
    }
    #[test]
    fn incomplete_projection_never_claims_parameters_were_removed() {
        let mut owner = state(1, "owner");
        owner.projected_parameters.insert(
            7,
            ParameterState {
                candidates: vec![Candidate {
                    storage_type: "String".into(),
                    raw_value: json!("retained"),
                }],
                ..ParameterState::default()
            },
        );
        let before = snapshot(vec![owner]);
        let mut after = before.clone();
        let owner = after.elements.get_mut(&1).unwrap();
        owner.projected_parameters.clear();
        owner.parameter_projection_available = false;
        owner.bodies.get_mut(&102).unwrap().sha256 = "changed".into();
        owner.bodies.get_mut(&102).unwrap().graph_status = "unsupported".into();
        let delta = compare(&before, &after).unwrap();
        assert!(!delta.changed[0].projected_parameter_comparison_available);
        assert!(delta.changed[0].projected_parameter_changes.is_empty());
    }
    #[test]
    fn reused_local_id_is_removed_plus_added_and_namespace_reload_is_not_an_alias() {
        let mut family = state(2, "family");
        family.family_content_key_raw_hex = Some("old-local-namespace".into());
        let before = snapshot(vec![state(1, "old-uid"), family]);
        let mut after = before.clone();
        after.elements.get_mut(&1).unwrap().identity.unique_id = "new-uid".into();
        after
            .elements
            .get_mut(&2)
            .unwrap()
            .family_content_key_raw_hex = Some("new-local-namespace".into());
        let delta = compare(&before, &after).unwrap();
        assert_eq!(delta.replaced_local_ids, vec![1]);
        assert_eq!(delta.added[0].unique_id, "new-uid");
        assert_eq!(delta.removed[0].unique_id, "old-uid");
        assert!(delta.changed[0].family_content_namespace_changed);
    }
}
