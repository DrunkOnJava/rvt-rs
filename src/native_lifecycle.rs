//! Authored phase references and current-record provenance, not installed state.
use crate::{native_document::Record, native_equipment::Identity, native_metadata::identifier};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub owner: Identity,
    pub source_field: String,
    pub body_sha256: String,
    pub stream: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct Reference {
    pub raw_target_id: i64,
    pub target_identity: Option<Identity>,
    pub phase_index: Option<usize>,
    pub status: String,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct PhaseState {
    pub phase_id: i64,
    pub status: Option<&'static str>,
    pub diagnostic: Option<&'static str>,
    pub semantics: &'static str,
    pub phase_order_source: Source,
    pub created_phase_source: Source,
    pub demolished_phase_source: Source,
}
#[derive(Debug, Serialize)]
pub struct Element {
    pub phase_states: Vec<PhaseState>,
    pub identity: Identity,
    pub class_name: String,
    pub references: BTreeMap<String, Reference>,
    pub native_index_identity: Value,
    pub source: Source,
    pub lifecycle_semantics: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct Phase {
    pub identity: Identity,
    pub name: String,
    pub description: Option<String>,
    pub index: Option<usize>,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct PhaseOrder {
    pub phase_ids: Vec<i64>,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub element_id: u64,
    pub code: String,
    pub message: String,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_supported_lifecycle: bool,
    pub complete_lifecycle_parity: bool,
    pub elements: Vec<Element>,
    pub phases: Vec<Phase>,
    pub phase_order: Option<PhaseOrder>,
    pub diagnostics: Vec<Diagnostic>,
    pub unresolved_scope: Vec<&'static str>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    elements: BTreeMap<i64, Element>,
    phases: BTreeMap<i64, Phase>,
    phase_order: Option<PhaseOrder>,
    diagnostics: Vec<Diagnostic>,
}
impl InventoryBuilder {
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        if record.channel != 102 {
            return Ok(());
        }
        let id = i64::try_from(record.identity.element_id)?;
        ensure!(
            !self.elements.contains_key(&id),
            "duplicate current lifecycle record"
        );
        let mut element = Element {
            phase_states: Vec::new(),
            identity: identity(record),
            class_name: record.class_name.clone().unwrap_or("<unknown>".into()),
            references: BTreeMap::new(),
            native_index_identity: serde_json::to_value(&record.identity)?,
            source: source(record, "current_index_and_body"),
            lifecycle_semantics: "authored_phase_references_not_observed_physical_state",
        };
        if let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) {
            ensure!(
                root.class_name == element.class_name,
                "lifecycle owner class mismatch"
            );
            for (key, field) in [
                ("created_phase", "m_createdPhaseId"),
                ("demolished_phase", "m_demolishedPhaseId"),
                ("design_option", "m_designOptionId"),
            ] {
                if let Some(value) = root.fields.get(field) {
                    element.references.insert(
                        key.into(),
                        Reference {
                            raw_target_id: identifier(value)?,
                            target_identity: None,
                            phase_index: None,
                            status: "unresolved".into(),
                            source: source(record, field),
                        },
                    );
                }
            }
            if root.class_name == "ProjectPhase" {
                self.phases.insert(
                    id,
                    Phase {
                        identity: identity(record),
                        name: root.fields["m_name"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("phase name absent"))?
                            .into(),
                        description: root.fields["m_description"].as_str().map(str::to_owned),
                        index: None,
                        source: source(record, "ProjectPhase.m_name,m_description"),
                    },
                );
            }
            if root.class_name == "AllProjectPhases" {
                ensure!(
                    self.phase_order.is_none(),
                    "multiple current phase catalogs"
                );
                let ids = root.fields["m_phaseIds"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("phase order absent"))?
                    .iter()
                    .map(identifier)
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    ids.iter().collect::<std::collections::BTreeSet<_>>().len() == ids.len(),
                    "duplicate phase in ordered catalog"
                );
                self.phase_order = Some(PhaseOrder {
                    phase_ids: ids,
                    source: source(record, "AllProjectPhases.m_phaseIds"),
                });
            }
        } else {
            self.diagnostics.push(Diagnostic {
                element_id: record.identity.element_id,
                code: "unsupported_lifecycle_owner_graph".into(),
                message: record
                    .diagnostic
                    .clone()
                    .unwrap_or("native graph absent; identity provenance retained".into()),
            });
        }
        self.elements.insert(id, element);
        Ok(())
    }
    pub fn finish(mut self) -> Result<Inventory> {
        if let Some(order) = &self.phase_order {
            for (index, id) in order.phase_ids.iter().enumerate() {
                if let Some(phase) = self.phases.get_mut(id) {
                    phase.index = Some(index);
                } else {
                    self.diagnostics.push(Diagnostic {
                        element_id: order.source.owner.element_id,
                        code: "missing_phase_record".into(),
                        message: format!("ordered phase {id} lacks decoded ProjectPhase"),
                    });
                }
            }
        }
        for phase in self.phases.values() {
            if phase.index.is_none() {
                self.diagnostics.push(Diagnostic {
                    element_id: phase.identity.element_id,
                    code: "unresolved_phase_order".into(),
                    message: "ProjectPhase missing from decoded ordered catalog".into(),
                });
            }
        }
        let identities: BTreeMap<_, _> = self
            .elements
            .iter()
            .map(|(id, e)| (*id, e.identity.clone()))
            .collect();
        for element in self.elements.values_mut() {
            for (key, reference) in &mut element.references {
                if reference.raw_target_id < 0 {
                    reference.status = if key == "design_option" {
                        "negative_serialized_reference_API_meaning_unresolved"
                    } else {
                        "negative_serialized_reference"
                    }
                    .into();
                    continue;
                }
                if key == "design_option" {
                    reference.target_identity = identities.get(&reference.raw_target_id).cloned();
                    reference.status = if reference.target_identity.is_some() {
                        "resolved_raw_option_reference_semantics_unresolved"
                    } else {
                        "unresolved_target"
                    }
                    .into();
                } else if let Some(phase) = self.phases.get(&reference.raw_target_id) {
                    reference.target_identity = Some(phase.identity.clone());
                    reference.phase_index = phase.index;
                    reference.status = "resolved_project_phase".into();
                } else {
                    reference.status = "unresolved_target".into();
                }
                if reference.status == "unresolved_target" {
                    self.diagnostics.push(Diagnostic {
                        element_id: element.identity.element_id,
                        code: "unresolved_lifecycle_reference".into(),
                        message: format!(
                            "{key} references absent or wrong-class element {}",
                            reference.raw_target_id
                        ),
                    });
                }
            }
        }
        if let Some(order) = &self.phase_order {
            for element in self.elements.values_mut() {
                if element.class_name != "FamilyInstance" {
                    continue;
                }
                let (Some(created), Some(demolished)) = (
                    element.references.get("created_phase"),
                    element.references.get("demolished_phase"),
                ) else {
                    continue;
                };
                let Some(created_index) = created.phase_index else {
                    continue;
                };
                for (query_index, &phase_id) in order.phase_ids.iter().enumerate() {
                    let status = relative_state(
                        created_index,
                        demolished.raw_target_id,
                        demolished.phase_index,
                        query_index,
                    );
                    element.phase_states.push(PhaseState {
                        phase_id,
                        status,
                        diagnostic: if status.is_none() {
                            Some("unqualified_phase_relationship")
                        } else {
                            None
                        },
                        semantics: "derived_authored_phase_status_not_physical_installation",
                        phase_order_source: order.source.clone(),
                        created_phase_source: created.source.clone(),
                        demolished_phase_source: demolished.source.clone(),
                    });
                }
            }
        }
        let mut phases: Vec<_> = self.phases.into_values().collect();
        phases.sort_by_key(|p| (p.index, p.identity.element_id));
        Ok(Inventory {
            format: "rvt-native-lifecycle/v1",
            complete_supported_lifecycle: self.diagnostics.is_empty(),
            complete_lifecycle_parity: false,
            elements: self.elements.into_values().collect(),
            phases,
            phase_order: self.phase_order,
            diagnostics: self.diagnostics,
            unresolved_scope: vec![
                "non_family_instance_and_temporary_past_phase_status",
                "design_option_visibility_and_primary_selection",
                "workset_ownership",
                "physical_installation_status",
                "episode_to_wall_clock_time",
            ],
        })
    }
}
fn relative_state(
    created: usize,
    raw_demolished: i64,
    demolished: Option<usize>,
    query: usize,
) -> Option<&'static str> {
    if raw_demolished != -1 && demolished.is_none() {
        return None;
    }
    if demolished.is_some_and(|index| index <= created || query > index) {
        return None;
    }
    if query < created {
        Some("Future")
    } else if query == created {
        Some("New")
    } else if demolished == Some(query) {
        Some("Demolished")
    } else {
        Some("Existing")
    }
}

fn identity(r: &Record) -> Identity {
    Identity {
        element_id: r.identity.element_id,
        unique_id: r.identity.unique_id.clone(),
    }
}
fn source(r: &Record, field: &str) -> Source {
    Source {
        owner: identity(r),
        source_field: field.into(),
        body_sha256: r.source.body_sha256.clone(),
        stream: r.source.stream.clone(),
        group_record_offset: r.source.group_record_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn provenance(id: u64) -> Source {
        Source {
            owner: Identity {
                element_id: id,
                unique_id: format!("uid-{id}"),
            },
            source_field: "synthetic".into(),
            body_sha256: "body".into(),
            stream: "Partitions/0".into(),
            group_record_offset: 0,
        }
    }
    fn phase(id: u64) -> Phase {
        Phase {
            identity: provenance(id).owner,
            name: format!("Phase {id}"),
            description: None,
            index: None,
            source: provenance(id),
        }
    }
    #[test]
    fn phase_order_is_serialized_order_not_numeric_id_or_name() {
        let mut builder = InventoryBuilder {
            phases: [(9, phase(9)), (2, phase(2))].into_iter().collect(),
            phase_order: Some(PhaseOrder {
                phase_ids: vec![9, 2],
                source: provenance(0),
            }),
            ..InventoryBuilder::default()
        };
        let references = [
            (
                "created_phase".into(),
                Reference {
                    raw_target_id: 9,
                    target_identity: None,
                    phase_index: None,
                    status: "unresolved".into(),
                    source: provenance(30),
                },
            ),
            (
                "design_option".into(),
                Reference {
                    raw_target_id: -4,
                    target_identity: None,
                    phase_index: None,
                    status: "unresolved".into(),
                    source: provenance(30),
                },
            ),
        ]
        .into_iter()
        .collect();
        builder.elements.insert(
            30,
            Element {
                phase_states: Vec::new(),
                identity: provenance(30).owner,
                class_name: "SWall".into(),
                references,
                native_index_identity: json!({"creation_episode":3,"stored_revision":8}),
                source: provenance(30),
                lifecycle_semantics: "authored",
            },
        );
        let inventory = builder.finish().unwrap();
        assert_eq!(
            inventory
                .phases
                .iter()
                .map(|p| p.identity.element_id)
                .collect::<Vec<_>>(),
            vec![9, 2]
        );
        assert_eq!(
            inventory.elements[0].references["created_phase"].phase_index,
            Some(0)
        );
        assert_eq!(
            inventory.elements[0].references["design_option"].raw_target_id,
            -4
        );
        assert_eq!(
            inventory.elements[0].native_index_identity["stored_revision"],
            8
        );
    }
    #[test]
    fn missing_phase_order_or_target_is_incomplete_not_guessed() {
        let mut builder = InventoryBuilder::default();
        builder.phases.insert(2, phase(2));
        let inventory = builder.finish().unwrap();
        assert!(!inventory.complete_supported_lifecycle);
        assert_eq!(inventory.phases[0].index, None);
    }
    #[test]
    fn measured_relative_states_do_not_guess_temporary_or_past() {
        assert_eq!(relative_state(1, -1, None, 0), Some("Future"));
        assert_eq!(relative_state(1, -1, None, 1), Some("New"));
        assert_eq!(relative_state(0, -1, None, 1), Some("Existing"));
        assert_eq!(relative_state(0, 4, Some(1), 1), Some("Demolished"));
        assert_eq!(relative_state(0, 4, Some(0), 0), None);
        assert_eq!(relative_state(0, 4, Some(1), 2), None);
        assert_eq!(relative_state(0, -4, None, 1), None);
    }
}
