//! File-local Extensible Storage definitions. These are schema declarations,
//! independent of whether an owner has a stored entity or an API default value.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    /// Canonical GUID strings, interpreted in the serialized Windows byte order.
    pub schemas: BTreeMap<String, Schema>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub guid: String,
    pub name: String,
    pub fields: Vec<Field>,
    pub raw_metadata: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub index: u32,
    pub name: String,
    pub type_name: String,
    pub container_type: i32,
    pub subschema_guid: Option<String>,
    pub spec_type_id: Option<String>,
    pub raw_metadata: Value,
}
impl Catalog {
    pub fn insert(&mut self, schema: Schema) -> Result<()> {
        ensure!(
            !self.schemas.contains_key(&schema.guid),
            "duplicate ES schema GUID"
        );
        let mut names = BTreeSet::new();
        let mut indices = BTreeSet::new();
        for field in &schema.fields {
            ensure!(names.insert(&field.name), "duplicate ES field name");
            ensure!(indices.insert(field.index), "duplicate ES field index");
        }
        ensure!(
            indices.iter().copied().eq(0..schema.fields.len() as u32),
            "noncontiguous ES field indices"
        );
        self.schemas.insert(schema.guid.clone(), schema);
        Ok(())
    }
}

/// Format the GUID's little-endian first three components, retaining the
/// remaining eight bytes in their serialized order.
pub fn guid_string(raw: [u8; 16]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes(raw[0..4].try_into().unwrap()),
        u16::from_le_bytes(raw[4..6].try_into().unwrap()),
        u16::from_le_bytes(raw[6..8].try_into().unwrap()),
        raw[8],
        raw[9],
        raw[10],
        raw[11],
        raw[12],
        raw[13],
        raw[14],
        raw[15]
    )
}

#[derive(Debug, Serialize)]
pub struct Entity {
    pub owner: crate::native_index::Identity,
    pub schema_guid: String,
    pub state: &'static str,
    pub source: Value,
    pub cell_object_index: usize,
    pub payload_object_index: Option<usize>,
    pub fields: Vec<Value>,
    pub nested_payloads: Vec<Value>,
    pub payload_edges: Vec<crate::native_parameters::GraphEdge>,
}
#[derive(Debug, Serialize)]
pub struct Owner {
    pub identity: crate::native_index::Identity,
    pub source: Value,
    pub attached_schema_guids: Vec<String>,
    pub state: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_supported_entities: bool,
    pub complete_extensible_storage: bool,
    pub entities: Vec<Entity>,
    pub owners: Vec<Owner>,
    pub diagnostics: Vec<String>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    entities: Vec<Entity>,
    owners: Vec<Owner>,
    diagnostics: Vec<String>,
}
impl InventoryBuilder {
    pub fn ingest(&mut self, record: &crate::native_document::Record) -> Result<()> {
        let Some(graph) = &record.graph else {
            if record
                .diagnostic
                .as_deref()
                .is_some_and(|d| d.contains("ES ") || d.contains("ESEntity"))
            {
                self.diagnostics.push(format!(
                    "owner {}: {}",
                    record.identity.element_id,
                    record.diagnostic.as_deref().unwrap()
                ));
            }
            return Ok(());
        };
        let prior_diagnostics = self.diagnostics.len();
        let mut schemas = BTreeSet::new();
        for (index, cell) in graph
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.class_name == "ESEntityCell")
        {
            let result = (|| -> Result<()> {
                for entry in cell.fields["m_entityMap"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("ES entity map absent"))?
                {
                    let bytes = entry["first"]["m_guid"]["guid_bytes"]
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("ES owner schema key absent"))?;
                    ensure!(bytes.len() == 16, "ES owner schema key width");
                    let mut raw = [0u8; 16];
                    for (out, input) in raw.iter_mut().zip(bytes) {
                        *out = u8::try_from(
                            input
                                .as_u64()
                                .ok_or_else(|| anyhow::anyhow!("ES schema GUID byte"))?,
                        )?;
                    }
                    let guid = guid_string(raw);
                    ensure!(schemas.insert(guid.clone()), "duplicate owner ES schema");
                    let blob = &entry["second"]["m_blob"];
                    let discriminator = blob["discriminator"]
                        .as_i64()
                        .ok_or_else(|| anyhow::anyhow!("ES entity discriminator absent"))?;
                    let (state, payload, fields) = if discriminator == 0 {
                        ("serialized_absent", None, vec![])
                    } else {
                        ensure!(
                            discriminator == -1 && blob["schema_guid"].as_str() == Some(&guid),
                            "ES schema key/payload mismatch"
                        );
                        let offset = blob["offset"]
                            .as_u64()
                            .ok_or_else(|| anyhow::anyhow!("ES pointer offset absent"))?
                            as usize;
                        let edges = graph
                            .edges
                            .iter()
                            .filter(|e| {
                                e.source_object_index == index && e.pointer_offset == offset
                            })
                            .collect::<Vec<_>>();
                        ensure!(edges.len() == 1, "ES payload owning edge ambiguous");
                        let target = edges[0].target_object_index;
                        let object = graph
                            .objects
                            .get(target)
                            .ok_or_else(|| anyhow::anyhow!("ES payload target outside graph"))?;
                        ensure!(
                            object.class_name == "ExtensibleStorageEntity"
                                && object.fields["schema_guid"].as_str() == Some(&guid),
                            "ES dynamic target schema mismatch"
                        );
                        (
                            "serialized_present",
                            Some(target),
                            object.fields["fields"]
                                .as_array()
                                .ok_or_else(|| anyhow::anyhow!("ES dynamic fields absent"))?
                                .clone(),
                        )
                    };
                    let mut reachable = BTreeSet::new();
                    let mut pending = payload.into_iter().collect::<Vec<_>>();
                    while let Some(object) = pending.pop() {
                        if reachable.insert(object) {
                            pending.extend(
                                graph
                                    .edges
                                    .iter()
                                    .filter(|e| e.source_object_index == object)
                                    .map(|e| e.target_object_index),
                            );
                        }
                    }
                    let nested_payloads = graph
                        .objects
                        .iter()
                        .enumerate()
                        .filter(|(i, o)| {
                            reachable.contains(i)
                                && o.class_name == "ExtensibleStorageEntity"
                                && Some(*i) != payload
                        })
                        .map(|(i, o)| serde_json::json!({"object_index":i,"object":o}))
                        .collect();
                    let payload_edges = graph
                        .edges
                        .iter()
                        .filter(|e| reachable.contains(&e.source_object_index))
                        .cloned()
                        .collect();
                    self.entities.push(Entity {
                        owner: record.identity.clone(),
                        schema_guid: guid,
                        state,
                        source: serde_json::to_value(&record.source)?,
                        cell_object_index: index,
                        payload_object_index: payload,
                        fields,
                        nested_payloads,
                        payload_edges,
                    });
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.diagnostics.push(format!(
                    "owner {} ES cell {}: {error:#}",
                    record.identity.element_id, index
                ));
            }
        }
        if record.class_name.as_deref() == Some("DataStorage") || !schemas.is_empty() {
            self.owners.push(Owner {
                identity: record.identity.clone(),
                source: serde_json::to_value(&record.source)?,
                attached_schema_guids: schemas.into_iter().collect(),
                state: if self.diagnostics.len() == prior_diagnostics {
                    "complete_saved_owner_graph"
                } else {
                    "unresolved_saved_entity_map"
                },
            });
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Inventory> {
        Ok(Inventory {
            format: "rvt-native-extensible-storage-v1",
            complete_supported_entities: self.diagnostics.is_empty(),
            complete_extensible_storage: false,
            entities: self.entities,
            owners: self.owners,
            diagnostics: self.diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declaration_order_is_not_field_index_and_duplicates_are_refused() {
        let field = |index, name: &str| Field {
            index,
            name: name.into(),
            type_name: "int".into(),
            container_type: 0,
            subschema_guid: None,
            spec_type_id: None,
            raw_metadata: Value::Null,
        };
        let schema = Schema {
            guid: "test".into(),
            name: "Order".into(),
            fields: vec![field(1, "A"), field(0, "Z")],
            raw_metadata: Value::Null,
        };
        let mut catalog = Catalog::default();
        catalog.insert(schema.clone()).unwrap();
        assert!(catalog.insert(schema).is_err());
        let mut bad = catalog.schemas["test"].clone();
        bad.guid = "other".into();
        bad.fields[0].index = 0;
        assert!(catalog.insert(bad).is_err());
    }
}
