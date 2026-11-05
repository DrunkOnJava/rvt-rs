//! #1335: a projected graph decode is the full decode with fewer values built.
//!
//! FACT under test: `native_parameters::decode_graph_projected(.., keep)`
//! succeeds exactly where `decode_graph_with_catalog_usage` does and fails
//! with the same error where that fails; where it succeeds the graph is the
//! full one's in everything but the values of the fields not in `keep`: the
//! same objects (class, token, start, end of fields), edges, consumed bytes and
//! usage, and each object's fields exactly the full object's entries whose key
//! is in `keep`. And each reader that is given a projected graph reads the same
//! from it as from the full one: the definitions pass's
//! `Registry::ingest_binding_context` (the `Family` and `FamilySymbol` lists),
//! the wall read's `wall_facts` (`WALL_READ_FIELDS`) and the catalog's
//! `from_graph` (`CATALOG_FIELDS`). The lists are the readers' own constants,
//! imported, not copied.
//!
//! Every record of the file's native partitions is decoded in full and
//! projected under each of the readers' lists, whatever its class, and under
//! the identity alone and the empty list (every field left out). The graph the
//! catalog is read from, the `Global/Latest` document, is read both ways too.
//!
//! Output, one line each:
//!   file <path> release <layout release>
//!   global-catalog <same | same-refusal | absent | what differs>
//!   compared records <n> decodes <n> full-ok <n> full-refused <n> mismatches <n>
//!     budget-stopped <bool>
//!   keep <list> compared <n> mismatches <n>
//!   refused <Class> <n>                 (a full decode that failed, by class)
//!   binding-context family-categories <n> symbol-families <n> same <bool>
//!     ingest-refusals <n> same-refusals <bool>
//!   wall-facts compared <n> same <bool>
//!   mismatch <kind> <Class> (the first few)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1335_projected_graphs -- FILE
use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_es_catalog::{self, CATALOG_FIELDS};
use rvt::research::native_parameter_definitions::{
    FAMILY_BINDING_FIELDS, Registry as Definitions, SYMBOL_BINDING_FIELDS,
};
use rvt::research::native_parameters::{
    GraphLimits, ObjectGraph, decode_graph_projected, decode_graph_with_catalog_usage,
};
use rvt::research::native_walls::{WALL_READ_FIELDS, wall_facts};
use rvt::research::schema_registry;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(12 * 60);
const SHOWN: usize = 12;

/// The readers' lists, then two that stress the mechanism: the identity alone
/// and no field at all.
const KEEPS: [&[&str]; 6] = [
    FAMILY_BINDING_FIELDS,
    SYMBOL_BINDING_FIELDS,
    WALL_READ_FIELDS,
    CATALOG_FIELDS,
    &["m_id"],
    &[],
];

fn filtered(fields: &Value, keep: &[&str]) -> Value {
    match fields {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| keep.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn identifier(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

/// What the definitions pass does with a Family or FamilySymbol graph: the
/// identity check of `extract_records`, then `ingest_binding_context`.
fn ingest(definitions: &mut Definitions, graph: &ObjectGraph, id: u64) -> Result<(), String> {
    let root = graph.objects.first().ok_or("empty owner graph")?;
    let saved = identifier(&root.fields["m_id"]).ok_or("unsupported serialized ElementId value")?;
    if u64::try_from(saved).ok() != Some(id) {
        return Err(format!(
            "serialized owner identity disagrees with indexed record {id}"
        ));
    }
    definitions
        .ingest_binding_context(graph)
        .map_err(|error| error.to_string())
}

fn differences(full: &ObjectGraph, projected: &ObjectGraph, keep: &[&str]) -> Vec<&'static str> {
    let mut found = Vec::new();
    if full.consumed_bytes != projected.consumed_bytes {
        found.push("consumed");
    }
    if full.objects.len() != projected.objects.len() {
        found.push("object-count");
        return found;
    }
    for (a, b) in full.objects.iter().zip(&projected.objects) {
        if a.class_tag != b.class_tag
            || a.class_name != b.class_name
            || a.token != b.token
            || a.start != b.start
            || a.fields_end != b.fields_end
        {
            found.push("object");
        }
        let expected = if a.class_name == "ExtensibleStorageEntity" {
            a.fields.clone()
        } else {
            filtered(&a.fields, keep)
        };
        if expected != b.fields {
            found.push("fields");
        }
    }
    let same_edges = full.edges.len() == projected.edges.len()
        && full.edges.iter().zip(&projected.edges).all(|(a, b)| {
            a.source_object_index == b.source_object_index
                && a.pointer_offset == b.pointer_offset
                && a.pointer_token == b.pointer_token
                && a.target_object_index == b.target_object_index
                && a.target_class_tag == b.target_class_tag
        });
    if !same_edges {
        found.push("edges");
    }
    found
}

/// The catalog `from_graph` reads from the `Global/Latest` document decoded in
/// full and decoded under `CATALOG_FIELDS`.
fn global_catalog(rf: &mut RevitFile, registry: &schema_registry::Registry) -> String {
    let Ok(bytes) = native_document::read_single(rf, "Global/Latest") else {
        return "absent".into();
    };
    let Ok(document) = native_es_catalog::document(&bytes) else {
        return "absent".into();
    };
    let limits = native_es_catalog::document_limits();
    let full = decode_graph_with_catalog_usage(document, registry, &limits, None);
    let projected = decode_graph_projected(document, registry, &limits, None, CATALOG_FIELDS);
    match (&full, &projected) {
        (Ok((a, usage)), Ok((b, other))) => {
            if usage.values != other.values || usage.objects != other.objects {
                return "usage".into();
            }
            if let Some(kind) = differences(a, b, CATALOG_FIELDS).first() {
                return format!("graph-{kind}");
            }
            match (
                native_es_catalog::from_graph(a, &bytes),
                native_es_catalog::from_graph(b, &bytes),
            ) {
                (Ok((c, v)), Ok((d, w))) => {
                    let same = serde_json::to_value(&c).ok() == serde_json::to_value(&d).ok();
                    if same && v == w {
                        "same".into()
                    } else {
                        "catalog".into()
                    }
                }
                (Err(x), Err(y)) if x.to_string() == y.to_string() => "same-refusal".into(),
                _ => "catalog-refusal".into(),
            }
        }
        (Err(a), Err(b)) if a.to_string() == b.to_string() => "same-refusal".into(),
        (Err(_), Err(_)) => "error".into(),
        _ => "accepted-differently".into(),
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1335_projected_graphs FILE"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");
    if !native_document::NATIVE_RELEASES.contains(&release) {
        println!("native: release {release} is outside the native readers' range");
        return Ok(());
    }
    let formats = native_document::read_single(&mut rf, "Formats/Latest")?;
    let registry = schema_registry::parse(&formats)?;
    println!("global-catalog {}", global_catalog(&mut rf, &registry));
    let catalog = native_document::read_single(&mut rf, "Global/Latest")
        .and_then(|bytes| native_es_catalog::decode(&bytes, &registry))
        .ok()
        .map(|(catalog, _)| catalog);
    let limits = GraphLimits {
        max_values: 100_000,
        max_objects: 100_000,
        ..GraphLimits::default()
    };
    // Every record is handed over with its body and no graph: this probe
    // decodes them itself.
    let options = Options {
        releases: native_document::NATIVE_RELEASES,
        graph_classes: Some(BTreeSet::new()),
        keep_body: true,
        ..Options::default()
    };
    let started = Instant::now();
    let mut records = 0usize;
    let mut decodes = 0usize;
    let mut full_ok = 0usize;
    let mut full_refused = 0usize;
    let mut mismatches = 0usize;
    let mut stopped = false;
    let mut per_keep = [(0usize, 0usize); KEEPS.len()];
    let mut refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut full_definitions = Definitions::default();
    let mut projected_definitions = Definitions::default();
    let mut ingest_refusals = 0usize;
    let mut same_refusals = true;
    let mut walls = 0usize;
    let mut same_walls = true;
    let mut shown = 0usize;
    let result = native_document::extract_graphs(&mut rf, &options, |record| {
        if started.elapsed() > BUDGET {
            stopped = true;
            anyhow::bail!("time budget after {records} records");
        }
        let Some(body) = &record.body else {
            return Ok(());
        };
        records += 1;
        let class = record.class_name.clone().unwrap_or_default();
        let full = decode_graph_with_catalog_usage(body, &registry, &limits, catalog.as_ref());
        match &full {
            Ok(_) => full_ok += 1,
            Err(_) => {
                full_refused += 1;
                *refused.entry(class.clone()).or_default() += 1;
            }
        }
        for (index, keep) in KEEPS.iter().enumerate() {
            let projected =
                decode_graph_projected(body, &registry, &limits, catalog.as_ref(), keep);
            decodes += 1;
            per_keep[index].0 += 1;
            let kind = match (&full, &projected) {
                (Ok((graph, usage)), Ok((other, other_usage))) => {
                    if usage.values != other_usage.values || usage.objects != other_usage.objects {
                        Some("usage".to_string())
                    } else {
                        differences(graph, other, keep)
                            .first()
                            .map(|k| k.to_string())
                    }
                }
                (Err(a), Err(b)) => (a.to_string() != b.to_string()).then(|| "error".to_string()),
                (Ok(_), Err(_)) => Some("projected-refused".to_string()),
                (Err(_), Ok(_)) => Some("projected-accepted".to_string()),
            };
            if let Some(kind) = kind {
                mismatches += 1;
                per_keep[index].1 += 1;
                if shown < SHOWN {
                    shown += 1;
                    println!("mismatch {kind} {class}");
                }
            }
        }
        // The readers, on the graph they are given and on the full one.
        let id = record.identity.element_id;
        if matches!(class.as_str(), "Family" | "FamilySymbol") {
            let keep = if class == "Family" {
                FAMILY_BINDING_FIELDS
            } else {
                SYMBOL_BINDING_FIELDS
            };
            let projected =
                decode_graph_projected(body, &registry, &limits, catalog.as_ref(), keep);
            let a = match &full {
                Ok((graph, _)) => ingest(&mut full_definitions, graph, id),
                Err(error) => Err(error.to_string()),
            };
            let b = match &projected {
                Ok((graph, _)) => ingest(&mut projected_definitions, graph, id),
                Err(error) => Err(error.to_string()),
            };
            ingest_refusals += usize::from(a.is_err());
            same_refusals &= a == b;
        }
        if matches!(class.as_str(), "SWall" | "ArcWall") {
            let projected = decode_graph_projected(
                body,
                &registry,
                &limits,
                catalog.as_ref(),
                WALL_READ_FIELDS,
            );
            if let (Ok((graph, _)), Ok((other, _))) = (&full, &projected) {
                walls += 1;
                same_walls &= wall_facts(graph) == wall_facts(other);
            }
        }
        Ok(())
    });
    if let Err(error) = &result {
        if !stopped {
            println!("native: {error:#}");
        }
    }
    println!(
        "compared records {records} decodes {decodes} full-ok {full_ok} full-refused {full_refused} mismatches {mismatches} budget-stopped {stopped}"
    );
    for (keep, (compared, wrong)) in KEEPS.iter().zip(per_keep) {
        println!(
            "keep {} compared {compared} mismatches {wrong}",
            keep.join(",")
        );
    }
    for (class, n) in &refused {
        println!("refused {class} {n}");
    }
    let same = full_definitions.family_categories == projected_definitions.family_categories
        && full_definitions.symbol_families == projected_definitions.symbol_families;
    println!(
        "binding-context family-categories {} symbol-families {} same {same} ingest-refusals {ingest_refusals} same-refusals {same_refusals}",
        full_definitions.family_categories.len(),
        full_definitions.symbol_families.len(),
    );
    println!("wall-facts compared {walls} same {same_walls}");
    Ok(())
}
