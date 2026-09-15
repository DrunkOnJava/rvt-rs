//! Schema-directed native value prefixes and deferred parameter sets.
//! This decoder requires an independently bounded body. IDs remain serialized
//! references until the caller establishes the file's current identity mapping.
//! Object field keys are bare schema names unless the inheritance chain has
//! multiple serialized declarations of that name; those keys are all emitted
//! as `DeclaringClass::field`. Inline objects have independent naming scopes.
use crate::schema_registry::{Field, Registry};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub serialized_parameter_id: i64,
    pub storage_type: String,
    pub raw_value: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub serialized_element_id: i64,
    pub class_name: String,
    pub derived_fields_end: usize,
    pub parameter_sets_end: usize,
    pub parameters: Vec<Parameter>,
    pub fields: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpan {
    pub class_tag: u16,
    pub name: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectFields {
    pub field_spans: Vec<FieldSpan>,
    pub class_tag: u16,
    pub class_name: String,
    pub start: usize,
    pub end: usize,
    pub fields: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphObject {
    pub class_tag: u16,
    pub class_name: String,
    pub token: u32,
    pub start: usize,
    pub fields_end: usize,
    pub fields: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectGraph {
    pub consumed_bytes: usize,
    pub objects: Vec<GraphObject>,
    pub edges: Vec<GraphEdge>,
}
/// A non-null, non-external serialized pointer resolved within this bounded
/// owning body. Object indices refer to `ObjectGraph::objects`; pointer offsets
/// are absolute byte offsets within the body, including pointers inside inline
/// objects and containers. Null and external references remain in field values
/// but have no local object edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source_object_index: usize,
    pub pointer_offset: usize,
    pub pointer_token: u32,
    pub target_object_index: usize,
    pub target_class_tag: u16,
}
/// Implementation resource budgets, not limits of the RVT format.
#[derive(Debug, Clone, Copy)]
pub struct GraphLimits {
    pub max_values: usize,
    pub max_objects: usize,
    pub max_depth: usize,
}
#[derive(Debug, Clone, Copy, Serialize)]
pub struct GraphUsage {
    pub values: usize,
    pub objects: usize,
}
impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_values: 100_000,
            max_objects: 100_000,
            max_depth: 128,
        }
    }
}
/// Experimental strict deferred-object traversal. Returns success only when the
/// entire bounded body is consumed. Pointer sharing is scoped to this body.
pub fn decode_graph(body: &[u8], registry: &Registry) -> Result<ObjectGraph> {
    decode_graph_with_limits(body, registry, &GraphLimits::default())
}
/// Decode with explicit positive resource budgets. Per-container and string
/// allocation checks also remain enforced independently of these graph budgets.
pub fn decode_graph_with_limits(
    body: &[u8],
    registry: &Registry,
    limits: &GraphLimits,
) -> Result<ObjectGraph> {
    decode_graph_with_catalog(body, registry, limits, None)
}
/// Decode file-local dynamic entities when their saved schema catalog is known.
/// Dynamic nodes use class tag zero and an explicit schema GUID, never a
/// fabricated Formats/Latest class identity.
pub fn decode_graph_with_catalog(
    body: &[u8],
    registry: &Registry,
    limits: &GraphLimits,
    catalog: Option<&crate::native_extensible_storage::Catalog>,
) -> Result<ObjectGraph> {
    decode_graph_with_catalog_usage(body, registry, limits, catalog).map(|(graph, _)| graph)
}
/// Decode a graph and return the actual bounded cursor usage for reporting.
pub fn decode_graph_with_catalog_usage(
    body: &[u8],
    registry: &Registry,
    limits: &GraphLimits,
    catalog: Option<&crate::native_extensible_storage::Catalog>,
) -> Result<(ObjectGraph, GraphUsage)> {
    ensure!(
        limits.max_values > 0 && limits.max_objects > 0 && limits.max_depth > 0,
        "native graph resource budgets must be positive"
    );
    ensure!(body.len() >= 2, "truncated root object tag");
    let tag = u16::from_le_bytes(body[..2].try_into()?);
    let mut cursor = Cursor {
        body,
        registry,
        catalog,
        pos: 2,
        items: 0,
        pending: Vec::new(),
        spans: Vec::new(),
        limits: *limits,
    };
    let mut objects = Vec::new();
    let mut edges = Vec::new();
    let mut shared = BTreeMap::<u32, (usize, u16)>::new();
    let mut queue = VecDeque::from([(0, tag, None::<String>)]);
    while let Some((token, tag, schema_guid)) = queue.pop_front() {
        ensure!(
            objects.len() < limits.max_objects,
            "native graph object budget exceeded"
        );
        let start = cursor.pos;
        let pending_start = cursor.pending.len();
        let (name, fields) = if let Some(guid) = &schema_guid {
            (
                "ExtensibleStorageEntity".to_string(),
                cursor.entity_fields(guid)?,
            )
        } else {
            let name = registry
                .class(tag)
                .ok_or_else(|| anyhow::anyhow!("graph class absent"))?
                .name
                .clone();
            let fields = cursor
                .class(tag, 0)
                .map_err(|e| anyhow::anyhow!("graph {name} starts{start}: {e}"))?;
            (name, fields)
        };
        let source_object_index = objects.len();
        objects.push(GraphObject {
            class_tag: tag,
            class_name: name,
            token,
            start,
            fields_end: cursor.pos,
            fields,
        });
        for pointer in &cursor.pending[pending_start..] {
            let target_object_index = if let Some(&(index, tag)) = shared.get(&pointer.token) {
                ensure!(
                    tag == pointer.class_tag,
                    "shared native pointer token {} at {} changes class from {} to {}",
                    pointer.token,
                    pointer.offset,
                    tag,
                    pointer.class_tag
                );
                index
            } else {
                // FIFO discovery order is the eventual object-array order.
                // Register before reading the child so shared cycles resolve.
                let index = objects.len() + queue.len();
                ensure!(
                    index < limits.max_objects,
                    "native graph object budget exceeded"
                );
                queue.push_back((
                    pointer.token,
                    pointer.class_tag,
                    pointer.schema_guid.clone(),
                ));
                if pointer.token != u32::MAX {
                    shared.insert(pointer.token, (index, pointer.class_tag));
                }
                index
            };
            edges.push(GraphEdge {
                source_object_index,
                pointer_offset: pointer.offset,
                pointer_token: pointer.token,
                target_object_index,
                target_class_tag: pointer.class_tag,
            });
        }
    }
    ensure!(
        cursor.pos == body.len(),
        "native graph leaves {} unconsumed bytes",
        body.len() - cursor.pos
    );
    let usage = GraphUsage {
        values: cursor.items,
        objects: objects.len(),
    };
    Ok((
        ObjectGraph {
            consumed_bytes: cursor.pos,
            objects,
            edges,
        },
        usage,
    ))
}
/// Decode the field sequence of a caller-established inline/deferred object.
/// The caller supplies the bounded owning body and exact offset/class; this
/// routine does not locate objects or resolve pointer tokens automatically.
pub fn decode_object_fields(
    body: &[u8],
    registry: &Registry,
    start: usize,
    class_tag: u16,
) -> Result<ObjectFields> {
    ensure!(
        start <= body.len(),
        "native object offset outside bounded body"
    );
    let class_name = registry
        .class(class_tag)
        .ok_or_else(|| anyhow::anyhow!("native object class absent"))?
        .name
        .clone();
    let mut cursor = Cursor {
        body,
        registry,
        catalog: None,
        pos: start,
        items: 0,
        pending: Vec::new(),
        spans: Vec::new(),
        limits: GraphLimits::default(),
    };
    let fields = cursor.class(class_tag, 0)?;
    Ok(ObjectFields {
        class_tag,
        class_name,
        start,
        end: cursor.pos,
        fields,
        field_spans: cursor.spans,
    })
}
struct PendingPointer {
    token: u32,
    class_tag: u16,
    offset: usize,
    schema_guid: Option<String>,
}
struct Cursor<'a> {
    body: &'a [u8],
    registry: &'a Registry,
    catalog: Option<&'a crate::native_extensible_storage::Catalog>,
    pos: usize,
    items: usize,
    pending: Vec<PendingPointer>,
    spans: Vec<FieldSpan>,
    limits: GraphLimits,
}
impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| anyhow::anyhow!("native value size overflow"))?;
        let b = self
            .body
            .get(self.pos..end)
            .ok_or_else(|| anyhow::anyhow!("truncated native field at {}", self.pos))?;
        self.pos = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn pointer(&mut self, external: bool) -> Result<Value> {
        let offset = self.pos;
        let token = self.u32()?;
        let tag = if token != 0 && !external {
            let tag = u16::from_le_bytes(self.take(2)?.try_into()?);
            ensure!(
                self.registry.class(tag).is_some(),
                "unknown native pointer class {tag}"
            );
            self.pending.push(PendingPointer {
                token,
                class_tag: tag,
                offset,
                schema_guid: None,
            });
            Some(tag)
        } else {
            None
        };
        Ok(json!({"pointer_token":token,"class_tag":tag,"offset":offset}))
    }
    fn entity_pointer(&mut self) -> Result<Value> {
        let offset = self.pos;
        let discriminator = i32::from_le_bytes(self.take(4)?.try_into()?);
        ensure!(
            matches!(discriminator, 0 | -1),
            "unqualified ES entity discriminator {discriminator}"
        );
        if discriminator == 0 {
            return Ok(json!({"discriminator":0,"state":"serialized_absent","offset":offset}));
        }
        let raw: [u8; 16] = self.take(16)?.try_into()?;
        let guid = crate::native_extensible_storage::guid_string(raw);
        ensure!(
            self.catalog.is_some_and(|c| c.schemas.contains_key(&guid)),
            "ES entity schema catalog absent for {guid}"
        );
        self.pending.push(PendingPointer {
            token: u32::MAX,
            class_tag: 0,
            offset,
            schema_guid: Some(guid.clone()),
        });
        Ok(
            json!({"discriminator":-1,"schema_guid":guid,"state":"serialized_present","offset":offset}),
        )
    }
    fn entity_fields(&mut self, guid: &str) -> Result<Value> {
        let schema = self
            .catalog
            .and_then(|c| c.schemas.get(guid))
            .ok_or_else(|| anyhow::anyhow!("ES schema unavailable"))?
            .clone();
        let mut fields = schema.fields.clone();
        fields.sort_by_key(|f| f.index);
        ensure!(
            fields
                .iter()
                .enumerate()
                .all(|(i, f)| f.index as usize == i),
            "ES field indices are not unique and contiguous"
        );
        let mut values = Vec::new();
        for field in fields {
            let start = self.pos;
            let raw_value = self.es_value(&field.type_name, field.container_type)?;
            let (serialized_unit_type_id, unit_status) = match field.spec_type_id.as_deref() {
                Some("autodesk.spec.aec:length-2.0.1") => (
                    Some("autodesk.unit.unit:millimeters-1.0.1"),
                    "qualified_es_default_metric_length",
                ),
                None | Some("") => (None, "no_saved_spec"),
                _ => (None, "unqualified_spec_storage_units"),
            };
            values.push(json!({"index":field.index,"name":field.name,"type_name":field.type_name,"container_type":field.container_type,"spec_type_id":field.spec_type_id,"subschema_guid":field.subschema_guid,"start":start,"end":self.pos,"raw_value":raw_value,"value_semantics":"serialized_value","serialized_unit_type_id":serialized_unit_type_id,"unit_status":unit_status}));
        }
        Ok(json!({"schema_guid":guid,"schema_name":schema.name,"fields":values}))
    }
    fn es_value(&mut self, name: &str, container: i32) -> Result<Value> {
        ensure!(
            matches!(container, 0..=2),
            "unqualified ES container {container}"
        );
        let (base, mut modifier) = match name {
            "bool" => (1, 0),
            "char" => (2, 0),
            "short" => (3, 0),
            "int" => (4, 0),
            "int64_t" => (11, 0),
            "float" => (6, 0),
            "double" => (7, 0),
            "TCHAR" => (8, 0x60),
            _ => (14, 0),
        };
        let references = if base == 14 {
            let class = self.registry.named(name).ok_or_else(|| {
                anyhow::anyhow!("ES type absent from saved Formats registry: {name}")
            })?;
            vec![crate::schema_registry::Reference {
                offset: 0,
                tag: class.tag,
                introduces_definition: false,
            }]
        } else {
            vec![]
        };
        if container != 0 {
            ensure!(
                name != "TCHAR",
                "ES character-string array needs explicit element wrapper"
            );
            if container == 2 {
                ensure!(
                    name.starts_with("std::pair<"),
                    "ES map element lacks pair declaration"
                );
            }
            modifier = 0x50;
        }
        self.field(
            &Field {
                offset: 0,
                name: name.into(),
                descriptor_offset: 0,
                raw_descriptor: 0,
                base,
                modifier,
                uninterpreted_flags: 0,
                array_count: None,
                references,
                nested_descriptor: None,
                end: 0,
            },
            0,
        )
    }
    fn class(&mut self, tag: u16, depth: usize) -> Result<Value> {
        ensure!(
            depth < self.limits.max_depth,
            "native class recursion budget exceeded"
        );
        let mut lineage = Vec::new();
        let mut current = tag;
        loop {
            ensure!(
                depth + lineage.len() < self.limits.max_depth,
                "native class recursion budget exceeded"
            );
            let c = self
                .registry
                .class(current)
                .ok_or_else(|| anyhow::anyhow!("unknown inline native class {current}"))?
                .clone();
            current = c.parent_reference.tag;
            lineage.push(c);
            if current < 12 {
                break;
            }
        }
        // Field names are scoped to their declaring class in the schema. Keep
        // convenient bare keys where unambiguous, but qualify every occurrence
        // of an inherited collision (including the base field). Counting before
        // decoding also handles three or more classes shadowing the same name.
        let mut names = BTreeMap::new();
        for c in &lineage {
            for f in &c.fields {
                if f.uninterpreted_flags & 2 == 0 {
                    *names.entry(f.name.as_str()).or_insert(0usize) += 1;
                }
            }
        }
        let mut values = Map::new();
        for (distance, c) in lineage.iter().enumerate().rev() {
            for f in &c.fields {
                // Measured VWall transient flags; other flags remain unsupported.
                if f.uninterpreted_flags & 2 != 0 {
                    continue;
                }
                ensure!(
                    f.uninterpreted_flags == 0
                        || (c.name == "VarExpr"
                            && f.name == "m_refCt"
                            && f.uninterpreted_flags == 4
                            && f.base == 4
                            && f.modifier == 0),
                    "unsupported native field flags {}.{}: {}",
                    c.name,
                    f.name,
                    f.uninterpreted_flags
                );
                let start = self.pos;
                let value = if c.name == "ClassDefinitionRef"
                    && f.name == "m_ref"
                    && f.base == 10
                    && f.modifier == 0
                {
                    // ClassDefinitionRef stores a 16-bit schema class tag. The
                    // qualified external reader performs Int16Reader -> ClassesContainer.find.
                    let class_tag = u16::from_le_bytes(self.take(2)?.try_into()?);
                    json!({"class_tag":class_tag,"class_name":self.registry.class(class_tag).map(|class|class.name.as_str())})
                } else if c.name == "ESEntity"
                    && f.name == "m_blob"
                    && f.base == 14
                    && f.modifier == 1
                {
                    self.entity_pointer()?
                } else if c.name == "VarSketchObj"
                    && f.name == "m_params"
                    && f.base == 14
                    && f.modifier == 0x54
                {
                    // Qualified typeless owning-pointer vector: each entry still
                    // carries its concrete VarParam class tag in the wire data.
                    let mut qualified = f.clone();
                    qualified.modifier = 0x51;
                    self.field(&qualified, depth + distance + 1)?
                } else {
                    self.field(f, depth + distance + 1).map_err(|e| {
                        anyhow::anyhow!("{}.{} at {}: {e}", c.name, f.name, self.pos)
                    })?
                };
                self.spans.push(FieldSpan {
                    class_tag: c.tag,
                    name: f.name.clone(),
                    start,
                    end: self.pos,
                });
                let key = if names[f.name.as_str()] > 1 {
                    format!("{}::{}", c.name, f.name)
                } else {
                    f.name.clone()
                };
                ensure!(
                    !values.contains_key(&key),
                    "ambiguous native field key {key} in declaring class {} (tag {})",
                    c.name,
                    c.tag
                );
                values.insert(key, value);
            }
        }
        Ok(Value::Object(values))
    }
    fn field(&mut self, f: &Field, depth: usize) -> Result<Value> {
        ensure!(
            depth < self.limits.max_depth,
            "native field recursion budget exceeded ({})",
            self.limits.max_depth
        );
        ensure!(
            self.items < self.limits.max_values,
            "native field value budget exceeded ({})",
            self.limits.max_values
        );
        self.items += 1;
        match f.modifier {
            1..=3 if f.base == 14 => self.pointer(f.modifier == 3),
            0x60 if f.base == 8 => {
                let n = self.u32()? as usize;
                ensure!(n <= 1_000_000, "native string unit budget exceeded");
                let units = self
                    .take(n * 2)?
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>();
                Ok(Value::String(String::from_utf16(&units)?))
            }
            0x10 | 0x11 | 0x12 | 0x13 | 0x50 | 0x51 | 0x52 | 0x53 => {
                let n = if f.modifier < 0x50 {
                    f.array_count
                        .ok_or_else(|| anyhow::anyhow!("fixed array schema count absent"))?
                } else {
                    self.u32()?
                };
                ensure!(n <= 100_000, "native container count budget exceeded");
                let mut item = f.clone();
                item.modifier &= 0xf;
                item.array_count = None;
                let mut values = Vec::new();
                for _ in 0..n {
                    values.push(self.field(&item, depth + 1)?);
                }
                Ok(Value::Array(values))
            }
            0 => match f.base {
                1 => {
                    let b = self.take(1)?[0];
                    ensure!(b <= 1, "invalid native bool");
                    Ok(json!(b != 0))
                }
                2 => Ok(json!(self.take(1)?[0])),
                3 => Ok(json!(i16::from_le_bytes(self.take(2)?.try_into()?))),
                4 => Ok(json!(i32::from_le_bytes(self.take(4)?.try_into()?))),
                5 => Ok(json!(self.u32()?)),
                6 => {
                    let n = f32::from_le_bytes(self.take(4)?.try_into()?);
                    ensure!(n.is_finite(), "nonfinite native float");
                    Ok(json!(n))
                }
                7 => {
                    let n = f64::from_le_bytes(self.take(8)?.try_into()?);
                    ensure!(n.is_finite(), "nonfinite native double");
                    Ok(json!(n))
                }
                9 => Ok(json!({"guid_bytes":self.take(16)?})),
                11 => Ok(json!(i64::from_le_bytes(self.take(8)?.try_into()?))),
                13 => self.field(
                    f.nested_descriptor
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("nested schema absent"))?,
                    depth + 1,
                ),
                14 => {
                    ensure!(
                        f.references.len() == 1,
                        "inline native reference schema count"
                    );
                    self.class(f.references[0].tag, depth + 1)
                }
                _ => anyhow::bail!("unsupported native scalar {}", f.base),
            },
            _ => anyhow::bail!("unsupported native modifier {}", f.modifier),
        }
    }
}
fn identifier(value: &Value) -> Result<i64> {
    value
        .get("m_id")
        .and_then(|v| v.get("m_id64"))
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("unsupported serialized ElementId value"))
}
/// Decode scalar parameter sets after the schema-directed derived-field prefix.
/// This is not complete record decoding: subsequent geometry and graph objects
/// remain unconsumed. Callers must separately establish current record ownership.
pub fn decode(body: &[u8], registry: &Registry) -> Result<Parameters> {
    let base = crate::native_element::decode_base(body, registry)?;
    let mut cursor = Cursor {
        body,
        registry,
        catalog: None,
        pos: 2,
        items: 0,
        pending: Vec::new(),
        spans: Vec::new(),
        limits: GraphLimits::default(),
    };
    let fields = cursor.class(base.class_tag, 0)?;
    let derived_fields_end = cursor.pos;
    let mut parameters = Vec::new();
    for (name, class_name, storage) in [
        ("m_pParamValueSetDouble", "ParamValueSetDouble", "Double"),
        ("m_pParamValueSetInt", "ParamValueSetInt", "Integer"),
        ("m_pParamValueSetAString", "ParamValueSetAString", "String"),
        (
            "m_pParamValueSetElementId",
            "ParamValueSetElementId",
            "ElementId",
        ),
    ] {
        let pointer = &fields[name];
        let token = pointer["pointer_token"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("missing parameter pointer"))?;
        if token == 0 {
            continue;
        }
        ensure!(
            token == u64::from(u32::MAX),
            "shared/referenced parameter set requires object-graph resolution"
        );
        let tag = pointer["class_tag"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("parameter set class absent"))? as u16;
        ensure!(
            registry.class(tag).is_some_and(|c| c.name == class_name),
            "parameter set class mismatch"
        );
        let set = cursor.class(tag, 0)?;
        let pairs = set["m_paramSet"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("parameter pair container absent"))?;
        for pair in pairs {
            let id = identifier(&pair["m_paramId"])?;
            let raw = pair
                .get("m_value")
                .ok_or_else(|| anyhow::anyhow!("parameter value missing"))?;
            let raw_value = if storage == "ElementId" {
                json!(identifier(raw)?)
            } else {
                raw.clone()
            };
            parameters.push(Parameter {
                serialized_parameter_id: id,
                storage_type: storage.into(),
                raw_value,
            });
        }
    }
    Ok(Parameters {
        serialized_element_id: base.id,
        class_name: base.class_name,
        derived_fields_end,
        parameter_sets_end: cursor.pos,
        parameters,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_registry::{Class, Reference};

    fn field(name: &str) -> Field {
        Field {
            offset: 0,
            name: name.into(),
            descriptor_offset: 0,
            raw_descriptor: 5,
            base: 5,
            modifier: 0,
            uninterpreted_flags: 0,
            array_count: None,
            references: Vec::new(),
            nested_descriptor: None,
            end: 0,
        }
    }

    fn class(tag: u16, name: &str, parent: u16, fields: Vec<Field>) -> Class {
        Class {
            tag,
            name: name.into(),
            offset: 0,
            parent_reference: Reference {
                offset: 0,
                tag: parent,
                introduces_definition: false,
            },
            version_like_word: 0,
            fields,
            opaque_16byte_entry_count: 0,
            opaque_entries_offset: 0,
            opaque_entries_sha256: String::new(),
            end: 0,
        }
    }

    fn registry(classes: Vec<Class>) -> Registry {
        Registry {
            source_sha256: String::new(),
            consumed_bytes: 0,
            reference_count: 0,
            terminator_offset: 0,
            classes,
        }
    }

    #[test]
    fn es_guid_payload_is_deferred_without_fabricated_class_tag() {
        let registry = registry(vec![class(
            12,
            "ESEntity",
            0,
            vec![pointer_field("m_blob")],
        )]);
        let mut body = 12u16.to_le_bytes().to_vec();
        body.extend((-1i32).to_le_bytes());
        body.extend([0u8; 16]);
        body.extend(2u32.to_le_bytes());
        body.extend([b'O', 0, b'K', 0]);
        let guid = crate::native_extensible_storage::guid_string([0; 16]);
        let mut catalog = crate::native_extensible_storage::Catalog::default();
        catalog
            .insert(crate::native_extensible_storage::Schema {
                guid: guid.clone(),
                name: "Example".into(),
                fields: vec![crate::native_extensible_storage::Field {
                    index: 0,
                    name: "Label".into(),
                    type_name: "TCHAR".into(),
                    container_type: 0,
                    subschema_guid: None,
                    spec_type_id: None,
                    raw_metadata: Value::Null,
                }],
                raw_metadata: Value::Null,
            })
            .unwrap();
        assert!(
            decode_graph(&body, &registry)
                .unwrap_err()
                .to_string()
                .contains("schema catalog absent")
        );
        let graph =
            decode_graph_with_catalog(&body, &registry, &Default::default(), Some(&catalog))
                .unwrap();
        assert_eq!(graph.consumed_bytes, body.len());
        assert_eq!(graph.objects[1].class_tag, 0);
        assert_eq!(
            graph.objects[1].fields["fields"][0]["raw_value"],
            json!("OK")
        );
        body.push(0);
        assert!(
            decode_graph_with_catalog(&body, &registry, &Default::default(), Some(&catalog))
                .unwrap_err()
                .to_string()
                .contains("unconsumed")
        );
        let mut absent = 12u16.to_le_bytes().to_vec();
        absent.extend([0; 4]);
        let graph = decode_graph(&absent, &registry).unwrap();
        assert_eq!(
            graph.objects[0].fields["m_blob"]["state"],
            json!("serialized_absent")
        );
        assert!(graph.edges.is_empty());
    }

    fn pointer_field(name: &str) -> Field {
        Field {
            base: 14,
            modifier: 1,
            raw_descriptor: 0x10e,
            ..field(name)
        }
    }

    fn append_pointer(body: &mut Vec<u8>, token: u32, tag: u16) {
        body.extend(token.to_le_bytes());
        body.extend(tag.to_le_bytes());
    }

    #[test]
    fn graph_edges_distinguish_shared_and_fresh_same_class_objects() {
        let registry = registry(vec![
            class(
                12,
                "Root",
                0,
                vec![
                    pointer_field("shared_a"),
                    pointer_field("shared_b"),
                    pointer_field("fresh_a"),
                    pointer_field("fresh_b"),
                ],
            ),
            class(13, "Child", 0, vec![field("value")]),
        ]);
        let mut body = 12u16.to_le_bytes().to_vec();
        for token in [7, 7, u32::MAX, u32::MAX] {
            append_pointer(&mut body, token, 13);
        }
        for value in [10u32, 20, 30] {
            body.extend(value.to_le_bytes());
        }
        let graph = decode_graph(&body, &registry).unwrap();
        assert_eq!(graph.consumed_bytes, body.len());
        assert_eq!(graph.objects.len(), 4);
        let edges: Vec<_> = graph
            .edges
            .iter()
            .map(|edge| {
                (
                    edge.source_object_index,
                    edge.pointer_offset,
                    edge.pointer_token,
                    edge.target_object_index,
                    edge.target_class_tag,
                )
            })
            .collect();
        assert_eq!(
            edges,
            vec![
                (0, 2, 7, 1, 13),
                (0, 8, 7, 1, 13),
                (0, 14, u32::MAX, 2, 13),
                (0, 20, u32::MAX, 3, 13),
            ]
        );
        for (index, value) in [(1, 10), (2, 20), (3, 30)] {
            assert_eq!(graph.objects[index].fields["value"], value);
        }
    }

    #[test]
    fn graph_edges_resolve_a_cycle_without_redecoding_objects() {
        let registry = registry(vec![
            class(12, "Root", 0, vec![pointer_field("head")]),
            class(13, "Node", 0, vec![pointer_field("next")]),
        ]);
        let mut body = 12u16.to_le_bytes().to_vec();
        for token in [7, 8, 7] {
            append_pointer(&mut body, token, 13);
        }
        let graph = decode_graph(&body, &registry).unwrap();
        assert_eq!(graph.objects.len(), 3);
        assert_eq!(graph.consumed_bytes, body.len());
        let edges: Vec<_> = graph
            .edges
            .iter()
            .map(|edge| {
                (
                    edge.source_object_index,
                    edge.target_object_index,
                    edge.pointer_offset,
                )
            })
            .collect();
        assert_eq!(edges, vec![(0, 1, 2), (1, 2, 8), (2, 1, 14)]);
    }

    #[test]
    fn graph_refuses_shared_token_with_conflicting_class() {
        let registry = registry(vec![
            class(12, "Root", 0, vec![pointer_field("a"), pointer_field("b")]),
            class(13, "First", 0, vec![]),
            class(14, "Second", 0, vec![]),
        ]);
        let mut body = 12u16.to_le_bytes().to_vec();
        append_pointer(&mut body, 7, 13);
        append_pointer(&mut body, 7, 14);
        let error = decode_graph(&body, &registry).unwrap_err().to_string();
        assert!(
            error.contains("token 7 at 8 changes class from 13 to 14"),
            "{error}"
        );
    }

    #[test]
    fn graph_does_not_invent_targets_for_null_or_external_pointers() {
        let external = Field {
            modifier: 3,
            ..pointer_field("external")
        };
        let registry = registry(vec![class(
            12,
            "Root",
            0,
            vec![external, pointer_field("null")],
        )]);
        let mut body = 12u16.to_le_bytes().to_vec();
        body.extend(7u32.to_le_bytes());
        body.extend(0u32.to_le_bytes());
        let graph = decode_graph(&body, &registry).unwrap();
        assert_eq!(graph.objects.len(), 1);
        assert!(graph.edges.is_empty());
        assert_eq!(graph.objects[0].fields["external"]["pointer_token"], 7);
        assert_eq!(graph.objects[0].fields["null"]["pointer_token"], 0);
    }

    #[test]
    fn explicit_graph_budgets_enforce_and_can_raise_value_limits() {
        let registry = registry(vec![
            class(12, "Root", 0, vec![pointer_field("child")]),
            class(13, "Child", 0, vec![field("value")]),
        ]);
        let mut body = 12u16.to_le_bytes().to_vec();
        append_pointer(&mut body, 7, 13);
        body.extend(123u32.to_le_bytes());
        let mut limits = GraphLimits {
            max_values: 1,
            ..GraphLimits::default()
        };
        assert!(
            decode_graph_with_limits(&body, &registry, &limits)
                .unwrap_err()
                .to_string()
                .contains("native field value budget exceeded (1)")
        );
        limits.max_values = 2;
        assert_eq!(
            decode_graph_with_limits(&body, &registry, &limits)
                .unwrap()
                .objects[1]
                .fields["value"],
            123
        );
        limits.max_objects = 1;
        assert!(
            decode_graph_with_limits(&body, &registry, &limits)
                .unwrap_err()
                .to_string()
                .contains("native graph object budget exceeded")
        );
        limits.max_objects = 2;
        limits.max_depth = 1;
        assert!(
            decode_graph_with_limits(&body, &registry, &limits)
                .unwrap_err()
                .to_string()
                .contains("native field recursion budget exceeded")
        );
        limits.max_depth = 0;
        assert!(
            decode_graph_with_limits(&body, &registry, &limits)
                .unwrap_err()
                .to_string()
                .contains("budgets must be positive")
        );
    }

    #[test]
    fn inherited_collisions_keep_all_values_and_declaring_spans() {
        let mut transient = field("only_base");
        transient.uninterpreted_flags = 2;
        let registry = registry(vec![
            class(12, "Base", 0, vec![field("shared"), field("only_base")]),
            class(13, "Middle", 12, vec![field("shared"), transient]),
            class(14, "Leaf", 13, vec![field("shared"), field("only_leaf")]),
        ]);
        let mut body = 14u16.to_le_bytes().to_vec();
        for value in [11u32, 12, 21, 31, 32] {
            body.extend(value.to_le_bytes());
        }
        let decoded = decode_object_fields(&body, &registry, 2, 14).unwrap();
        assert_eq!(decoded.end, body.len());
        assert_eq!(
            decoded.fields,
            json!({
                "Base::shared": 11, "only_base": 12,
                "Middle::shared": 21,
                "Leaf::shared": 31, "only_leaf": 32,
            })
        );
        let spans: Vec<_> = decoded
            .field_spans
            .iter()
            .map(|span| (span.class_tag, span.name.as_str(), span.start, span.end))
            .collect();
        assert_eq!(
            spans,
            vec![
                (12, "shared", 2, 6),
                (12, "only_base", 6, 10),
                (13, "shared", 10, 14),
                (14, "shared", 14, 18),
                (14, "only_leaf", 18, 22),
            ]
        );
        let graph = decode_graph(&body, &registry).unwrap();
        assert_eq!(graph.objects[0].fields, decoded.fields);
        assert_eq!(graph.consumed_bytes, body.len());
        assert!(decode_graph(&body[..body.len() - 1], &registry).is_err());
    }

    #[test]
    fn duplicate_fields_within_one_declaring_class_remain_an_error() {
        let registry = registry(vec![class(
            12,
            "Base",
            0,
            vec![field("same"), field("same")],
        )]);
        let body = [12, 0, 1, 0, 0, 0, 2, 0, 0, 0];
        let error = decode_graph(&body, &registry).unwrap_err().to_string();
        assert!(
            error.contains("ambiguous native field key Base::same"),
            "{error}"
        );
    }
    #[test]
    fn signed_short_consumes_two_bytes_before_following_int() {
        let mut short = field("short");
        short.base = 3;
        short.raw_descriptor = 3;
        let mut int = field("following");
        int.base = 4;
        int.raw_descriptor = 4;
        let registry = registry(vec![class(12, "Pair", 0, vec![short, int])]);
        let mut bytes = 12u16.to_le_bytes().to_vec();
        bytes.extend((-123i16).to_le_bytes());
        bytes.extend(987654i32.to_le_bytes());
        let graph = decode_graph(&bytes, &registry).unwrap();
        assert_eq!(graph.consumed_bytes, 8);
        assert_eq!(graph.objects[0].fields["short"], -123);
        assert_eq!(graph.objects[0].fields["following"], 987654);
    }
    #[test]
    fn class_definition_ref_reads_schema_tag_without_swallowing_successor() {
        let mut reference = field("m_ref");
        reference.base = 10;
        reference.raw_descriptor = 10;
        let registry = registry(vec![
            class(12, "ClassDefinitionRef", 0, vec![reference]),
            class(13, "Target", 0, vec![]),
        ]);
        let bytes = [12, 0, 13, 0];
        let graph = decode_graph(&bytes, &registry).unwrap();
        assert_eq!(graph.objects[0].fields["m_ref"]["class_tag"], 13);
        assert_eq!(graph.objects[0].fields["m_ref"]["class_name"], "Target");
        assert_eq!(graph.consumed_bytes, 4);
    }
    #[test]
    fn sketch_typeless_parameters_keep_serialized_ref_count_and_value() {
        let params = Field {
            base: 14,
            modifier: 0x54,
            ..field("m_params")
        };
        let count = Field {
            base: 4,
            uninterpreted_flags: 4,
            ..field("m_refCt")
        };
        let value = Field {
            base: 7,
            ..field("m_val")
        };
        let registry = registry(vec![
            class(12, "VarSketchObj", 0, vec![params]),
            class(13, "VarExpr", 0, vec![count]),
            class(14, "VarParam", 13, vec![value]),
        ]);
        let mut body = 12u16.to_le_bytes().to_vec();
        body.extend(1u32.to_le_bytes());
        append_pointer(&mut body, 21, 14);
        body.extend(3i32.to_le_bytes());
        body.extend(7.25f64.to_le_bytes());
        let graph = decode_graph(&body, &registry).unwrap();
        assert_eq!(graph.consumed_bytes, body.len());
        assert_eq!(graph.objects[1].fields["m_refCt"], 3);
        assert_eq!(graph.objects[1].fields["m_val"], 7.25);
        assert_eq!(graph.edges[0].target_object_index, 1);
    }
    #[test]
    fn unqualified_flag_four_is_not_silently_skipped() {
        let value = Field {
            base: 4,
            uninterpreted_flags: 4,
            ..field("m_refCt")
        };
        let registry = registry(vec![class(12, "OtherClass", 0, vec![value])]);
        assert!(
            decode_graph(&[12, 0, 1, 0, 0, 0], &registry)
                .unwrap_err()
                .to_string()
                .contains("unsupported native field flags")
        );
    }
}
