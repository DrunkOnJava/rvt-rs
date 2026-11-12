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
//! And a decode through a `PlanCache` shared by every decode of the file gives
//! exactly the graph, usage and refusal of the same decode that makes its own
//! plans, whatever the lists and the order, and also through a cache whose limit
//! is so small that it empties itself or keeps nothing. What this proves is that
//! a shared cache equals a fresh one. That a decode without a cache equals the
//! one before the cache existed rests on the byte identity of the exports,
//! which Measure shows; this probe does not run the earlier code.
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
//!   plan-cache compared <n> mismatches <n>
//!     (every decode through one cache shared by the whole file and every
//!     list, against the same decode with its own plans)
//!   plan-cache limited compared <n> mismatches <n>
//!     (the same through a cache that holds at most 48 of cost)
//!   plan-cache size peak-entries <n> peak-cost <n> clears <n>
//!     (the most the shared cache held after any decode, across the seven
//!     decodes of each record; a pass shares a cache among at most three)
//!   mismatch <kind> <Class> (the first few)
//!
//! The exit status is 1 where anything above differs.
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
    GraphLimits, GraphUsage, ObjectGraph, PlanCache, decode_graph_cached, decode_graph_projected,
    decode_graph_with_catalog_usage,
};
use rvt::research::native_walls::{WALL_READ_FIELDS, wall_facts};
use rvt::research::schema_registry;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(12 * 60);
const SHOWN: usize = 12;
/// The limit of the cache that is made to empty itself (a plan of a class with
/// no field of its own costs 8).
const LIMITED_COST: usize = 48;

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
    let saved = identifier(&root.fields["m_id"].to_json())
        .ok_or("unsupported serialized ElementId value")?;
    if u64::try_from(saved).ok() != Some(id) {
        return Err(format!(
            "serialized owner identity disagrees with indexed record {id}"
        ));
    }
    definitions
        .ingest_binding_context(graph)
        .map_err(|error| error.to_string())
}

/// Whether two decodes of one body gave the same graph in every field.
fn same_graph(a: &ObjectGraph, b: &ObjectGraph) -> bool {
    a.consumed_bytes == b.consumed_bytes
        && a.objects.len() == b.objects.len()
        && a.objects.iter().zip(&b.objects).all(|(x, y)| {
            x.class_tag == y.class_tag
                && x.class_name == y.class_name
                && x.token == y.token
                && x.start == y.start
                && x.fields_end == y.fields_end
                && x.fields == y.fields
        })
        && a.edges.len() == b.edges.len()
        && a.edges.iter().zip(&b.edges).all(|(x, y)| {
            x.source_object_index == y.source_object_index
                && x.pointer_offset == y.pointer_offset
                && x.pointer_token == y.pointer_token
                && x.target_object_index == y.target_object_index
                && x.target_class_tag == y.target_class_tag
        })
}

type Decoded = Result<(ObjectGraph, GraphUsage)>;

/// Whether two decodes of one body agree: the same refusal, or the same graph
/// and usage.
fn same_outcome(a: &Decoded, b: &Decoded) -> bool {
    match (a, b) {
        (Ok((x, u)), Ok((y, v))) => {
            u.values == v.values && u.objects == v.objects && same_graph(x, y)
        }
        (Err(x), Err(y)) => x.to_string() == y.to_string(),
        _ => false,
    }
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
            a.fields.to_json()
        } else {
            filtered(&a.fields.to_json(), keep)
        };
        if expected != b.fields.to_json() {
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
    let catalog_verdict = global_catalog(&mut rf, &registry);
    println!("global-catalog {catalog_verdict}");
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
    // One cache for every decode of the file, under every list, as a pass
    // shares one: each decode is compared with the same decode that makes its
    // own plans.
    let mut cache = PlanCache::default();
    let mut cached = 0usize;
    let mut cached_wrong = 0usize;
    // And one that empties itself or keeps nothing, so that the bound is
    // exercised on every file.
    let mut limited = PlanCache::with_limit(LIMITED_COST);
    let mut limited_compared = 0usize;
    let mut limited_wrong = 0usize;
    let mut peak_entries = 0usize;
    let mut peak_cost = 0usize;
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
        let full_cached =
            decode_graph_cached(body, &registry, &limits, catalog.as_ref(), None, &mut cache);
        cached += 1;
        cached_wrong += usize::from(!same_outcome(&full, &full_cached));
        let full_limited = decode_graph_cached(
            body,
            &registry,
            &limits,
            catalog.as_ref(),
            None,
            &mut limited,
        );
        limited_compared += 1;
        limited_wrong += usize::from(!same_outcome(&full, &full_limited));
        peak_entries = peak_entries.max(cache.entries());
        peak_cost = peak_cost.max(cache.cost());
        for (index, keep) in KEEPS.iter().enumerate() {
            let projected =
                decode_graph_projected(body, &registry, &limits, catalog.as_ref(), keep);
            decodes += 1;
            per_keep[index].0 += 1;
            let projected_cached = decode_graph_cached(
                body,
                &registry,
                &limits,
                catalog.as_ref(),
                Some(keep),
                &mut cache,
            );
            cached += 1;
            cached_wrong += usize::from(!same_outcome(&projected, &projected_cached));
            let projected_limited = decode_graph_cached(
                body,
                &registry,
                &limits,
                catalog.as_ref(),
                Some(keep),
                &mut limited,
            );
            limited_compared += 1;
            limited_wrong += usize::from(!same_outcome(&projected, &projected_limited));
            peak_entries = peak_entries.max(cache.entries());
            peak_cost = peak_cost.max(cache.cost());
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
    println!("plan-cache compared {cached} mismatches {cached_wrong}");
    println!("plan-cache limited compared {limited_compared} mismatches {limited_wrong}");
    println!(
        "plan-cache size peak-entries {peak_entries} peak-cost {peak_cost} clears {}",
        cache.clears()
    );
    let mut failures = Vec::new();
    if mismatches > 0 {
        failures.push("a projected decode differs from the full one");
    }
    if cached_wrong > 0 {
        failures.push("a decode through the shared cache differs from one with its own plans");
    }
    if limited_wrong > 0 {
        failures.push("a decode through the limited cache differs from one with its own plans");
    }
    if !same || !same_refusals {
        failures.push("the binding context or its refusals differ");
    }
    if !same_walls {
        failures.push("wall_facts differs between the full and the projected graph");
    }
    if !matches!(catalog_verdict.as_str(), "same" | "same-refusal" | "absent") {
        failures.push("the catalog differs between the full and the projected document");
    }
    if !failures.is_empty() {
        anyhow::bail!("{}", failures.join("; "));
    }
    Ok(())
}
