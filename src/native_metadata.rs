//! Lossless projections of selected saved metadata from a complete native graph.
//! This is not API-evaluated data. Family value slots need parameter definitions
//! before a storage type can be selected; no nonzero-slot guessing is performed.
use crate::native_parameter_definitions::{
    BuiltinDefinition, Definition, Registry, ResolvedFamilyParameter,
};
use crate::native_parameters::{ObjectGraph, Parameter};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct SavedMetadata {
    pub value_semantics: &'static str,
    pub complete_element_metadata: bool,
    pub root_references: BTreeMap<String, i64>,
    pub parameter_sets: Vec<SavedParameterSet>,
    pub family_parameter_slots: Vec<FamilyParameterSlots>,
    pub parameter_definitions: Vec<Definition>,
    pub builtin_parameter_definitions: Vec<BuiltinDefinition>,
    pub declared_custom_parameters: Vec<DeclaredCustomParameter>,
    pub global_parameter_associations: Vec<GlobalParameterAssociation>,
    pub raw_field_parameters: Vec<RawFieldParameter>,
    pub resolved_family_parameters: Vec<ResolvedFamilyParameter>,
}
#[derive(Debug, Serialize)]
pub struct DeclaredCustomParameter {
    pub parameter_id: i64,
    pub storage_type: String,
    pub has_value: bool,
    pub has_value_evidence: &'static str,
    pub source_object: Option<usize>,
    pub binding_id: Option<i64>,
    pub owner_category_id: Option<i64>,
    pub is_read_only: Option<bool>,
    pub read_only_evidence: Option<&'static str>,
    pub associated_global_parameter_id: Option<i64>,
}
#[derive(Debug, Serialize)]
pub struct GlobalParameterAssociation {
    pub source_object: usize,
    pub parameter_id: i64,
    pub global_parameter_id: i64,
    pub stored_is_symbol: bool,
    pub geometry_tag: i64,
}
/// A validated native-field to built-in parameter mapping. Values remain
/// serialized; sentinel and evaluated API differences are deliberately retained.
#[derive(Debug, Serialize)]
pub struct RawFieldParameter {
    pub parameter_id: i64,
    pub source_object: usize,
    pub source_field: String,
    pub source_element_id: Option<i64>,
    pub projection_rule: &'static str,
    pub storage_type: String,
    pub raw_value: Value,
    pub value_semantics: &'static str,
}
#[derive(Debug, Serialize)]
pub struct SavedParameterSet {
    pub source_object: usize,
    pub root_pointer_field: String,
    pub parameters: Vec<Parameter>,
}
#[derive(Debug, Serialize)]
pub struct FamilyParameterSlots {
    pub source_object: usize,
    pub parameter_id: i64,
    pub stored_instance_flag: bool,
    pub reporting: bool,
    pub double_slot: Value,
    pub integer_slot: Value,
    pub string_slot: Value,
    pub element_id_slot: i64,
    pub expression_pointer: Value,
}
pub(crate) fn identifier(value: &Value) -> Result<i64> {
    if let Some(n) = value.as_i64() {
        return Ok(n);
    }
    for key in ["m_id", "m_id64"] {
        if let Some(inner) = value.get(key) {
            return identifier(inner);
        }
    }
    anyhow::bail!("unsupported saved identifier shape")
}
/// Project owner parameter sets only through explicit graph edges. Family slots
/// retain their containing object index and every stored slot without implying
/// which slot is active or whether a type value is inherited by an instance.
pub fn project(graph: &ObjectGraph) -> Result<SavedMetadata> {
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty native graph"))?;
    let mut result = SavedMetadata {
        value_semantics: "serialized_not_evaluated",
        complete_element_metadata: false,
        root_references: BTreeMap::new(),
        parameter_sets: Vec::new(),
        family_parameter_slots: Vec::new(),
        parameter_definitions: Vec::new(),
        builtin_parameter_definitions: Vec::new(),
        declared_custom_parameters: Vec::new(),
        global_parameter_associations: Vec::new(),
        raw_field_parameters: Vec::new(),
        resolved_family_parameters: Vec::new(),
    };
    let mut direct_fields = Vec::new();
    if matches!(
        root.class_name.as_str(),
        "FamilyInstance" | "RbsPipeCurve" | "SWall"
    ) {
        direct_fields.extend([
            ("m_createdPhaseId", -1012100),
            ("m_demolishedPhaseId", -1012101),
        ]);
    }
    if root.class_name == "FamilyInstance" {
        direct_fields.extend([
            ("m_masterSymbolId", -1002052),
            ("m_masterSymbolId", -1002051),
            ("m_masterSymbolId", -1002050),
            ("m_masterSymbolId", -1002000),
            ("m_assocLevelId", -1002062),
            ("m_assocLevelId", -1001352),
            ("m_hostId", -1002108),
            ("m_designOptionId", -1013201),
        ]);
    }
    if root.class_name == "MaterialElem" {
        direct_fields.push(("m_designOptionId", -1013201));
    }
    for (field, parameter_id) in direct_fields {
        if let Some(value) = root.fields.get(field) {
            result.raw_field_parameters.push(RawFieldParameter {
                parameter_id,
                source_object: 0,
                source_field: field.into(),
                source_element_id: None,
                projection_rule: "direct_saved_field",
                storage_type: "ElementId".into(),
                raw_value: identifier(value)?.into(),
                value_semantics: "serialized_not_evaluated",
            });
        }
    }
    if root.class_name == "MaterialElem"
        && let Some(pointer) = root.fields.get("m_pMaterial")
        && pointer["pointer_token"].as_u64() != Some(0)
    {
        let offset = pointer["offset"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("material pointer offset absent"))?
            as usize;
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.source_object_index == 0 && edge.pointer_offset == offset)
            .ok_or_else(|| anyhow::anyhow!("material edge absent"))?;
        let material = graph
            .objects
            .get(edge.target_object_index)
            .ok_or_else(|| anyhow::anyhow!("material outside graph"))?;
        ensure!(
            material.class_name == "Material",
            "material target class mismatch"
        );
        append_fields(
            &mut result,
            graph,
            edge.target_object_index,
            "m_pMaterial.",
            &[
                ("m_name", -1140355, "String"),
                ("m_color", -1002550, "Integer"),
                ("m_shininess", -1002554, "Integer"),
            ],
        )?;
        for (field, parameter_id) in [("m_transparency", -1002551), ("m_smoothness", -1002553)] {
            if let Some(value) = material.fields.get(field) {
                let fraction = value.as_f64().ok_or_else(|| {
                    anyhow::anyhow!("material normalized surface property is not numeric")
                })?;
                ensure!(
                    fraction.is_finite() && (0.0..=1.0).contains(&fraction),
                    "material normalized surface property outside range"
                );
                result.raw_field_parameters.push(RawFieldParameter {
                    parameter_id,
                    source_object: edge.target_object_index,
                    source_field: format!("m_pMaterial.{field}"),
                    source_element_id: None,
                    projection_rule: "normalized_fraction_to_nearest_integer_percent",
                    storage_type: "Integer".into(),
                    raw_value: ((fraction * 100.0).round() as i64).into(),
                    value_semantics: "serialized_not_evaluated",
                });
            }
        }
    }
    if root.class_name == "RbsPipeCurve" {
        append_fields(
            &mut result,
            graph,
            0,
            "",
            &[
                ("m_dWidthOrDiameter", -1140225, "Double"),
                ("m_assocLevelId", -1114000, "ElementId"),
                ("m_idType", -1002000, "ElementId"),
                ("m_dOffsetStart", -1114002, "Double"),
                ("m_dOffsetEnd", -1114003, "Double"),
            ],
        )?;
        if let Some(pointer) = root.fields.get("m_pDesignPropManager")
            && pointer["pointer_token"].as_u64() != Some(0)
        {
            let offset = pointer["offset"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("pipe property pointer offset absent"))?
                as usize;
            let edge = graph
                .edges
                .iter()
                .find(|edge| edge.source_object_index == 0 && edge.pointer_offset == offset)
                .ok_or_else(|| anyhow::anyhow!("pipe property manager edge absent"))?;
            let object = graph
                .objects
                .get(edge.target_object_index)
                .ok_or_else(|| anyhow::anyhow!("pipe property manager outside graph"))?;
            ensure!(
                object.class_name == "PipeDomainDesignPropertyManager",
                "pipe property manager class mismatch"
            );
            append_fields(
                &mut result,
                graph,
                edge.target_object_index,
                "m_pDesignPropManager.",
                &[
                    ("m_dFlow", -1140213, "Double"),
                    ("m_dFriction", -1140206, "Double"),
                    ("m_dFrictionFactor", -1140208, "Double"),
                    ("m_dInnerDiameter", -1140212, "Double"),
                    ("m_dOuterDiameter", -1140238, "Double"),
                    ("m_dPressureDrop", -1140205, "Double"),
                    ("m_dReynoldsNumber", -1140211, "Double"),
                    ("m_dVelocity", -1140207, "Double"),
                    ("m_dVelocityPressure", -1140285, "Double"),
                    ("m_idSystemType", -1140334, "ElementId"),
                    ("m_kFlowState", -1140209, "Integer"),
                    ("m_sOverallSize", -1150434, "String"),
                    ("m_sCalculatedSize", -1114240, "String"),
                ],
            )?;
        }
    }
    for field in [
        "m_masterSymbolId",
        "m_familyId",
        "m_assocLevelId",
        "m_hostId",
        "m_createdPhaseId",
        "m_demolishedPhaseId",
        "m_designOptionId",
    ] {
        if let Some(value) = root.fields.get(field) {
            result
                .root_references
                .insert(field.into(), identifier(value)?);
        }
    }
    let direct = [
        ("m_pParamValueSetDouble", "ParamValueSetDouble", "Double"),
        ("m_pParamValueSetInt", "ParamValueSetInt", "Integer"),
        ("m_pParamValueSetAString", "ParamValueSetAString", "String"),
        (
            "m_pParamValueSetElementId",
            "ParamValueSetElementId",
            "ElementId",
        ),
    ];
    let mut contexts = vec![(0usize, "", direct)];
    if root.class_name == "PropertySetElement"
        && let Some(pointer) = root.fields.get("m_oParamSet")
        && pointer["pointer_token"].as_u64() != Some(0)
    {
        let offset = pointer["offset"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("asset parameter pointer offset absent"))?
            as usize;
        let edge = graph
            .edges
            .iter()
            .find(|e| e.source_object_index == 0 && e.pointer_offset == offset)
            .ok_or_else(|| anyhow::anyhow!("asset parameter set edge absent"))?;
        let target = graph
            .objects
            .get(edge.target_object_index)
            .ok_or_else(|| anyhow::anyhow!("asset parameter set outside graph"))?;
        ensure!(
            target.class_name == "ParamSet",
            "asset parameter container class mismatch"
        );
        contexts.push((
            edge.target_object_index,
            "m_oParamSet.",
            [
                ("m_pDoubleParams", "ParamValueSetDouble", "Double"),
                ("m_pIntParams", "ParamValueSetInt", "Integer"),
                ("m_pAStringParams", "ParamValueSetAString", "String"),
                ("m_pElementIdParams", "ParamValueSetElementId", "ElementId"),
            ],
        ));
    }
    for (owner_index, prefix, fields) in contexts {
        let parameter_owner = &graph.objects[owner_index];
        for (field, class, storage) in fields {
            let Some(pointer) = parameter_owner.fields.get(field) else {
                continue;
            };
            if pointer["pointer_token"].as_u64() == Some(0) {
                continue;
            }
            let offset = pointer["offset"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("parameter pointer offset absent"))?
                as usize;
            let edge = graph
                .edges
                .iter()
                .find(|e| e.source_object_index == owner_index && e.pointer_offset == offset)
                .ok_or_else(|| anyhow::anyhow!("owner parameter edge absent"))?;
            let object = graph
                .objects
                .get(edge.target_object_index)
                .ok_or_else(|| anyhow::anyhow!("parameter edge target outside graph"))?;
            ensure!(
                object.class_name == class,
                "owner parameter set class mismatch"
            );
            let pairs = object.fields["m_paramSet"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("parameter pairs absent"))?;
            let mut parameters = Vec::new();
            for pair in pairs {
                let raw = pair
                    .get("m_value")
                    .ok_or_else(|| anyhow::anyhow!("parameter value absent"))?;
                parameters.push(Parameter {
                    serialized_parameter_id: identifier(&pair["m_paramId"])?,
                    storage_type: storage.into(),
                    raw_value: if storage == "ElementId" {
                        identifier(raw)?.into()
                    } else {
                        raw.clone()
                    },
                });
            }
            result.parameter_sets.push(SavedParameterSet {
                source_object: edge.target_object_index,
                root_pointer_field: format!("{prefix}{field}"),
                parameters,
            });
        }
    }
    if matches!(
        root.class_name.as_str(),
        "FamilySymbol"
            | "BasicWallType"
            | "LevelAttributes"
            | "FloorAttributes"
            | "RbsPipeType"
            | "RbsPipingSystemType"
    ) && let Some(pointer) = root.fields.get("m_symbolInfo")
        && pointer["pointer_token"].as_u64() != Some(0)
    {
        let offset = pointer["offset"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("symbol information pointer offset absent"))?
            as usize;
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.source_object_index == 0 && edge.pointer_offset == offset)
            .ok_or_else(|| anyhow::anyhow!("symbol information edge absent"))?;
        let info = graph
            .objects
            .get(edge.target_object_index)
            .ok_or_else(|| anyhow::anyhow!("symbol information outside graph"))?;
        ensure!(
            info.class_name == "SymbolInfo",
            "symbol information class mismatch"
        );
        let stored: Vec<_> = result
            .parameter_sets
            .iter()
            .flat_map(|set| &set.parameters)
            .filter(|p| p.serialized_parameter_id == -1002001)
            .collect();
        if stored.is_empty() {
            append_fields(
                &mut result,
                graph,
                edge.target_object_index,
                "m_symbolInfo.",
                &[("m_name", -1002001, "String")],
            )?;
        } else {
            ensure!(
                stored.len() == 1
                    && stored[0].storage_type == "String"
                    && stored[0].raw_value == info.fields["m_name"],
                "typed name and symbol information disagree"
            );
        }
    }
    for (source_object, object) in graph.objects.iter().enumerate() {
        if object.class_name != "FamilyParams" {
            continue;
        }
        for value in object.fields["m_params"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("family parameter array absent"))?
        {
            let required = |name: &str| {
                value
                    .get(name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("family slot {name} absent"))
            };
            result.family_parameter_slots.push(FamilyParameterSlots {
                source_object,
                parameter_id: identifier(&value["m_paramId"])?,
                stored_instance_flag: value["m_instance"]
                    .as_bool()
                    .ok_or_else(|| anyhow::anyhow!("instance flag absent"))?,
                reporting: value["m_reporting"]
                    .as_bool()
                    .ok_or_else(|| anyhow::anyhow!("reporting flag absent"))?,
                double_slot: required("m_value")?,
                integer_slot: required("m_int")?,
                string_slot: required("m_str")?,
                element_id_slot: identifier(&value["m_elemId"])?,
                expression_pointer: required("m_oExpression")?,
            });
        }
    }
    Ok(result)
}

fn append_fields(
    result: &mut SavedMetadata,
    graph: &ObjectGraph,
    source_object: usize,
    prefix: &str,
    fields: &[(&str, i64, &str)],
) -> Result<()> {
    for &(field, parameter_id, storage_type) in fields {
        if let Some(value) = graph.objects[source_object].fields.get(field) {
            let raw_value = if storage_type == "ElementId" {
                identifier(value)?.into()
            } else {
                value.clone()
            };
            ensure!(
                match storage_type {
                    "Double" => raw_value.is_number(),
                    "Integer" | "ElementId" => raw_value.as_i64().is_some(),
                    "String" => raw_value.is_string(),
                    _ => false,
                },
                "native field storage shape mismatch"
            );
            result.raw_field_parameters.push(RawFieldParameter {
                parameter_id,
                source_object,
                source_field: format!("{prefix}{field}"),
                source_element_id: None,
                projection_rule: "direct_saved_field",
                storage_type: storage_type.into(),
                raw_value,
                value_semantics: "serialized_not_evaluated",
            });
        }
    }
    Ok(())
}

/// Enrich local saved values using definitions from the same document namespace.
/// Missing definitions remain unresolved, including negative built-in IDs.
pub fn project_with_definitions(graph: &ObjectGraph, registry: &Registry) -> Result<SavedMetadata> {
    let mut result = project(graph)?;
    let root = &graph.objects[0];
    result.global_parameter_associations = project_global_associations(graph)?;
    let family_id = match root.class_name.as_str() {
        "FamilySymbol" => root.fields.get("m_familyId").map(identifier).transpose()?,
        "FamilyInstance" => root
            .fields
            .get("m_masterSymbolId")
            .map(identifier)
            .transpose()?
            .and_then(|symbol| registry.symbol_families.get(&symbol).copied()),
        _ => None,
    };
    if let Some(family_id) = family_id
        && let Some(category) = registry.family_categories.get(&family_id)
    {
        for parameter_id in [-1140362, -1140363] {
            result.raw_field_parameters.push(RawFieldParameter {
                parameter_id,
                source_object: 0,
                source_field: "m_categoryId".into(),
                source_element_id: Some(family_id),
                projection_rule: "owner_symbol_family_category_reference_chain",
                storage_type: "ElementId".into(),
                raw_value: (*category).into(),
                value_semantics: "serialized_not_evaluated",
            });
        }
    }
    let ids: std::collections::BTreeSet<i64> = result
        .parameter_sets
        .iter()
        .flat_map(|set| set.parameters.iter().map(|p| p.serialized_parameter_id))
        .chain(
            result
                .raw_field_parameters
                .iter()
                .map(|parameter| parameter.parameter_id),
        )
        .chain(
            result
                .family_parameter_slots
                .iter()
                .map(|slot| slot.parameter_id),
        )
        .collect();
    result.builtin_parameter_definitions = ids
        .iter()
        .filter_map(|id| registry.builtin_definitions.get(id).cloned())
        .collect();
    result.parameter_definitions = ids
        .iter()
        .filter_map(|id| registry.definitions.get(id).cloned())
        .collect();
    let root = &graph.objects[0];
    let owner_field = match root.class_name.as_str() {
        "Family" => Some("m_familyParams"),
        "FamilySymbol" => Some("m_pParams"),
        "FamilyInstance" => Some("m_pInstParams"),
        _ => None,
    };
    let owner_target = owner_field
        .and_then(|field| root.fields.get(field))
        .and_then(|pointer| pointer["offset"].as_u64())
        .and_then(|offset| {
            graph.edges.iter().find(|edge| {
                edge.source_object_index == 0 && edge.pointer_offset == offset as usize
            })
        })
        .map(|edge| edge.target_object_index);
    result.resolved_family_parameters = result
        .family_parameter_slots
        .iter()
        .filter(|slot| Some(slot.source_object) == owner_target)
        .filter_map(|slot| registry.resolve_slot(slot))
        .collect();
    if matches!(root.class_name.as_str(), "FamilyInstance" | "FamilySymbol") {
        let mut states = BTreeMap::new();
        for set in &result.parameter_sets {
            for parameter in &set.parameters {
                if parameter.serialized_parameter_id > 0
                    && registry
                        .definitions
                        .contains_key(&parameter.serialized_parameter_id)
                {
                    states.insert(
                        parameter.serialized_parameter_id,
                        DeclaredCustomParameter {
                            parameter_id: parameter.serialized_parameter_id,
                            storage_type: parameter.storage_type.clone(),
                            has_value: true,
                            has_value_evidence: "validated_present_owner_typed_parameter",
                            source_object: Some(set.source_object),
                            binding_id: None,
                            owner_category_id: None,
                            is_read_only: None,
                            read_only_evidence: None,
                            associated_global_parameter_id: None,
                        },
                    );
                }
            }
        }
        for parameter in &result.resolved_family_parameters {
            if parameter.parameter_id > 0 {
                ensure!(
                    !states.contains_key(&parameter.parameter_id),
                    "custom parameter present in both owner typed set and family slots"
                );
                states.insert(
                    parameter.parameter_id,
                    DeclaredCustomParameter {
                        parameter_id: parameter.parameter_id,
                        storage_type: parameter.storage_type.clone(),
                        has_value: true,
                        has_value_evidence: "validated_present_family_parameter_slot",
                        source_object: Some(parameter.source_object),
                        binding_id: None,
                        owner_category_id: None,
                        is_read_only: None,
                        read_only_evidence: None,
                        associated_global_parameter_id: None,
                    },
                );
            }
        }
        for binding in registry.matching_bindings(graph)? {
            let Some(storage_type) = registry.definitions[&binding.parameter_id]
                .storage_type
                .clone()
            else {
                continue;
            };
            let state = states
                .entry(binding.parameter_id)
                .or_insert(DeclaredCustomParameter {
                    parameter_id: binding.parameter_id,
                    storage_type,
                    has_value: false,
                    has_value_evidence: "matched_category_binding_absent_owner_value",
                    source_object: None,
                    binding_id: None,
                    owner_category_id: None,
                    is_read_only: None,
                    read_only_evidence: None,
                    associated_global_parameter_id: None,
                });
            ensure!(
                state.binding_id.is_none(),
                "ambiguous matching custom parameter bindings"
            );
            state.binding_id = Some(binding.binding_id);
            state.owner_category_id = Some(binding.category_id);
        }
        for state in states.values_mut() {
            let definition = &registry.definitions[&state.parameter_id];
            if let Some(read_only) = definition.definition_fields["m_readOnly"].as_bool() {
                state.is_read_only = Some(read_only);
                state.read_only_evidence =
                    Some("saved_definition_flag_and_owner_global_associations");
            }
            for association in result
                .global_parameter_associations
                .iter()
                .filter(|association| association.parameter_id == state.parameter_id)
            {
                let recognized = association.geometry_tag == -1
                    && association.stored_is_symbol == (root.class_name == "FamilySymbol")
                    && registry
                        .definitions
                        .get(&association.global_parameter_id)
                        .is_some_and(|target| target.owner_class == "ParamElemGlobal");
                if recognized {
                    ensure!(
                        state.associated_global_parameter_id.is_none(),
                        "multiple global parameter drivers for owner parameter"
                    );
                    state.is_read_only = Some(true);
                    state.read_only_evidence = Some("verified_owner_global_parameter_association");
                    state.associated_global_parameter_id = Some(association.global_parameter_id);
                } else {
                    state.is_read_only = None;
                    state.read_only_evidence = Some("unresolved_project_parameter_driver");
                }
            }
        }
        result.declared_custom_parameters = states.into_values().collect();
        let ids: std::collections::BTreeSet<_> = result
            .parameter_definitions
            .iter()
            .map(|definition| definition.parameter_id)
            .chain(
                result
                    .declared_custom_parameters
                    .iter()
                    .map(|state| state.parameter_id),
            )
            .collect();
        result.parameter_definitions = ids
            .iter()
            .filter_map(|id| registry.definitions.get(id).cloned())
            .collect();
    }
    Ok(result)
}

fn project_global_associations(graph: &ObjectGraph) -> Result<Vec<GlobalParameterAssociation>> {
    let root = &graph.objects[0];
    let Some(pointer) = root.fields.get("m_cellList") else {
        return Ok(Vec::new());
    };
    if pointer["pointer_token"].as_u64() == Some(0) {
        return Ok(Vec::new());
    }
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("owner cell-list pointer offset absent"))?
        as usize;
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.source_object_index == 0 && edge.pointer_offset == offset)
        .ok_or_else(|| anyhow::anyhow!("owner cell-list edge absent"))?;
    let list_index = edge.target_object_index;
    let list = graph
        .objects
        .get(list_index)
        .ok_or_else(|| anyhow::anyhow!("owner cell list outside graph"))?;
    ensure!(
        list.class_name == "CellList",
        "owner cell-list class mismatch"
    );
    let cells = list.fields["m_cells"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("owner cell array absent"))?;
    let mut result = Vec::new();
    for pointer in cells {
        if pointer["pointer_token"].as_u64() == Some(0) {
            continue;
        }
        let offset = pointer["offset"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("cell pointer offset absent"))?
            as usize;
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.source_object_index == list_index && edge.pointer_offset == offset)
            .ok_or_else(|| anyhow::anyhow!("owner cell edge absent"))?;
        let cell = graph
            .objects
            .get(edge.target_object_index)
            .ok_or_else(|| anyhow::anyhow!("owner cell outside graph"))?;
        if cell.class_name != "ProjectParametrizedElemParamsCell" {
            continue;
        }
        for association in cell.fields["m_paramDrivenData"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("project parameter driver array absent"))?
        {
            result.push(GlobalParameterAssociation {
                source_object: edge.target_object_index,
                parameter_id: identifier(&association["m_elemPropId"])?,
                global_parameter_id: identifier(&association["m_famParamId"])?,
                stored_is_symbol: association["m_bIsSymbol"]
                    .as_bool()
                    .ok_or_else(|| anyhow::anyhow!("parameter driver symbol flag absent"))?,
                geometry_tag: identifier(&association["m_geomTag"])?,
            });
        }
    }
    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct EffectiveIfcParameter {
    pub parameter_id: i64,
    pub value: String,
    pub value_source: &'static str,
    pub stored_source_object: Option<usize>,
    pub stored_pointer_field: Option<String>,
    pub exporter_behavior: &'static str,
}
/// Resolve the witnessed API parameter value while keeping the independent
/// default export identifier intact. The override pair demonstrates that
/// ExportUtils.GetExportId can remain unchanged after this parameter is set.
pub fn effective_ifc_parameter(
    class_name: &str,
    metadata: &SavedMetadata,
    default_guid: &str,
    registry: &Registry,
) -> Result<Option<EffectiveIfcParameter>> {
    if registry.builtin_catalog_provenance.is_none() {
        return Ok(None);
    }
    let parameter_id = match class_name {
        "FamilyInstance" | "MaterialElem" | "Level" | "RbsPipeCurve" | "RbsPipingSystem"
        | "SWall" | "Floor" => -1019000,
        "FamilySymbol" | "LevelAttributes" | "BasicWallType" | "FloorAttributes"
        | "RbsPipeType" => -1019001,
        _ => return Ok(None),
    };
    let stored: Vec<_> = metadata
        .parameter_sets
        .iter()
        .flat_map(|set| {
            set.parameters
                .iter()
                .filter(move |parameter| parameter.serialized_parameter_id == parameter_id)
                .map(move |parameter| (set, parameter))
        })
        .collect();
    ensure!(
        stored.len() <= 1,
        "ambiguous stored IFC identifier parameters"
    );
    let (value, value_source, stored_source_object, stored_pointer_field) =
        if let Some((set, parameter)) = stored.first() {
            ensure!(
                parameter.storage_type == "String",
                "IFC identifier parameter storage is not String"
            );
            (
                parameter
                    .raw_value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("IFC identifier string absent"))?
                    .to_owned(),
                "stored_parameter_override",
                Some(set.source_object),
                Some(set.root_pointer_field.clone()),
            )
        } else {
            (
                default_guid.to_owned(),
                "derived_default_parameter_value",
                None,
                None,
            )
        };
    Ok(Some(EffectiveIfcParameter {
        parameter_id,
        value,
        value_source,
        stored_source_object,
        stored_pointer_field,
        exporter_behavior: "exporter_use_of_parameter_override_not_inferred",
    }))
}

/// Derive the default IFC export identifier observed for a native UniqueId.
/// This is an independently named derived identifier, not a stored IfcGUID
/// parameter. Callers must preserve explicit export overrides separately.
pub fn derive_default_ifc_guid(unique_id: &str) -> Result<String> {
    let (episode, suffix) = unique_id
        .rsplit_once('-')
        .ok_or_else(|| anyhow::anyhow!("native UniqueId suffix absent"))?;
    ensure!(
        episode.len() == 36 && episode.is_ascii(),
        "invalid UniqueId episode GUID"
    );
    for offset in [8, 13, 18, 23] {
        ensure!(
            episode.as_bytes()[offset] == b'-',
            "invalid UniqueId GUID separator"
        );
    }
    let compact: String = episode.chars().filter(|ch| *ch != '-').collect();
    ensure!(
        compact.len() == 32 && compact.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid UniqueId GUID digits"
    );
    ensure!(
        (8..=16).contains(&suffix.len()) && suffix.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid UniqueId identifier suffix"
    );
    let mut value =
        u128::from_str_radix(&compact, 16)? ^ u128::from(u64::from_str_radix(suffix, 16)?);
    const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut result = [b'0'; 22];
    for digit in result.iter_mut().rev() {
        *digit = ALPHABET[(value & 63) as usize];
        value >>= 6;
    }
    Ok(String::from_utf8(result.to_vec())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn global_driver_changes_readonly_without_changing_stored_value() {
        let object = |name, fields| json!({"class_tag":20,"class_name":name,"token":4294967295u32,"start":2,"fields_end":4,"fields":fields});
        let edge = |source, offset, target| json!({"source_object_index":source,"pointer_offset":offset,"pointer_token":4294967295u32,"target_object_index":target,"target_class_tag":20});
        let mut registry = Registry::default();
        for (id, class) in [(42, "ParamElemExternal"), (43, "ParamElemGlobal")] {
            let definition: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":12,"objects":[
                object(class,json!({"m_id":id,"m_pParamDef":{"offset":2,"pointer_token":4294967295u32}})),
                object("ParamDefValue",json!({"m_paramElemId":id,"m_caption":"Length","m_readOnly":false,"m_specTypeId":{"m_typeId":"length"}}))],"edges":[edge(0,2,1)]})).unwrap();
            registry
                .insert(
                    crate::native_parameter_definitions::project(&definition)
                        .unwrap()
                        .unwrap(),
                )
                .unwrap();
        }
        let mut graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":12,"objects":[
            object("FamilyInstance",json!({"m_masterSymbolId":-1,"m_pParamValueSetDouble":{"offset":2,"pointer_token":4294967295u32},"m_cellList":{"offset":4,"pointer_token":4294967295u32}})),
            object("ParamValueSetDouble",json!({"m_paramSet":[{"m_paramId":42,"m_value":14.375}]})),
            object("CellList",json!({"m_cells":[{"offset":6,"pointer_token":4294967295u32}]})),
            object("ProjectParametrizedElemParamsCell",json!({"m_paramDrivenData":[{"m_elemPropId":42,"m_famParamId":43,"m_bIsSymbol":false,"m_geomTag":-1}]}))],
            "edges":[edge(0,2,1),edge(0,4,2),edge(2,6,3)]})).unwrap();
        let associated = project_with_definitions(&graph, &registry).unwrap();
        assert_eq!(
            associated.declared_custom_parameters[0].is_read_only,
            Some(true)
        );
        assert_eq!(
            associated.declared_custom_parameters[0].associated_global_parameter_id,
            Some(43)
        );
        assert_eq!(associated.parameter_sets[0].parameters[0].raw_value, 14.375);
        graph.objects[2].fields["m_cells"] = json!([]);
        let unassociated = project_with_definitions(&graph, &registry).unwrap();
        assert_eq!(
            unassociated.declared_custom_parameters[0].is_read_only,
            Some(false)
        );
        assert!(unassociated.global_parameter_associations.is_empty());
    }
    #[test]
    fn material_fraction_projects_integer_percent_through_owner_edge() {
        let object = |name, fields| json!({"class_tag":20,"class_name":name,"token":4294967295u32,"start":2,"fields_end":4,"fields":fields});
        let mut graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":12,
            "objects":[object("MaterialElem",json!({"m_pMaterial":{"offset":2,"pointer_token":4294967295u32}})),
                object("Material",json!({"m_name":"Glass","m_color":255,"m_transparency":0.8999999761581421,"m_smoothness":0.23000000417232513,"m_shininess":17}))],
            "edges":[{"source_object_index":0,"pointer_offset":2,"pointer_token":4294967295u32,"target_object_index":1,"target_class_tag":20}]})).unwrap();
        let projection = project(&graph).unwrap();
        let transparency = projection
            .raw_field_parameters
            .iter()
            .find(|parameter| parameter.parameter_id == -1002551)
            .unwrap();
        assert_eq!(transparency.raw_value, 90);
        assert_eq!(
            projection
                .raw_field_parameters
                .iter()
                .find(|p| p.parameter_id == -1002553)
                .unwrap()
                .raw_value,
            23
        );
        assert_eq!(
            projection
                .raw_field_parameters
                .iter()
                .find(|p| p.parameter_id == -1002554)
                .unwrap()
                .raw_value,
            17
        );
        assert_eq!(
            transparency.projection_rule,
            "normalized_fraction_to_nearest_integer_percent"
        );
        graph.objects[1].fields["m_transparency"] = 1.2.into();
        assert!(project(&graph).is_err());
    }
    #[test]
    fn stored_ifc_parameter_overrides_parameter_value_without_changing_default() {
        let graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":2,"objects":[{"class_tag":20,"class_name":"FamilyInstance","token":0,"start":2,"fields_end":2,"fields":{}}],"edges":[]})).unwrap();
        let mut metadata = project(&graph).unwrap();
        let default = "2dcivZ8tT0cvrYU1_3B247";
        let mut registry = Registry::default();
        assert!(
            effective_ifc_parameter("FamilyInstance", &metadata, default, &registry)
                .unwrap()
                .is_none()
        );
        let catalog: Value =
            serde_json::from_str(include_str!("data/builtin-parameter-definitions-2027.json"))
                .unwrap();
        registry
            .enable_builtin_catalog(catalog["schema_sha256"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            effective_ifc_parameter("FamilyInstance", &metadata, default, &registry)
                .unwrap()
                .unwrap()
                .value,
            default
        );
        metadata.parameter_sets.push(SavedParameterSet {
            source_object: 7,
            root_pointer_field: "m_pParamValueSetAString".into(),
            parameters: vec![Parameter {
                serialized_parameter_id: -1019000,
                storage_type: "String".into(),
                raw_value: "0AAAAAAAAAAAAAAAAAAAAA".into(),
            }],
        });
        let effective = effective_ifc_parameter("FamilyInstance", &metadata, default, &registry)
            .unwrap()
            .unwrap();
        assert_eq!(effective.value, "0AAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(effective.value_source, "stored_parameter_override");
        assert_eq!(effective.stored_source_object, Some(7));
        assert_eq!(default, "2dcivZ8tT0cvrYU1_3B247");
    }
    #[test]
    fn default_ifc_identifier_uses_original_suffix_and_rejects_bad_identity() {
        assert_eq!(
            derive_default_ifc_guid("a79ace63-2377-409b-9d62-781f832c2ec9-00000fce").unwrap(),
            "2dcivZ8tT0cvrYU1_3B247"
        );
        assert!(derive_default_ifc_guid("a79ace63-2377-409b-9d62-781f832c2ec9-invalid").is_err());
    }
    #[test]
    fn follows_asset_parameter_container_edges() {
        let object = |name, fields| json!({"class_tag":20,"class_name":name,"token":4294967295u32,"start":2,"fields_end":4,"fields":fields});
        let mut graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":12,
            "objects":[object("PropertySetElement",json!({"m_oParamSet":{"offset":2,"pointer_token":4294967295u32}})),
                object("ParamSet",json!({"m_pDoubleParams":{"offset":6,"pointer_token":4294967295u32}})),
                object("ParamValueSetDouble",json!({"m_paramSet":[{"m_paramId":-99,"m_value":321.25}]}))],
            "edges":[{"source_object_index":0,"pointer_offset":2,"pointer_token":4294967295u32,"target_object_index":1,"target_class_tag":20},
                {"source_object_index":1,"pointer_offset":6,"pointer_token":4294967295u32,"target_object_index":2,"target_class_tag":20}]})).unwrap();
        let projected = project(&graph).unwrap();
        assert_eq!(
            projected.parameter_sets[0].root_pointer_field,
            "m_oParamSet.m_pDoubleParams"
        );
        assert_eq!(projected.parameter_sets[0].parameters[0].raw_value, 321.25);
        graph.edges[1].source_object_index = 0;
        assert!(project(&graph).is_err());
    }
    #[test]
    fn direct_fields_preserve_internal_sentinels_and_require_class() {
        let mut graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":12,"objects":[
            {"class_tag":20,"class_name":"FamilyInstance","token":0,"start":2,"fields_end":12,
             "fields":{"m_masterSymbolId":24,"m_designOptionId":-4}}],"edges":[]}))
        .unwrap();
        let projected = project(&graph).unwrap();
        assert_eq!(projected.raw_field_parameters.len(), 5);
        assert_eq!(
            projected
                .raw_field_parameters
                .iter()
                .find(|p| p.parameter_id == -1013201)
                .unwrap()
                .raw_value,
            -4
        );
        graph.objects[0].class_name = "UnknownOwner".into();
        assert!(project(&graph).unwrap().raw_field_parameters.is_empty());
    }
    #[test]
    fn selects_owner_edge_and_preserves_all_family_slots() {
        let object = |name, fields| json!({"class_tag":20,"class_name":name,"token":4294967295u32,"start":2,"fields_end":4,"fields":fields});
        let graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":12,
            "objects":[
                object("FamilyInstance",json!({"m_pParamValueSetAString":{"offset":2,"pointer_token":4294967295u32}})),
                object("ParamValueSetAString",json!({"m_paramSet":[{"m_paramId":-1001203,"m_value":"WRONG OWNER"}]})),
                object("ParamValueSetAString",json!({"m_paramSet":[{"m_paramId":-1001203,"m_value":"EQ_1"}]})),
                object("FamilyParams",json!({"m_params":[{"m_paramId":2816,"m_instance":false,"m_reporting":false,
                    "m_value":0.0,"m_int":0,"m_str":"","m_elemId":-1,"m_oExpression":{"pointer_token":0}}]}))
            ],"edges":[{"source_object_index":0,"pointer_offset":2,"pointer_token":4294967295u32,
                "target_object_index":2,"target_class_tag":20}]})).unwrap();
        let projection = project(&graph).unwrap();
        assert_eq!(projection.parameter_sets[0].parameters[0].raw_value, "EQ_1");
        assert_eq!(projection.family_parameter_slots[0].integer_slot, 0);
        assert_eq!(projection.family_parameter_slots[0].string_slot, "");
        assert_eq!(projection.family_parameter_slots[0].element_id_slot, -1);
        let mut broken = graph;
        broken.edges.clear();
        assert!(project(&broken).is_err());
    }
}
