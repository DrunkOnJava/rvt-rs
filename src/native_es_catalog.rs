//! File-local Extensible Storage catalog from the owned Global/Latest graph.
use crate::{
    native_extensible_storage::{Catalog, Field, Schema, guid_string},
    native_parameters::{self, GraphLimits, ObjectGraph},
    schema_registry::Registry,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Global object streams carry a fixed u32 footer after the deferred graph.
/// Only the independently observed zero-footer envelope is qualified here.
pub fn decode(bytes: &[u8], registry: &Registry) -> Result<(Catalog, Value)> {
    ensure!(
        bytes.len() >= 6 && bytes.len() <= 32 * 1024 * 1024,
        "ES global stream size unqualified"
    );
    let end = bytes.len() - 4;
    ensure!(
        bytes[end..] == [0; 4],
        "ES global fixed footer is not the qualified zero value"
    );
    let graph = native_parameters::decode_graph_with_limits(
        &bytes[..end],
        registry,
        &GraphLimits {
            max_values: 2_000_000,
            ..Default::default()
        },
    )?;
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty global graph"))?;
    ensure!(
        root.class_name == "ADocument",
        "ES catalog requires ADocument root"
    );
    let manager = owned(&graph, 0, &root.fields["m_pAppInfoManager"])?;
    ensure!(
        graph.objects[manager].class_name == "AppInfoManager",
        "ES manager class mismatch"
    );
    let pointers = graph.objects[manager].fields["m_appInfoArr"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("ES app info array absent"))?;
    let mut storages = Vec::new();
    for pointer in pointers {
        if pointer["pointer_token"].as_u64() == Some(0) {
            continue;
        }
        let index = owned(&graph, manager, pointer)?;
        if graph.objects[index].class_name == "ESSchemaStorage" {
            storages.push(index);
        }
    }
    ensure!(
        storages.len() == 1,
        "ES schema storage missing or ambiguous"
    );
    let storage = &graph.objects[storages[0]];
    let old = storage.fields.get("m_storedSchemas");
    let current = storage.fields.get("m_schemaUsageMap");
    ensure!(
        old.is_some() ^ current.is_some(),
        "ES schema storage layout missing or ambiguous"
    );
    let usage_map = current.is_some();
    let rows = old
        .or(current)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("ES schema entries absent"))?;
    let mut catalog = Catalog::default();
    for row in rows {
        let s = if usage_map {
            &row["second"]["m_schema"]
        } else {
            &row["second"]
        };
        let guid = guid(&row["first"])?;
        ensure!(
            guid == self::guid(&s["m_guid"])?,
            "ES map key and schema GUID disagree"
        );
        let mut fields = Vec::new();
        for f in s["m_fields"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("ES schema fields absent"))?
        {
            let sub = self::guid(&f["m_subSchemaGUID"])?;
            let spec = string(&f["m_specTypeId"], "m_typeId")?;
            fields.push(Field {
                index: u32::try_from(
                    f["m_entryIndex"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("invalid ES field index"))?,
                )?,
                name: string(f, "m_fieldName")?,
                type_name: string(f, "m_fieldTypeName")?,
                container_type: i32::try_from(
                    f["m_containerType"]
                        .as_i64()
                        .ok_or_else(|| anyhow::anyhow!("invalid ES container type"))?,
                )?,
                subschema_guid: (sub != "00000000-0000-0000-0000-000000000000").then_some(sub),
                spec_type_id: (!spec.is_empty()).then_some(spec),
                raw_metadata: f.clone(),
            });
        }
        catalog.insert(Schema {
            guid,
            name: string(s, "m_schemaName")?,
            fields,
            raw_metadata: {
                let mut metadata = s.clone();
                if usage_map {
                    metadata["_storage_usage"] = row["second"].clone();
                }
                metadata
            },
        })?;
    }
    Ok((
        catalog,
        json!({"stream":"Global/Latest","inflated_sha256":format!("{:x}",Sha256::digest(bytes)),"graph_bytes":end,"stream_bytes":bytes.len(),"fixed_footer":0,"root_class":"ADocument","app_info_object_index":manager,"storage_object_index":storages[0],"storage_start":storage.start,"storage_end":storage.fields_end,"complete_global_graph":true,"scope":"Owned file-local schemas; general nonzero fixed footers remain unqualified"}),
    ))
}
fn string(v: &Value, key: &str) -> Result<String> {
    v[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("ES string {key} absent"))
}
fn guid(v: &Value) -> Result<String> {
    let v = if v.get("guid_bytes").is_some() {
        v
    } else {
        &v["m_guid"]
    };
    let a = v["guid_bytes"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("ES GUID bytes absent"))?;
    ensure!(a.len() == 16, "ES GUID width");
    let mut b = [0; 16];
    for (dst, src) in b.iter_mut().zip(a) {
        *dst = u8::try_from(
            src.as_u64()
                .ok_or_else(|| anyhow::anyhow!("ES GUID byte invalid"))?,
        )?;
    }
    Ok(guid_string(b))
}
fn owned(g: &ObjectGraph, owner: usize, pointer: &Value) -> Result<usize> {
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("ES ownership pointer absent"))?;
    let edges: Vec<_> = g
        .edges
        .iter()
        .filter(|e| e.source_object_index == owner && e.pointer_offset as u64 == offset)
        .collect();
    ensure!(edges.len() == 1, "ES owned pointer ambiguous or unresolved");
    let e = edges[0];
    ensure!(
        e.target_object_index < g.objects.len(),
        "ES target outside graph"
    );
    Ok(e.target_object_index)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guid_width_and_bytes_are_strict() {
        assert_eq!(guid(&json!({"m_guid":{"guid_bytes":[43,42,2,27,34,87,135,71,146,138,79,41,202,157,136,17]}})).unwrap(),"1b022a2b-5722-4787-928a-4f29ca9d8811");
        assert!(guid(&json!({"guid_bytes":vec![256;16]})).is_err());
    }
}
