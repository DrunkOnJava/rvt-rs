//! Equipment inventory from current native records. Type attributes are claims
//! about the type, never relabeled as instance-stored values or asset identity.
use crate::native_document::Record;
use crate::native_metadata::identifier;
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identity {
    pub element_id: u64,
    pub unique_id: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct AttributeSource {
    pub record_owner: Identity,
    pub source_object: Option<usize>,
    pub source_field: String,
    pub source_kind: String,
    pub referenced_source_element_id: Option<i64>,
    pub record_body_sha256: String,
    pub stream: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct Definition {
    pub name: String,
    pub shared_guid: Option<String>,
    pub spec_type_id: Option<String>,
    pub display_unit_type_id: Option<String>,
    pub group_type_id: Option<String>,
    pub definition_source: String,
    pub caption_locale: Option<String>,
    pub is_shared: Option<bool>,
    pub unit_resolution: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Attribute {
    pub parameter_id: i64,
    pub storage_type: String,
    pub value: Value,
    pub has_value: Option<bool>,
    pub is_read_only: Option<bool>,
    pub value_semantics: &'static str,
    pub value_unit_basis: &'static str,
    pub definition: Option<Definition>,
    pub sources: Vec<AttributeSource>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Claim {
    pub owner_role: &'static str,
    pub attribute: Attribute,
}
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub element_id: u64,
    pub code: String,
    pub message: String,
}
#[derive(Debug, Serialize)]
pub struct Equipment {
    pub placement: Placement,
    pub identity: Identity,
    pub type_identity: Option<Identity>,
    pub family_identity: Option<Identity>,
    pub category_id: i64,
    pub instance_attributes: BTreeMap<i64, Attribute>,
    pub type_attributes: BTreeMap<i64, Attribute>,
    pub identity_claims: BTreeMap<String, Vec<Claim>>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Placement {
    pub status: &'static str,
    pub raw_unplaced_owner_id: Option<i64>,
    pub unplaced_owner_identity: Option<Identity>,
    pub defining_group_instance_id: Option<i64>,
    pub source_body_sha256: String,
    pub source_field: &'static str,
}
#[derive(Debug, Default, Serialize)]
pub struct Population {
    pub family_instances_seen: usize,
    pub equipment_instances: usize,
    pub unplaced_equipment_prototypes: usize,
    pub excluded_by_category: BTreeMap<i64, usize>,
    pub unresolved_instance_categories: usize,
    pub ignored_non_family_records: usize,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_selected_equipment: bool,
    pub complete_attribute_parity: bool,
    pub selected_category_ids: Vec<i64>,
    pub equipment: Vec<Equipment>,
    pub unplaced_equipment: Vec<Equipment>,
    pub diagnostics: Vec<Diagnostic>,
    pub population: Population,
    pub identity_semantics: &'static str,
}
#[derive(Debug)]
struct Owner {
    unplaced_owner_id: Option<i64>,
    body_sha256: String,
    identity: Identity,
    class: String,
    type_id: Option<u64>,
    family_id: Option<u64>,
    category_id: Option<i64>,
    attributes: BTreeMap<i64, Attribute>,
    diagnostics: Vec<Diagnostic>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    owners: BTreeMap<u64, Owner>,
    ignored_non_family_records: usize,
    group_types: BTreeMap<u64, (Identity, Option<i64>)>,
}
impl InventoryBuilder {
    /// Feed current native records from one file/content namespace. Input order
    /// does not matter. Duplicate records are errors, including equal duplicates.
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        if record.channel != 102 {
            return Ok(());
        }
        let class = record.class_name.as_deref().unwrap_or("");
        if class == "ElementGroupType" {
            if let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) {
                ensure!(root.class_name == class, "group type graph mismatch");
                let defining = root
                    .fields
                    .get("m_definingGroupInstId")
                    .map(identifier)
                    .transpose()?;
                ensure!(
                    self.group_types
                        .insert(
                            record.identity.element_id,
                            (
                                Identity {
                                    element_id: record.identity.element_id,
                                    unique_id: record.identity.unique_id.clone()
                                },
                                defining
                            )
                        )
                        .is_none(),
                    "duplicate group type context"
                );
            }
            return Ok(());
        }
        if !matches!(class, "Family" | "FamilySymbol" | "FamilyInstance") {
            self.ignored_non_family_records += 1;
            return Ok(());
        }
        let identity = Identity {
            element_id: record.identity.element_id,
            unique_id: record.identity.unique_id.clone(),
        };
        ensure!(
            !self.owners.contains_key(&identity.element_id),
            "duplicate current equipment-context owner"
        );
        let mut owner = Owner {
            unplaced_owner_id: None,
            body_sha256: record.source.body_sha256.clone(),
            identity,
            class: class.into(),
            type_id: None,
            family_id: None,
            category_id: None,
            attributes: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        if let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        {
            ensure!(
                root.class_name == class,
                "equipment record and graph class mismatch"
            );
            let reference = |field: &str| -> Result<Option<u64>> {
                root.fields
                    .get(field)
                    .map(identifier)
                    .transpose()
                    .map(|value| value.and_then(|id| u64::try_from(id).ok()))
            };
            owner.unplaced_owner_id = root
                .fields
                .get("m_unplacedOwnerId")
                .map(identifier)
                .transpose()?;
            owner.type_id = reference("m_masterSymbolId")?;
            owner.family_id = reference("m_familyId")?;
            owner.category_id = root
                .fields
                .get("m_categoryId")
                .map(identifier)
                .transpose()?;
        } else {
            owner.diagnostics.push(problem(
                owner.identity.element_id,
                "unsupported_owner_graph",
                record
                    .diagnostic
                    .as_deref()
                    .unwrap_or("native owner graph unavailable"),
            ));
        }
        if let Some(metadata) = &record.saved_metadata {
            let mut definitions = BTreeMap::new();
            for definition in &metadata.parameter_definitions {
                definitions.insert(
                    definition.parameter_id,
                    Definition {
                        name: definition.caption.clone(),
                        shared_guid: definition.shared_guid.clone(),
                        spec_type_id: definition.spec_type_id.clone(),
                        display_unit_type_id: definition.unit_type_id.clone(),
                        group_type_id: definition.group_type_id.clone(),
                        definition_source: "native_parameter_definition".into(),
                        caption_locale: None,
                        is_shared: definition.is_shared,
                        unit_resolution: definition.unit_resolution.clone(),
                    },
                );
            }
            for definition in &metadata.builtin_parameter_definitions {
                definitions.insert(
                    definition.parameter_id,
                    Definition {
                        name: definition.caption.clone(),
                        shared_guid: None,
                        spec_type_id: Some(definition.spec_type_id.clone()),
                        display_unit_type_id: definition.unit_type_id.clone(),
                        group_type_id: Some(definition.group_type_id.clone()),
                        definition_source: definition.definition_evidence.clone(),
                        caption_locale: Some(definition.caption_locale.clone()),
                        is_shared: Some(definition.is_shared),
                        unit_resolution: definition.unit_resolution.clone(),
                    },
                );
            }
            let mut add = |parameter_id,
                           storage_type: &str,
                           value: &Value,
                           source_object,
                           source_field: &str,
                           source_kind: &str,
                           referenced_source_element_id|
             -> Result<()> {
                let source = AttributeSource {
                    record_owner: owner.identity.clone(),
                    source_object: Some(source_object),
                    source_field: source_field.into(),
                    source_kind: source_kind.into(),
                    referenced_source_element_id,
                    record_body_sha256: record.source.body_sha256.clone(),
                    stream: record.source.stream.clone(),
                    group_record_offset: record.source.group_record_offset,
                };
                if let Some(existing) = owner.attributes.get_mut(&parameter_id) {
                    ensure!(
                        existing.storage_type == storage_type && existing.value == *value,
                        "conflicting saved values for equipment owner parameter {parameter_id}"
                    );
                    existing.sources.push(source);
                } else {
                    owner.attributes.insert(
                        parameter_id,
                        Attribute {
                            parameter_id,
                            storage_type: storage_type.into(),
                            value: value.clone(),
                            has_value: None,
                            is_read_only: None,
                            value_semantics: "serialized_not_evaluated",
                            value_unit_basis: if storage_type == "Double" {
                                "revit_internal_units_for_spec"
                            } else {
                                "not_a_measured_double"
                            },
                            definition: definitions.get(&parameter_id).cloned(),
                            sources: vec![source],
                        },
                    );
                }
                Ok(())
            };
            for set in &metadata.parameter_sets {
                for parameter in &set.parameters {
                    add(
                        parameter.serialized_parameter_id,
                        &parameter.storage_type,
                        &parameter.raw_value,
                        set.source_object,
                        &set.root_pointer_field,
                        "owner_typed_parameter_set",
                        None,
                    )?;
                }
            }
            for parameter in &metadata.resolved_family_parameters {
                add(
                    parameter.parameter_id,
                    &parameter.storage_type,
                    &parameter.raw_value,
                    parameter.source_object,
                    "FamilyParams.m_params",
                    "definition_selected_owner_family_slot",
                    None,
                )?;
            }
            for parameter in &metadata.raw_field_parameters {
                add(
                    parameter.parameter_id,
                    &parameter.storage_type,
                    &parameter.raw_value,
                    parameter.source_object,
                    &parameter.source_field,
                    parameter.projection_rule,
                    parameter.source_element_id,
                )?;
            }
            for state in &metadata.declared_custom_parameters {
                if let Some(attribute) = owner.attributes.get_mut(&state.parameter_id) {
                    attribute.has_value = Some(state.has_value);
                    attribute.is_read_only = state.is_read_only;
                } else {
                    ensure!(
                        !state.has_value,
                        "declared valued equipment parameter has no recovered value"
                    );
                    owner.attributes.insert(
                        state.parameter_id,
                        Attribute {
                            parameter_id: state.parameter_id,
                            storage_type: state.storage_type.clone(),
                            value: Value::Null,
                            has_value: Some(false),
                            is_read_only: state.is_read_only,
                            value_semantics: "declared_unset",
                            value_unit_basis: "no_value",
                            definition: definitions.get(&state.parameter_id).cloned(),
                            sources: vec![AttributeSource {
                                record_owner: owner.identity.clone(),
                                source_object: state.source_object,
                                source_field: "matched_category_parameter_binding".into(),
                                source_kind: state.has_value_evidence.into(),
                                referenced_source_element_id: state.binding_id,
                                record_body_sha256: record.source.body_sha256.clone(),
                                stream: record.source.stream.clone(),
                                group_record_offset: record.source.group_record_offset,
                            }],
                        },
                    );
                }
            }
            if let Some(category) = owner
                .attributes
                .get(&-1140362)
                .and_then(|attribute| attribute.value.as_i64())
            {
                ensure!(
                    owner.category_id.is_none_or(|stored| stored == category),
                    "equipment category sources disagree"
                );
                owner.category_id = Some(category);
            }
        } else {
            owner.diagnostics.push(problem(
                owner.identity.element_id,
                "unsupported_owner_metadata",
                record
                    .metadata_diagnostic
                    .as_deref()
                    .unwrap_or("native owner metadata unavailable"),
            ));
        }
        self.owners.insert(owner.identity.element_id, owner);
        Ok(())
    }
    pub fn finish(self) -> Result<Inventory> {
        let mut inventory = Inventory {
            format: "rvt-native-equipment/v1",
            complete_selected_equipment: true,
            complete_attribute_parity: false,
            selected_category_ids: vec![-2001140, -2001040],
            equipment: Vec::new(),
            unplaced_equipment: Vec::new(),
            diagnostics: Vec::new(),
            population: Population {
                ignored_non_family_records: self.ignored_non_family_records,
                ..Population::default()
            },
            identity_semantics: "native_element_identity_not_physical_asset_reconciliation",
        };
        for instance in self
            .owners
            .values()
            .filter(|owner| owner.class == "FamilyInstance")
        {
            inventory.population.family_instances_seen += 1;
            let symbol = instance
                .type_id
                .and_then(|id| self.owners.get(&id))
                .filter(|owner| owner.class == "FamilySymbol");
            let family = symbol
                .and_then(|symbol| symbol.family_id)
                .and_then(|id| self.owners.get(&id))
                .filter(|owner| owner.class == "Family");
            let category = family
                .and_then(|family| family.category_id)
                .or(instance.category_id);
            if let (Some(a), Some(b)) = (
                family.and_then(|family| family.category_id),
                instance.category_id,
            ) {
                ensure!(a == b, "equipment family and instance category disagree");
            }
            let Some(category_id) = category else {
                inventory.population.unresolved_instance_categories += 1;
                inventory.diagnostics.push(problem(
                    instance.identity.element_id,
                    "unresolved_instance_category",
                    "cannot decide whether this instance belongs to selected equipment categories",
                ));
                inventory.diagnostics.extend(instance.diagnostics.clone());
                continue;
            };
            if !inventory.selected_category_ids.contains(&category_id) {
                *inventory
                    .population
                    .excluded_by_category
                    .entry(category_id)
                    .or_default() += 1;
                continue;
            }
            let mut diagnostics = instance.diagnostics.clone();
            if let Some(symbol) = symbol {
                diagnostics.extend(symbol.diagnostics.clone());
            } else {
                diagnostics.push(problem(
                    instance.identity.element_id,
                    "missing_type_record",
                    "native type reference is missing or does not identify a FamilySymbol",
                ));
            }
            if let Some(family) = family {
                diagnostics.extend(family.diagnostics.clone());
            } else {
                diagnostics.push(problem(
                    instance.identity.element_id,
                    "missing_family_record",
                    "native type-to-family reference is missing or does not identify a Family",
                ));
            }
            let type_attributes = symbol
                .map(|owner| owner.attributes.clone())
                .unwrap_or_default();
            let mut identity_claims: BTreeMap<String, Vec<Claim>> = BTreeMap::new();
            for (name, id) in [
                ("manufacturer", -1010108),
                ("model", -1010109),
                ("mark", -1001203),
                ("type_mark", -1001405),
                ("description", -1010103),
                ("url", -1010104),
                ("assembly_code", -1002500),
                ("classification_number", -1002502),
                ("classification_title", -1002503),
            ] {
                for (role, attributes) in [
                    ("instance", &instance.attributes),
                    ("type", &type_attributes),
                ] {
                    if let Some(attribute) = attributes.get(&id) {
                        identity_claims.entry(name.into()).or_default().push(Claim {
                            owner_role: role,
                            attribute: attribute.clone(),
                        });
                    }
                }
            }
            inventory.diagnostics.extend(diagnostics.clone());
            let group_type = instance
                .unplaced_owner_id
                .and_then(|id| u64::try_from(id).ok())
                .and_then(|id| self.group_types.get(&id));
            let placement = Placement {
                status: if group_type.is_some() {
                    "unplaced_group_definition_member"
                } else if instance.unplaced_owner_id.is_some_and(|id| id >= 0) {
                    "unresolved_unplaced_owner"
                } else {
                    "no_positive_unplaced_owner"
                },
                raw_unplaced_owner_id: instance.unplaced_owner_id,
                unplaced_owner_identity: group_type.map(|(identity, _)| identity.clone()),
                defining_group_instance_id: group_type.and_then(|(_, id)| *id),
                source_body_sha256: instance.body_sha256.clone(),
                source_field: "m_unplacedOwnerId -> ElementGroupType.m_definingGroupInstId",
            };
            if placement.status == "unresolved_unplaced_owner" {
                let diagnostic = problem(
                    instance.identity.element_id,
                    "unresolved_unplaced_owner",
                    "positive unplaced owner does not resolve to a decoded group type; placement is unresolved",
                );
                inventory.diagnostics.push(diagnostic.clone());
                diagnostics.push(diagnostic);
            }
            let equipment = Equipment {
                placement,
                identity: instance.identity.clone(),
                type_identity: symbol.map(|owner| owner.identity.clone()),
                family_identity: family.map(|owner| owner.identity.clone()),
                category_id,
                instance_attributes: instance.attributes.clone(),
                type_attributes,
                identity_claims,
                diagnostics,
            };
            if group_type.is_some() {
                inventory.unplaced_equipment.push(equipment);
            } else {
                inventory.equipment.push(equipment);
            }
        }
        inventory.population.unplaced_equipment_prototypes = inventory.unplaced_equipment.len();
        inventory.population.equipment_instances = inventory.equipment.len();
        inventory.complete_selected_equipment = inventory.diagnostics.is_empty();
        Ok(inventory)
    }
}
fn problem(element_id: u64, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        element_id,
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner(id: u64, class: &str) -> Owner {
        Owner {
            unplaced_owner_id: None,
            body_sha256: "synthetic".into(),
            identity: Identity {
                element_id: id,
                unique_id: format!("test-native-{id}"),
            },
            class: class.into(),
            type_id: None,
            family_id: None,
            category_id: None,
            attributes: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }
    fn value(owner: &Owner, id: i64, text: &str) -> Attribute {
        Attribute {
            parameter_id: id,
            storage_type: "String".into(),
            value: text.into(),
            has_value: Some(true),
            is_read_only: None,
            value_semantics: "serialized_not_evaluated",
            value_unit_basis: "not_a_measured_double",
            definition: None,
            sources: vec![AttributeSource {
                record_owner: owner.identity.clone(),
                source_object: Some(1),
                source_field: "FamilyParams.m_params".into(),
                source_kind: "definition_selected_owner_family_slot".into(),
                referenced_source_element_id: None,
                record_body_sha256: "synthetic".into(),
                stream: "Partitions/0".into(),
                group_record_offset: 0,
            }],
        }
    }
    #[test]
    fn duplicated_marks_do_not_merge_assets_and_type_switch_changes_type_claims() {
        let mut family = owner(1, "Family");
        family.category_id = Some(-2001140);
        let mut type_a = owner(2, "FamilySymbol");
        type_a.family_id = Some(1);
        type_a
            .attributes
            .insert(-1010109, value(&type_a, -1010109, "Model A"));
        let mut type_b = owner(3, "FamilySymbol");
        type_b.family_id = Some(1);
        type_b
            .attributes
            .insert(-1010109, value(&type_b, -1010109, "Model B"));
        let mut first = owner(4, "FamilyInstance");
        first.type_id = Some(2);
        first
            .attributes
            .insert(-1001203, value(&first, -1001203, "DUPLICATED_MARK"));
        let mut second = owner(5, "FamilyInstance");
        second.type_id = Some(3);
        second
            .attributes
            .insert(-1001203, value(&second, -1001203, "DUPLICATED_MARK"));
        let builder = InventoryBuilder {
            owners: [family, type_a, type_b, first, second]
                .into_iter()
                .map(|owner| (owner.identity.element_id, owner))
                .collect(),
            ignored_non_family_records: 10,
            group_types: BTreeMap::new(),
        };
        let inventory = builder.finish().unwrap();
        assert!(inventory.complete_selected_equipment);
        assert!(!inventory.complete_attribute_parity);
        assert_eq!(inventory.equipment.len(), 2);
        assert_ne!(
            inventory.equipment[0].identity,
            inventory.equipment[1].identity
        );
        assert_eq!(
            inventory.equipment[0].identity_claims["model"][0]
                .attribute
                .value,
            "Model A"
        );
        assert_eq!(
            inventory.equipment[1].identity_claims["model"][0]
                .attribute
                .value,
            "Model B"
        );
        assert_eq!(
            inventory.equipment[1].identity_claims["model"][0].owner_role,
            "type"
        );
        assert_eq!(
            inventory.equipment[1].identity_claims["model"][0]
                .attribute
                .sources[0]
                .record_owner
                .element_id,
            3
        );
        assert!(
            !inventory.equipment[1]
                .instance_attributes
                .contains_key(&-1010109)
        );
    }
    #[test]
    fn missing_context_is_not_an_empty_success_or_non_equipment_guess() {
        let unknown = owner(1, "FamilyInstance");
        let mut known = owner(2, "FamilyInstance");
        known.category_id = Some(-2001140);
        let mut other = owner(3, "FamilyInstance");
        other.category_id = Some(-2000151);
        let builder = InventoryBuilder {
            owners: [unknown, known, other]
                .into_iter()
                .map(|owner| (owner.identity.element_id, owner))
                .collect(),
            ignored_non_family_records: 0,
            group_types: BTreeMap::new(),
        };
        let inventory = builder.finish().unwrap();
        assert!(!inventory.complete_selected_equipment);
        assert_eq!(inventory.population.unresolved_instance_categories, 1);
        assert_eq!(inventory.population.excluded_by_category[&-2000151], 1);
        assert_eq!(inventory.equipment.len(), 1);
        assert!(inventory.equipment[0].type_identity.is_none());
        assert!(
            inventory.equipment[0]
                .diagnostics
                .iter()
                .any(|d| d.code == "missing_type_record")
        );
    }
    #[test]
    fn group_definition_members_are_preserved_separately_from_placed_population() {
        let mut family = owner(1, "Family");
        family.category_id = Some(-2001140);
        let mut symbol = owner(2, "FamilySymbol");
        symbol.family_id = Some(1);
        let mut placed = owner(3, "FamilyInstance");
        placed.type_id = Some(2);
        placed.unplaced_owner_id = Some(-1);
        let mut prototype = owner(4, "FamilyInstance");
        prototype.type_id = Some(2);
        prototype.unplaced_owner_id = Some(8);
        let mut unresolved = owner(5, "FamilyInstance");
        unresolved.type_id = Some(2);
        unresolved.unplaced_owner_id = Some(9);
        let mut builder = InventoryBuilder {
            owners: [family, symbol, placed, prototype, unresolved]
                .into_iter()
                .map(|o| (o.identity.element_id, o))
                .collect(),
            ..InventoryBuilder::default()
        };
        builder.group_types.insert(
            8,
            (
                Identity {
                    element_id: 8,
                    unique_id: "group-type".into(),
                },
                Some(10),
            ),
        );
        let inventory = builder.finish().unwrap();
        assert_eq!(inventory.equipment.len(), 2);
        assert_eq!(inventory.unplaced_equipment.len(), 1);
        assert_eq!(inventory.unplaced_equipment[0].identity.element_id, 4);
        assert_eq!(
            inventory.unplaced_equipment[0]
                .placement
                .defining_group_instance_id,
            Some(10)
        );
        assert_eq!(
            inventory.equipment[1].placement.status,
            "unresolved_unplaced_owner"
        );
        assert!(!inventory.complete_selected_equipment);
    }
}
