//! Content-local current element tables from Global/ContentDocuments.
//! Namespace-local identities are not project or companion-RFA UniqueIds.
use crate::{
    native_metadata::identifier,
    native_parameters::{self, GraphLimits, ObjectGraph},
    schema_registry::Registry,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize)]
pub struct LocalIdentity {
    pub local_element_id: u64,
    pub original_id_suffix: u64,
    pub creation_episode: i64,
    pub stored_revision: i64,
    pub other_revision: i64,
    pub owning_element_id: i64,
    pub partition_id: i64,
    pub row_index: usize,
    pub raw_fields: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct TableSource {
    pub stream: &'static str,
    pub content_marker_offset: usize,
    pub document_body_offset: usize,
    pub document_body_bytes: usize,
    pub document_body_sha256: String,
    pub element_table_object: usize,
    pub element_table_offset: usize,
    pub root_pointer_field: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct ContentDocument {
    pub content_key_raw_hex: String,
    pub owner_family_local_id: i64,
    pub identities: BTreeMap<u64, LocalIdentity>,
    pub graveyard_rows: Vec<Value>,
    pub local_history_pointer_present: bool,
    pub source: TableSource,
}
#[derive(Debug, Clone, Serialize)]
pub struct Catalog {
    pub source_sha256: String,
    pub schema_sha256: String,
    pub documents: BTreeMap<String, ContentDocument>,
}
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| anyhow::anyhow!("content document cursor overflow"))?;
        let v = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| anyhow::anyhow!("truncated content document at {}", self.pos))?;
        self.pos = end;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
}
/// Parse exact marker/body/check-length framing, then follow ADocument.m_elemTable.
/// All graph bytes must decode; unsupported layouts fail rather than selecting records.
pub fn parse(bytes: &[u8], registry: &Registry, max_graph_values: usize) -> Result<Catalog> {
    ensure!(max_graph_values > 0, "zero content graph budget");
    let marker = registry
        .named("ContentMarker")
        .ok_or_else(|| anyhow::anyhow!("ContentMarker schema absent"))?
        .tag;
    let key_tag = registry
        .named("ContentKey")
        .ok_or_else(|| anyhow::anyhow!("ContentKey schema absent"))?
        .tag;
    let mut cursor = Cursor { bytes, pos: 0 };
    let mut documents = BTreeMap::new();
    loop {
        let marker_offset = cursor.pos;
        ensure!(
            cursor.u16()? == marker,
            "content document marker class mismatch"
        );
        let token = cursor.u32()?;
        if token == 0 {
            ensure!(
                cursor.u32()? == u32::MAX && cursor.take(4)? == [0; 4] && cursor.pos == bytes.len(),
                "invalid content document terminal"
            );
            break;
        }
        ensure!(
            token == u32::MAX && cursor.u16()? == key_tag,
            "unsupported content document key pointer"
        );
        ensure!(
            cursor.u32()? == u32::MAX,
            "unsupported content document count marker"
        );
        let key: String = cursor
            .take(16)?
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let size = cursor.u32()? as usize;
        ensure!(
            size <= 256 * 1024 * 1024,
            "content document body budget exceeded"
        );
        let body_offset = cursor.pos;
        let body = cursor.take(size)?;
        ensure!(
            cursor.u32()? as usize == size,
            "content document dual lengths differ"
        );
        let graph = native_parameters::decode_graph_with_limits(
            body,
            registry,
            &GraphLimits {
                max_values: max_graph_values,
                ..Default::default()
            },
        )?;
        let document = project(
            &key,
            &graph,
            TableSource {
                stream: "Global/ContentDocuments",
                content_marker_offset: marker_offset,
                document_body_offset: body_offset,
                document_body_bytes: size,
                document_body_sha256: format!("{:x}", Sha256::digest(body)),
                element_table_object: 0,
                element_table_offset: 0,
                root_pointer_field: "ADocument.m_elemTable",
            },
        )?;
        ensure!(
            documents.insert(key, document).is_none(),
            "duplicate content namespace catalog"
        );
    }
    Ok(Catalog {
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
        schema_sha256: registry.source_sha256.clone(),
        documents,
    })
}
fn project(key: &str, graph: &ObjectGraph, mut source: TableSource) -> Result<ContentDocument> {
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty content document graph"))?;
    ensure!(
        root.class_name == "ADocument",
        "unsupported content document root"
    );
    let pointer = &root.fields["m_elemTable"];
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("content element table pointer offset absent"))?
        as usize;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == 0 && e.pointer_offset == offset)
        .collect();
    ensure!(
        edges.len() == 1,
        "content element table requires exactly one owner edge"
    );
    let index = edges[0].target_object_index;
    let table = graph
        .objects
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("content element table outside graph"))?;
    ensure!(
        table.class_name == "ElemTable",
        "content element table target class mismatch"
    );
    source.element_table_object = index;
    source.element_table_offset = table.start;
    let mut identities = BTreeMap::new();
    for (row_index, row) in table.fields["m_elemArr"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("content current element array absent"))?
        .iter()
        .enumerate()
    {
        let local_element_id = u64::try_from(identifier(&row["m_id"])?)?;
        let history = &row["m_history"];
        let history_date = |field: &str| -> Result<i64> {
            history[field]["m_id"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("content history date absent"))
        };
        let identity = LocalIdentity {
            local_element_id,
            original_id_suffix: u64::try_from(identifier(&history["m_originalElementId"])?)?,
            creation_episode: history_date("m_creationDate")?,
            stored_revision: history_date("m_lastModificationDate")?,
            other_revision: history_date("m_lastUserModificationDate")?,
            owning_element_id: identifier(&row["m_OwningElementId"])?,
            partition_id: row["m_partitionId"]["m_id"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("content partition ID absent"))?,
            row_index,
            raw_fields: row.clone(),
        };
        ensure!(
            identity.creation_episode >= 0 && identity.stored_revision >= 0,
            "negative content identity revision"
        );
        ensure!(
            identities.insert(local_element_id, identity).is_none(),
            "duplicate current content-local element ID"
        );
    }
    let graveyard_rows = table.fields["m_graveyardRecs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("content graveyard array absent"))?
        .clone();
    Ok(ContentDocument {
        content_key_raw_hex: key.into(),
        owner_family_local_id: identifier(&root.fields["m_ownerFamilyId"])?,
        identities,
        graveyard_rows,
        local_history_pointer_present: root.fields["m_pHistory"]["pointer_token"]
            .as_u64()
            .is_some_and(|v| v != 0),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn graph(ids: &[u64], edge_owner: usize) -> ObjectGraph {
        let rows:Vec<_>=ids.iter().map(|id|json!({"m_id":{"m_id":{"m_id64":id}},"m_history":{"m_originalElementId":{"m_id64":70},"m_creationDate":{"m_id":0},"m_lastModificationDate":{"m_id":4},"m_lastUserModificationDate":{"m_id":-1}},"m_OwningElementId":{"m_id":{"m_id64":-1}},"m_partitionId":{"m_id":0}})).collect();
        serde_json::from_value(json!({"consumed_bytes":100,"objects":[{"class_tag":12,"class_name":"ADocument","token":0,"start":2,"fields_end":20,"fields":{"m_elemTable":{"pointer_token":4294967295u32,"offset":2},"m_ownerFamilyId":{"m_id":{"m_id64":3}},"m_pHistory":{"pointer_token":0}}},{"class_tag":13,"class_name":"ElemTable","token":4294967295u32,"start":20,"fields_end":100,"fields":{"m_elemArr":rows,"m_graveyardRecs":[]}}],"edges":[{"source_object_index":edge_owner,"pointer_offset":2,"pointer_token":4294967295u32,"target_object_index":1,"target_class_tag":13}]})).unwrap()
    }
    fn source() -> TableSource {
        TableSource {
            stream: "Global/ContentDocuments",
            content_marker_offset: 0,
            document_body_offset: 32,
            document_body_bytes: 100,
            document_body_sha256: "synthetic".into(),
            element_table_object: 0,
            element_table_offset: 0,
            root_pointer_field: "ADocument.m_elemTable",
        }
    }
    #[test]
    fn current_table_keys_are_local_ids_not_original_suffixes() {
        let document = project("namespace", &graph(&[7, 8], 0), source()).unwrap();
        assert_eq!(document.identities.len(), 2);
        assert_eq!(document.identities[&8].original_id_suffix, 70);
        assert_eq!(document.identities[&8].row_index, 1);
        assert_eq!(document.source.element_table_object, 1);
        assert_eq!(document.identities[&7].other_revision, -1);
        assert!(project("namespace", &graph(&[7, 7], 0), source()).is_err());
    }
    #[test]
    fn wrong_owning_pointer_cannot_supply_a_content_current_table() {
        assert!(project("namespace", &graph(&[7], 1), source()).is_err());
    }
}
