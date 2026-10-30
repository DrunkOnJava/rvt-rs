//! #852 (probe): what the schema path's typed decoders get on real files,
//! and every production row the schema path yields.
//!
//! `elements::decode_instance_prefer_typed_with_limits` sends every
//! `Global/Latest` candidate of a class in `elements::MVP_TYPED_CLASSES`
//! to that class's registered decoder. For each of those classes this
//! prints the fields the file's schema declares, the candidates
//! `walker::scan_candidates` finds at `PRODUCTION_ELEMENT_MIN_SCORE`, and
//! each candidate decoded directly by its typed decoder: its offset,
//! score, each field's decoded kind (`Bytes` where the walk could not read
//! the field as its schema type) and whether production accepts it.
//!
//! It then counts every row `walker::iter_elements` yields from the
//! schema path (`record_kind` `schema_instance`, typed or generic) by
//! class, with and without an ElementId, and every row's `source_stream`,
//! so a row is counted whatever stream name it carries.
//!
//! It exits 1 when any file could not be read.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_typed_schema_rows -- FILE.rvt [...]

use rvt::walker::{self, HandleIndex, InstanceField, WalkerLimits};
use rvt::{RevitFile, compression, elements, formats, streams};
use std::collections::BTreeMap;

fn kind(value: &InstanceField) -> String {
    match value {
        InstanceField::Pointer { raw } => format!("Pointer({:x},{:x})", raw[0], raw[1]),
        InstanceField::ElementId { tag, id } => format!("ElementId({tag}:{id})"),
        InstanceField::RefContainer { col_a, .. } => format!("RefContainer[{}]", col_a.len()),
        InstanceField::Integer { value, size, .. } => format!("Int{}({value})", size * 8),
        InstanceField::Float { value, size } => format!("F{}({value})", size * 8),
        InstanceField::Bool(b) => format!("Bool({b})"),
        InstanceField::Guid(_) => "Guid".into(),
        InstanceField::String(s) => {
            format!("String({:?})", s.chars().take(40).collect::<String>())
        }
        InstanceField::Vector(v) => format!("Vector[{}]", v.len()),
        InstanceField::Bytes(b) => format!("Bytes[{}]", b.len()),
        _ => "Other".into(),
    }
}

fn probe(path: &str) -> rvt::Result<()> {
    let mut rf = RevitFile::open(path)?;
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_stream_at(streams::FORMATS_LATEST, &formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let (_, data) = compression::inflate_stream_auto(streams::GLOBAL_LATEST, &raw)?;

    for class in elements::MVP_TYPED_CLASSES {
        match schema.classes.iter().find(|c| c.name == *class) {
            Some(c) => {
                let fields: Vec<String> = c
                    .fields
                    .iter()
                    .map(|f| format!("{}:{:?}", f.name, f.field_type))
                    .collect();
                println!(
                    "schema {class} parent={:?} fields={} [{}]",
                    c.parent,
                    c.fields.len(),
                    fields.join(" | ")
                );
            }
            None => println!("schema {class} absent"),
        }
    }

    let limits = WalkerLimits::default();
    let candidates = walker::scan_candidates(&schema, &data, walker::PRODUCTION_ELEMENT_MIN_SCORE);
    let mut candidates_by_class: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for c in &candidates {
        let Some(class) = elements::MVP_TYPED_CLASSES
            .iter()
            .find(|k| **k == c.class_name)
        else {
            continue;
        };
        let Some(cls) = schema.classes.iter().find(|k| k.name == c.class_name) else {
            continue;
        };
        let accepted =
            elements::decode_instance_prefer_typed_with_limits(&data, c.offset, cls, limits)
                .is_some();
        let entry = candidates_by_class.entry(*class).or_default();
        entry.0 += 1;
        entry.1 += usize::from(accepted);
        let decoded = data.get(c.offset..).and_then(|slice| {
            elements::decode_typed_with_limits(slice, cls, &HandleIndex::new(), limits).ok()
        });
        let fields: Vec<String> = decoded
            .iter()
            .flat_map(|d| d.fields.iter())
            .map(|(name, value)| format!("{name}={}", kind(value)))
            .collect();
        println!(
            "candidate {} offset=0x{:x} score={} accepted={accepted} decoded={} [{}]",
            c.class_name,
            c.offset,
            c.score,
            decoded.is_some(),
            fields.join(" | "),
        );
    }
    for class in elements::MVP_TYPED_CLASSES {
        let (found, accepted) = candidates_by_class.get(class).copied().unwrap_or_default();
        println!("candidates {class} {found} accepted={accepted}");
    }

    // Every production row, typed or generic, by where it says it is from.
    let mut streams_seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut schema_rows: BTreeMap<String, (usize, usize, String)> = BTreeMap::new();
    for e in walker::iter_elements(&mut rf)? {
        let stream = e.provenance.source_stream.clone().unwrap_or_default();
        *streams_seen.entry(stream.clone()).or_default() += 1;
        if e.provenance.record_kind.as_deref() != Some("schema_instance") {
            continue;
        }
        let entry = schema_rows.entry(e.class.clone()).or_default();
        entry.0 += 1;
        entry.1 += usize::from(e.id.is_some());
        entry.2 = format!(
            "{}@{stream}",
            e.provenance.decoder.as_deref().unwrap_or("-")
        );
    }
    for (stream, rows) in &streams_seen {
        println!("source_stream {stream:?} rows={rows}");
    }
    for (class, (rows, with_id, decoder)) in &schema_rows {
        println!("schema_row {class} rows={rows} with_id={with_id} decoder={decoder}");
    }
    println!(
        "schema_rows total={} without_id={}",
        schema_rows.values().map(|r| r.0).sum::<usize>(),
        schema_rows.values().map(|r| r.0 - r.1).sum::<usize>()
    );
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    if paths.is_empty() {
        eprintln!("usage: probe_typed_schema_rows FILE.rvt [...]");
        std::process::exit(2);
    }
    let mut failed = false;
    for path in &paths {
        println!("file {}", path.rsplit('/').next().unwrap_or(path));
        if let Err(e) = probe(path) {
            println!("error {e}");
            failed = true;
        }
    }
    if failed {
        std::process::exit(1);
    }
}
