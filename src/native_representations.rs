//! Saved family representation declarations, separated from evaluated geometry.
use crate::{
    native_document::Record, native_equipment::Identity, native_metadata::identifier,
    native_parameters::ObjectGraph,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub group: Option<crate::native_segments::GroupSource>,
    pub stream: String,
    pub group_record_offset: usize,
    pub body_sha256: String,
    pub source_object: usize,
    pub source_field: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Owner {
    pub content_current_selection: Option<crate::native_embedded::CurrentSelection>,
    pub referenced_by_project_families: Vec<Identity>,
    pub local_element_id: u64,
    pub current_identity: Option<Identity>,
    pub content_key_raw_hex: Option<String>,
    pub current_state_established: bool,
}
#[derive(Debug, Serialize)]
pub struct ParameterAssociation {
    pub element_property_id: i64,
    pub family_parameter_local_id: i64,
    pub raw_fields: Value,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Declaration {
    pub raw_category_id: Option<i64>,
    pub parameter_associations: Vec<ParameterAssociation>,
    pub saved_parameter_sets: Value,
    pub owner: Owner,
    pub class_name: String,
    pub raw_visibility_flags: i64,
    pub detail_visibility: Option<BTreeMap<String, bool>>,
    pub interpretation: &'static str,
    pub raw_material_id: Option<i64>,
    pub saved_root_fields: Value,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct FamilyContent {
    pub family: Identity,
    pub content_key_raw_hex: String,
    pub source: Source,
    pub current_embedded_geometry_established: bool,
}
#[derive(Debug, Serialize)]
pub struct OwnerReference {
    pub owner: Identity,
    pub role: &'static str,
    pub raw_target_id: i64,
    pub target_identity: Option<Identity>,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub local_element_id: u64,
    pub content_key_raw_hex: Option<String>,
    pub code: String,
    pub message: String,
}
#[derive(Debug, Serialize)]
pub struct LabeledResource {
    pub owner: Owner,
    pub class_name: String,
    pub name: String,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub labeled_resources: Vec<LabeledResource>,
    pub format: &'static str,
    pub complete_supported_declarations: bool,
    pub complete_representation_parity: bool,
    pub current_geometry_complete: bool,
    pub declarations: Vec<Declaration>,
    pub family_contents: Vec<FamilyContent>,
    pub owner_references: Vec<OwnerReference>,
    pub diagnostics: Vec<Diagnostic>,
    pub unresolved_scope: Vec<&'static str>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    labels: Vec<LabeledResource>,
    identities: BTreeMap<i64, Identity>,
    declarations: Vec<Declaration>,
    family_contents: Vec<FamilyContent>,
    references: Vec<OwnerReference>,
    diagnostics: Vec<Diagnostic>,
}
impl InventoryBuilder {
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        if record.channel != 102 {
            return Ok(());
        }
        let identity = Identity {
            element_id: record.identity.element_id,
            unique_id: record.identity.unique_id.clone(),
        };
        ensure!(
            self.identities
                .insert(i64::try_from(identity.element_id)?, identity.clone())
                .is_none(),
            "duplicate current representation owner"
        );
        let owner = Owner {
            content_current_selection: None,
            referenced_by_project_families: Vec::new(),
            local_element_id: identity.element_id,
            current_identity: Some(identity.clone()),
            content_key_raw_hex: None,
            current_state_established: true,
        };
        let source = Source {
            group: Some(record.source.group.clone()),
            stream: record.source.stream.clone(),
            group_record_offset: record.source.group_record_offset,
            body_sha256: record.source.body_sha256.clone(),
            source_object: 0,
            source_field: "root".into(),
        };
        let class = record.class_name.as_deref().unwrap_or("<unknown>");
        if let Some(graph) = &record.graph {
            if let Err(e) = self.project(owner, graph, source.clone()) {
                self.diagnostics.push(Diagnostic {
                    local_element_id: identity.element_id,
                    content_key_raw_hex: None,
                    code: "unsupported_representation_graph".into(),
                    message: e.to_string(),
                });
            }
            let Some(root) = graph.objects.first() else {
                return Ok(());
            };
            for (owner_class, field, role) in [
                ("FamilySymbol", "m_familyId", "symbol_family"),
                ("FamilyInstance", "m_masterSymbolId", "instance_symbol"),
                (
                    "FamilyInstance",
                    "m_superInstanceId",
                    "nested_supercomponent",
                ),
            ] {
                if class == owner_class
                    && let Some(value) = root.fields.get(field)
                {
                    let mut source = source.clone();
                    source.source_field = field.into();
                    self.references.push(OwnerReference {
                        owner: identity.clone(),
                        role,
                        raw_target_id: identifier(value)?,
                        target_identity: None,
                        source,
                    });
                }
            }
            if class == "Family"
                && let Some(pointer) = root.fields.get("m_oFamDoc")
                && let Some(index) = target(graph, 0, pointer)?
            {
                let doc = &graph.objects[index];
                ensure!(
                    doc.class_name == "FamilyDocument",
                    "family document class mismatch"
                );
                let bytes = doc.fields["m_contentDocGUID"]["m_guid"]["guid_bytes"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("family content key absent"))?;
                ensure!(bytes.len() == 16, "family content key requires16bytes");
                let key = bytes
                    .iter()
                    .map(|v| {
                        v.as_u64()
                            .filter(|b| *b <= 255)
                            .map(|b| format!("{b:02x}"))
                            .ok_or_else(|| anyhow::anyhow!("invalid family content key"))
                    })
                    .collect::<Result<Vec<_>>>()?
                    .join("");
                let mut source = source;
                source.source_object = index;
                source.source_field = "m_oFamDoc.m_contentDocGUID".into();
                self.family_contents.push(FamilyContent {
                    family: identity,
                    content_key_raw_hex: key,
                    source,
                    current_embedded_geometry_established: false,
                });
            }
        } else if matches!(
            class,
            "Family" | "FamilySymbol" | "FamilyInstance" | "ExtrusionElem"
        ) {
            self.diagnostics.push(Diagnostic {
                local_element_id: identity.element_id,
                content_key_raw_hex: None,
                code: "unsupported_representation_graph".into(),
                message: record
                    .diagnostic
                    .clone()
                    .unwrap_or("native graph absent".into()),
            });
        }
        Ok(())
    }
    pub fn ingest_embedded(&mut self, record: &crate::native_embedded::Record) -> Result<()> {
        let owner = Owner {
            content_current_selection: record.current_selection.clone(),
            referenced_by_project_families: Vec::new(),
            local_element_id: record.local_element_id,
            current_identity: None,
            content_key_raw_hex: Some(record.content_key_raw_hex.clone()),
            current_state_established: record.current_selection.is_some(),
        };
        let source = Source {
            group: Some(record.source.group.clone()),
            stream: record.source.stream.clone(),
            group_record_offset: record.source.group_record_offset,
            body_sha256: record.source.body_sha256.clone(),
            source_object: 0,
            source_field: "root".into(),
        };
        if let Some(graph) = &record.graph {
            if let Err(e) = self.project(owner, graph, source) {
                self.diagnostics.push(Diagnostic {
                    local_element_id: record.local_element_id,
                    content_key_raw_hex: Some(record.content_key_raw_hex.clone()),
                    code: "unsupported_embedded_declaration".into(),
                    message: e.to_string(),
                });
            }
        } else if record.class_name.as_deref() == Some("ExtrusionElem") {
            self.diagnostics.push(Diagnostic {
                local_element_id: record.local_element_id,
                content_key_raw_hex: Some(record.content_key_raw_hex.clone()),
                code: "unsupported_embedded_declaration".into(),
                message: record.diagnostic.clone().unwrap_or("graph absent".into()),
            });
        }
        Ok(())
    }
    fn project(&mut self, owner: Owner, graph: &ObjectGraph, mut source: Source) -> Result<()> {
        let root = graph
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty representation graph"))?;
        for (class, field, target_class) in [
            ("CategoryElem", "m_pCategory", "Category"),
            ("MaterialElem", "m_pMaterial", "Material"),
        ] {
            if root.class_name == class
                && let Some(pointer) = root.fields.get(field)
                && let Some(index) = target(graph, 0, pointer)?
            {
                let resource = &graph.objects[index];
                ensure!(
                    resource.class_name == target_class,
                    "representation label target class mismatch"
                );
                if let Some(name) = resource.fields["m_name"].as_str() {
                    let mut provenance = source.clone();
                    provenance.source_object = index;
                    provenance.source_field = format!("{field}.m_name");
                    self.labels.push(LabeledResource {
                        owner: owner.clone(),
                        class_name: class.into(),
                        name: name.into(),
                        source: provenance,
                    });
                }
            }
        }
        let Some(flags) = root
            .fields
            .get("m_famElemVisibility")
            .and_then(|v| v.get("m_flags"))
        else {
            return Ok(());
        };
        let flags = flags
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("invalid visibility flags"))?;
        let detail = if root.class_name == "ExtrusionElem" {
            measured_visibility(flags)
        } else {
            None
        };
        source.source_field = "m_famElemVisibility.m_flags".into();
        let mut fields = serde_json::Map::new();
        for field in [
            "m_invisible",
            "m_cutting",
            "m_materialId",
            "m_categoryId",
            "m_subcategoryId",
            "m_name",
        ] {
            if let Some(v) = root.fields.get(field) {
                fields.insert(field.into(), v.clone());
            }
        }
        let mut associations = Vec::new();
        if let Some(pointer) = root.fields.get("m_cellList")
            && let Some(list_index) = target(graph, 0, pointer)?
        {
            let list = &graph.objects[list_index];
            ensure!(
                list.class_name == "CellList",
                "representation cell list class mismatch"
            );
            for pointer in list.fields["m_cells"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("representation cell array absent"))?
            {
                if let Some(index) = target(graph, list_index, pointer)? {
                    let cell = &graph.objects[index];
                    if cell.class_name == "FamilyParametrizedElemParamsCell" {
                        for (ordinal, row) in cell.fields["m_paramDrivenData"]
                            .as_array()
                            .ok_or_else(|| {
                                anyhow::anyhow!("form parameter association rows absent")
                            })?
                            .iter()
                            .enumerate()
                        {
                            let mut provenance = source.clone();
                            provenance.source_object = index;
                            provenance.source_field = format!(
                                "m_cellList.m_cells.FamilyParametrizedElemParamsCell.m_paramDrivenData[{ordinal}]"
                            );
                            associations.push(ParameterAssociation {
                                element_property_id: identifier(&row["m_elemPropId"])?,
                                family_parameter_local_id: identifier(&row["m_famParamId"])?,
                                raw_fields: row.clone(),
                                source: provenance,
                            });
                        }
                    }
                }
            }
        }
        let metadata = crate::native_metadata::project(graph)?;
        self.declarations.push(Declaration {
            raw_category_id: root
                .fields
                .get("m_categoryId")
                .map(identifier)
                .transpose()?,
            parameter_associations: associations,
            saved_parameter_sets: serde_json::to_value(metadata.parameter_sets)?,
            owner,
            class_name: root.class_name.clone(),
            raw_visibility_flags: flags,
            interpretation: if detail.is_some() {
                "measured_extrusion_detail_declaration"
            } else {
                "raw_visibility_flags_unqualified"
            },
            detail_visibility: detail,
            raw_material_id: root
                .fields
                .get("m_materialId")
                .map(identifier)
                .transpose()?,
            saved_root_fields: Value::Object(fields),
            source,
        });
        Ok(())
    }
    pub fn finish(mut self) -> Result<Inventory> {
        let mut active: BTreeMap<String, Vec<Identity>> = BTreeMap::new();
        for family in &self.family_contents {
            active
                .entry(family.content_key_raw_hex.clone())
                .or_default()
                .push(family.family.clone());
        }
        for owner in self
            .declarations
            .iter_mut()
            .map(|d| &mut d.owner)
            .chain(self.labels.iter_mut().map(|r| &mut r.owner))
        {
            if let Some(key) = &owner.content_key_raw_hex {
                owner.referenced_by_project_families = active.get(key).cloned().unwrap_or_default();
            }
        }

        for reference in &mut self.references {
            if reference.raw_target_id >= 0 {
                reference.target_identity = self.identities.get(&reference.raw_target_id).cloned();
                if reference.target_identity.is_none() {
                    self.diagnostics.push(Diagnostic {
                        local_element_id: reference.owner.element_id,
                        content_key_raw_hex: None,
                        code: "unresolved_representation_owner_reference".into(),
                        message: format!("{} targets {}", reference.role, reference.raw_target_id),
                    });
                }
            }
        }
        Ok(Inventory {
            labeled_resources: self.labels,
            format: "rvt-native-representations/v1",
            complete_supported_declarations: self.diagnostics.is_empty(),
            complete_representation_parity: false,
            current_geometry_complete: false,
            declarations: self.declarations,
            family_contents: self.family_contents,
            owner_references: self.references,
            diagnostics: self.diagnostics,
            unresolved_scope: vec![
                "embedded_current_revision_selection",
                "type_parameter_evaluated_geometry",
                "detail_level_mesh_generation",
                "visibility_combinations_and_view_filters",
                "clearance_semantic_classification",
                "physical_namespace_to_global_element_identity",
            ],
        })
    }
}
fn measured_visibility(flags: i64) -> Option<BTreeMap<String, bool>> {
    let selected = match flags {
        0x203e => "Coarse",
        0x403e => "Medium",
        0x803e => "Fine",
        0xe03e => "All",
        _ => return None,
    };
    Some(
        ["Coarse", "Medium", "Fine"]
            .into_iter()
            .map(|name| (name.into(), selected == "All" || name == selected))
            .collect(),
    )
}
fn target(graph: &ObjectGraph, i: usize, p: &Value) -> Result<Option<usize>> {
    if p["pointer_token"].as_u64() == Some(0) {
        return Ok(None);
    }
    let offset = p["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("representation pointer offset absent"))?
        as usize;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == i && e.pointer_offset == offset)
        .collect();
    ensure!(
        edges.len() == 1,
        "representation pointer requires one owner edge"
    );
    let j = edges[0].target_object_index;
    ensure!(
        j < graph.objects.len(),
        "representation pointer outside graph"
    );
    Ok(Some(j))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn initial_visibility_scope_does_not_guess_combinations_or_instance_sentinel() {
        assert!(measured_visibility(0x203e).unwrap()["Coarse"]);
        assert!(!measured_visibility(0x403e).unwrap()["Fine"]);
        assert!(measured_visibility(0xe03e).unwrap().values().all(|v| *v));
        assert!(measured_visibility(0x603e).is_none());
        assert!(measured_visibility(-1).is_none());
    }
    #[test]
    fn physical_occurrences_retain_namespace_without_fabricating_current_identity() {
        let graph:ObjectGraph=serde_json::from_value(json!({"consumed_bytes":20,"objects":[{"class_tag":1,"class_name":"ExtrusionElem","token":0,"start":2,"fields_end":20,"fields":{"m_famElemVisibility":{"m_flags":8254},"m_materialId":{"m_id":{"m_id64":-1}}}}],"edges":[]})).unwrap();
        let mut builder = InventoryBuilder::default();
        for key in ["aaa", "bbb"] {
            builder
                .project(
                    Owner {
                        content_current_selection: None,
                        referenced_by_project_families: Vec::new(),
                        local_element_id: 10,
                        current_identity: None,
                        content_key_raw_hex: Some(key.into()),
                        current_state_established: false,
                    },
                    &graph,
                    Source {
                        group: None,
                        stream: "Partitions/0".into(),
                        group_record_offset: 0,
                        body_sha256: key.into(),
                        source_object: 0,
                        source_field: "root".into(),
                    },
                )
                .unwrap();
        }
        let inventory = builder.finish().unwrap();
        assert_eq!(inventory.declarations.len(), 2);
        assert!(!inventory.current_geometry_complete);
        assert!(
            inventory
                .declarations
                .iter()
                .all(|d| d.owner.current_identity.is_none() && !d.owner.current_state_established)
        );
        assert_ne!(
            inventory.declarations[0].owner.content_key_raw_hex,
            inventory.declarations[1].owner.content_key_raw_hex
        );
    }
}
