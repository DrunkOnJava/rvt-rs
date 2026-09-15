//! Material assets and construction layers from explicit native ownership.
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
    pub owner: Identity,
    pub source_object: usize,
    pub source_field: String,
    pub body_sha256: String,
    pub stream: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct Reference {
    pub raw_target_id: i64,
    pub target_identity: Option<Identity>,
    pub role: String,
    pub resolution: String,
    pub raw_role_key: Option<Value>,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Material {
    pub identity: Identity,
    pub name: Option<String>,
    pub references: Vec<Reference>,
    pub saved_graphics: Value,
    pub saved_metadata: Option<Value>,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Asset {
    pub parameter_value_unit_basis: &'static str,
    pub identity: Identity,
    pub class_name: String,
    pub property_set_type: Option<i64>,
    pub saved_metadata: Option<Value>,
    pub saved_asset_graph: Option<Value>,
    pub appearance: Option<crate::native_appearance::Appearance>,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Layer {
    pub index: usize,
    pub width: f64,
    pub length_unit: &'static str,
    pub function: i64,
    pub material: Reference,
    pub raw_fields: Value,
    pub source: Source,
}
#[derive(Debug, Serialize)]
pub struct Construction {
    pub identity: Identity,
    pub class_name: String,
    pub layers: Vec<Layer>,
    pub saved_structure_fields: Value,
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
    pub complete_supported_materials: bool,
    pub complete_material_parity: bool,
    pub materials: Vec<Material>,
    pub assets: Vec<Asset>,
    pub constructions: Vec<Construction>,
    pub owner_type_references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
    pub unresolved_scope: Vec<&'static str>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    identities: BTreeMap<i64, (Identity, String, Option<i64>)>,
    materials: Vec<Material>,
    assets: Vec<Asset>,
    constructions: Vec<Construction>,
    owner_types: Vec<Reference>,
    diagnostics: Vec<Diagnostic>,
}
impl InventoryBuilder {
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        if record.channel != 102 {
            return Ok(());
        }
        let class = record.class_name.as_deref().unwrap_or("<unknown>");
        let root = record.graph.as_ref().and_then(|g| g.objects.first());
        let property_type = root
            .and_then(|r| r.fields.get("m_propertySetType"))
            .and_then(Value::as_i64);
        ensure!(
            self.identities
                .insert(
                    i64::try_from(record.identity.element_id)?,
                    (identity(record), class.into(), property_type)
                )
                .is_none(),
            "duplicate material context owner"
        );
        if !matches!(
            class,
            "MaterialElem"
                | "PropertySetElement"
                | "AppearanceAssetElem"
                | "BasicWallType"
                | "FloorAttributes"
                | "RoofAttributes"
                | "SWall"
                | "Floor"
        ) {
            return Ok(());
        }
        if let Err(e) = self.project(record) {
            self.diagnostics.push(Diagnostic {
                element_id: record.identity.element_id,
                code: "unsupported_material_graph".into(),
                message: e.to_string(),
            });
        }
        Ok(())
    }
    fn project(&mut self, record: &Record) -> Result<()> {
        let graph = record
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("material context graph absent"))?;
        let root = graph
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty material graph"))?;
        let metadata = record
            .saved_metadata
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        match root.class_name.as_str() {
            "MaterialElem" => {
                let i = target(graph, 0, &root.fields["m_pMaterial"])?
                    .ok_or_else(|| anyhow::anyhow!("material pointer null"))?;
                let material = &graph.objects[i];
                ensure!(
                    material.class_name == "Material",
                    "material owner target mismatch"
                );
                let mut refs = Vec::new();
                if let Some(value) = material.fields.get("m_appearanceAssetId") {
                    refs.push(reference(
                        record,
                        i,
                        "m_pMaterial.m_appearanceAssetId",
                        identifier(value)?,
                        "appearance",
                        None,
                    ));
                }
                for (j, pair) in material.fields["m_assetMap"]["m_sharedAssetIdMap"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("shared material asset map absent"))?
                    .iter()
                    .enumerate()
                {
                    refs.push(reference(
                        record,
                        i,
                        &format!("m_pMaterial.m_assetMap.m_sharedAssetIdMap[{j}]"),
                        identifier(&pair["second"])?,
                        "unresolved_shared_asset",
                        Some(pair["first"].clone()),
                    ));
                }
                let mut graphics = serde_json::Map::new();
                for field in [
                    "m_color",
                    "m_shininess",
                    "m_smoothness",
                    "m_transparency",
                    "m_glow",
                    "m_bUseRenderAppearance",
                ] {
                    if let Some(v) = material.fields.get(field) {
                        graphics.insert(field.into(), v.clone());
                    }
                }
                self.materials.push(Material {
                    identity: identity(record),
                    name: material.fields["m_name"].as_str().map(str::to_owned),
                    references: refs,
                    saved_graphics: Value::Object(graphics),
                    saved_metadata: metadata,
                    source: source(record, i, "m_pMaterial"),
                });
            }
            "PropertySetElement" | "AppearanceAssetElem" => {
                self.assets.push(Asset {
                    parameter_value_unit_basis: "revit_internal_units_for_parameter_spec",
                    identity: identity(record),
                    class_name: root.class_name.clone(),
                    property_set_type: root.fields["m_propertySetType"].as_i64(),
                    saved_metadata: metadata,
                    appearance: (root.class_name == "AppearanceAssetElem")
                        .then(|| crate::native_appearance::project(graph)),
                    saved_asset_graph: if root.class_name == "AppearanceAssetElem" {
                        Some(serde_json::to_value(graph)?)
                    } else {
                        None
                    },
                    source: source(record, 0, "asset_owner"),
                });
            }
            _ => {
                if let Some(pointer) = root.fields.get("m_pCompoundStructure")
                    && let Some(i) = target(graph, 0, pointer)?
                {
                    let structure = &graph.objects[i];
                    ensure!(
                        structure.class_name == "CompoundStructure",
                        "compound structure class mismatch"
                    );
                    let mut layers = Vec::new();
                    for (j, layer) in structure.fields["m_layers"]
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("compound layer array absent"))?
                        .iter()
                        .enumerate()
                    {
                        let width = layer["m_layerWidth"]
                            .as_f64()
                            .ok_or_else(|| anyhow::anyhow!("layer width absent"))?;
                        ensure!(
                            width.is_finite() && width >= 0.,
                            "invalid compound layer width"
                        );
                        layers.push(Layer {
                            index: j,
                            width,
                            length_unit: "feet",
                            function: layer["m_layerFunction"]
                                .as_i64()
                                .ok_or_else(|| anyhow::anyhow!("layer function absent"))?,
                            material: reference(
                                record,
                                i,
                                &format!("m_pCompoundStructure.m_layers[{j}].m_materialId"),
                                identifier(&layer["m_materialId"])?,
                                "layer_material",
                                None,
                            ),
                            raw_fields: layer.clone(),
                            source: source(
                                record,
                                i,
                                &format!("m_pCompoundStructure.m_layers[{j}]"),
                            ),
                        });
                    }
                    self.constructions.push(Construction {
                        identity: identity(record),
                        class_name: root.class_name.clone(),
                        layers,
                        saved_structure_fields: structure.fields.clone(),
                        source: source(record, i, "m_pCompoundStructure"),
                    });
                }
                if root.class_name == "SWall" {
                    self.owner_types.push(reference(
                        record,
                        0,
                        "m_WallAttributesId",
                        identifier(&root.fields["m_WallAttributesId"])?,
                        "construction_type",
                        None,
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn finish(mut self) -> Result<Inventory> {
        for material in &mut self.materials {
            for reference in &mut material.references {
                resolve(reference, &self.identities, &mut self.diagnostics);
            }
        }
        for construction in &mut self.constructions {
            for layer in &mut construction.layers {
                resolve(&mut layer.material, &self.identities, &mut self.diagnostics);
            }
        }
        for reference in &mut self.owner_types {
            resolve(reference, &self.identities, &mut self.diagnostics);
        }
        Ok(Inventory {
            format: "rvt-native-materials/v1",
            complete_supported_materials: self.diagnostics.is_empty(),
            complete_material_parity: false,
            materials: self.materials,
            assets: self.assets,
            constructions: self.constructions,
            owner_type_references: self.owner_types,
            diagnostics: self.diagnostics,
            unresolved_scope: vec![
                "per_face_material_assignment",
                "appearance_shader_evaluation",
                "texture_resource_resolution",
                "vertically_variable_layer_geometry",
            ],
        })
    }
}
fn identity(r: &Record) -> Identity {
    Identity {
        element_id: r.identity.element_id,
        unique_id: r.identity.unique_id.clone(),
    }
}
fn source(r: &Record, i: usize, f: &str) -> Source {
    Source {
        owner: identity(r),
        source_object: i,
        source_field: f.into(),
        body_sha256: r.source.body_sha256.clone(),
        stream: r.source.stream.clone(),
        group_record_offset: r.source.group_record_offset,
    }
}
fn reference(r: &Record, i: usize, f: &str, id: i64, role: &str, key: Option<Value>) -> Reference {
    Reference {
        raw_target_id: id,
        target_identity: None,
        role: role.into(),
        resolution: "unresolved".into(),
        raw_role_key: key,
        source: source(r, i, f),
    }
}
fn target(graph: &ObjectGraph, i: usize, p: &Value) -> Result<Option<usize>> {
    if p["pointer_token"].as_u64() == Some(0) {
        return Ok(None);
    }
    let offset = p["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("material pointer offset absent"))?
        as usize;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == i && e.pointer_offset == offset)
        .collect();
    ensure!(edges.len() == 1, "material pointer requires one owner edge");
    let j = edges[0].target_object_index;
    ensure!(j < graph.objects.len(), "material pointer outside graph");
    Ok(Some(j))
}
fn resolve(
    reference: &mut Reference,
    identities: &BTreeMap<i64, (Identity, String, Option<i64>)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if reference.raw_target_id < 0 {
        reference.resolution = "negative_serialized_reference".into();
        return;
    }
    if let Some((identity, class, kind)) = identities.get(&reference.raw_target_id) {
        let expected = match reference.role.as_str() {
            "appearance" => class == "AppearanceAssetElem",
            "unresolved_shared_asset" => {
                class == "PropertySetElement" && matches!(kind, Some(1 | 2))
            }
            "layer_material" => class == "MaterialElem",
            "construction_type" => matches!(
                class.as_str(),
                "BasicWallType" | "FloorAttributes" | "RoofAttributes"
            ),
            _ => false,
        };
        if !expected {
            reference.resolution = "target_class_or_role_unresolved".into();
            diagnostics.push(Diagnostic {
                element_id: reference.source.owner.element_id,
                code: "material_target_class_mismatch".into(),
                message: format!(
                    "role {} targets {} property-set type {:?}",
                    reference.role, class, kind
                ),
            });
            return;
        }
        reference.target_identity = Some(identity.clone());
        reference.resolution = "resolved_current_element".into();
        if reference.role == "unresolved_shared_asset" && class == "PropertySetElement" {
            reference.role = match kind {
                Some(1) => "structural",
                Some(2) => "thermal",
                _ => "unclassified_property_set",
            }
            .into();
        }
    } else {
        reference.resolution = "unresolved_target".into();
        diagnostics.push(Diagnostic {
            element_id: reference.source.owner.element_id,
            code: "unresolved_material_reference".into(),
            message: format!(
                "{} targets absent element {}",
                reference.source.source_field, reference.raw_target_id
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn reference_to(id: i64) -> Reference {
        Reference {
            raw_target_id: id,
            target_identity: None,
            role: "unresolved_shared_asset".into(),
            resolution: "unresolved".into(),
            raw_role_key: Some(json!({"guid":"retained"})),
            source: Source {
                owner: Identity {
                    element_id: 1,
                    unique_id: "material".into(),
                },
                source_object: 1,
                source_field: "m_assetMap".into(),
                body_sha256: "synthetic".into(),
                stream: "Partitions/0".into(),
                group_record_offset: 0,
            },
        }
    }
    #[test]
    fn asset_role_uses_resolved_property_set_type_and_rejects_wrong_target_class() {
        let identities = [
            (
                2,
                (
                    Identity {
                        element_id: 2,
                        unique_id: "thermal".into(),
                    },
                    "PropertySetElement".into(),
                    Some(2),
                ),
            ),
            (
                3,
                (
                    Identity {
                        element_id: 3,
                        unique_id: "wall".into(),
                    },
                    "SWall".into(),
                    None,
                ),
            ),
        ]
        .into_iter()
        .collect();
        let mut diagnostics = Vec::new();
        let mut good = reference_to(2);
        resolve(&mut good, &identities, &mut diagnostics);
        assert_eq!(good.role, "thermal");
        assert_eq!(good.target_identity.unwrap().unique_id, "thermal");
        assert!(diagnostics.is_empty());
        let mut bad = reference_to(3);
        resolve(&mut bad, &identities, &mut diagnostics);
        assert_eq!(bad.resolution, "target_class_or_role_unresolved");
        assert_eq!(diagnostics.len(), 1);
        let mut negative = reference_to(-4);
        resolve(&mut negative, &identities, &mut diagnostics);
        assert_eq!(negative.raw_target_id, -4);
        assert_eq!(negative.resolution, "negative_serialized_reference");
    }
    #[test]
    fn compound_pointer_requires_correct_owning_edge() {
        let graph:ObjectGraph=serde_json::from_value(json!({"consumed_bytes":20,"objects":[{"class_tag":1,"class_name":"CompoundStructure","token":0,"start":2,"fields_end":20,"fields":{}}],"edges":[{"source_object_index":1,"pointer_offset":4,"pointer_token":99,"target_object_index":0,"target_class_tag":1}]})).unwrap();
        assert!(target(&graph, 0, &json!({"offset":4,"pointer_token":99})).is_err());
    }
}
