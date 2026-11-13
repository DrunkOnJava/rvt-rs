//! RE-460 (#1191): what a `HermiteSurf` record holds in the saved graphics of the
//! elements that keep their record box, and whether it has the layout a second reader
//! of the format gives it.
//!
//! FACT under test (mahach666's RvtIO, Discussion #1241, 2026-11-13): `HermiteSurf`
//! derives from `Surface` and holds `m_periodic`, `m_constructedOK`, a node array of
//! `SplineSrfNode` (`m_iPoint`, `m_iTangent[2]`, `m_iMixedDer`: point, both tangents
//! and the mixed derivative, 12 f64 a node) and the parameter arrays `m_uParams` and
//! `m_vParams`, with `n = nu * nv` nodes and node `(iu, iv)` at `iv * nu + iu`. It is
//! the one surface class no reader of ours decodes (RE-329: `unsupported surface
//! HermiteSurf`, 512 faces on the BIM4LCA office as built).
//!
//! The candidate set is `probe_re329_saved_graphics_census`'s: a building element
//! exported with the record box as its body and not profile-resolved, whose Tag no
//! other entity carries, and the symbols its instances name. This prints, over every
//! `HermiteSurf` object of their graphs, counts only (no id, name or coordinate):
//!
//!   release <year>
//!   census hermite_objects <n> in <m> records
//!   census hermite_first <the shape of the first object: field names, and for an array
//!     its first item's shape and its length>
//!   census hermite_shape <n> <the shape with every array length replaced by n>
//!   census hermite_layout <n> <product_matches=<bool> u_increasing=<bool>
//!     v_increasing=<bool> periodic=<the flags>>
//!   census hermite_sizes nu <min>..<max> nv <min>..<max> nodes <min>..<max>
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re460_hermite_surf -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, Record, extract_graphs};

const RECORD_BOX: &str = "partition_element_record_bbox";
const NODE_FIELDS: [&str; 3] = ["m_NodeArray", "m_nodeArray", "m_nodes"];

fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    for key in ["m_id", "m_id64"] {
        if let Some(inner) = value.get(key) {
            return identifier(inner);
        }
    }
    None
}

/// The shape of a value: field names, and an array's first item and its length (or `n`).
fn shape(value: &Fv, depth: usize, lengths: bool) -> String {
    if let Some(items) = value.as_array() {
        let first = items
            .first()
            .map_or("empty".to_string(), |item| shape(item, depth + 1, lengths));
        let length = if lengths {
            items.len().to_string()
        } else {
            "n".to_string()
        };
        return format!("[{first}; {length}]");
    }
    if let Some(object) = value.as_object() {
        if depth > 5 {
            return "{..}".to_string();
        }
        let fields: Vec<String> = object
            .iter()
            .map(|(name, inner)| format!("{name}: {}", shape(inner, depth + 1, lengths)))
            .collect();
        return format!("{{{}}}", fields.join(", "));
    }
    if value.is_boolean() {
        "bool".to_string()
    } else if value.is_f64() {
        "f64".to_string()
    } else if value.is_i64() || value.is_u64() {
        "int".to_string()
    } else if value.is_string() {
        "str".to_string()
    } else if value.is_null() {
        "null".to_string()
    } else {
        "other".to_string()
    }
}

fn increasing(value: Option<&Fv>) -> Option<bool> {
    let items = value?.as_array()?;
    let numbers: Option<Vec<f64>> = items.iter().map(Fv::as_f64).collect();
    let numbers = numbers?;
    Some(numbers.windows(2).all(|pair| pair[0] < pair[1]))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    println!("release {}", rf.basic_file_info()?.version);
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    let mut by_id: BTreeMap<u64, usize> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            location_feet: Some(_),
            ..
        } = entity
        else {
            continue;
        };
        let boxed = set.properties.iter().any(|p| {
            p.name == "BodySource" && matches!(&p.value, PropertyValue::Text(t) if t == RECORD_BOX)
        });
        let resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        if !boxed || resolved {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            *by_id.entry(id).or_default() += 1;
        }
    }
    let mut options = Options {
        selected_ids: by_id
            .iter()
            .filter(|(_, count)| **count == 1)
            .map(|(id, _)| *id)
            .collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let mut records: BTreeMap<u64, Record> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |r| {
        records.insert(r.identity.element_id, r);
        Ok(())
    })?;
    let mut attempted: BTreeSet<u64> = records.keys().copied().collect();
    for _ in 0..128 {
        let mut wanted = BTreeSet::new();
        for r in records.values() {
            let Some(g) = &r.graph else { continue };
            for o in &g.objects {
                if o.class_name != "InstanceInfo" {
                    continue;
                }
                if let Some(id) = identifier(&o.fields["m_symbolId"])
                    .and_then(|id| u64::try_from(id).ok())
                    .filter(|id| *id > 0)
                {
                    if !attempted.contains(&id) {
                        wanted.insert(id);
                    }
                }
            }
        }
        if wanted.is_empty() {
            break;
        }
        attempted.extend(&wanted);
        options.selected_ids = wanted;
        extract_graphs(&mut rf, &options, |r| {
            records.insert(r.identity.element_id, r);
            Ok(())
        })?;
    }

    let mut total = 0usize;
    let mut holders = BTreeSet::new();
    let mut first: Option<String> = None;
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    let mut layouts: BTreeMap<String, usize> = BTreeMap::new();
    let (mut nu_range, mut nv_range, mut node_range) =
        ((usize::MAX, 0), (usize::MAX, 0), (usize::MAX, 0));
    for (id, record) in &records {
        let Some(graph) = &record.graph else { continue };
        for object in &graph.objects {
            if object.class_name != "HermiteSurf" {
                continue;
            }
            total += 1;
            holders.insert(*id);
            let fields = &object.fields;
            first.get_or_insert_with(|| shape(fields, 0, true));
            *shapes.entry(shape(fields, 0, false)).or_default() += 1;
            let nodes = NODE_FIELDS
                .iter()
                .find_map(|name| fields.get(*name))
                .and_then(Fv::as_array)
                .map(<[Fv]>::len);
            let u_params = fields
                .get("m_uParams")
                .and_then(Fv::as_array)
                .map(<[Fv]>::len);
            let v_params = fields
                .get("m_vParams")
                .and_then(Fv::as_array)
                .map(<[Fv]>::len);
            let matches = match (nodes, u_params, v_params) {
                (Some(n), Some(nu), Some(nv)) => (n == nu * nv).to_string(),
                _ => "unread".to_string(),
            };
            if let (Some(n), Some(nu), Some(nv)) = (nodes, u_params, v_params) {
                nu_range = (nu_range.0.min(nu), nu_range.1.max(nu));
                nv_range = (nv_range.0.min(nv), nv_range.1.max(nv));
                node_range = (node_range.0.min(n), node_range.1.max(n));
            }
            let periodic = fields.get("m_periodic").and_then(Fv::as_array).map_or(
                "unread".to_string(),
                |flags| {
                    flags
                        .iter()
                        .map(|flag| flag.as_bool().map_or("?".to_string(), |b| b.to_string()))
                        .collect::<Vec<_>>()
                        .join(",")
                },
            );
            let label = format!(
                "product_matches={matches} u_increasing={:?} v_increasing={:?} periodic={periodic}",
                increasing(fields.get("m_uParams")),
                increasing(fields.get("m_vParams")),
            );
            *layouts.entry(label).or_default() += 1;
        }
    }
    println!(
        "census hermite_objects {total} in {} records",
        holders.len()
    );
    if let Some(first) = first {
        println!("census hermite_first {first}");
    }
    for (label, n) in &shapes {
        println!("census hermite_shape {n} {label}");
    }
    for (label, n) in &layouts {
        println!("census hermite_layout {n} {label}");
    }
    if total > 0 && nu_range.0 != usize::MAX {
        println!(
            "census hermite_sizes nu {}..{} nv {}..{} nodes {}..{}",
            nu_range.0, nu_range.1, nv_range.0, nv_range.1, node_range.0, node_range.1
        );
    }
    Ok(())
}
