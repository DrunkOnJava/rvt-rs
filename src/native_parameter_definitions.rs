//! Parameter definitions recovered through native owner-to-definition edges.
//! Registry scope is one document/content namespace. Captions never identify a
//! parameter, and unknown definition classes never select a family value slot.
use crate::native_metadata::{FamilyParameterSlots, identifier};
use crate::native_parameters::ObjectGraph;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Definition {
    pub parameter_id: i64,
    pub source_object: usize,
    pub definition_class: String,
    pub owner_class: String,
    pub caption: String,
    pub type_id: Option<String>,
    pub spec_type_id: Option<String>,
    pub group_type_id: Option<String>,
    pub spec_evidence: &'static str,
    pub unit_type_id: Option<String>,
    pub unit_format_fields: Option<Value>,
    pub unit_resolution: String,
    pub shared_guid: Option<String>,
    pub is_shared: Option<bool>,
    pub storage_type: Option<String>,
    pub storage_evidence: &'static str,
    pub definition_fields: Value,
    pub owner_fields: BTreeMap<String, Value>,
}

/// Return None for records that do not own a definition. Missing or ambiguous
/// graph edges and mismatching IDs are errors, not caption-based fallbacks.
pub fn project(graph: &ObjectGraph) -> Result<Option<Definition>> {
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty native graph"))?;
    let Some(pointer) = root.fields.get("m_pParamDef") else {
        return Ok(None);
    };
    if pointer["pointer_token"].as_u64() == Some(0) {
        return Ok(None);
    }
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("definition pointer offset absent"))?
        as usize;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == 0 && e.pointer_offset == offset)
        .collect();
    ensure!(
        edges.len() == 1,
        "definition pointer must have exactly one edge"
    );
    let source_object = edges[0].target_object_index;
    let definition = graph
        .objects
        .get(source_object)
        .ok_or_else(|| anyhow::anyhow!("definition edge outside graph"))?;
    let parameter_id = identifier(&root.fields["m_id"])?;
    ensure!(
        identifier(&definition.fields["m_paramElemId"])? == parameter_id,
        "definition owner ID mismatch"
    );
    let caption = definition.fields["m_caption"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("definition caption absent"))?
        .to_owned();
    let type_id = |name: &str| -> Result<Option<String>> {
        definition
            .fields
            .get(name)
            .map(|v| {
                v["m_typeId"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| anyhow::anyhow!("invalid definition {name}"))
            })
            .transpose()
    };
    // Exact concrete classes, not inheritance: MaterialBrowse derives from
    // ParamDefString but its value is an ElementId.
    let storage = match definition.class_name.as_str() {
        "ParamDefString" => Some("String"),
        "ParamDefValue" => Some("Double"),
        "ParamDefInt" | "ParamDefYesNo" => Some("Integer"),
        "ParamDefMaterialBrowse" => Some("ElementId"),
        _ => None,
    };
    let saved_spec = type_id("m_specTypeId")?;
    let class_spec = match definition.class_name.as_str() {
        "ParamDefString" => Some("autodesk.spec:spec.string-2.0.0"),
        "ParamDefInt" => Some("autodesk.spec:spec.int64-2.0.0"),
        "ParamDefYesNo" => Some("autodesk.spec:spec.bool-1.0.0"),
        "ParamDefMaterialBrowse" => Some("autodesk.spec.aec:material-1.0.0"),
        _ => None,
    };
    if let (Some(saved), Some(expected)) = (&saved_spec, class_spec) {
        ensure!(
            saved == expected,
            "definition class and serialized specification disagree"
        );
    }
    let spec_evidence = if saved_spec.is_some() {
        "serialized_spec_type_id"
    } else if class_spec.is_some() {
        "validated_concrete_definition_class"
    } else {
        "unresolved"
    };
    let spec_type_id = saved_spec.or_else(|| class_spec.map(str::to_owned));
    let shared_guid = root
        .fields
        .get("m_externalParamKey")
        .map(|v| {
            let bytes = v["m_guidValue"]["m_guid"]["guid_bytes"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("shared GUID bytes absent"))?;
            ensure!(bytes.len() == 16, "shared GUID length invalid");
            let mut b = [0u8; 16];
            for (i, v) in bytes.iter().enumerate() {
                b[i] = u8::try_from(
                    v.as_u64()
                        .ok_or_else(|| anyhow::anyhow!("invalid GUID byte"))?,
                )?;
            }
            Ok::<_, anyhow::Error>(format!(
                "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                u32::from_le_bytes(b[0..4].try_into().unwrap()),
                u16::from_le_bytes(b[4..6].try_into().unwrap()),
                u16::from_le_bytes(b[6..8].try_into().unwrap()),
                b[8],
                b[9],
                b[10],
                b[11],
                b[12],
                b[13],
                b[14],
                b[15]
            ))
        })
        .transpose()?;
    let owner_fields = [
        "m_bindingIds",
        "m_instanceParam",
        "m_famId",
        "m_description",
        "m_userModifiable",
        "m_hideWhenNoValue",
    ]
    .into_iter()
    .filter_map(|name| root.fields.get(name).map(|v| (name.to_owned(), v.clone())))
    .collect();
    Ok(Some(Definition {
        parameter_id,
        source_object,
        definition_class: definition.class_name.clone(),
        owner_class: root.class_name.clone(),
        caption,
        type_id: type_id("m_typeId")?,
        spec_type_id,
        spec_evidence,
        unit_type_id: None,
        unit_format_fields: None,
        unit_resolution: if matches!(storage, Some("String" | "Integer" | "ElementId")) {
            "not_applicable"
        } else {
            "unresolved"
        }
        .into(),
        group_type_id: type_id("m_groupTypeId")?,
        shared_guid,
        is_shared: match root.class_name.as_str() {
            "ParamElemExternal" => Some(true),
            "ParamElemFamily" => Some(false),
            _ => None,
        },
        storage_type: storage.map(str::to_owned),
        storage_evidence: if storage.is_some() {
            "validated_concrete_definition_class"
        } else {
            "unsupported_definition_class"
        },
        definition_fields: definition.fields.clone(),
        owner_fields,
    }))
}

#[derive(Debug, Default, Serialize)]
pub struct Registry {
    pub definitions: BTreeMap<i64, Definition>,
    pub unit_formats: BTreeMap<String, Value>,
    pub builtin_definitions: BTreeMap<i64, BuiltinDefinition>,
    pub builtin_catalog_provenance: Option<Value>,
    pub bindings: BTreeMap<i64, Binding>,
    pub family_categories: BTreeMap<i64, i64>,
    pub symbol_families: BTreeMap<i64, i64>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Binding {
    pub binding_id: i64,
    pub parameter_id: i64,
    pub category_id: i64,
    pub elem_or_symbol: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinDefinition {
    pub parameter_id: i64,
    pub caption: String,
    pub storage_type: String,
    pub spec_type_id: String,
    pub group_type_id: String,
    pub is_shared: bool,
    #[serde(default)]
    pub unit_type_id: Option<String>,
    #[serde(default)]
    pub unit_format_fields: Option<Value>,
    #[serde(default)]
    pub unit_resolution: String,
    #[serde(default)]
    pub definition_evidence: String,
    #[serde(default)]
    pub caption_locale: String,
}
impl Registry {
    /// Retain only the native links needed to resolve category bindings. All IDs
    /// must come from the same current document namespace as the definitions.
    pub fn ingest_binding_context(&mut self, graph: &ObjectGraph) -> Result<()> {
        let root = graph
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty binding context"))?;
        let id = identifier(&root.fields["m_id"])?;
        match root.class_name.as_str() {
            "ParamBinding" => {
                let binding = Binding {
                    binding_id: id,
                    parameter_id: identifier(&root.fields["m_paramId"])?,
                    category_id: identifier(&root.fields["m_categoryId"])?,
                    elem_or_symbol: identifier(&root.fields["m_elemOrSymbol"])?,
                };
                ensure!(
                    self.bindings
                        .get(&id)
                        .is_none_or(|previous| previous == &binding),
                    "conflicting parameter bindings"
                );
                self.bindings.insert(id, binding);
            }
            "Family" => {
                let category = identifier(&root.fields["m_categoryId"])?;
                ensure!(
                    self.family_categories
                        .get(&id)
                        .is_none_or(|previous| *previous == category),
                    "conflicting family categories"
                );
                self.family_categories.insert(id, category);
            }
            "FamilySymbol" => {
                let family = identifier(&root.fields["m_familyId"])?;
                ensure!(
                    self.symbol_families
                        .get(&id)
                        .is_none_or(|previous| *previous == family),
                    "conflicting symbol family references"
                );
                self.symbol_families.insert(id, family);
            }
            _ => anyhow::bail!("unsupported parameter binding context class"),
        }
        Ok(())
    }
    pub fn matching_bindings(&self, graph: &ObjectGraph) -> Result<Vec<&Binding>> {
        let root = graph
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty binding owner"))?;
        let (family, kind) = match root.class_name.as_str() {
            "FamilySymbol" => (identifier(&root.fields["m_familyId"])?, 2),
            "FamilyInstance" => {
                let symbol = identifier(&root.fields["m_masterSymbolId"])?;
                let Some(family) = self.symbol_families.get(&symbol) else {
                    return Ok(Vec::new());
                };
                (*family, 1)
            }
            _ => return Ok(Vec::new()),
        };
        let Some(category) = self.family_categories.get(&family) else {
            return Ok(Vec::new());
        };
        let mut result = Vec::new();
        for binding in self
            .bindings
            .values()
            .filter(|b| b.category_id == *category && b.elem_or_symbol == kind)
        {
            let Some(definition) = self.definitions.get(&binding.parameter_id) else {
                continue;
            };
            let Some(ids) = definition
                .owner_fields
                .get("m_bindingIds")
                .and_then(Value::as_array)
            else {
                continue;
            };
            let backlinks = ids.iter().map(identifier).collect::<Result<Vec<_>>>()?;
            ensure!(
                backlinks.contains(&binding.binding_id),
                "parameter binding lacks definition backlink"
            );
            result.push(binding);
        }
        Ok(result)
    }
    /// Enable independently witnessed built-in schema facts only for the exact
    /// qualified schema. Captions are catalog English, not file-localized text.
    pub fn enable_builtin_catalog(&mut self, schema_sha256: &str) -> Result<()> {
        let mut catalog: Value =
            serde_json::from_str(include_str!("data/builtin-parameter-definitions-2027.json"))?;
        if catalog["schema_sha256"].as_str() != Some(schema_sha256) {
            return Ok(());
        }
        let entries = catalog
            .as_object_mut()
            .unwrap()
            .remove("definitions")
            .ok_or_else(|| anyhow::anyhow!("built-in catalog definitions absent"))?;
        let entries: Vec<BuiltinDefinition> = serde_json::from_value(entries)?;
        for mut definition in entries {
            ensure!(
                definition.parameter_id < 0,
                "built-in catalog contains nonnegative ID"
            );
            definition.definition_evidence = "schema_qualified_api_definition_catalog".into();
            definition.caption_locale = "en-US".into();
            definition.unit_resolution = if definition.storage_type == "Double" {
                "unresolved"
            } else {
                "not_applicable"
            }
            .into();
            if let Some(format) = self.unit_formats.get(&definition.spec_type_id) {
                definition.unit_type_id = format["m_unitTypeId"]["m_typeId"]
                    .as_str()
                    .map(str::to_owned);
                definition.unit_format_fields = Some(format.clone());
                definition.unit_resolution = "resolved_native_project_units".into();
            }
            ensure!(
                self.builtin_definitions
                    .insert(definition.parameter_id, definition)
                    .is_none(),
                "duplicate builtin catalog activation or ID"
            );
        }
        self.builtin_catalog_provenance = Some(catalog);
        Ok(())
    }
    /// Ingest project display units from the current UnitsElem. Values themselves
    /// remain in native internal units; this never numerically converts them.
    pub fn ingest_units(&mut self, graph: &ObjectGraph) -> Result<()> {
        let root = graph
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty units graph"))?;
        ensure!(
            root.class_name == "UnitsElem",
            "unit registry requires UnitsElem"
        );
        let pairs = root.fields["m_units"]["m_formatOptionsMap"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("native unit format map absent"))?;
        let mut formats = BTreeMap::new();
        for pair in pairs {
            let spec = pair["first"]["m_typeId"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("native unit specification absent"))?;
            let format = pair
                .get("second")
                .ok_or_else(|| anyhow::anyhow!("native unit format absent"))?;
            ensure!(
                format["m_unitTypeId"]["m_typeId"].is_string(),
                "native display unit type absent"
            );
            ensure!(
                formats.insert(spec.to_owned(), format.clone()).is_none(),
                "duplicate native unit specification"
            );
        }
        ensure!(
            self.unit_formats.is_empty() || self.unit_formats == formats,
            "conflicting native unit registries"
        );
        self.unit_formats = formats;
        for definition in self.definitions.values_mut() {
            enrich_unit(definition, &self.unit_formats);
        }
        for definition in self.builtin_definitions.values_mut() {
            if let Some(format) = self.unit_formats.get(&definition.spec_type_id) {
                definition.unit_type_id = format["m_unitTypeId"]["m_typeId"]
                    .as_str()
                    .map(str::to_owned);
                definition.unit_format_fields = Some(format.clone());
                definition.unit_resolution = "resolved_native_project_units".into();
            }
        }
        Ok(())
    }
    pub fn insert(&mut self, mut definition: Definition) -> Result<()> {
        enrich_unit(&mut definition, &self.unit_formats);
        if let Some(existing) = self.definitions.get(&definition.parameter_id) {
            ensure!(
                existing == &definition,
                "conflicting native parameter definitions"
            );
        } else {
            self.definitions.insert(definition.parameter_id, definition);
        }
        Ok(())
    }
    /// Select a slot only from an independently recovered definition. This does
    /// not claim HasValue, evaluated formula values, or instance inheritance.
    pub fn resolve_slot(&self, slot: &FamilyParameterSlots) -> Option<ResolvedFamilyParameter> {
        let storage_type = if slot.parameter_id < 0 {
            self.builtin_definitions
                .get(&slot.parameter_id)?
                .storage_type
                .as_str()
        } else {
            self.definitions
                .get(&slot.parameter_id)?
                .storage_type
                .as_deref()?
        };
        let raw_value = match storage_type {
            "Double" => slot.double_slot.clone(),
            "Integer" => slot.integer_slot.clone(),
            "String" => slot.string_slot.clone(),
            "ElementId" => slot.element_id_slot.into(),
            _ => return None,
        };
        Some(ResolvedFamilyParameter {
            source_object: slot.source_object,
            parameter_id: slot.parameter_id,
            storage_type: storage_type.into(),
            raw_value,
            stored_instance_flag: slot.stored_instance_flag,
            reporting: slot.reporting,
            definition_parameter_id: slot.parameter_id,
            has_value: true,
            has_value_evidence: "validated_present_family_parameter_slot",
            value_semantics: "serialized_not_evaluated",
        })
    }
}
fn enrich_unit(definition: &mut Definition, formats: &BTreeMap<String, Value>) {
    if let Some(format) = definition
        .spec_type_id
        .as_ref()
        .and_then(|spec| formats.get(spec))
    {
        definition.unit_type_id = format["m_unitTypeId"]["m_typeId"]
            .as_str()
            .map(str::to_owned);
        definition.unit_format_fields = Some(format.clone());
        definition.unit_resolution = "resolved_native_project_units".into();
    }
}
#[derive(Debug, Serialize)]
pub struct ResolvedFamilyParameter {
    pub source_object: usize,
    pub parameter_id: i64,
    pub storage_type: String,
    pub raw_value: Value,
    pub stored_instance_flag: bool,
    pub reporting: bool,
    pub definition_parameter_id: i64,
    pub has_value: bool,
    pub has_value_evidence: &'static str,
    pub value_semantics: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn graph(class: &str) -> ObjectGraph {
        serde_json::from_value(json!({"consumed_bytes":200,"objects":[
            {"class_tag":10,"class_name":"ParamElemExternal","token":0,"start":2,"fields_end":100,"fields":{
                "m_id":42,"m_pParamDef":{"offset":90,"pointer_token":4294967295u32},
                "m_externalParamKey":{"m_guidValue":{"m_guid":{"guid_bytes":[245,6,106,179,61,175,227,78,137,172,33,127,0,0,1,1]}}}}},
            {"class_tag":11,"class_name":class,"token":4294967295u32,"start":100,"fields_end":200,"fields":{
                "m_paramElemId":42,"m_caption":"Same caption","m_typeId":{"m_typeId":"test:definition"}}}],
            "edges":[{"source_object_index":0,"pointer_offset":90,"pointer_token":4294967295u32,"target_object_index":1,"target_class_tag":11}]})).unwrap()
    }
    #[test]
    fn absent_state_requires_category_kind_and_bidirectional_binding() {
        let mut definition_graph = graph("ParamDefString");
        definition_graph.objects[0].fields["m_bindingIds"] = json!([7]);
        let definition = project(&definition_graph).unwrap().unwrap();
        let mut registry = Registry::default();
        registry.insert(definition).unwrap();
        let mut binding = graph("Unused");
        binding.objects[0].class_name = "ParamBinding".into();
        binding.objects[0].fields =
            json!({"m_id":7,"m_paramId":42,"m_categoryId":99,"m_elemOrSymbol":2});
        registry.ingest_binding_context(&binding).unwrap();
        let mut family = graph("Unused");
        family.objects[0].class_name = "Family".into();
        family.objects[0].fields = json!({"m_id":8,"m_categoryId":99});
        registry.ingest_binding_context(&family).unwrap();
        let mut owner = graph("Unused");
        owner.objects.truncate(1);
        owner.edges.clear();
        owner.objects[0].class_name = "FamilySymbol".into();
        owner.objects[0].fields = json!({"m_id":10,"m_familyId":8});
        registry.ingest_binding_context(&owner).unwrap();
        let projected =
            crate::native_metadata::project_with_definitions(&owner, &registry).unwrap();
        assert_eq!(projected.declared_custom_parameters.len(), 1);
        assert!(!projected.declared_custom_parameters[0].has_value);
        assert_eq!(projected.declared_custom_parameters[0].binding_id, Some(7));
        assert_eq!(projected.parameter_definitions[0].parameter_id, 42);
        owner.objects[0].class_name = "FamilyInstance".into();
        owner.objects[0].fields = json!({"m_id":11,"m_masterSymbolId":10});
        assert!(registry.matching_bindings(&owner).unwrap().is_empty());
        owner.objects[0].class_name = "FamilySymbol".into();
        owner.objects[0].fields = json!({"m_id":10,"m_familyId":8});
        registry
            .definitions
            .get_mut(&42)
            .unwrap()
            .owner_fields
            .insert("m_bindingIds".into(), json!([]));
        assert!(registry.matching_bindings(&owner).is_err());
    }
    #[test]
    fn builtin_catalog_is_profile_qualified_and_has_no_values() {
        let mut registry = Registry::default();
        registry.enable_builtin_catalog("unqualified").unwrap();
        assert!(registry.builtin_definitions.is_empty());
        let catalog: Value =
            serde_json::from_str(include_str!("data/builtin-parameter-definitions-2027.json"))
                .unwrap();
        registry
            .enable_builtin_catalog(catalog["schema_sha256"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            registry.builtin_definitions[&-1010108].storage_type,
            "String"
        );
        assert_eq!(
            registry.builtin_definitions[&-1010108].caption_locale,
            "en-US"
        );
        for definition in catalog["definitions"].as_array().unwrap() {
            assert!(definition.get("raw_value").is_none());
            assert!(definition.get("has_value").is_none());
            assert!(definition.get("is_read_only").is_none());
            assert!(definition.get("owner_id").is_none());
        }
    }
    #[test]
    fn integer_specs_and_native_units_are_independent_of_values() {
        let yes_no = project(&graph("ParamDefYesNo")).unwrap().unwrap();
        assert_eq!(yes_no.storage_type.as_deref(), Some("Integer"));
        assert_eq!(
            yes_no.spec_type_id.as_deref(),
            Some("autodesk.spec:spec.bool-1.0.0")
        );
        let mut value_graph = graph("ParamDefValue");
        value_graph.objects[1].fields["m_specTypeId"] = json!({"m_typeId":"test:length"});
        let definition = project(&value_graph).unwrap().unwrap();
        let mut units = graph("Unused");
        units.objects[0].class_name = "UnitsElem".into();
        units.objects[0].fields = json!({"m_units":{"m_formatOptionsMap":[{"first":{"m_typeId":"test:length"},"second":{"m_unitTypeId":{"m_typeId":"test:millimeters"},"m_accuracy":0.1}}]}});
        let mut registry = Registry::default();
        registry.insert(definition.clone()).unwrap();
        registry.ingest_units(&units).unwrap();
        assert_eq!(
            registry.definitions[&42].unit_type_id.as_deref(),
            Some("test:millimeters")
        );
        registry.insert(definition).unwrap();
        units.objects[0].fields["m_units"]["m_formatOptionsMap"][0]["second"]["m_unitTypeId"]["m_typeId"] =
            "test:feet".into();
        assert!(registry.ingest_units(&units).is_err());
    }
    #[test]
    fn definition_identity_and_material_storage_are_structural() {
        let definition = project(&graph("ParamDefMaterialBrowse")).unwrap().unwrap();
        assert_eq!(definition.storage_type.as_deref(), Some("ElementId"));
        assert_eq!(
            definition.shared_guid.as_deref(),
            Some("b36a06f5-af3d-4ee3-89ac-217f00000101")
        );
        let mut invalid = graph("ParamDefString");
        invalid.objects[1]
            .fields
            .as_object_mut()
            .unwrap()
            .insert("m_paramElemId".into(), 43.into());
        assert!(project(&invalid).is_err());
        let mut invalid = graph("ParamDefString");
        invalid.edges.clear();
        assert!(project(&invalid).is_err());
    }
    #[test]
    fn zero_slots_are_selected_from_definition_and_conflicts_rejected() {
        let mut registry = Registry::default();
        let definition = project(&graph("ParamDefString")).unwrap().unwrap();
        registry.insert(definition.clone()).unwrap();
        let slot = FamilyParameterSlots {
            source_object: 2,
            parameter_id: 42,
            stored_instance_flag: true,
            reporting: false,
            double_slot: 99.into(),
            integer_slot: 77.into(),
            string_slot: "".into(),
            element_id_slot: 6,
            expression_pointer: Value::Null,
        };
        assert_eq!(registry.resolve_slot(&slot).unwrap().raw_value, "");
        let mut conflict = definition;
        conflict.caption = "Different".into();
        assert!(registry.insert(conflict).is_err());
        let mut unknown = Registry::default();
        unknown
            .insert(project(&graph("ParamDefUnknown")).unwrap().unwrap())
            .unwrap();
        assert!(unknown.resolve_slot(&slot).is_none());
    }
}
