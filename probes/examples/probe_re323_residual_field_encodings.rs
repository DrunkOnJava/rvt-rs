//! RE-323 (#1196): what do the residual `Formats/Latest` field encodings
//! (the fields `FieldType::decode` leaves `Unknown` in every release) and
//! Revit 2027's flagged runtime-state fields hold in the instance data?
//!
//! FACT under test: each residual field has an exact schema descriptor
//! (`schema_registry`, the forward-only grammar) and, where a record holds an
//! instance, a wire layout under which the native graph decoder consumes the
//! record's whole body. A graph that decodes completely, with the field read
//! by that layout, is the byte evidence; a flagged field (descriptor flags
//! with bit 1 set) that the decoder skips, in a graph that still decodes
//! completely, has no instance bytes.
//!
//! Output, one record per line:
//!   file <path> release <layout release> stream-bytes <Formats/Latest bytes>
//!   shape <base> <modifier> flags <flags> count <fields> unknown <fields the
//!     old classifier leaves Unknown> e.g. <Class::field>
//!   old-unknown <Class::field> <the 32 bytes the old classifier reads>
//!   residual <Class::field> tag <t> parent <Parent> desc <descriptor bytes>
//!     base .. mod .. flags .. count .. refs .. window <bytes the old
//!     classifier reads> | prev <field desc> | next <field desc>
//!   flagged <Class> <field>=<flags>/<base>/<mod>,...  (every class with flagged fields)
//!   summary <native totals>; unsupported <class> <records> <diagnostic>
//!   instance <Class::field> instances <n> records <n> kinds <..> samples <..>
//!   flagged-instances <Class::field> flags .. class-instances <n>
//!   flagged-total fields <n> with-instances <n> without <n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re323_residual_field_encodings -- FILE [--records]
use anyhow::Result;
use rvt::RevitFile;
use rvt::formats::FieldType;
use rvt::research::native_document::{self, Options};
use rvt::research::schema_registry::{Class, Field, Registry};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(14 * 60);

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn describe(field: &Field) -> String {
    format!(
        "{} {:02x}/{:02x}/{:04x}/{}",
        field.name,
        field.base,
        field.modifier,
        field.uninterpreted_flags,
        field.array_count.map_or("-".into(), |c| c.to_string())
    )
}

fn window(stream: &[u8], field: &Field) -> Vec<u8> {
    let start = field.descriptor_offset.min(stream.len());
    stream[start..(start + 32).min(stream.len())].to_vec()
}

fn lineage(registry: &Registry, tag: u16) -> Vec<&Class> {
    let mut out = Vec::new();
    let mut current = tag;
    for _ in 0..256 {
        let Some(class) = registry.class(current) else {
            break;
        };
        current = class.parent_reference.tag;
        out.push(class);
        if current < 12 {
            break;
        }
    }
    out
}

struct Shape {
    count: usize,
    unknown: usize,
    example: String,
}

#[derive(Default)]
struct FieldStats {
    descriptor: String,
    instances: usize,
    ok: usize,
    bad: usize,
    bad_samples: Vec<String>,
    zero: usize,
    resolved: usize,
    unresolved: usize,
    targets: BTreeMap<String, usize>,
    ints: Option<(i64, i64)>,
    records: BTreeSet<u64>,
    kinds: BTreeMap<String, usize>,
    samples: BTreeMap<String, usize>,
}

struct Walk<'a> {
    registry: &'a Registry,
    residual: &'a BTreeSet<(String, String)>,
    class_instances: BTreeMap<String, usize>,
    fields: BTreeMap<String, FieldStats>,
    record: u64,
    tokens: BTreeSet<u64>,
}

fn pointer_items(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(items) => items.iter().collect(),
        other => vec![other],
    }
}

fn token_of(item: &Value) -> Option<u64> {
    item.get("pointer_token").and_then(Value::as_u64)
}

fn layout(field: &Field, value: &Value) -> std::result::Result<(), String> {
    match (field.base, field.modifier) {
        (0x0a, 0) => match value.get("class_name") {
            Some(Value::String(_)) => Ok(()),
            _ => Err("the u16 names no class".into()),
        },
        (0x0e, 0x13) | (0x0e, 0x53) | (0x0e, 0x54) => {
            let Value::Array(items) = value else {
                return Err("not an array".into());
            };
            if field.modifier == 0x13 && Some(items.len() as u32) != field.array_count {
                return Err(format!(
                    "{} items for count {:?}",
                    items.len(),
                    field.array_count
                ));
            }
            for item in items {
                let Some(token) = token_of(item) else {
                    return Err("an item is not a pointer".into());
                };
                let tagged = !item.get("class_tag").is_none_or(Value::is_null);
                let wants_tag = field.modifier == 0x54 && token != 0;
                if tagged != wants_tag {
                    return Err(format!("item token {token} class tag present {tagged}"));
                }
            }
            Ok(())
        }
        (0x04, 0) => {
            if value.is_i64() {
                Ok(())
            } else {
                Err("not an integer".into())
            }
        }
        _ => Ok(()),
    }
}

fn kind(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "bool".into(),
        Value::Number(_) => "number".into(),
        Value::String(_) => "string".into(),
        Value::Array(items) => format!("array[{}]", items.len()),
        Value::Object(map) => format!(
            "object{{{}}}",
            map.keys().cloned().collect::<Vec<_>>().join(",")
        ),
    }
}

impl Walk<'_> {
    fn note(&mut self, class: &str, field: &Field, value: &Value) {
        let verdict = layout(field, value);
        let tokens = &self.tokens;
        let stats = self
            .fields
            .entry(format!("{class}::{}", field.name))
            .or_default();
        stats.descriptor = format!(
            "{:02x}/{:02x}/{:04x}/{}",
            field.base,
            field.modifier,
            field.uninterpreted_flags,
            field.array_count.map_or("-".into(), |c| c.to_string())
        );
        stats.instances += 1;
        stats.records.insert(self.record);
        match verdict {
            Ok(()) => stats.ok += 1,
            Err(reason) => {
                stats.bad += 1;
                if stats.bad_samples.len() < 3 {
                    stats.bad_samples.push(reason);
                }
            }
        }
        if field.base == 0x0e {
            for item in pointer_items(value) {
                match token_of(item) {
                    Some(0) => stats.zero += 1,
                    Some(token) if tokens.contains(&token) => stats.resolved += 1,
                    Some(_) => stats.unresolved += 1,
                    None => {}
                }
                if let Some(tag) = item.get("class_tag").and_then(Value::as_u64) {
                    let name = u16::try_from(tag)
                        .ok()
                        .and_then(|tag| self.registry.class(tag))
                        .map_or("?".to_string(), |c| c.name.clone());
                    *stats.targets.entry(name).or_default() += 1;
                }
            }
        }
        if let Some(number) = value.as_i64().filter(|_| field.base == 0x04) {
            stats.ints = Some(match stats.ints {
                Some((low, high)) => (low.min(number), high.max(number)),
                None => (number, number),
            });
        }
        *stats.kinds.entry(kind(value)).or_default() += 1;
        if stats.samples.len() < 10 || stats.samples.contains_key(&value.to_string()) {
            let text: String = value.to_string().chars().take(160).collect();
            *stats.samples.entry(text).or_default() += 1;
        }
    }

    fn class(&mut self, tag: u16, value: &Value, depth: usize) {
        if depth > 64 {
            return;
        }
        let registry = self.registry;
        let chain = lineage(registry, tag);
        let mut names: BTreeMap<&str, usize> = BTreeMap::new();
        for class in &chain {
            for field in &class.fields {
                if field.uninterpreted_flags & 2 == 0 {
                    *names.entry(field.name.as_str()).or_default() += 1;
                }
            }
        }
        for class in chain.iter().rev() {
            *self.class_instances.entry(class.name.clone()).or_default() += 1;
            for field in &class.fields {
                if field.uninterpreted_flags & 2 != 0 {
                    continue;
                }
                let key = if names[field.name.as_str()] > 1 {
                    format!("{}::{}", class.name, field.name)
                } else {
                    field.name.clone()
                };
                let Some(found) = value.get(&key) else {
                    continue;
                };
                if self
                    .residual
                    .contains(&(class.name.clone(), field.name.clone()))
                {
                    self.note(&class.name, field, found);
                }
                self.field(field, found, depth + 1);
            }
        }
    }

    fn field(&mut self, field: &Field, value: &Value, depth: usize) {
        if depth > 96 {
            return;
        }
        match field.modifier {
            0 => match field.base {
                14 => {
                    if let Some(reference) = field.references.first() {
                        self.class(reference.tag, value, depth);
                    }
                }
                13 => {
                    if let Some(nested) = &field.nested_descriptor {
                        self.field(nested, value, depth + 1);
                    }
                }
                _ => {}
            },
            0x10..=0x13 | 0x50..=0x54 => {
                if let Value::Array(items) = value {
                    let mut item = field.clone();
                    item.modifier &= 0xf;
                    item.array_count = None;
                    for found in items {
                        self.field(&item, found, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }
}

fn normalise(text: &str) -> String {
    let mut out = String::new();
    let mut digits = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            if !digits {
                out.push('#');
            }
            digits = true;
        } else {
            digits = false;
            out.push(c);
        }
    }
    out.chars().take(220).collect()
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re323_residual_field_encodings FILE"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    let stream = native_document::read_single(&mut rf, "Formats/Latest")?;
    println!(
        "file {path} release {release} stream-bytes {}",
        stream.len()
    );

    let registry = rvt::research::schema_registry::parse(&stream)?;
    let mut residual: BTreeSet<(String, String)> = BTreeSet::new();
    let mut shapes: BTreeMap<(u8, u8, u16), Shape> = BTreeMap::new();
    let mut flagged: Vec<(String, String)> = Vec::new();
    for class in &registry.classes {
        let mut classes_line = Vec::new();
        for (index, field) in class.fields.iter().enumerate() {
            let decoded = FieldType::decode(&window(&stream, field));
            let unknown = matches!(decoded, FieldType::Unknown { .. });
            let entry = shapes
                .entry((field.base, field.modifier, field.uninterpreted_flags))
                .or_insert_with(|| Shape {
                    count: 0,
                    unknown: 0,
                    example: format!("{}::{}", class.name, field.name),
                });
            entry.count += 1;
            if unknown {
                entry.unknown += 1;
                residual.insert((class.name.clone(), field.name.clone()));
                let parent = registry
                    .class(class.parent_reference.tag)
                    .map_or("-", |p| p.name.as_str());
                let side = |at: Option<&Field>| at.map_or("-".to_string(), describe);
                println!(
                    "residual {}::{} tag {} parent {} desc {} base {:02x} mod {:02x} flags {:04x} count {:?} refs {:?} window {} | prev {} | next {}",
                    class.name,
                    field.name,
                    class.tag,
                    parent,
                    hex(&stream[field.descriptor_offset
                        ..field
                            .end
                            .min(field.descriptor_offset + 24)
                            .min(stream.len())]),
                    field.base,
                    field.modifier,
                    field.uninterpreted_flags,
                    field.array_count,
                    field.references.iter().map(|r| r.tag).collect::<Vec<_>>(),
                    hex(&window(&stream, field)),
                    side(index.checked_sub(1).and_then(|i| class.fields.get(i))),
                    side(class.fields.get(index + 1)),
                );
            }
            if field.uninterpreted_flags != 0 {
                flagged.push((class.name.clone(), field.name.clone()));
                classes_line.push(format!(
                    "{}={:04x}/{:02x}/{:02x}{}",
                    field.name,
                    field.uninterpreted_flags,
                    field.base,
                    field.modifier,
                    if unknown { "/unknown" } else { "" }
                ));
            }
        }
        if !classes_line.is_empty() {
            println!("flagged {} {}", class.name, classes_line.join(","));
        }
    }
    for ((base, modifier, flags), shape) in &shapes {
        if shape.unknown > 0 || *flags != 0 {
            println!(
                "shape {base:02x} {modifier:02x} flags {flags:04x} count {} unknown {} e.g. {}",
                shape.count, shape.unknown, shape.example
            );
        }
    }
    match rf.schema() {
        Ok(table) => {
            for class in &table.classes {
                for field in &class.fields {
                    if let Some(FieldType::Unknown { bytes }) = &field.field_type {
                        println!("old-unknown {}::{} {}", class.name, field.name, hex(bytes));
                        if bytes.len() < 6 {
                            println!(
                                "clipped {}::{} old-bytes {} old-class-offset {}",
                                class.name,
                                field.name,
                                bytes.len(),
                                class.offset
                            );
                            for other in &registry.classes {
                                for found in other.fields.iter().filter(|f| f.name == field.name) {
                                    let full = window(&stream, found);
                                    println!(
                                        "clipped-match {}::{} descriptor-offset {} minus-old-class-offset {} window {} decodes {:?}",
                                        other.name,
                                        found.name,
                                        found.descriptor_offset,
                                        found.descriptor_offset as i64 - class.offset as i64,
                                        hex(&full[..full.len().min(12)]),
                                        FieldType::decode(&full)
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(error) => println!("old schema: {error}"),
    }

    let mut walk = Walk {
        registry: &registry,
        residual: &residual,
        class_instances: BTreeMap::new(),
        fields: BTreeMap::new(),
        record: 0,
        tokens: BTreeSet::new(),
    };
    let started = Instant::now();
    let mut seen = 0usize;
    let mut failures: BTreeMap<(String, String), usize> = BTreeMap::new();
    let options = Options {
        releases: native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    let result = native_document::extract_graphs(&mut rf, &options, |record| {
        seen += 1;
        if started.elapsed() > BUDGET {
            anyhow::bail!("time budget after {seen} records");
        }
        if let Some(graph) = &record.graph {
            walk.record = record.identity.element_id;
            walk.tokens = graph.objects.iter().map(|o| u64::from(o.token)).collect();
            for object in &graph.objects {
                walk.class(object.class_tag, &object.fields, 0);
            }
        } else if let Some(diagnostic) = &record.diagnostic {
            *failures
                .entry((
                    record.class_name.clone().unwrap_or_else(|| "-".into()),
                    normalise(diagnostic),
                ))
                .or_default() += 1;
        }
        Ok(())
    });
    match &result {
        Ok(summary) => {
            println!(
                "summary release {} indexed {} emitted {} complete {} unsupported {} graveyard {}",
                summary.revit_version,
                summary.indexed_elements,
                summary.emitted_records,
                summary.complete_graph_records,
                summary.unsupported_graph_records,
                summary.graveyard_records,
            );
        }
        Err(error) => println!("native: {error:#} (after {seen} records)"),
    }
    let mut ranked: Vec<_> = failures.iter().collect();
    ranked.sort_by_key(|entry| std::cmp::Reverse(*entry.1));
    for ((class, diagnostic), count) in ranked.into_iter().take(40) {
        println!("unsupported {class} {count} {diagnostic}");
    }
    for (name, stats) in &walk.fields {
        let kinds: Vec<_> = stats
            .kinds
            .iter()
            .map(|(k, n)| format!("{k}x{n}"))
            .collect();
        let samples: Vec<_> = stats
            .samples
            .iter()
            .map(|(k, n)| format!("{k}x{n}"))
            .collect();
        let targets: Vec<_> = stats
            .targets
            .iter()
            .map(|(k, n)| format!("{k}x{n}"))
            .collect();
        println!(
            "instance {name} desc {} instances {} records {} layout-ok {} layout-bad {} {:?} pointers zero {} resolved {} unresolved {} targets {} ints {:?} kinds {} samples {}",
            stats.descriptor,
            stats.instances,
            stats.records.len(),
            stats.ok,
            stats.bad,
            stats.bad_samples,
            stats.zero,
            stats.resolved,
            stats.unresolved,
            targets.join(","),
            stats.ints,
            kinds.join(";"),
            samples.join(" ; ")
        );
    }
    let mut with = 0usize;
    for (class, field) in &flagged {
        let Some(declared) = registry
            .named(class)
            .and_then(|c| c.fields.iter().find(|f| &f.name == field))
        else {
            continue;
        };
        if declared.uninterpreted_flags & 2 == 0 {
            continue;
        }
        let instances = walk.class_instances.get(class).copied().unwrap_or(0);
        if instances > 0 {
            with += 1;
            println!(
                "flagged-instances {class}::{field} flags {:04x} base {:02x} mod {:02x} class-instances {instances}",
                declared.uninterpreted_flags, declared.base, declared.modifier
            );
        }
    }
    let total = flagged
        .iter()
        .filter(|(class, field)| {
            registry
                .named(class)
                .and_then(|c| c.fields.iter().find(|f| &f.name == field))
                .is_some_and(|f| f.uninterpreted_flags & 2 != 0)
        })
        .count();
    println!(
        "flagged-total fields {total} with-instances {with} without {}",
        total - with
    );
    Ok(())
}
