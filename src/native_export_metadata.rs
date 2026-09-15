//! Explicit exporter-facing metadata projections from native current records.
//! Display labels do not replace native parameter IDs; each value retains its
//! owner and native field provenance. No converter output is a runtime input.
use crate::{native_document::Record, native_metadata::identifier};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Field {
    pub value: Value,
    pub sources: Vec<Value>,
    pub interpretation: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Owner {
    pub element_id: u64,
    pub unique_id: String,
    pub class_name: String,
    pub fields: BTreeMap<String, Field>,
    pub diagnostics: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_metadata_parity: bool,
    pub owners: Vec<Owner>,
    pub diagnostics: Vec<String>,
}
type MaterialQuantityMap = BTreeMap<i64, (Option<f64>, Option<f64>, Vec<Value>)>;
type MaterialLayerMap = BTreeMap<i64, (f64, i64, Value)>;
struct Entry {
    id: u64,
    uid: String,
    class: String,
    root: Value,
    source: Value,
    name: Option<String>,
    fields: BTreeMap<String, Field>,
    type_id: Option<u64>,
    computed_type_id: Option<u64>,
    family_id: Option<u64>,
    category_id: Option<i64>,
    partition_id: i64,
    level_elevation: Option<f64>,
    parameters: BTreeMap<i64, Value>,
}
#[derive(Default)]
pub struct Builder {
    entries: BTreeMap<u64, Entry>,
    diagnostics: Vec<String>,
    episodes: Vec<String>,
    history_sha256: String,
    graphics: BTreeMap<u64, BTreeMap<String, Field>>,
    partitions: BTreeMap<i64, (String, Value)>,
    headers: BTreeMap<u64, (Value, Value)>,
    material_quantities: BTreeMap<u64, MaterialQuantityMap>,
    material_layers: BTreeMap<u64, MaterialLayerMap>,
    compound_core: BTreeMap<u64, (Value, Value)>,
    system_family_names: BTreeMap<u64, (String, Value)>,
    single_layer_widths: BTreeMap<u64, (f64, Value)>,
    structural_assets: BTreeMap<u64, (u64, Value)>,
    geo_elevations: BTreeMap<u64, (f64, Value)>,
    associations: Option<(std::collections::BTreeSet<u64>, Value)>,
}
fn owned_child<'a>(
    graph: &'a crate::native_parameters::ObjectGraph,
    owner: usize,
    field: &str,
    class: &str,
) -> Option<(usize, &'a crate::native_parameters::GraphObject)> {
    let offset = graph
        .objects
        .get(owner)?
        .fields
        .get(field)?
        .get("offset")?
        .as_u64()? as usize;
    let edges = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == owner && e.pointer_offset == offset)
        .collect::<Vec<_>>();
    if edges.len() != 1 {
        return None;
    }
    let i = edges[0].target_object_index;
    let object = graph.objects.get(i)?;
    (object.class_name == class).then_some((i, object))
}
fn owned_cell<'a>(
    graph: &'a crate::native_parameters::ObjectGraph,
    class: &str,
) -> Option<(usize, &'a crate::native_parameters::GraphObject)> {
    let (i, list) = owned_child(graph, 0, "m_cellList", "CellList")?;
    let indices = list
        .fields
        .get("m_cells")?
        .as_array()?
        .iter()
        .map(|p| {
            crate::native_saved_mesh::pointer(graph, i, p)
                .ok()
                .flatten()
        })
        .collect::<Option<Vec<_>>>()?;
    let matches = indices
        .into_iter()
        .filter(|&j| graph.objects[j].class_name == class)
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| (matches[0], &graph.objects[matches[0]]))
}
pub fn native_reference_id(v: &Value) -> Result<i64> {
    identifier(v)
}
fn id(v: &Value) -> Option<i64> {
    identifier(v).ok()
}
fn positive(v: &Value) -> Option<u64> {
    id(v).and_then(|i| u64::try_from(i).ok()).filter(|i| *i > 0)
}
fn category_name(id: i64) -> Option<&'static str> {
    Some(match id {
        -2000011 => "OST_Walls",
        -2000032 => "OST_Floors",
        -2000014 => "OST_Windows",
        -2000023 => "OST_Doors",
        -2000100 => "OST_Columns",
        -2001330 => "OST_StructuralColumns",
        -2000700 => "OST_Materials",
        -2001140 => "OST_MechanicalEquipment",
        -2001040 => "OST_ElectricalEquipment",
        -2008044 => "OST_PipeCurves",
        -2008000 => "OST_DuctCurves",
        _ => return None,
    })
}
fn label(pid: i64) -> Option<(&'static str, &'static str)> {
    Some(match pid {
        -1140422 => ("Keynote", "String"),
        -1012829 => ("Base Extension Distance", "Double"),
        -1012828 => ("Top Extension Distance", "Double"),
        -1005436 => ("Roughness", "Integer"),
        -1005435 => ("Absorptance", "Double"),
        -1002503 => ("Classification Title", "String"),
        -1002502 => ("Classification Number", "String"),
        -1002110 => ("Coarse Scale Fill Color", "Integer"),
        -1002106 => ("Coarse Scale Fill Pattern", "ElementId"),
        -1140355 => ("Name", "String"),
        -1019014 => ("Export to IFC As", "String"),
        -1019015 => ("Export Type to IFC As", "String"),
        -1019016 => ("IFC Predefined Type", "String"),
        -1019017 => ("Type IFC Predefined Type", "String"),
        -1019012 => ("Export to IFC", "Integer"),
        -1019013 => ("Export Type to IFC", "Integer"),
        -1010106 => ("Comments", "String"),
        -1010103 => ("Description", "String"),
        -1002554 => ("Shininess", "Integer"),
        -1002553 => ("Smoothness", "Integer"),
        -1002551 => ("Transparency", "Integer"),
        -1002550 => ("Color", "Integer"),
        -1002001 => ("Type Name", "String"),
        -1001951 => ("Height Offset From Level", "Double"),
        -1001405 => ("Type Mark", "String"),
        -1001206 => ("Fire Rating", "String"),
        -1001203 => ("Mark", "String"),
        -1001109 => ("Top Offset", "Double"),
        -1001108 => ("Base Offset", "Double"),
        -1001300 => ("Height", "Double"),
        -1001302 => ("Thickness", "Double"),
        -1001006 => ("Function", "Integer"),
        -1001390 => ("Wall Closure", "Integer"),
        -1005439 => ("Define Thermal Properties by", "Integer"),
        -1005437 => ("Analytic Construction", "String"),
        -1001211 => ("Operation", "String"),
        -1140400 => ("Material Type", "Integer"),
        -1001361 => ("Sill Height", "Double"),
        -1001362 => ("Head Height", "Double"),
        -1013449 => ("Has Association", "Boolean"),
        -1005500 => ("Structural Material", "ElementId"),
        -1001953 => ("Perimeter", "Double"),
        -1001954 => ("Structural", "Boolean"),
        -1001900 => ("Thickness", "Double"),
        -1001301 => ("Width", "Double"),
        -1001304 => ("Rough Height", "Double"),
        -1001305 => ("Rough Width", "Double"),
        -1001205 => ("Cost", "Double"),
        -1010108 => ("Manufacturer", "String"),
        -1010109 => ("Model", "String"),
        -1010104 => ("URL", "String"),
        _ => return None,
    })
}
fn decimal_15(value: f64) -> String {
    if value == 0. {
        return "0".into();
    }
    let scientific = format!("{value:.14e}");
    let (mantissa, exp) = scientific.split_once('e').expect("formatted exponent");
    let exponent = exp.parse::<i32>().expect("formatted integer exponent");
    if !(-4..15).contains(&exponent) {
        return format!(
            "{}e{:+03}",
            mantissa.trim_end_matches('0').trim_end_matches('.'),
            exponent
        );
    }
    let negative = mantissa.starts_with('-');
    let digits = mantissa.trim_start_matches('-').replace('.', "");
    let point = exponent + 1;
    let mut out = if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), digits)
    } else if point as usize >= digits.len() {
        format!("{}{}", digits, "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    };
    if out.contains('.') {
        out = out.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    if negative {
        out.insert(0, '-');
    }
    out
}
fn floor_perimeter(graph: &crate::native_parameters::ObjectGraph) -> Option<(f64, Vec<usize>)> {
    let (helper_index, helper) = owned_cell(graph, "FloorExtrusionHelper")?;
    if helper.fields.get("m_complete")?.as_bool() != Some(true) {
        return None;
    }
    let mut sources = vec![helper_index];
    let mut total = 0.;
    let loops = helper.fields.get("m_pCurveLoops")?.as_array()?;
    if loops.is_empty() {
        return None;
    }
    for p in loops {
        let i = crate::native_saved_mesh::pointer(graph, helper_index, p).ok()??;
        let curve_loop = graph.objects.get(i)?;
        if curve_loop.class_name != "CurveLoop"
            || curve_loop.fields.get("m_open")?.as_bool() != Some(false)
        {
            return None;
        }
        sources.push(i);
        let curves = curve_loop.fields.get("m_curves")?.as_array()?;
        if curves.is_empty() {
            return None;
        }
        for p in curves {
            let j = crate::native_saved_mesh::pointer(graph, i, p).ok()??;
            let line = graph.objects.get(j)?;
            if line.class_name != "GLine" {
                return None;
            }
            let d = line
                .fields
                .get("m_dirVec")?
                .as_array()?
                .iter()
                .map(Value::as_f64)
                .collect::<Option<Vec<_>>>()?;
            let e = line
                .fields
                .get("m_endParams")?
                .as_array()?
                .iter()
                .map(Value::as_f64)
                .collect::<Option<Vec<_>>>()?;
            if d.len() != 3 || e.len() != 2 {
                return None;
            }
            let length = (e[1] - e[0]).abs() * d.iter().map(|x| x * x).sum::<f64>().sqrt();
            if !length.is_finite() || length <= 0. {
                return None;
            }
            total += length;
            sources.push(j);
        }
    }
    total.is_finite().then_some((total, sources))
}
fn omni_class_catalog() -> &'static Value {
    static CATALOG: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("data/ddc-omniclass-18-4-3.json"))
            .expect("validated bundled OmniClass catalog")
    })
}
fn ifc_export_integer(parameters: &BTreeMap<i64, Value>, pid: i64) -> Option<(i64, bool)> {
    if !matches!(pid, -1019013 | -1019012) {
        return None;
    }
    match parameters.get(&pid) {
        Some(value) => value.as_i64().map(|v| (v, false)),
        None => Some((0, true)),
    }
}
fn put(fields: &mut BTreeMap<String, Field>, key: String, value: Value, source: Value) {
    if let Some(old) = fields.get_mut(&key) {
        if old.value == value {
            old.sources.push(source);
        } else {
            old.interpretation = "conflicting_native_candidates";
            old.sources
                .push(json!({"conflicting_value":value,"source":source}));
        }
    } else {
        fields.insert(
            key,
            Field {
                value,
                sources: vec![source],
                interpretation: "native_saved_value_or_explicit_projection",
            },
        );
    }
}
impl Builder {
    pub fn with_history(episodes: Vec<String>, history_sha256: String) -> Self {
        Self {
            episodes,
            history_sha256,
            ..Self::default()
        }
    }
    pub fn set_partition_table(
        &mut self,
        bytes: &[u8],
        registry: &crate::schema_registry::Registry,
    ) -> Result<()> {
        ensure!(bytes.len() >= 2, "truncated partition table");
        let tag = u16::from_le_bytes(bytes[..2].try_into()?);
        let root = crate::native_parameters::decode_object_fields(bytes, registry, 2, tag)?;
        ensure!(
            root.class_name == "PartitionTable" && bytes.get(root.end..) == Some(&[0, 0, 0, 0]),
            "partition catalog requires registered bounded root and exact four-zero opaque outer suffix"
        );
        let rows = root
            .fields
            .get("m_set")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("partition rows absent"))?;
        let mut seen = std::collections::BTreeSet::new();
        let mut active = BTreeMap::new();
        for row in rows {
            let pid = row
                .get("m_id")
                .and_then(id)
                .ok_or_else(|| anyhow::anyhow!("partition ID absent"))?;
            ensure!(
                pid >= 0 && seen.insert(pid),
                "invalid or duplicate partition ID"
            );
            let deleted = row
                .get("m_deleted")
                .and_then(Value::as_bool)
                .ok_or_else(|| anyhow::anyhow!("partition deletion state absent"))?;
            if !deleted {
                let name = row
                    .get("m_name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("partition name absent"))?;
                active.insert(pid,(name.to_owned(),json!({"stream":"Global/PartitionTable","body_sha256":format!("{:x}",Sha256::digest(bytes)),"source_field":"m_set","raw_partition":row,"decoded_root_end":root.end,"opaque_uninterpreted_outer_suffix":&bytes[root.end..],"complete_global_stream":false})));
            }
        }
        self.partitions = active;
        Ok(())
    }
    pub fn set_global_catalog(
        &mut self,
        bytes: &[u8],
        registry: &crate::schema_registry::Registry,
    ) -> Result<()> {
        ensure!(
            bytes.len() > 10 && bytes.ends_with(&[0, 0, 0, 0]),
            "unqualified Global/Latest framing"
        );
        let graph = crate::native_parameters::decode_graph_with_limits(
            &bytes[..bytes.len() - 4],
            registry,
            &crate::native_parameters::GraphLimits {
                max_values: 1_000_000,
                max_objects: 100_000,
                max_depth: 128,
            },
        )?;
        ensure!(
            graph
                .objects
                .first()
                .is_some_and(|o| o.class_name == "ADocument"),
            "global root is not ADocument"
        );
        let objects = graph
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.class_name == "AppInfoElementsAssociations")
            .collect::<Vec<_>>();
        ensure!(objects.len() == 1, "association repository owner ambiguous");
        let (index, object) = objects[0];
        let repositories = object
            .fields
            .get("m_RepoByName")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("association repository map absent"))?;
        let matches = repositories
            .iter()
            .filter(|r| r["first"] == "LB_Associations")
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1,
            "association repository missing or duplicate"
        );
        let rows = matches[0]["second"]["m_DataSet"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("association dataset absent"))?;
        let mut local = std::collections::BTreeSet::new();
        let mut keys = std::collections::BTreeSet::new();
        for row in rows {
            let key = row["first"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("association key not string"))?;
            ensure!(keys.insert(key), "duplicate association key");
            let parts = key.split('.').collect::<Vec<_>>();
            ensure!(
                parts.len() == 3
                    && parts
                        .iter()
                        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
                "unqualified association key grammar"
            );
            let values = parts
                .iter()
                .map(|p| p.parse::<u64>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(
                values[0] > 0 && values[2] > 0,
                "invalid association endpoints"
            );
            local.insert(values[0]);
        }
        self.associations = Some((
            local,
            json!({"stream":"Global/Latest","body_sha256":format!("{:x}",Sha256::digest(bytes)),"source_object":index,"source_field":"m_RepoByName.LB_Associations.m_DataSet.keys","raw_rows":rows,"graph_consumed_bytes":graph.consumed_bytes,"opaque_uninterpreted_suffix":[0,0,0,0],"complete_global_semantics":false}),
        ));
        Ok(())
    }
    fn version_guid(&self, revision: u32) -> Option<&String> {
        self.episodes
            .len()
            .checked_sub(revision as usize + 1)
            .and_then(|i| self.episodes.get(i))
    }
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        if record.channel == 101 {
            if let Some(graph) = &record.graph {
                ensure!(
                    graph.consumed_bytes == record.source.body_bytes,
                    "metadata header requires exact EOF"
                );
                if let Some(root) = graph
                    .objects
                    .first()
                    .filter(|o| o.class_name == "ElementHeader")
                {
                    let source = json!({"owner_id":record.identity.element_id,"owner_unique_id":record.identity.unique_id,"stream":record.source.stream,"body_sha256":record.source.body_sha256,"channel":101});
                    ensure!(
                        self.headers
                            .insert(record.identity.element_id, (json!(root.fields), source))
                            .is_none(),
                        "duplicate current metadata header"
                    );
                }
            }
            return Ok(());
        }
        if record.channel == 103 {
            if let Some(graph) = &record.graph {
                ensure!(
                    graph.consumed_bytes == record.source.body_bytes,
                    "metadata graphics require exact EOF"
                );
                if let Some(root) = graph.objects.first().filter(|o| o.class_name == "GElement") {
                    if let Some(bounds) = root
                        .fields
                        .get("m_bBox")
                        .and_then(Value::as_array)
                        .filter(|a| a.len() == 2)
                    {
                        let coordinates = bounds
                            .iter()
                            .map(|v| {
                                v.as_array().filter(|a| a.len() == 3).and_then(|a| {
                                    a.iter().map(Value::as_f64).collect::<Option<Vec<_>>>()
                                })
                            })
                            .collect::<Option<Vec<_>>>();
                        if let Some(coords) = coordinates.filter(|a| {
                            a.iter().flatten().all(|x| x.is_finite())
                                && (0..3).all(|i| a[0][i] <= a[1][i])
                        }) {
                            let mut fields = BTreeMap::new();
                            for (side, side_name) in ["Min", "Max"].iter().enumerate() {
                                for (axis, axis_name) in ["X", "Y", "Z"].iter().enumerate() {
                                    put(
                                        &mut fields,
                                        format!("BoundingBox{side_name}_{axis_name} : Double"),
                                        json!(coords[side][axis]),
                                        json!({"owner_id":record.identity.element_id,"owner_unique_id":record.identity.unique_id,"channel":103,"body_sha256":record.source.body_sha256,"source_field":format!("GElement.m_bBox[{side}][{axis}]")}),
                                    );
                                }
                            }
                            ensure!(
                                self.graphics
                                    .insert(record.identity.element_id, fields)
                                    .is_none(),
                                "duplicate current graphics metadata owner"
                            );
                        }
                    }
                }
            }
            return Ok(());
        }
        if record.channel != 102 {
            return Ok(());
        }
        let Some(graph) = &record.graph else {
            self.diagnostics.push(format!(
                "{}: incomplete native graph",
                record.identity.element_id
            ));
            let source = json!({"owner_id":record.identity.element_id,"owner_unique_id":record.identity.unique_id,"source_field":"validated_current_index","graph_status":record.status});
            let mut fields = BTreeMap::new();
            put(
                &mut fields,
                "ID".into(),
                json!(record.identity.element_id),
                source.clone(),
            );
            put(
                &mut fields,
                "UniqueId : String".into(),
                json!(record.identity.unique_id),
                source.clone(),
            );
            if let Some(guid) = self.version_guid(record.identity.stored_revision) {
                put(
                    &mut fields,
                    "VersionGuid : String".into(),
                    json!(guid),
                    json!({"source":source,"stored_revision":record.identity.stored_revision,"history_sha256":self.history_sha256}),
                );
            }
            let e = Entry {
                id: record.identity.element_id,
                uid: record.identity.unique_id.clone(),
                class: record.class_name.clone().unwrap_or_default(),
                root: Value::Null,
                source,
                name: None,
                fields,
                type_id: None,
                computed_type_id: None,
                family_id: None,
                category_id: None,
                partition_id: record.identity.partition_id,
                level_elevation: None,
                parameters: BTreeMap::new(),
            };
            ensure!(
                self.entries.insert(e.id, e).is_none(),
                "duplicate metadata index owner"
            );
            return Ok(());
        };
        ensure!(
            graph.consumed_bytes == record.source.body_bytes,
            "metadata requires exact owner EOF"
        );
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let root_value = json!(root.fields);
        let source = json!({"owner_id":record.identity.element_id,"owner_unique_id":record.identity.unique_id,"body_sha256":record.source.body_sha256,"stream":record.source.stream,"group_record_offset":record.source.group_record_offset});
        let owned =
            |field: &str, class: &str| -> Option<(usize, &crate::native_parameters::GraphObject)> {
                let offset = root.fields.get(field)?.get("offset")?.as_u64()? as usize;
                let matches = graph
                    .edges
                    .iter()
                    .filter(|e| e.source_object_index == 0 && e.pointer_offset == offset)
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return None;
                }
                let i = matches[0].target_object_index;
                let o = graph.objects.get(i)?;
                if o.class_name == class {
                    Some((i, o))
                } else {
                    None
                }
            };
        let symbol = owned("m_symbolInfo", "SymbolInfo");
        let material = owned("m_pMaterial", "Material");
        let compound = owned("m_pCompoundStructure", "CompoundStructure");
        let level_elevation = if root.class_name == "Level" {
            owned("m_pSurface", "Plane").and_then(|(_, p)| {
                let x = p.fields.get("m_xVec")?.as_array()?;
                let y = p.fields.get("m_yVec")?.as_array()?;
                if x.get(2)?.as_f64()?.abs() > 1e-12 || y.get(2)?.as_f64()?.abs() > 1e-12 {
                    return None;
                }
                p.fields.get("m_origin")?.get(2)?.as_f64()
            })
        } else {
            None
        };
        let mut parameters = BTreeMap::new();
        let name = symbol
            .and_then(|(_, o)| o.fields.get("m_name"))
            .or_else(|| material.and_then(|(_, o)| o.fields.get("m_name")))
            .or_else(|| root.fields.get("m_name"))
            .or_else(|| {
                if root.class_name == "Level" {
                    root.fields.get("m_text")
                } else {
                    None
                }
            })
            .and_then(Value::as_str)
            .map(str::to_owned);
        let type_id = match root.class_name.as_str() {
            "FamilyInstance" => positive(&root_value["m_masterSymbolId"]),
            "SWall" => positive(&root_value["m_WallAttributesId"]),
            "Floor" => positive(&root_value["m_floorAttributesId"]),
            _ => None,
        };
        let family_id = positive(&root_value["m_familyId"]);
        let category_id = id(&root_value["m_categoryId"]).or(match root.class_name.as_str() {
            "SWall" | "BasicWallType" | "NewCurtainWallType" => Some(-2000011),
            "Floor" | "FloorAttributes" => Some(-2000032),
            "MaterialElem" => Some(-2000700),
            "RbsPipeCurve" => Some(-2008044),
            "RbsDuctCurve" => Some(-2008000),
            _ => None,
        });
        let mut fields = BTreeMap::new();
        let evidence = |field: &str, object: usize| json!({"record":source,"source_field":field,"source_object":object});
        put(
            &mut fields,
            "ID".into(),
            json!(record.identity.element_id),
            evidence("native_index.element_id", 0),
        );
        put(
            &mut fields,
            "UniqueId : String".into(),
            json!(record.identity.unique_id),
            evidence("native_index.unique_id", 0),
        );
        if let Some(n) = &name {
            put(
                &mut fields,
                "Name : String".into(),
                json!(n),
                evidence("owned_name", symbol.or(material).map_or(0, |x| x.0)),
            );
        }
        if let Some(version) = self.version_guid(record.identity.stored_revision) {
            put(
                &mut fields,
                "VersionGuid : String".into(),
                json!(version),
                json!({"record":source,"source_field":"native_index.stored_revision_to_history_episode_guid","episode_index":record.identity.stored_revision,"history_sha256":self.history_sha256}),
            );
        }
        let is_type = symbol.is_some();
        if let Some(n) = name.as_ref().filter(|_| is_type) {
            put(
                &mut fields,
                "Type Name : String".into(),
                json!(n),
                evidence("m_symbolInfo.m_name", symbol.unwrap().0),
            );
        }
        if let Some(guid) = &record.derived_default_ifc_guid {
            put(
                &mut fields,
                if is_type {
                    "Type IfcGUID : String"
                } else {
                    "IfcGUID : String"
                }
                .into(),
                json!(guid),
                evidence("native_saved_identity_derived_ifc_guid", 0),
            );
        }
        if let Some(metadata) = &record.saved_metadata {
            let definitions = metadata
                .parameter_definitions
                .iter()
                .map(|d| (d.parameter_id, d))
                .collect::<BTreeMap<_, _>>();
            let mut add_parameter = |pid: i64,
                                     storage: &str,
                                     value: Value,
                                     object: usize,
                                     path: &str| {
                let known = label(pid)
                    .map(|(c, t)| (c.to_string(), t.to_string()))
                    .or_else(|| {
                        definitions.get(&pid).map(|d| {
                            (
                                d.caption.clone(),
                                if d.definition_class == "ParamDefYesNo" {
                                    "Boolean".into()
                                } else {
                                    storage.to_string()
                                },
                            )
                        })
                    });
                if let Some((caption, mut suffix)) = known {
                    let mut value = value;
                    if storage == "ElementId" {
                        suffix = "String".into();
                        value = json!({"unresolved_element_reference":value});
                    } else if suffix == "Boolean" {
                        if let Some(n) = value.as_i64() {
                            if n == 0 || n == 1 {
                                value = json!(n == 1);
                            } else {
                                return;
                            }
                        }
                    }
                    put(
                        &mut fields,
                        format!("{caption} : {suffix}"),
                        value,
                        json!({"record":source,"source_object":object,"parameter_id":pid,"source_field":path}),
                    );
                }
            };
            for set in &metadata.parameter_sets {
                for p in &set.parameters {
                    add_parameter(
                        p.serialized_parameter_id,
                        &p.storage_type,
                        p.raw_value.clone(),
                        set.source_object,
                        &set.root_pointer_field,
                    );
                }
            }
            for p in &metadata.raw_field_parameters {
                add_parameter(
                    p.parameter_id,
                    &p.storage_type,
                    p.raw_value.clone(),
                    p.source_object,
                    &p.source_field,
                );
            }
            for p in &metadata.resolved_family_parameters {
                add_parameter(
                    p.parameter_id,
                    &p.storage_type,
                    p.raw_value.clone(),
                    p.source_object,
                    "resolved_family_parameters",
                );
            }
            for p in &metadata.family_parameter_slots {
                if let Some((_, storage)) = label(p.parameter_id) {
                    let value = match storage {
                        "Double" => p.double_slot.clone(),
                        "String" => p.string_slot.clone(),
                        "Integer" => p.integer_slot.clone(),
                        _ => continue,
                    };
                    add_parameter(
                        p.parameter_id,
                        storage,
                        value,
                        p.source_object,
                        "typed_builtin_family_slot",
                    );
                }
            }
        }
        let mappings: &[(&str, &str)] = match root.class_name.as_str() {
            "Floor" => &[
                ("m_top", "Elevation at Top : Double"),
                ("m_bottom", "Elevation at Bottom : Double"),
                ("m_roomBounding", "Room Bounding : Boolean"),
            ],
            "SWall" => &[
                ("m_roomBounding", "Room Bounding : Boolean"),
                ("m_isStructuralSignificant", "Structural : Boolean"),
                ("m_wallKeyRef", "Location Line : Integer"),
                ("m_wallCrossSection", "Cross-Section : Integer"),
            ],
            "BasicWallType" => &[("m_function", "Function : Integer")],
            "NewCurtainWallType" => &[
                ("m_function", "Function : Integer"),
                ("m_allowAutoEmbed", "Automatically Embed : Boolean"),
                ("m_autoJoinCond", "Join Condition : Integer"),
            ],
            _ => &[],
        };
        for (field, key) in mappings {
            if let Some(v) = root.fields.get(*field) {
                put(&mut fields, (*key).into(), v.clone(), evidence(field, 0));
            }
        }
        if root.class_name == "SWall" {
            if let Some((driver, _)) = owned_child(graph, 0, "m_pCurveDriver", "VWallDriver") {
                if let Some((curve, line)) = owned_child(graph, driver, "m_pCrv", "GLine") {
                    let direction = line
                        .fields
                        .get("m_dirVec")
                        .and_then(Value::as_array)
                        .and_then(|v| v.iter().map(Value::as_f64).collect::<Option<Vec<_>>>());
                    let end = line
                        .fields
                        .get("m_endParams")
                        .and_then(Value::as_array)
                        .and_then(|v| v.iter().map(Value::as_f64).collect::<Option<Vec<_>>>());
                    if let (Some(d), Some(e)) = (direction, end) {
                        if d.len() == 3 && e.len() == 2 {
                            let length =
                                (e[1] - e[0]).abs() * d.iter().map(|x| x * x).sum::<f64>().sqrt();
                            if length.is_finite() && length > 0. {
                                put(
                                    &mut fields,
                                    "Length : Double".into(),
                                    json!(length),
                                    evidence("m_pCurveDriver.m_pCrv.GLine", curve),
                                );
                            }
                        }
                    }
                }
            }
        }
        if let Some((i, m)) = material {
            if let Some(v) = m.fields.get("m_glow") {
                put(
                    &mut fields,
                    "Glow : Boolean".into(),
                    v.clone(),
                    evidence("m_pMaterial.m_glow", i),
                );
            }
        }
        if root.class_name == "NewCurtainWallType" {
            if let Some((i, rule)) = owned("m_oVLineSpec", "SpacingRule") {
                for (field, key) in [
                    ("m_spacing", "Spacing : Double"),
                    ("m_layoutType", "Layout : Integer"),
                    ("m_adjustBorder", "Adjust for Mullion Size : Boolean"),
                ] {
                    if let Some(value) = rule.fields.get(field) {
                        put(
                            &mut fields,
                            key.into(),
                            value.clone(),
                            evidence(
                                &format!(
                                    "m_oVLineSpec.{field}:exported_duplicate_caption_projection"
                                ),
                                i,
                            ),
                        );
                    }
                }
            }
            for (value, key, path) in [
                (&root_value["m_panel"], "Curtain Panel : String", "m_panel"),
                (
                    &root_value["m_intMullions"][0],
                    "Interior Type : String",
                    "m_intMullions[0]",
                ),
                (
                    &root_value["m_bordMullions"][0][0],
                    "Border 1 Type : String",
                    "m_bordMullions[0][0]",
                ),
                (
                    &root_value["m_bordMullions"][1][0],
                    "Border 2 Type : String",
                    "m_bordMullions[1][0]",
                ),
            ] {
                if let Some(target) = id(value) {
                    put(
                        &mut fields,
                        key.into(),
                        json!({"unresolved_element_reference":target}),
                        evidence(path, 0),
                    );
                }
            }
        }
        if root.class_name == "Floor" {
            if let Some((perimeter, indices)) = floor_perimeter(graph) {
                put(
                    &mut fields,
                    "Perimeter : Double".into(),
                    json!(perimeter),
                    json!({"record":source,"source_field":"FloorExtrusionHelper complete closed CurveLoops all GLine lengths","source_objects":indices}),
                );
            }
        }
        if let Some((i, m)) = material {
            if let Some(map) = m.fields.get("m_assetMap") {
                if map["m_ownedAssets"].as_array().is_some_and(Vec::is_empty)
                    && map["m_ownedAssetIndexMap"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
                {
                    if let Some(rows) = map["m_sharedAssetIdMap"].as_array() {
                        let matches = rows
                            .iter()
                            .filter(|r| {
                                r["first"]["m_value"]["m_guid"]["guid_bytes"]
                                    == json!([
                                        45, 246, 201, 237, 228, 72, 158, 74, 171, 185, 34, 200,
                                        102, 76, 1, 44
                                    ])
                            })
                            .collect::<Vec<_>>();
                        if matches.len() == 1 {
                            if let Some(target) = positive(&matches[0]["second"]) {
                                self.structural_assets.insert(record.identity.element_id,(target,evidence("m_assetMap shared structural GUID edc9f62d-48e4-4a9e-abb9-22c8664c012c",i)));
                            }
                        }
                    }
                }
            }
        }
        if root.class_name == "GeoLocation" {
            if let Some((index, instance)) =
                owned_child(graph, 0, "m_pInstanceInfo", "InstanceInfo")
            {
                if let Some(origin) = instance
                    .fields
                    .get("m_Trf")
                    .and_then(|t| t.get("m_or"))
                    .and_then(Value::as_array)
                    .filter(|a| {
                        a.len() == 3 && a.iter().all(|v| v.as_f64().is_some_and(f64::is_finite))
                    })
                {
                    self.geo_elevations.insert(record.identity.element_id,(origin[2].as_f64().unwrap(),json!({"record":source,"source_object":index,"source_field":"InstanceInfo.m_Trf.m_or[2]"})));
                }
            }
        }
        if root.class_name == "NewCurtainWallType"
            && root_value["m_oULineSpec"]["pointer_token"].as_u64() == Some(0)
            && root_value["m_oVLineSpec"]["pointer_token"].as_u64() == Some(0)
        {
            for (key, value) in [
                ("Layout : Integer", json!(0)),
                ("Spacing : Double", json!(0.0)),
                ("Adjust for Mullion Size : Boolean", json!(false)),
            ] {
                put(
                    &mut fields,
                    key.into(),
                    value,
                    evidence(
                        "both owned U/V LineSpec null -> qualified native layout/spacing/adjust defaults",
                        0,
                    ),
                );
            }
        }
        let system_family = match root.class_name.as_str() {
            "DirectShapeType" => {
                Some(("Coordination Model", json!({"native_family_name_enum":106})))
            }
            "RbsPipeType" => Some(("Pipe Types", json!({"native_family_name_enum":81}))),
            "AbsDuctType" => [
                ("AbsSysCircSweepProfile", "Round Duct", 37),
                ("AbsSysRectSweepProfile", "Rectangular Duct", 36),
                ("AbsSysOvalSweepProfile", "Oval Duct", 38),
            ]
            .iter()
            .find_map(|(class, name, number)| {
                owned_child(graph, 0, "m_Profile", class).map(|(index, _)| {
                    (
                        *name,
                        json!({"native_family_name_enum":number,"owned_profile_index":index}),
                    )
                })
            }),
            _ => None,
        };
        if let Some((name, branch)) = system_family {
            self.system_family_names.insert(record.identity.element_id,(name.into(),json!({"record":source,"native_getter":branch,"vocabulary_profile":"DDC18.4.3 g_FamilyNames"})));
        }
        if let Some((i, c)) = compound {
            self.compound_core.insert(
                record.identity.element_id,
                (
                    json!(c.fields),
                    evidence("m_pCompoundStructure core boundaries and layer widths", i),
                ),
            );
            if let Some(layers) = c.fields.get("m_layers").and_then(Value::as_array) {
                let mut mapped = MaterialLayerMap::new();
                let mut valid = true;
                for layer in layers {
                    if let (Some(material), Some(width), Some(function)) = (
                        positive(&layer["m_materialId"]),
                        layer["m_layerWidth"].as_f64(),
                        layer["m_layerFunction"].as_i64(),
                    ) {
                        if !width.is_finite()||width<0.||function!=1||mapped.insert(material as i64,(width,function,json!({"record":source,"source_object":i,"raw_layer":layer}))).is_some() {valid=false;}
                    } else {
                        valid = false;
                    }
                }
                if valid && !mapped.is_empty() {
                    self.material_layers
                        .insert(record.identity.element_id, mapped);
                }
            }
            if let Some(layers) = c.fields.get("m_layers").and_then(Value::as_array) {
                let widths = layers
                    .iter()
                    .map(|x| x.get("m_layerWidth").and_then(Value::as_f64))
                    .collect::<Option<Vec<_>>>();
                if let Some(widths) = widths.filter(|w| w.iter().all(|v| v.is_finite() && *v >= 0.))
                {
                    let key = if root.class_name == "FloorAttributes" {
                        "Default Thickness : Double"
                    } else {
                        "Width : Double"
                    };
                    put(
                        &mut fields,
                        key.into(),
                        json!(widths.iter().sum::<f64>()),
                        evidence("m_pCompoundStructure.m_layers.m_layerWidth", i),
                    );
                }
            }
            if let Some(layers) = c.fields.get("m_layers").and_then(Value::as_array) {
                if layers.len() == 1 {
                    if let Some(width) = layers[0]["m_layerWidth"]
                        .as_f64()
                        .filter(|v| v.is_finite() && *v > 0.)
                    {
                        self.single_layer_widths.insert(
                            record.identity.element_id,
                            (
                                width,
                                evidence("m_pCompoundStructure.single_layer.m_layerWidth", i),
                            ),
                        );
                    }
                }
                if root.class_name == "FloorAttributes" {
                    if let Some(index) = c
                        .fields
                        .get("m_structuralMaterialLayerIndex")
                        .and_then(Value::as_i64)
                    {
                        let target = if index == -1 {
                            Some(-1)
                        } else {
                            usize::try_from(index)
                                .ok()
                                .and_then(|j| layers.get(j))
                                .and_then(|v| id(&v["m_materialId"]))
                        };
                        if let Some(target) = target {
                            put(
                                &mut fields,
                                "Structural Material : String".into(),
                                json!({"unresolved_element_reference":target}),
                                evidence(
                                    "m_structuralMaterialLayerIndex -> m_layers[index].m_materialId",
                                    i,
                                ),
                            );
                        }
                    }
                }
            }
            if let Some(target) = c.fields.get("m_coarseScaleFillPatternElemId").and_then(id) {
                put(
                    &mut fields,
                    "Coarse Scale Fill Pattern : String".into(),
                    json!({"unresolved_element_reference":target}),
                    evidence("m_coarseScaleFillPatternElemId", i),
                );
            }
            for (field, key) in [
                (
                    "m_coarseScaleFillColor",
                    "Coarse Scale Fill Color : Integer",
                ),
                ("m_openingWrapping", "Wrapping at Inserts : Integer"),
                ("m_endCap", "Wrapping at Ends : Integer"),
            ] {
                if let Some(v) = c.fields.get(field) {
                    put(&mut fields, key.into(), v.clone(), evidence(field, i));
                }
            }
        }
        if root.class_name == "SWall" {
            if let Some((i, spacing)) =
                owned_child(graph, 0, "m_drivenCGLineDataV", "SpacingRuleInst")
            {
                for (field, key) in [
                    ("m_angle", "Angle : Double"),
                    ("m_fixedNum", "Number : Integer"),
                    ("m_justify", "Justification : Integer"),
                    ("m_origin", "Offset : Double"),
                ] {
                    if let Some(v) = spacing.fields.get(field) {
                        put(&mut fields, key.into(), v.clone(), evidence(field, i));
                    }
                }
            }
        }
        if root.class_name == "FamilyInstance" {
            if let Some(v) = root.fields.get("m_useOffsetPos").filter(|v| v.is_boolean()) {
                put(
                    &mut fields,
                    "Moves With Grids : Boolean".into(),
                    v.clone(),
                    evidence(
                        "m_useOffsetPos; native builtin -1001371/-1001370 shared getter",
                        0,
                    ),
                );
            }
            if let Some(v) = root.fields.get("m_roomBounding") {
                put(
                    &mut fields,
                    "Room Bounding : Boolean".into(),
                    v.clone(),
                    evidence("m_roomBounding", 0),
                );
            }
            if let Some(target) = root.fields.get("m_hostId").and_then(id) {
                put(
                    &mut fields,
                    "Host Id : String".into(),
                    json!({"unresolved_element_reference":target}),
                    evidence("m_hostId", 0),
                );
            }
        }
        if root.class_name == "FamilyInstance"
            && root_value["m_useOffsetPos"].as_bool() == Some(true)
            && root_value["m_pCurveDriver"]["pointer_token"].as_u64() == Some(0)
        {
            if let Some(refs) = root_value["m_refs"].as_array().filter(|a| a.len() == 2) {
                let resolved = refs
                    .iter()
                    .map(|v| {
                        crate::native_saved_mesh::pointer(graph, 0, v)
                            .ok()
                            .flatten()
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(indices) = resolved.filter(|indices| {
                    indices
                        .iter()
                        .all(|&i| graph.objects[i].class_name == "FamilyLevelRef")
                }) {
                    for (end, &i) in indices.iter().enumerate() {
                        let r = &graph.objects[i];
                        let label = if end == 0 { "Base" } else { "Top" };
                        if let Some(value) = r
                            .fields
                            .get("m_offset")
                            .and_then(Value::as_f64)
                            .filter(|v| v.is_finite())
                        {
                            put(
                                &mut fields,
                                format!("{label} Offset : Double"),
                                json!(value),
                                evidence("m_refs[index].FamilyLevelRef.m_offset", i),
                            );
                        }
                        if let Some(target) = r.fields.get("m_elemId").and_then(id) {
                            put(
                                &mut fields,
                                format!("{label} Level : String"),
                                json!({"unresolved_element_reference":target}),
                                evidence("m_refs[index].FamilyLevelRef.m_elemId", i),
                            );
                        }
                    }
                }
            }
        }

        if root.class_name == "SWall" {
            if let Some(structural) = root_value["m_isStructuralSignificant"].as_bool() {
                let absence = if let Some((i, cells)) =
                    owned_child(graph, 0, "m_cellList", "CellList")
                {
                    cells.fields.get("m_cells").and_then(Value::as_array).and_then(|pointers|pointers.iter().map(|p|crate::native_saved_mesh::pointer(graph,i,p).ok().flatten()).collect::<Option<Vec<_>>>()).filter(|indices|indices.iter().all(|&j|graph.objects[j].class_name!="AnalyticalSettingsCell")).map(|indices|json!({"source_object":i,"cell_indices":indices,"source_field":"m_cells"}))
                } else if root_value["m_cellList"]["pointer_token"].as_u64() == Some(0) {
                    Some(json!({"source_field":"m_cellList","pointer_token":0}))
                } else {
                    None
                };
                if !structural || absence.is_some() {
                    put(
                        &mut fields,
                        "Enable Analytical Model : Boolean".into(),
                        json!(false),
                        json!({"record":source,"source_field":"m_isStructuralSignificant","structural":structural,"analytical_settings_absence":absence,"interpretation":"qualified_VWall_builtin_-1001552_false_structural_or_missing_settings"}),
                    );
                }
            }
        }
        if root.class_name == "SWall" {
            if let Some((i, steps)) = owned_child(graph, 0, "m_geomSteps", "GeomStepList") {
                if let Some(pointers) = steps
                    .fields
                    .get("m_bRepAdjustGList")
                    .and_then(Value::as_array)
                {
                    let resolved = pointers
                        .iter()
                        .map(|p| {
                            crate::native_saved_mesh::pointer(graph, i, p)
                                .ok()
                                .flatten()
                        })
                        .collect::<Option<Vec<_>>>();
                    if let Some(indices) = resolved {
                        let ends = indices
                            .iter()
                            .filter(|&&j| graph.objects[j].class_name == "JoinToRoofGStep")
                            .map(|&j| {
                                graph.objects[j]
                                    .fields
                                    .get("m_baseOrTop")
                                    .and_then(Value::as_i64)
                                    .filter(|e| matches!(e, 0 | 1))
                            })
                            .collect::<Option<Vec<_>>>();
                        if let Some(ends) = ends {
                            for (end, key) in [
                                (0, "Base is Attached : Boolean"),
                                (1, "Top is Attached : Boolean"),
                            ] {
                                put(
                                    &mut fields,
                                    key.into(),
                                    json!(ends.contains(&end)),
                                    json!({"record":source,"source_object":i,"source_field":"m_bRepAdjustGList","resolved_step_indices":indices,"join_to_roof_ends":ends,"requested_end":end,"interpretation":"qualified_VWall_attachment_getter_complete_adjust_list_JoinToRoofGStep_baseOrTop"}),
                                );
                            }
                        }
                    }
                }
            }
        }
        if matches!(root.class_name.as_str(), "SWall" | "Floor")
            && root_value["m_oDefiningFaceRefs"]["pointer_token"].as_u64() == Some(0)
        {
            put(
                &mut fields,
                "Related to Mass : Boolean".into(),
                json!(false),
                evidence(
                    "m_oDefiningFaceRefs:null -> qualified HostObj -1001713 getter",
                    0,
                ),
            );
        }
        for (class, field, key) in [
            (
                "TaperableWallTypeAngleParametersCell",
                "m_defaultTaperedInteriorAngle",
                "Default Interior Angle : Double",
            ),
            (
                "TaperableWallTypeAngleParametersCell",
                "m_defaultTaperedExteriorAngle",
                "Default Exterior Angle : Double",
            ),
            (
                "TaperableWallTypeWidthAtParametersCell",
                "m_widthMeasuredAt",
                "Width Measured At : Integer",
            ),
        ] {
            if let Some((i, cell)) = owned_cell(graph, class) {
                if let Some(value) = cell.fields.get(field) {
                    put(&mut fields, key.into(), value.clone(), evidence(field, i));
                }
            }
        }
        if root.class_name == "FamilyInstance" {
            if let Some((i, door)) = owned_child(graph, 0, "m_oFamInstSpec", "FamInstDoor") {
                if let Some(value) = door.fields.get("m_doorNumber").and_then(Value::as_str) {
                    put(
                        &mut fields,
                        "Mark : String".into(),
                        json!(value),
                        evidence("m_oFamInstSpec.FamInstDoor.m_doorNumber", i),
                    );
                }
            }
        }
        if let Some((i, properties)) = owned_cell(graph, "AnalyticalPropertiesCell") {
            for (field, key) in [
                ("m_absorptance", "Absorptance : Double"),
                ("m_roughness", "Roughness : Integer"),
            ] {
                if let Some(value) = properties.fields.get(field) {
                    put(&mut fields, key.into(), value.clone(), evidence(field, i));
                }
            }
        }
        if root.class_name == "FloorAttributes" {
            if let Some(v) = root.fields.get("m_structHiddenViewDisplayType") {
                put(
                    &mut fields,
                    "Display in Hidden Views : Integer".into(),
                    v.clone(),
                    evidence("m_structHiddenViewDisplayType", 0),
                );
            }
        }
        if root.class_name == "SWall" && root_value["m_wallStructuralUsage"].as_i64() == Some(1) {
            if let Some(structural) = root_value["m_isStructuralSignificant"].as_bool() {
                put(
                    &mut fields,
                    "Structural Usage : Integer".into(),
                    json!(if structural { 1 } else { 0 }),
                    evidence("m_wallStructuralUsage=1 + m_isStructuralSignificant", 0),
                );
            }
        }
        if let Some(metadata) = &record.saved_metadata {
            for set in &metadata.parameter_sets {
                for p in &set.parameters {
                    parameters.insert(p.serialized_parameter_id, p.raw_value.clone());
                }
            }
        }
        for (pid, key) in [
            (-1019012, "Export to IFC : Integer"),
            (-1019013, "Export Type to IFC : Integer"),
        ] {
            if let Some((value, defaulted)) = ifc_export_integer(&parameters, pid) {
                put(
                    &mut fields,
                    key.into(),
                    json!(value),
                    json!({"record":source,"parameter_id":pid,"source_field":"complete_saved_builtin_parameter_map","defaulted":defaulted,"interpretation":"qualified_native_getBuiltInParameter_integer","binary_rule":"only -1019013 and -1019012 default to zero after missing-value status26; stored integer wins"}),
                );
            }
        }

        for pid in -1019017..=-1019014 {
            let value = match parameters.get(&pid) {
                Some(v) => v.as_str(),
                None => Some(""),
            };
            if let Some(value) = value {
                if let Some((caption, _)) = label(pid) {
                    put(
                        &mut fields,
                        format!("{caption} : String"),
                        json!(value),
                        json!({"record":source,"parameter_id":pid,"source_field":"complete_saved_builtin_parameter_map","defaulted":!parameters.contains_key(&pid),"interpretation":"qualified_native_string_getter_only_four_IFC_ids_empty_on_status26"}),
                    );
                }
            }
        }
        if root.class_name == "FamilySymbol"
            && !parameters.contains_key(&-1005439)
            && !fields.contains_key("Define Thermal Properties by : Integer")
        {
            let named_absent = if root_value["m_pParams"]["pointer_token"].as_u64() == Some(0) {
                true
            } else {
                owned_child(graph, 0, "m_pParams", "FamilyParams")
                    .and_then(|(_, p)| p.fields.get("m_params"))
                    .and_then(Value::as_array)
                    .is_some_and(|rows| {
                        rows.iter()
                            .all(|r| id(&r["m_paramId"]).is_some_and(|p| p != -1005439))
                    })
            };
            if named_absent {
                put(
                    &mut fields,
                    "Define Thermal Properties by : Integer".into(),
                    json!(1),
                    json!({"record":source,"source_field":"named FamilyParams and built-in parameter map absence","parameter_id":-1005439,"interpretation":"qualified FamilySymbol.getThermalPropertiesMethod default1"}),
                );
            }
        }
        if root.class_name == "FamilySymbol"
            && !fields.contains_key("Analytic Construction : String")
        {
            if let Some(method) = fields
                .get("Define Thermal Properties by : Integer")
                .and_then(|f| f.value.as_i64())
            {
                let named = if root_value["m_pParams"]["pointer_token"].as_u64() == Some(0) {
                    Some(Vec::new())
                } else {
                    owned_child(graph, 0, "m_pParams", "FamilyParams")
                        .and_then(|(_, p)| p.fields.get("m_params"))
                        .and_then(Value::as_array)
                        .cloned()
                };
                if let Some(rows) = named {
                    let ids = rows
                        .iter()
                        .map(|r| id(&r["m_paramId"]))
                        .collect::<Option<Vec<_>>>();
                    if let Some(ids) = ids {
                        let construction = rows
                            .iter()
                            .zip(ids)
                            .filter(|(_, pid)| *pid == -1005438)
                            .map(|(r, _)| r)
                            .collect::<Vec<_>>();
                        let invalid_id = construction.is_empty()
                            || (construction.len() == 1
                                && construction[0]["m_str"].as_str() == Some(""));
                        if method == 2 || (matches!(method, 0 | 1) && invalid_id) {
                            put(
                                &mut fields,
                                "Analytic Construction : String".into(),
                                json!("<None>"),
                                json!({"record":source,"thermal_method":method,"construction_type_id_rows":construction,"native_formatter":"ParamDefComboAnalyticConstruction enum -1","vocabulary":"analyticConstructionLookupTable-1.0.0","interpretation":"qualified unavailable thermal construction formatter branch"}),
                            );
                        }
                    }
                }
            }
        }
        if let Some((cell_index, _)) = owned_cell(graph, "AnalyticalElementParametersCell") {
            if let Some((locals, repository)) = &self.associations {
                put(
                    &mut fields,
                    "Has Association : Boolean".into(),
                    json!(locals.contains(&record.identity.element_id)),
                    json!({"record":source,"source_object":cell_index,"repository":repository,"interpretation":"owned AnalyticalElementParametersCell native local association membership"}),
                );
            }
        }
        if let Some((cell_index, cell)) = owned_cell(graph, "CoverSettingsCell") {
            if cell.fields.get("m_active").and_then(Value::as_bool) == Some(true) {
                if let Some(rows) = cell
                    .fields
                    .get("m_stableFaceRefsMap")
                    .and_then(Value::as_array)
                {
                    for (pid, label) in [
                        (-1013435, "Rebar Cover - Exterior Face"),
                        (-1013436, "Rebar Cover - Interior Face"),
                        (-1013437, "Rebar Cover - Other Faces"),
                        (-1013438, "Rebar Cover - Top Face"),
                        (-1013439, "Rebar Cover - Bottom Face"),
                    ] {
                        let settings = rows
                            .iter()
                            .filter(|r| id(&r["second"]["m_paramId"]) == Some(pid))
                            .map(|r| id(&r["second"]["m_setting"]))
                            .collect::<Option<Vec<_>>>();
                        if let Some(settings) =
                            settings.filter(|v| !v.is_empty() && v.iter().all(|x| *x == v[0]))
                        {
                            put(
                                &mut fields,
                                format!("{label} : String"),
                                json!({"unresolved_element_reference":settings[0]}),
                                json!({"record":source,"source_object":cell_index,"parameter_id":pid,"source_field":"m_stableFaceRefsMap unanimous matching m_paramId settings"}),
                            );
                        }
                    }
                }
            }
        }
        let entry = Entry {
            id: record.identity.element_id,
            uid: record.identity.unique_id.clone(),
            class: root.class_name.clone(),
            root: root_value,
            source,
            name,
            fields,
            type_id,
            computed_type_id: owned_child(graph, 0, "m_pInstanceInfo", "InstanceInfo")
                .and_then(|(_, o)| o.fields.get("m_symbolId"))
                .and_then(positive),
            family_id,
            category_id,
            partition_id: record.identity.partition_id,
            level_elevation,
            parameters,
        };
        ensure!(
            self.entries.insert(entry.id, entry).is_none(),
            "duplicate current metadata owner"
        );
        Ok(())
    }
    /// Explicitly qualified geometry metrics; generic surface area is not accepted
    /// as semantic element Area by this interface.
    fn material_type(&self, owner_id: u64) -> Option<(i64, Value)> {
        let (asset_id, source) = self.structural_assets.get(&owner_id)?;
        let asset = self
            .entries
            .get(asset_id)
            .filter(|a| a.class == "PropertySetElement")?;
        let raw = asset.parameters.get(&-1150464)?.as_i64()?;
        let value = match raw {
            0 => 0,
            1 => 4,
            2 => 6,
            3 => 1,
            4 => 2,
            5 => 3,
            _ => 4,
        };
        Some((
            value,
            json!({"material_structural_asset":source,"asset":asset.source,"parameter_id":-1150464,"raw_class":raw,"interpretation":"qualified native material class to MaterialType enum"}),
        ))
    }
    pub fn add_standard_view_positive(&mut self, owner_id: u64, source: Value) -> Result<()> {
        let owner = self
            .entries
            .get_mut(&owner_id)
            .ok_or_else(|| anyhow::anyhow!("standard view owner absent"))?;
        put(
            &mut owner.fields,
            "On the standard 3D View : Boolean".into(),
            json!(true),
            source,
        );
        Ok(())
    }
    pub fn material_layer_widths(&self, owner_id: u64) -> Option<Vec<(i64, f64, Value)>> {
        let entry = self.entries.get(&owner_id)?;
        let type_id = entry.type_id.unwrap_or(owner_id);
        self.material_layers.get(&type_id).map(|layers| {
            layers
                .iter()
                .map(|(&id, (width, _, source))| {
                    (
                        id,
                        *width,
                        json!({"owner":entry.source,"type_id":type_id,"layer":source}),
                    )
                })
                .collect()
        })
    }
    pub fn single_layer_width(&self, owner_id: u64) -> Option<(f64, Value)> {
        let entry = self.entries.get(&owner_id)?;
        let type_id = entry.type_id.unwrap_or(owner_id);
        self.single_layer_widths
            .get(&type_id)
            .map(|(width, source)| {
                (
                    *width,
                    json!({"owner":entry.source,"type_id":type_id,"compound_structure":source}),
                )
            })
    }
    pub fn add_material_quantities(
        &mut self,
        owner_id: u64,
        material_id: i64,
        area: f64,
        volume: f64,
        sources: Vec<Value>,
    ) -> Result<()> {
        ensure!(
            material_id > 0
                && area.is_finite()
                && area >= 0.
                && volume.is_finite()
                && volume >= 0.
                && !sources.is_empty(),
            "invalid material quantities or absent source"
        );
        ensure!(
            self.entries.contains_key(&owner_id),
            "material quantity owner absent"
        );
        ensure!(
            self.material_quantities
                .entry(owner_id)
                .or_default()
                .insert(material_id, (Some(area), Some(volume), sources))
                .is_none(),
            "duplicate material quantities"
        );
        Ok(())
    }
    pub fn add_material_area_only(
        &mut self,
        owner_id: u64,
        material_id: i64,
        area: f64,
        sources: Vec<Value>,
    ) -> Result<()> {
        ensure!(
            material_id > 0 && area.is_finite() && area >= 0. && !sources.is_empty(),
            "invalid material area"
        );
        ensure!(self.entries.contains_key(&owner_id), "area owner missing");
        ensure!(
            self.material_quantities
                .entry(owner_id)
                .or_default()
                .insert(material_id, (Some(area), None, sources))
                .is_none(),
            "duplicate material area"
        );
        Ok(())
    }
    fn material_report(
        &self,
        material: &Entry,
        area: Option<f64>,
        volume: Option<f64>,
    ) -> Option<(Vec<String>, Vec<Value>)> {
        if material.class != "MaterialElem" {
            return None;
        }
        let mut report = vec!["Category=[-2000700]".into()];
        if let Some(area) = area {
            report.push(format!("Area={} ft²", decimal_15(area)));
        }
        if let Some(volume) = volume {
            report.push(format!("Volume={} ft³", decimal_15(volume)));
        }
        let mut sources = vec![material.source.clone()];
        let header = self.headers.get(&material.id)?;
        let base = id(&material.root["m_designOptionId"])?;
        let option = if base > 0 {
            base
        } else {
            id(&header.0["m_designOptionId"])?
        };
        report.push(format!(
            "Design Option=[{}]",
            if option > 0 { option } else { -1 }
        ));
        for key in [
            "Name",
            "Color",
            "Shininess",
            "Smoothness",
            "Transparency",
            "Glow",
            "IfcGUID",
            "Export to IFC As",
            "Export to IFC",
            "IFC Predefined Type",
            "Comments",
            "Cost",
            "Description",
            "Keynote",
            "Manufacturer",
            "Mark",
            "Model",
            "URL",
            "Material Type",
        ] {
            let found = material
                .fields
                .iter()
                .find(|(caption, _)| caption.split(" : ").next() == Some(key));
            let Some((_, field)) = found else {
                if matches!(
                    key,
                    "Name"
                        | "Color"
                        | "Shininess"
                        | "Smoothness"
                        | "Transparency"
                        | "Glow"
                        | "IfcGUID"
                        | "Export to IFC As"
                        | "Export to IFC"
                        | "IFC Predefined Type"
                ) {
                    return None;
                } else {
                    continue;
                }
            };
            let rendered = match &field.value {
                Value::String(value) => {
                    if value.is_empty() {
                        "\"\"".into()
                    } else {
                        value.clone()
                    }
                }
                Value::Bool(value) => i32::from(*value).to_string(),
                Value::Number(value) => {
                    if key == "Cost" {
                        format!("{:.15}", value.as_f64()?)
                    } else {
                        value.to_string()
                    }
                }
                _ => return None,
            };
            report.push(format!("{key}={rendered}"));
            sources.extend(field.sources.clone());
        }
        if !report.iter().any(|s| s.starts_with("Material Type=")) {
            if let Some((value, source)) = self.material_type(material.id) {
                report.push(format!("Material Type={value}"));
                sources.push(source);
            }
        }
        report.sort();
        Some((report, sources))
    }
    pub fn add_derived_metrics(
        &mut self,
        owner_id: u64,
        volume: Option<f64>,
        semantic_area: Option<f64>,
        sources: Vec<Value>,
    ) -> Result<()> {
        ensure!(
            !sources.is_empty(),
            "derived metadata metrics require native provenance"
        );
        let entry = self.entries.get_mut(&owner_id).ok_or_else(|| {
            anyhow::anyhow!("derived metrics owner absent from current index metadata")
        })?;
        for (key, value) in [
            ("Volume : Double", volume),
            ("Area : Double", semantic_area),
        ] {
            if let Some(value) = value {
                ensure!(
                    value.is_finite() && value >= 0.,
                    "invalid derived metadata metric"
                );
                put(
                    &mut entry.fields,
                    key.into(),
                    json!(value),
                    json!({"native_metric_sources":sources,"owner_id":owner_id,"source_field":"qualified_geometry_measure"}),
                );
            }
        }
        Ok(())
    }
    fn project_design_option(
        &self,
        entry: &Entry,
        key: &str,
        fields: &mut BTreeMap<String, Field>,
    ) {
        let base = id(&entry.root["m_designOptionId"]);
        let (target, source) = if let Some(target) = base.filter(|v| *v > 0) {
            (
                target,
                json!({"record":entry.source,"source_field":"m_designOptionId","raw_target_id":target}),
            )
        } else if let Some((header, source)) = self.headers.get(&entry.id) {
            let Some(target) = id(&header["m_designOptionId"]) else {
                return;
            };
            (
                target,
                json!({"record":entry.source,"header":source,"source_field":"ElementHeader.m_designOptionId","base_raw_target_id":base,"header_raw_target_id":target,"interpretation":"qualified_base_then_header_regular_id_precedence"}),
            )
        } else {
            return;
        };
        if target <= 0 {
            put(fields, key.into(), json!("None"), source);
        } else if let Some(name) = u64::try_from(target)
            .ok()
            .and_then(|id| self.entries.get(&id))
            .and_then(|e| e.name.as_ref())
        {
            put(fields, key.into(), json!(name), source);
        }
    }
    pub fn finish(self) -> Inventory {
        let mut owners = Vec::new();
        for entry in self.entries.values() {
            let mut fields = entry.fields.clone();
            if let Some(graphics) = self.graphics.get(&entry.id) {
                fields.extend(graphics.clone());
            }
            let mut diagnostics = Vec::new();
            if let Some((header, source)) = self.headers.get(&entry.id) {
                if let Some(name) = id(&header["m_categroryId"]).and_then(category_name) {
                    put(
                        &mut fields,
                        "Category : String".into(),
                        json!(name),
                        json!({"header":source,"source_field":"ElementHeader.m_categroryId","raw_value":header["m_categroryId"]}),
                    );
                }
            }
            if let Some((name, source)) = self.partitions.get(&entry.partition_id) {
                put(
                    &mut fields,
                    "Workset : String".into(),
                    json!(name),
                    json!({"record":entry.source,"partition_catalog":source,"source_field":"native_index.partition_id","raw_partition_id":entry.partition_id}),
                );
            }

            if entry.class == "Floor" {
                if fields
                    .get("Structural : Boolean")
                    .is_some_and(|f| f.value == json!(true))
                {
                    let trackers = self
                        .entries
                        .values()
                        .filter(|e| e.class == "ActiveGeoLocationTrackingElement")
                        .collect::<Vec<_>>();
                    if trackers.len() == 1 {
                        if let Some((geo_id, (height, geo_source))) =
                            positive(&trackers[0].root["m_activeGeoLocationId"])
                                .and_then(|id| self.geo_elevations.get(&id).map(|v| (id, v)))
                        {
                            for (field, key) in [
                                ("m_top", "Elevation at Top Survey : Double"),
                                ("m_bottom", "Elevation at Bottom Survey : Double"),
                            ] {
                                if let Some(value) =
                                    entry.root[field].as_f64().filter(|v| v.is_finite())
                                {
                                    put(
                                        &mut fields,
                                        key.into(),
                                        json!(value - height),
                                        json!({"owner":entry.source,"tracking":trackers[0].source,"active_geo_location_id":geo_id,"geo_location":geo_source,"source_field":field,"interpretation":"stored floor elevation minus native active geolocation origin Z"}),
                                    );
                                }
                            }
                        }
                    }
                }

                if let Some(typ) = entry.type_id.and_then(|id| self.entries.get(&id)) {
                    if fields
                        .get("Structural : Boolean")
                        .is_some_and(|f| f.value == json!(true))
                    {
                        if let (Some(bottom), Some((compound, source))) = (
                            entry.root["m_bottom"].as_f64(),
                            self.compound_core.get(&typ.id),
                        ) {
                            if let (Some(layers), Some(ext), Some(int), Some(variable)) = (
                                compound["m_layers"].as_array(),
                                compound["m_numShellLayersExt"].as_i64(),
                                compound["m_numShellLayersInt"].as_i64(),
                                compound["m_variableLayerIdx"].as_i64(),
                            ) {
                                let widths = layers
                                    .iter()
                                    .map(|l| {
                                        l["m_layerWidth"]
                                            .as_f64()
                                            .filter(|v| v.is_finite() && *v >= 0.)
                                    })
                                    .collect::<Option<Vec<_>>>();
                                if let Some(widths) = widths.filter(|w| {
                                    ext >= 0
                                        && int >= 0
                                        && ext + int < (w.len() as i64)
                                        && variable >= -1
                                        && variable < (w.len() as i64)
                                }) {
                                    let boundary_top = ext - 1;
                                    let boundary_bottom = widths.len() as i64 - 1 - int;
                                    let elevation = |boundary: i64| {
                                        let variable = variable as u64;
                                        if (boundary as u64) < variable
                                            && variable < (widths.len() as u64)
                                        {
                                            None
                                        } else {
                                            Some(
                                                bottom
                                                    + widths[(boundary + 1) as usize..]
                                                        .iter()
                                                        .sum::<f64>(),
                                            )
                                        }
                                    };
                                    if let (Some(top), Some(bottom_core)) =
                                        (elevation(boundary_top), elevation(boundary_bottom))
                                    {
                                        for (key, value) in [
                                            ("Elevation at Top Core : Double", top),
                                            ("Elevation at Bottom Core : Double", bottom_core),
                                            ("Core Thickness : Double", top - bottom_core),
                                        ] {
                                            put(
                                                &mut fields,
                                                key.into(),
                                                json!(value),
                                                json!({"owner":entry.source,"compound":source,"bottom":bottom,"boundary_top":boundary_top,"boundary_bottom":boundary_bottom,"variable_layer":variable,"interpretation":"qualified structural Floor calculateCoreElevation from saved layer boundaries"}),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(thickness) = typ.fields.get("Default Thickness : Double") {
                        put(
                            &mut fields,
                            "Thickness : Double".into(),
                            thickness.value.clone(),
                            json!({"owner":entry.source,"type":typ.source,"compound_structure":thickness.sources,"interpretation":"native Floor thickness from current compoundstructure width"}),
                        );
                    }
                }
            }
            if let Some((value, source)) = self.material_type(entry.id) {
                put(
                    &mut fields,
                    "Material Type : Integer".into(),
                    json!(value),
                    source,
                );
            }
            let typ = entry.type_id.and_then(|i| self.entries.get(&i));
            if entry.class == "FamilySymbol" && entry.name.as_deref() == Some("") {
                if let Some(master) = positive(&entry.root["m_masterId"])
                    .and_then(|id| self.entries.get(&id))
                    .filter(|m| m.class == "FamilySymbol" && m.family_id == entry.family_id)
                {
                    if let Some(name) = &master.name {
                        fields.insert("Name : String".into(),Field{value:json!(name),sources:vec![json!({"record":entry.source,"master":master.source,"source_field":"m_masterId_to_symbol_name"})],interpretation:"native_computed_symbol_master_name"});
                    }
                }
            }
            let fam = entry
                .family_id
                .or_else(|| typ.and_then(|t| t.family_id))
                .and_then(|i| self.entries.get(&i));
            if let Some(family) = fam {
                if let Some(code) = family.root["m_omniClassCode"].as_str() {
                    let key = if entry.class == "FamilySymbol" {
                        "Classification Number : String"
                    } else {
                        "[Type] Classification Number : String"
                    };
                    put(
                        &mut fields,
                        key.into(),
                        json!(code),
                        json!({"record":entry.source,"family":family.source,"source_field":"Family.m_omniClassCode","parameter_id":-1002502}),
                    );
                    let catalog = omni_class_catalog();
                    let title = catalog["entries"]
                        .get(code)
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let title_key = if entry.class == "FamilySymbol" {
                        "Classification Title : String"
                    } else {
                        "[Type] Classification Title : String"
                    };
                    put(
                        &mut fields,
                        title_key.into(),
                        json!(title),
                        json!({"record":entry.source,"family":family.source,"source_field":"Family.m_omniClassCode","code":code,"catalog_profile":catalog["profile"],"catalog_source_binary_sha256":catalog["source_binary_sha256"],"catalog_source_bytes_sha256":catalog["source_bytes_sha256"],"catalog_source_symbol":catalog["source_symbol"],"interpretation":"DDC18.4.3 exact OmniClass vocabulary; unknowncode returns empty"}),
                    );
                }
            }
            if entry.class == "FamilyInstance" {
                if let Some(family) = fam.filter(|f| f.root["m_inPlace"].as_bool() == Some(false)) {
                    let nonstandalone = family.root["m_isCurtainPanel"].as_bool() == Some(true)
                        || family.category_id == Some(-2000171)
                        || family.root["m_refTypeIds"]
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(Value::as_i64)
                            == Some(1);
                    let host =
                        positive(&entry.root["m_hostId"]).and_then(|id| self.entries.get(&id));
                    if nonstandalone && host.is_some_and(|h| h.class == "SWall") {
                        if let Some(sill) =
                            entry.root["m_elevation"].as_f64().filter(|v| v.is_finite())
                        {
                            let proof = json!({"record":entry.source,"family":family.source,"host":host.map(|h|&h.source),"source_field":"nonstandalone ordinary SWall host m_elevation","host_type_ids":family.root["m_refTypeIds"]});
                            put(
                                &mut fields,
                                "Sill Height : Double".into(),
                                json!(sill),
                                proof.clone(),
                            );
                            if let Some(symbol) =
                                entry.computed_type_id.and_then(|id| self.entries.get(&id))
                            {
                                if let Some(height) = symbol
                                    .fields
                                    .get("Height : Double")
                                    .and_then(|f| f.value.as_f64())
                                {
                                    if (sill + height).is_finite() {
                                        put(
                                            &mut fields,
                                            "Head Height : Double".into(),
                                            json!(sill + height),
                                            json!({"sill":proof,"computed_symbol":symbol.source,"height_parameter":-1001300,"height_sources":symbol.fields["Height : Double"].sources}),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let category = entry
                .category_id
                .or_else(|| fam.and_then(|f| f.category_id));
            if let Some(c) = category.and_then(category_name) {
                put(
                    &mut fields,
                    "Category : String".into(),
                    json!(c),
                    json!({"record":entry.source,"family_source":fam.map(|f|&f.source),"source_field":"class_or_native_family_category"}),
                );
            }
            let family_name = fam
                .and_then(|f| f.name.clone())
                .or_else(|| self.system_family_names.get(&entry.id).map(|v| v.0.clone()))
                .or_else(|| match entry.class.as_str() {
                    "BasicWallType" => Some("Basic Wall".into()),
                    "NewCurtainWallType" => Some("Curtain Wall".into()),
                    "SWall" => typ.and_then(|t| match t.class.as_str() {
                        "BasicWallType" => Some("Basic Wall".into()),
                        "NewCurtainWallType" => Some("Curtain Wall".into()),
                        _ => None,
                    }),
                    "FloorAttributes" | "Floor" => Some("Floor".into()),
                    _ => None,
                });
            if let Some(n) = &family_name {
                put(
                    &mut fields,
                    "Family Name : String".into(),
                    json!(n),
                    json!({"record":entry.source,"family_source":fam.map(|f|&f.source),"source_field":"native_family_name_or_system_family_class"}),
                );
            }
            if let Some(t) = typ {
                if let Some(name) = &family_name {
                    put(
                        &mut fields,
                        "[Type] Family Name : String".into(),
                        json!(name),
                        json!({"record":t.source,"source_field":"native_type_family_name"}),
                    );
                }
                self.project_design_option(t, "[Type] Design Option : String", &mut fields);
                if let Some(n) = &t.name {
                    for key in [
                        "Name : String",
                        "Type Name : String",
                        "Family and Type : String",
                        "Family : String",
                        "Type : String",
                        "Type Id : String",
                    ] {
                        put(
                            &mut fields,
                            key.into(),
                            json!(n),
                            json!({"record":entry.source,"type_source":t.source,"source_field":"native_type_reference_to_owned_symbol_name"}),
                        );
                    }
                }
                for (key, value) in &t.fields {
                    if !matches!(key.as_str(), "ID" | "UniqueId : String" | "Name : String") {
                        fields.insert(format!("[Type] {key}"), value.clone());
                    }
                }
                if let Some(c) = category.and_then(category_name) {
                    put(
                        &mut fields,
                        "[Type] Category : String".into(),
                        json!(c),
                        json!({"record":t.source,"source_field":"class_or_native_family_category"}),
                    );
                }
            }
            if entry.class == "SWall" {
                let base =
                    positive(&entry.root["m_assocLevelId"]).and_then(|i| self.entries.get(&i));
                let top = id(&entry.root["m_upToLevelId"]);
                if let Some(b) = base {
                    if let Some(n) = &b.name {
                        put(
                            &mut fields,
                            "Base Constraint : String".into(),
                            json!(n),
                            json!({"record":entry.source,"target":b.source,"source_field":"m_assocLevelId"}),
                        );
                    }
                }
                if top == Some(-1) {
                    put(
                        &mut fields,
                        "Top Constraint : String".into(),
                        json!("None"),
                        json!({"record":entry.source,"source_field":"m_upToLevelId","raw_value":-1}),
                    );
                    if let Some(v) = entry.parameters.get(&-1001105) {
                        put(
                            &mut fields,
                            "Unconnected Height : Double".into(),
                            v.clone(),
                            json!({"record":entry.source,"source_field":"unconnected_wall_parameter","parameter_id":-1001105}),
                        );
                    }
                } else if let Some(t) = top
                    .and_then(|i| u64::try_from(i).ok())
                    .and_then(|i| self.entries.get(&i))
                {
                    if let Some(n) = &t.name {
                        put(
                            &mut fields,
                            "Top Constraint : String".into(),
                            json!(n),
                            json!({"record":entry.source,"target":t.source,"source_field":"m_upToLevelId"}),
                        );
                    }
                    if let (Some(b), Some(te), Some(bo), Some(to)) = (
                        base,
                        t.level_elevation,
                        entry.parameters.get(&-1001108).and_then(Value::as_f64),
                        entry.parameters.get(&-1001109).and_then(Value::as_f64),
                    ) {
                        if let Some(be) = b.level_elevation {
                            let height = te + to - be - bo;
                            if height.is_finite() && height > 0. {
                                put(
                                    &mut fields,
                                    "Unconnected Height : Double".into(),
                                    json!(height),
                                    json!({"record":entry.source,"base_level":b.source,"top_level":t.source,"source_field":"qualified_level_planes_plus_top_minus_base_offset","parameter_ids":[-1001108,-1001109]}),
                                );
                            }
                        }
                    }
                }
            }
            for (native, key) in [
                ("m_createdPhaseId", "Phase Created : String"),
                ("m_demolishedPhaseId", "Phase Demolished : String"),
                ("m_assocLevelId", "Level : String"),
            ] {
                if let Some(target) = id(&entry.root[native]) {
                    if target == -1 {
                        put(
                            &mut fields,
                            key.into(),
                            json!("None"),
                            json!({"record":entry.source,"source_field":native,"raw_target_id":target,"interpretation":"explicit_invalid_element_reference_display"}),
                        );
                    } else if let Some(reference) = u64::try_from(target)
                        .ok()
                        .and_then(|i| self.entries.get(&i))
                    {
                        if let Some(name) = &reference.name {
                            put(
                                &mut fields,
                                key.into(),
                                json!(name),
                                json!({"record":entry.source,"target":reference.source,"source_field":native}),
                            );
                        }
                    }
                }
            }
            self.project_design_option(entry, "Design Option : String", &mut fields);
            for field in fields.values_mut() {
                if let Some(target) = field
                    .value
                    .get("unresolved_element_reference")
                    .and_then(Value::as_i64)
                {
                    if target == -1 {
                        field.value = json!("None");
                    } else if let Some(reference) = u64::try_from(target)
                        .ok()
                        .and_then(|i| self.entries.get(&i))
                    {
                        if let Some(name) = reference.name.as_ref().filter(|n| !n.is_empty()) {
                            field.value = json!(name);
                            field.sources.push(reference.source.clone());
                        } else if let Some(typ) =
                            reference.type_id.and_then(|id| self.entries.get(&id))
                        {
                            if let Some(name) = typ.name.as_ref().filter(|n| !n.is_empty()) {
                                field.value = json!(name);
                                field.sources.push(json!({"reference_owner":reference.source,"reference_type":typ.source,"interpretation":"named_type_of_referenced_instance"}));
                            }
                        }
                    }
                }
            }
            if let Some(layers) = self.material_layers.get(&entry.id) {
                for (&material_id, (width, _function, source)) in layers {
                    if let Some(material) = self.entries.get(&(material_id as u64)) {
                        if let (Some(name), Some((mut report, mut sources))) =
                            (&material.name, self.material_report(material, None, None))
                        {
                            report.push("Function=Structure".into());
                            report.push(format!("Width={}", decimal_15(*width)));
                            report.sort();
                            sources.push(source.clone());
                            fields.insert(
                                format!("Material - {name} : String"),
                                Field {
                                    value: json!(report),
                                    sources,
                                    interpretation: "native_compound_layer_material_report",
                                },
                            );
                        }
                    }
                }
            }
            if let Some(materials) = self.material_quantities.get(&entry.id) {
                for (&material_id, (area, volume, quantity_sources)) in materials {
                    if let Some(material) = u64::try_from(material_id)
                        .ok()
                        .and_then(|id| self.entries.get(&id))
                    {
                        if let Some((report, material_sources)) =
                            self.material_report(material, *area, *volume)
                        {
                            if let Some(name) = &material.name {
                                put(
                                    &mut fields,
                                    format!("Material - {name} : String"),
                                    json!(report),
                                    json!({"owner":entry.source,"material":material.source,"material_sources":material_sources,"quantity_sources":quantity_sources,"interpretation":"native material property report with independently computed unit-qualified quantities"}),
                                );
                            }
                        }
                    }
                }
            }
            fields.retain(|key, field| {
                let good = field.interpretation != "conflicting_native_candidates"
                    && field.value.get("unresolved_element_reference").is_none();
                if !good {
                    diagnostics.push(format!("{key}: conflicting or unresolved projection"));
                }
                good
            });
            owners.push(Owner {
                element_id: entry.id,
                unique_id: entry.uid.clone(),
                class_name: entry.class.clone(),
                fields,
                diagnostics,
            });
        }
        Inventory {
            format: "native-docker-metadata/v1",
            complete_metadata_parity: false,
            owners,
            diagnostics: self.diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: u64, class: &str) -> Entry {
        Entry {
            id,
            uid: format!("native-{id}"),
            class: class.into(),
            root: json!({}),
            source: json!({"owner_id":id}),
            name: None,
            fields: BTreeMap::new(),
            type_id: None,
            computed_type_id: None,
            family_id: None,
            category_id: None,
            partition_id: -1,
            level_elevation: None,
            parameters: BTreeMap::new(),
        }
    }
    #[test]
    fn material_quantity_format_preserves_fifteen_digits_and_small_values() {
        assert_eq!(decimal_15(1.0 / 3.0), "0.333333333333333");
        assert_eq!(decimal_15(7.0 / 6.0), "1.16666666666667");
        assert_eq!(decimal_15(3305.00000000007), "3305.00000000007");
    }
    #[test]
    fn shared_structural_material_type_requires_current_typed_asset() {
        let mut b = Builder::default();
        let mut asset = entry(22, "PropertySetElement");
        asset.parameters.insert(-1150464, json!(3));
        b.entries.insert(22, asset);
        b.structural_assets
            .insert(11, (22, json!({"native_shared_guid":true})));
        assert_eq!(b.material_type(11).map(|v| v.0), Some(1));
        b.entries
            .get_mut(&22)
            .unwrap()
            .parameters
            .insert(-1150464, json!("3"));
        assert!(b.material_type(11).is_none());
        assert!(b.material_type(12).is_none());
    }
    #[test]
    fn ifc_integer_default_is_narrow_and_never_overrides_saved_or_malformed_values() {
        assert_eq!(
            ifc_export_integer(&BTreeMap::new(), -1019012),
            Some((0, true))
        );
        assert_eq!(
            ifc_export_integer(&BTreeMap::new(), -1019013),
            Some((0, true))
        );
        assert_eq!(ifc_export_integer(&BTreeMap::new(), -1019014), None);
        assert_eq!(
            ifc_export_integer(&BTreeMap::from([(-1019012, json!(2))]), -1019012),
            Some((2, false))
        );
        assert_eq!(
            ifc_export_integer(&BTreeMap::from([(-1019012, json!("2"))]), -1019012),
            None
        );
    }
    #[test]
    fn history_episode_storage_is_reversed_and_out_of_range_refuses() {
        let b = Builder::with_history(vec!["new".into(), "old".into()], "history".into());
        assert_eq!(b.version_guid(0).map(String::as_str), Some("old"));
        assert_eq!(b.version_guid(1).map(String::as_str), Some("new"));
        assert_eq!(b.version_guid(2), None);
    }
    #[test]
    fn connected_height_uses_current_level_planes_not_cached_height() {
        let mut b = Builder::default();
        let mut wall = entry(1, "SWall");
        wall.root = json!({"m_assocLevelId":2,"m_upToLevelId":3});
        wall.parameters = BTreeMap::from([
            (-1001108, json!(0.)),
            (-1001109, json!(-0.5)),
            (-1001101, json!(19.3333333333)),
        ]);
        let mut bottom = entry(2, "Level");
        bottom.level_elevation = Some(166.);
        bottom.name = Some("Base".into());
        let mut top = entry(3, "Level");
        top.level_elevation = Some(185.5);
        top.name = Some("Top".into());
        b.entries = BTreeMap::from([(1, wall), (2, bottom), (3, top)]);
        let out = b.finish();
        assert_eq!(
            out.owners[0].fields["Unconnected Height : Double"].value,
            json!(19.)
        );
        assert_eq!(
            out.owners[0].fields["Top Constraint : String"].value,
            json!("Top")
        );
    }
    #[test]
    fn computed_symbol_name_preserves_owner_identity_and_master_source() {
        let mut b = Builder::default();
        let mut computed = entry(1, "FamilySymbol");
        computed.name = Some("".into());
        computed.family_id = Some(9);
        computed.root = json!({"m_masterId":2});
        let mut master = entry(2, "FamilySymbol");
        master.name = Some("Master type".into());
        master.family_id = Some(9);
        b.entries = BTreeMap::from([(1, computed), (2, master)]);
        let out = b.finish();
        assert_eq!(out.owners[0].unique_id, "native-1");
        assert_eq!(
            out.owners[0].fields["Name : String"].value,
            json!("Master type")
        );
        assert_eq!(
            out.owners[0].fields["Name : String"].sources[0]["master"]["owner_id"],
            2
        );
    }
    #[test]
    fn missing_reference_or_unqualified_sentinel_is_not_a_default_value() {
        let mut b = Builder::default();
        let mut e = entry(1, "SWall");
        e.root = json!({"m_designOptionId":-4,"m_upToLevelId":999});
        e.type_id = Some(999);
        b.entries.insert(1, e);
        let out = b.finish();
        assert!(!out.owners[0].fields.contains_key("Design Option : String"));
        assert!(
            !out.owners[0]
                .fields
                .contains_key("Unconnected Height : Double")
        );
        assert!(!out.owners[0].fields.contains_key("[Type] Name : String"));
    }
    #[test]
    fn design_option_requires_header_for_nonregular_base_and_prefers_regular_base() {
        let mut b = Builder::default();
        let mut owner = entry(1, "FamilyInstance");
        owner.root = json!({"m_designOptionId":-4});
        b.headers
            .insert(1, (json!({"m_designOptionId":2}), json!({"channel":101})));
        let mut option = entry(2, "DesignOption");
        option.name = Some("Alternate".into());
        b.entries.insert(2, option);
        let mut fields = BTreeMap::new();
        b.project_design_option(&owner, "option", &mut fields);
        assert_eq!(fields["option"].value, json!("Alternate"));
        owner.root = json!({"m_designOptionId":2});
        b.headers
            .insert(1, (json!({"m_designOptionId":-1}), json!({"channel":101})));
        fields.clear();
        b.project_design_option(&owner, "option", &mut fields);
        assert_eq!(fields["option"].value, json!("Alternate"));
        owner.root = json!({"m_designOptionId":-4});
        fields.clear();
        b.project_design_option(&owner, "option", &mut fields);
        assert_eq!(fields["option"].value, json!("None"));
        b.headers.clear();
        fields.clear();
        b.project_design_option(&owner, "option", &mut fields);
        assert!(fields.is_empty());
    }
    #[test]
    fn conflicting_sources_are_withheld() {
        let mut b = Builder::default();
        let mut e = entry(1, "MaterialElem");
        put(
            &mut e.fields,
            "Name : String".into(),
            json!("A"),
            json!({"field":"a"}),
        );
        put(
            &mut e.fields,
            "Name : String".into(),
            json!("B"),
            json!({"field":"b"}),
        );
        b.entries.insert(1, e);
        let out = b.finish();
        assert!(!out.owners[0].fields.contains_key("Name : String"));
        assert_eq!(out.owners[0].diagnostics.len(), 1);
    }
}
