//! Schema-directed native value prefixes and deferred parameter sets.
//! This decoder requires an independently bounded body. IDs remain serialized
//! references until the caller establishes the file's current identity mapping.
//! Object field keys are bare schema names unless the inheritance chain has
//! multiple serialized declarations of that name; those keys are all emitted
//! as `DeclaringClass::field`. Inline objects have independent naming scopes.
use crate::le_bytes;
use crate::schema_registry::{Class, Field, Registry};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

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
    /// Pointer lookup tables, built once on first use.
    #[serde(skip)]
    pub(crate) lookup: std::sync::OnceLock<PointerLookup>,
}
/// Edges by (source object, pointer offset) and objects by token.
#[derive(Clone, Default)]
pub(crate) struct PointerLookup {
    pub(crate) edges: BTreeMap<(usize, usize), Vec<usize>>,
    pub(crate) tokens: BTreeMap<u64, Vec<usize>>,
}
impl std::fmt::Debug for PointerLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PointerLookup")
    }
}
impl ObjectGraph {
    pub(crate) fn lookup(&self) -> &PointerLookup {
        self.lookup.get_or_init(|| {
            let mut lookup = PointerLookup::default();
            for (i, e) in self.edges.iter().enumerate() {
                lookup
                    .edges
                    .entry((e.source_object_index, e.pointer_offset))
                    .or_default()
                    .push(i);
            }
            for (i, o) in self.objects.iter().enumerate() {
                lookup.tokens.entry(o.token as u64).or_default().push(i);
            }
            lookup
        })
    }
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
    decode_graph_in(
        body,
        registry,
        limits,
        catalog,
        None,
        &mut PlanCache::default(),
    )
}
/// [`decode_graph_with_catalog_usage`] keeping only the fields of every object
/// whose key is in `keep` (a bare field name, or `Declaring::name` where the
/// class chain declares the name more than once). Any other field is read and
/// checked as the whole decode reads and checks it, and left out of the
/// object's `fields`, which then hold exactly the entries of `keep` that the
/// full decode's do, with the same values. The graph is otherwise the full
/// decode's: it succeeds or fails with the same error, reads the same pointers
/// and objects, and has the same object starts and ends, edges, consumed bytes
/// and usage. A caller that reads only the fields in `keep` of a graph gets
/// what it gets from the full decode, without the values it ignores being
/// built and freed (#1335).
pub fn decode_graph_projected(
    body: &[u8],
    registry: &Registry,
    limits: &GraphLimits,
    catalog: Option<&crate::native_extensible_storage::Catalog>,
    keep: &[&str],
) -> Result<(ObjectGraph, GraphUsage)> {
    decode_graph_in(
        body,
        registry,
        limits,
        catalog,
        Some(keep),
        &mut PlanCache::default(),
    )
}
/// The class plans of the decodes that share it, kept so that each class's is
/// made once for a pass over a file's records, not once for each record
/// (#1335: building them was 3.1% of the export time of the 33 samples of
/// Revit 2015 to 2023). A plan depends on the schema, the class and the fields
/// a projected decode keeps, and nothing else, so a decode that finds a plan
/// made by an earlier one reads exactly as it would have with its own. A cache
/// is for one schema: [`decode_graph_cached`] empties it when it is given
/// another.
///
/// What a cache holds is bounded: a plan costs [`PLAN_ENTRY_COST`] and one for
/// each field of every class of its chain and each field kept, the cache holds
/// at most [`PLAN_CACHE_COST`] of them (or the limit given to
/// [`Self::with_limit`]), and one that would pass it empties the cache first. A
/// plan dearer than the whole limit is not kept. Either way a decode that
/// finds no plan makes its own, which is what every decode did before the cache,
/// so the bound cannot change a graph: a schema made to hold plans of tens of
/// thousands of fields, or a caller that passes a new list of fields each call,
/// gets the decode it always got, not an allocation that grows with the file.
#[derive(Debug)]
pub struct PlanCache {
    /// The checksum of the schema the plans were made for.
    registry: String,
    /// By class tag and the address and length of the fields kept; the plan
    /// says which fields, so that the same address holding other fields is
    /// not taken for the same list.
    plans: HashMap<PlanKey, Rc<ClassPlan>>,
    /// The cost of the plans held.
    cost: usize,
    /// The most cost the cache holds.
    limit: usize,
    /// How many times the cache was emptied for passing its limit.
    clears: usize,
}
/// What a [`PlanCache`] holds by default, as a sum of plan costs: an estimate
/// of about 60 bytes for each unit. Measured over 88 files of Revit 2015 to
/// 2027 with one cache shared by seven field lists at once, more than any pass
/// shares, the most a file made it hold was 174,316 (8,127 plans, Snowdon
/// Towers architectural), 17.4% of it, and no file emptied it (the probe of
/// #1335 prints the figure).
pub const PLAN_CACHE_COST: usize = 1_000_000;
/// What each plan costs besides its fields: its entry in the table, its
/// vectors and its share of the cache's overhead.
pub const PLAN_ENTRY_COST: usize = 8;
impl Default for PlanCache {
    fn default() -> Self {
        Self::with_limit(PLAN_CACHE_COST)
    }
}
impl PlanCache {
    /// An empty cache that holds at most `limit` of cost.
    pub fn with_limit(limit: usize) -> Self {
        Self {
            registry: String::new(),
            plans: HashMap::new(),
            cost: 0,
            limit,
            clears: 0,
        }
    }
    /// The number of plans held.
    pub fn entries(&self) -> usize {
        self.plans.len()
    }
    /// The cost of the plans held.
    pub fn cost(&self) -> usize {
        self.cost
    }
    /// How many times the cache was emptied for passing its limit.
    pub fn clears(&self) -> usize {
        self.clears
    }
}
/// A class's tag and the address and length of the fields a decode keeps.
type PlanKey = (u16, usize, usize);
/// [`decode_graph_with_catalog_usage`], or [`decode_graph_projected`] where
/// `keep` is given, taking the class plans from `plans` and leaving there those
/// it makes. The graph is the one the decode without a cache gives.
pub fn decode_graph_cached(
    body: &[u8],
    registry: &Registry,
    limits: &GraphLimits,
    catalog: Option<&crate::native_extensible_storage::Catalog>,
    keep: Option<&[&str]>,
    plans: &mut PlanCache,
) -> Result<(ObjectGraph, GraphUsage)> {
    decode_graph_in(body, registry, limits, catalog, keep, plans)
}
fn decode_graph_in(
    body: &[u8],
    registry: &Registry,
    limits: &GraphLimits,
    catalog: Option<&crate::native_extensible_storage::Catalog>,
    keep: Option<&[&str]>,
    plans: &mut PlanCache,
) -> Result<(ObjectGraph, GraphUsage)> {
    ensure!(
        limits.max_values > 0 && limits.max_objects > 0 && limits.max_depth > 0,
        "native graph resource budgets must be positive"
    );
    let tag =
        le_bytes::u16_at(body, 0).ok_or_else(|| anyhow::anyhow!("truncated root object tag"))?;
    if plans.registry != registry.source_sha256 {
        plans.plans.clear();
        plans.cost = 0;
        plans.registry.clone_from(&registry.source_sha256);
    }
    let mut cursor =
        Cursor::new(body, registry, catalog, 2, *limits, Spans::None, plans).with_keep(keep);
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
                .class_in::<true>(tag, 0, true)
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
            lookup: Default::default(),
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
    decode_object_fields_with_limits(body, registry, start, class_tag, &GraphLimits::default())
}
/// [`decode_object_fields`] with explicit resource budgets.
pub fn decode_object_fields_with_limits(
    body: &[u8],
    registry: &Registry,
    start: usize,
    class_tag: u16,
    limits: &GraphLimits,
) -> Result<ObjectFields> {
    decode_object_fields_spanning(body, registry, start, class_tag, limits, Spans::All)
}

/// Which fields' byte spans a decode keeps ([`ObjectFields::field_spans`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spans {
    /// None: a graph decode never reads them.
    None,
    /// Every field's, nested objects' included.
    All,
    /// Those of the fields declared by the class with this tag.
    Class(u16),
}

/// [`decode_object_fields_with_limits`], keeping the spans `spans` names: a
/// span copies its field's name, and a caller that reads one field's span
/// has no use for the rest (#1237).
pub fn decode_object_fields_spanning(
    body: &[u8],
    registry: &Registry,
    start: usize,
    class_tag: u16,
    limits: &GraphLimits,
    spans: Spans,
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
    let mut plans = PlanCache::default();
    let mut cursor = Cursor::new(body, registry, None, start, *limits, spans, &mut plans);
    let fields = cursor.class::<true>(class_tag, 0)?;
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
/// A class's inheritance chain, base class last, and the key each of its
/// fields takes in a decoded object: the bare name, or `Declaring::name`
/// where the chain declares that name more than once. It depends on the
/// schema alone, so a decode makes it once for each class it meets (#1237:
/// most of an object's allocations were this, made again for every one).
#[derive(Debug)]
struct ClassPlan {
    /// The tags of the chain's classes, as the schema registry finds them.
    lineage: Vec<u16>,
    keys: Vec<Vec<String>>,
    /// Whether each field is one a projected decode keeps
    /// ([`decode_graph_projected`]); empty for a decode that is not one.
    kept: Vec<Vec<bool>>,
    /// The position, as (index in `lineage`, index in the class's fields), of
    /// the first field whose key an earlier one in decoding order has already
    /// taken: where a decode that builds no object stops with the error a
    /// built one meets there. Found only for a projected decode; `None`
    /// otherwise.
    duplicate: Option<(usize, usize)>,
    /// The fields the plan was made for, where it was made for a projected
    /// decode.
    keep: Option<Vec<String>>,
}
impl ClassPlan {
    /// What the plan costs a [`PlanCache`]: its entry, a unit for each field
    /// of every class of its chain, and one for each field kept.
    fn cost(&self) -> usize {
        PLAN_ENTRY_COST
            + self.keys.iter().map(Vec::len).sum::<usize>()
            + self.keep.as_ref().map_or(0, Vec::len)
    }
    /// Whether the plan was made for the fields `keep` names.
    fn is_for(&self, keep: Option<&[&str]>) -> bool {
        match (&self.keep, keep) {
            (None, None) => true,
            (Some(own), Some(wanted)) => {
                own.len() == wanted.len() && own.iter().zip(wanted).all(|(a, b)| a.as_str() == *b)
            }
            _ => false,
        }
    }
}
struct Cursor<'a> {
    body: &'a [u8],
    registry: &'a Registry,
    catalog: Option<&'a crate::native_extensible_storage::Catalog>,
    pos: usize,
    items: usize,
    pending: Vec<PendingPointer>,
    spans: Vec<FieldSpan>,
    keep_spans: Spans,
    plans: &'a mut PlanCache,
    limits: GraphLimits,
    /// The fields a projected decode keeps ([`decode_graph_projected`]).
    keep: Option<&'a [&'a str]>,
}
impl<'a> Cursor<'a> {
    fn new(
        body: &'a [u8],
        registry: &'a Registry,
        catalog: Option<&'a crate::native_extensible_storage::Catalog>,
        pos: usize,
        limits: GraphLimits,
        keep_spans: Spans,
        plans: &'a mut PlanCache,
    ) -> Self {
        Self {
            body,
            registry,
            catalog,
            pos,
            items: 0,
            pending: Vec::new(),
            spans: Vec::new(),
            keep_spans,
            plans,
            limits,
            keep: None,
        }
    }
    fn with_keep(mut self, keep: Option<&'a [&'a str]>) -> Self {
        self.keep = keep;
        self
    }
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
    fn pointer<const BUILD: bool>(&mut self, external: bool) -> Result<Value> {
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
        Ok(if BUILD {
            pointer_value(token, tag, offset)
        } else {
            Value::Null
        })
    }
    fn entity_pointer<const BUILD: bool>(&mut self) -> Result<Value> {
        let offset = self.pos;
        let discriminator = i32::from_le_bytes(self.take(4)?.try_into()?);
        ensure!(
            matches!(discriminator, 0 | -1),
            "unqualified ES entity discriminator {discriminator}"
        );
        if discriminator == 0 {
            return Ok(if BUILD {
                json!({"discriminator":0,"state":"serialized_absent","offset":offset})
            } else {
                Value::Null
            });
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
        Ok(if BUILD {
            json!({"discriminator":-1,"schema_guid":guid,"state":"serialized_present","offset":offset})
        } else {
            Value::Null
        })
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
        self.field::<true>(
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
    /// The object of class `tag` at the cursor. With `BUILD` unset nothing is
    /// built: every byte is read, every check made and every pointer noted as
    /// when it is set, and the value is `Null` (#1335).
    fn class<const BUILD: bool>(&mut self, tag: u16, depth: usize) -> Result<Value> {
        self.class_in::<BUILD>(tag, depth, false)
    }
    /// [`Self::class`], where `top` says the object is one the graph decode
    /// itself starts on, not an inline one inside a field: only then are the
    /// fields chosen by [`Self::keep`] the only ones built.
    fn class_in<const BUILD: bool>(&mut self, tag: u16, depth: usize, top: bool) -> Result<Value> {
        ensure!(
            depth < self.limits.max_depth,
            "native class recursion budget exceeded"
        );
        let plan = self.plan(tag, depth)?;
        let registry = self.registry;
        let project = BUILD && top && self.keep.is_some();
        let mut values = Map::new();
        for (distance, class_tag) in plan.lineage.iter().enumerate().rev() {
            let c = registry
                .class(*class_tag)
                .ok_or_else(|| anyhow::anyhow!("unknown inline native class {class_tag}"))?;
            for (field_index, f) in c.fields.iter().enumerate() {
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
                let kept = BUILD && (!project || plan.kept[distance][field_index]);
                let value = if kept {
                    self.class_field::<true>(c, f, depth, distance)?
                } else {
                    self.class_field::<false>(c, f, depth, distance)?
                };
                if self.keeps_spans_of(c.tag) {
                    self.spans.push(FieldSpan {
                        class_tag: c.tag,
                        name: f.name.clone(),
                        start,
                        end: self.pos,
                    });
                }
                if kept {
                    let key = plan.keys[distance][field_index].clone();
                    ensure!(
                        !values.contains_key(&key),
                        "ambiguous native field key {key} in declaring class {} (tag {})",
                        c.name,
                        c.tag
                    );
                    values.insert(key, value);
                } else {
                    ensure!(
                        plan.duplicate != Some((distance, field_index)),
                        "ambiguous native field key {} in declaring class {} (tag {})",
                        plan.keys[distance][field_index],
                        c.name,
                        c.tag
                    );
                }
            }
        }
        Ok(if BUILD {
            Value::Object(values)
        } else {
            Value::Null
        })
    }
    /// One field `f` of class `c`, the `distance`th of the chain from the
    /// object's own class, as [`Self::class_in`] reads it.
    fn class_field<const BUILD: bool>(
        &mut self,
        c: &Class,
        f: &Field,
        depth: usize,
        distance: usize,
    ) -> Result<Value> {
        if c.name == "ClassDefinitionRef" && f.name == "m_ref" && f.base == 10 && f.modifier == 0 {
            // ClassDefinitionRef stores a 16-bit schema class tag. The
            // qualified external reader performs Int16Reader -> ClassesContainer.find.
            let class_tag = u16::from_le_bytes(self.take(2)?.try_into()?);
            Ok(if BUILD {
                json!({"class_tag":class_tag,"class_name":self.registry.class(class_tag).map(|class|class.name.as_str())})
            } else {
                Value::Null
            })
        } else if c.name == "ESEntity" && f.name == "m_blob" && f.base == 14 && f.modifier == 1 {
            self.entity_pointer::<BUILD>()
        } else if c.name == "VarSketchObj"
            && f.name == "m_params"
            && f.base == 14
            && f.modifier == 0x54
        {
            // Qualified typeless owning-pointer vector: each entry still
            // carries its concrete VarParam class tag in the wire data.
            self.field_as::<BUILD>(f, 0x51, f.array_count, depth + distance + 1)
        } else {
            self.field::<BUILD>(f, depth + distance + 1)
                .map_err(|e| anyhow::anyhow!("{}.{} at {}: {e}", c.name, f.name, self.pos))
        }
    }
    /// Whether the spans of the fields class `tag` declares are kept.
    fn keeps_spans_of(&self, tag: u16) -> bool {
        match self.keep_spans {
            Spans::None => false,
            Spans::All => true,
            Spans::Class(only) => only == tag,
        }
    }
    /// The class `tag`'s [`ClassPlan`], made on first use by any decode that
    /// shares this cursor's [`PlanCache`].
    fn plan(&mut self, tag: u16, depth: usize) -> Result<Rc<ClassPlan>> {
        let key = (
            tag,
            self.keep.map_or(0, |keep| keep.as_ptr() as usize),
            self.keep.map_or(0, |keep| keep.len()),
        );
        if let Some(plan) = self.plans.plans.get(&key) {
            if plan.is_for(self.keep) {
                // The chain was made under a budget that held at its own
                // depth: the check that matters is the one before its last
                // class.
                ensure!(
                    depth + plan.lineage.len() - 1 < self.limits.max_depth,
                    "native class recursion budget exceeded"
                );
                return Ok(Rc::clone(plan));
            }
        }
        // The lineage borrows the registry's classes: cloning them, every
        // field name with them, for each object decoded was most of a
        // graph's allocations (#1237).
        let registry: &'a Registry = self.registry;
        let mut lineage: Vec<&'a Class> = Vec::new();
        let mut current = tag;
        loop {
            ensure!(
                depth + lineage.len() < self.limits.max_depth,
                "native class recursion budget exceeded"
            );
            let c = registry
                .class(current)
                .ok_or_else(|| anyhow::anyhow!("unknown inline native class {current}"))?;
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
        let keys: Vec<Vec<String>> = lineage
            .iter()
            .map(|c| {
                c.fields
                    .iter()
                    .map(|f| {
                        if f.uninterpreted_flags & 2 != 0 {
                            String::new()
                        } else if names[f.name.as_str()] > 1 {
                            format!("{}::{}", c.name, f.name)
                        } else {
                            f.name.clone()
                        }
                    })
                    .collect::<Vec<String>>()
            })
            .collect();
        let kept = match self.keep {
            Some(keep) => lineage
                .iter()
                .zip(&keys)
                .map(|(c, keys)| {
                    c.fields
                        .iter()
                        .zip(keys)
                        .map(|(f, key)| {
                            f.uninterpreted_flags & 2 == 0 && keep.contains(&key.as_str())
                        })
                        .collect::<Vec<bool>>()
                })
                .collect(),
            None => Vec::new(),
        };
        // Only a projected decode reads fields without building them, and so
        // needs the position; every other decode finds a repeated key in the
        // object it builds.
        let duplicate = if self.keep.is_some() {
            let mut taken = std::collections::HashSet::new();
            let mut duplicate = None;
            'chain: for (distance, c) in lineage.iter().enumerate().rev() {
                for (field_index, f) in c.fields.iter().enumerate() {
                    if f.uninterpreted_flags & 2 == 0
                        && !taken.insert(keys[distance][field_index].as_str())
                    {
                        duplicate = Some((distance, field_index));
                        break 'chain;
                    }
                }
            }
            duplicate
        } else {
            None
        };
        let plan = Rc::new(ClassPlan {
            lineage: lineage.iter().map(|c| c.tag).collect(),
            keys,
            kept,
            duplicate,
            keep: self
                .keep
                .map(|keep| keep.iter().map(|field| (*field).to_string()).collect()),
        });
        // A plan is kept only while the cache's cost stays within its limit:
        // one that would pass it empties the cache, and one dearer than the
        // limit alone is not kept. The decode has the plan either way.
        let cost = plan.cost();
        if cost <= self.plans.limit {
            if self.plans.cost + cost > self.plans.limit {
                self.plans.plans.clear();
                self.plans.cost = 0;
                self.plans.clears += 1;
            }
            if let Some(replaced) = self.plans.plans.insert(key, Rc::clone(&plan)) {
                self.plans.cost = self.plans.cost.saturating_sub(replaced.cost());
            }
            self.plans.cost += cost;
        }
        Ok(plan)
    }
    fn field<const BUILD: bool>(&mut self, f: &Field, depth: usize) -> Result<Value> {
        self.field_as::<BUILD>(f, f.modifier, f.array_count, depth)
    }
    /// [`Self::field`] reading `f` with `modifier` and `array_count` in place
    /// of its own: a container's items are `f` itself, one level down, which
    /// copying the field for each container was a share of a graph's
    /// allocations (#1237).
    fn field_as<const BUILD: bool>(
        &mut self,
        f: &Field,
        modifier: u8,
        array_count: Option<u32>,
        depth: usize,
    ) -> Result<Value> {
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
        match modifier {
            1..=3 if f.base == 14 => self.pointer::<BUILD>(modifier == 3),
            0x60 if f.base == 8 => {
                let n = self.u32()? as usize;
                ensure!(n <= 1_000_000, "native string unit budget exceeded");
                let bytes = self.take(n * 2)?;
                if BUILD {
                    let units = le_bytes::u16_units(bytes).collect::<Vec<_>>();
                    Ok(Value::String(String::from_utf16(&units)?))
                } else {
                    if char::decode_utf16(le_bytes::u16_units(bytes)).any(|unit| unit.is_err()) {
                        let units = le_bytes::u16_units(bytes).collect::<Vec<_>>();
                        String::from_utf16(&units)?;
                    }
                    Ok(Value::Null)
                }
            }
            0x10 | 0x11 | 0x12 | 0x13 | 0x50 | 0x51 | 0x52 | 0x53 => {
                let n = if modifier < 0x50 {
                    array_count.ok_or_else(|| anyhow::anyhow!("fixed array schema count absent"))?
                } else {
                    self.u32()?
                };
                ensure!(n <= 100_000, "native container count budget exceeded");
                if BUILD {
                    let mut values = Vec::with_capacity(n.min(1024) as usize);
                    for _ in 0..n {
                        values.push(self.field_as::<true>(f, modifier & 0xf, None, depth + 1)?);
                    }
                    Ok(Value::Array(values))
                } else {
                    for _ in 0..n {
                        self.field_as::<false>(f, modifier & 0xf, None, depth + 1)?;
                    }
                    Ok(Value::Null)
                }
            }
            0 => match f.base {
                1 => {
                    let b = self.take(1)?[0];
                    ensure!(b <= 1, "invalid native bool");
                    Ok(if BUILD { json!(b != 0) } else { Value::Null })
                }
                2 => {
                    let b = self.take(1)?[0];
                    Ok(if BUILD { json!(b) } else { Value::Null })
                }
                3 => {
                    let n = i16::from_le_bytes(self.take(2)?.try_into()?);
                    Ok(if BUILD { json!(n) } else { Value::Null })
                }
                4 => {
                    let n = i32::from_le_bytes(self.take(4)?.try_into()?);
                    Ok(if BUILD { json!(n) } else { Value::Null })
                }
                5 => {
                    let n = self.u32()?;
                    Ok(if BUILD { json!(n) } else { Value::Null })
                }
                6 => {
                    let n = f32::from_le_bytes(self.take(4)?.try_into()?);
                    ensure!(n.is_finite(), "nonfinite native float");
                    Ok(if BUILD { json!(n) } else { Value::Null })
                }
                7 => {
                    let n = f64::from_le_bytes(self.take(8)?.try_into()?);
                    ensure!(n.is_finite(), "nonfinite native double");
                    Ok(if BUILD { json!(n) } else { Value::Null })
                }
                9 => {
                    let bytes = self.take(16)?;
                    Ok(if BUILD {
                        json!({"guid_bytes":bytes})
                    } else {
                        Value::Null
                    })
                }
                11 => {
                    let n = i64::from_le_bytes(self.take(8)?.try_into()?);
                    Ok(if BUILD { json!(n) } else { Value::Null })
                }
                13 => self.field::<BUILD>(
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
                    self.class::<BUILD>(f.references[0].tag, depth + 1)
                }
                _ => anyhow::bail!("unsupported native scalar {}", f.base),
            },
            _ => anyhow::bail!("unsupported native modifier {modifier}"),
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
/// A pointer as a decoded graph holds it: the array `[token, class tag or null,
/// offset, true]`. As a map of `pointer_token`, `class_tag` and `offset` it came
/// to one 631-byte `BTreeMap` leaf and three key strings, about 730 bytes for
/// three numbers, and a face or an edge holds five or six of them (#1694); the
/// array is one allocation of 128. The trailing `true` tells a pointer from any
/// array of three integers (a facet's vertex indices are one), which a reader
/// that walks a graph without knowing its fields could not otherwise. Readers
/// take a pointer's parts through [`pointer_parts`] and the three functions on
/// it, so none names the shape.
pub(crate) fn pointer_value(token: u32, class_tag: Option<u16>, offset: usize) -> Value {
    Value::Array(vec![
        Value::from(token),
        class_tag.map_or(Value::Null, Value::from),
        Value::from(offset),
        Value::Bool(true),
    ])
}
/// The token, the class tag (`None` where the pointer is null or external) and
/// the offset of a pointer value, `None` for any value that is not one.
pub fn pointer_parts(value: &Value) -> Option<(u64, Option<u64>, u64)> {
    let [token, tag, offset, Value::Bool(true)] = value.as_array()?.as_slice() else {
        return None;
    };
    let tag = if tag.is_null() {
        None
    } else {
        Some(tag.as_u64()?)
    };
    Some((token.as_u64()?, tag, offset.as_u64()?))
}
/// The token of a pointer value ([`pointer_parts`]); 0 is null.
pub fn pointer_token(value: &Value) -> Option<u64> {
    pointer_parts(value).map(|(token, _, _)| token)
}
/// The class tag of a pointer value, `None` where it names none or is no pointer.
pub fn pointer_class_tag(value: &Value) -> Option<u64> {
    pointer_parts(value)?.1
}
/// The offset of a pointer value in its body.
pub fn pointer_offset(value: &Value) -> Option<u64> {
    pointer_parts(value).map(|(_, _, offset)| offset)
}
/// Decode scalar parameter sets after the schema-directed derived-field prefix.
/// This is not complete record decoding: subsequent geometry and graph objects
/// remain unconsumed. Callers must separately establish current record ownership.
pub fn decode(body: &[u8], registry: &Registry) -> Result<Parameters> {
    let base = crate::native_element::decode_base(body, registry)?;
    let mut plans = PlanCache::default();
    let mut cursor = Cursor::new(
        body,
        registry,
        None,
        2,
        GraphLimits::default(),
        Spans::None,
        &mut plans,
    );
    let fields = cursor.class::<true>(base.class_tag, 0)?;
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
        let token =
            pointer_token(pointer).ok_or_else(|| anyhow::anyhow!("missing parameter pointer"))?;
        if token == 0 {
            continue;
        }
        ensure!(
            token == u64::from(u32::MAX),
            "shared/referenced parameter set requires object-graph resolution"
        );
        let tag = pointer_class_tag(pointer)
            .ok_or_else(|| anyhow::anyhow!("parameter set class absent"))? as u16;
        ensure!(
            registry.class(tag).is_some_and(|c| c.name == class_name),
            "parameter set class mismatch"
        );
        let set = cursor.class::<true>(tag, 0)?;
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
